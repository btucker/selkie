use roxmltree::Document;
use selkie::render::svg::SvgStructure;
use selkie::{parse, render};

fn render_architecture_svg(input: &str) -> String {
    let diagram = parse(input).expect("Failed to parse architecture diagram");
    render(&diagram).expect("Failed to render architecture diagram")
}

fn svg_structure(svg: &str) -> SvgStructure {
    SvgStructure::from_svg(svg).expect("Failed to parse SVG structure")
}

fn parse_svg(svg: &str) -> Document<'_> {
    Document::parse(svg).expect("Failed to parse SVG")
}

fn has_id(doc: &Document<'_>, id: &str) -> bool {
    doc.descendants()
        .any(|node| node.attribute("id") == Some(id))
}

fn count_elements_with_class(doc: &Document<'_>, class_name: &str) -> usize {
    doc.descendants()
        .filter(|node| {
            node.attribute("class")
                .map(|class| class.split_whitespace().any(|c| c == class_name))
                .unwrap_or(false)
        })
        .count()
}

fn assert_labels(structure: &SvgStructure, expected: &[&str]) {
    for label in expected {
        assert!(
            structure.labels.contains(&label.to_string()),
            "Missing label '{}', got: {:?}",
            label,
            structure.labels
        );
    }
}

#[test]
fn test_architecture_simple_with_groups() {
    let input = r#"architecture-beta
        group api(cloud)[API]

        service db(database)[Database] in api
        service disk1(disk)[Storage] in api
        service disk2(disk)[Storage] in api
        service server(server)[Server] in api
        service gateway(internet)[Gateway]

        db:L -- R:server
        disk1:T -- B:server
        disk2:T -- B:db
        server:T -- B:gateway
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);
    let doc = parse_svg(&svg);

    assert_eq!(structure.node_count, 5);
    assert_eq!(structure.edge_count, 4);
    assert_labels(
        &structure,
        &["API", "Database", "Storage", "Server", "Gateway"],
    );
    assert!(has_id(&doc, "group-api"));
}

#[test]
fn test_architecture_ids() {
    let input = r#"architecture-beta
        group api(cloud)[API]

        service db(database)[Database] in api
        service disk1(disk)[Storage] in api
        service disk2(disk)[Storage] in api
        service server(server)[Server] in api

        db:L -- R:server
        disk1:T -- B:server
        disk2:T -- B:db
    "#;

    let svg = render_architecture_svg(input);
    let doc = parse_svg(&svg);

    assert!(has_id(&doc, "group-api"));
    for id in [
        "service-db",
        "service-disk1",
        "service-disk2",
        "service-server",
    ] {
        assert!(has_id(&doc, id));
    }

    let mut edge_ids: Vec<String> = doc
        .descendants()
        .filter(|node| {
            node.attribute("class")
                .map(|class| class.split_whitespace().any(|c| c == "edge"))
                .unwrap_or(false)
        })
        .filter_map(|node| node.attribute("id").map(|id| id.to_string()))
        .collect();
    edge_ids.sort();
    assert_eq!(
        edge_ids,
        vec![
            "L_db_server_0".to_string(),
            "L_disk1_server_0".to_string(),
            "L_disk2_db_0".to_string(),
        ]
    );
}

#[test]
fn test_architecture_title_and_accessibility() {
    let input = r#"architecture-beta
        title Simple Architecture Diagram
        accTitle: Accessibility Title
        accDescr: Accessibility Description
        group api(cloud)[API]

        service db(database)[Database] in api
        service disk1(disk)[Storage] in api
        service disk2(disk)[Storage] in api
        service server(server)[Server] in api

        db:L -- R:server
        disk1:T -- B:server
        disk2:T -- B:db
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);

    assert_eq!(structure.node_count, 4);
    assert_eq!(structure.edge_count, 3);
    assert_labels(&structure, &["Database", "Storage", "Server"]);
}

#[test]
fn test_architecture_groups_within_groups() {
    let input = r#"architecture-beta
        group api[API]
        group public[Public API] in api
        group private[Private API] in api

        service serv1(server)[Server] in public
        service serv2(server)[Server] in private
        service db(database)[Database] in private

        service gateway(internet)[Gateway] in api

        serv1:B -- T:serv2
        serv2:L -- R:db
        serv1:L -- R:gateway
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);
    let doc = parse_svg(&svg);

    assert_eq!(structure.node_count, 4);
    assert_eq!(structure.edge_count, 3);
    assert_labels(&structure, &["API", "Public API", "Private API", "Gateway"]);
    assert!(has_id(&doc, "group-api"));
    assert!(has_id(&doc, "group-public"));
    assert!(has_id(&doc, "group-private"));
}

#[test]
fn test_architecture_fallback_icon() {
    let input = r#"architecture-beta
        service unknown(iconnamedoesntexist)[Unknown Icon]
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);

    assert_eq!(structure.node_count, 1);
    assert_eq!(structure.edge_count, 0);
    assert_labels(&structure, &["Unknown Icon"]);
    assert!(
        svg.contains(">?</tspan>"),
        "Expected fallback icon to include '?'"
    );
}

#[test]
fn test_architecture_split_directioning() {
    let input = r#"architecture-beta
        service db(database)[Database]
        service s3(disk)[Storage]
        service serv1(server)[Server 1]
        service serv2(server)[Server 2]
        service disk(disk)[Disk]

        db:L -- R:s3
        serv1:L -- T:s3
        serv2:L -- B:s3
        serv1:T -- B:disk
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);

    assert_eq!(structure.node_count, 5);
    assert_eq!(structure.edge_count, 4);
}

#[test]
fn test_architecture_directional_arrows() {
    let input = r#"architecture-beta
        service servC(server)[Server 1]
        service servL(server)[Server 2]
        service servR(server)[Server 3]
        service servT(server)[Server 4]
        service servB(server)[Server 5]

        servC:L --> R:servL
        servC:R <-- L:servR
        servC:T --> B:servT
        servC:B <-- T:servB

        servL:T <-- L:servT
        servL:B <-- L:servB
        servR:T --> R:servT
        servR:B --> R:servB
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);
    let doc = parse_svg(&svg);

    assert_eq!(structure.node_count, 5);
    assert_eq!(structure.edge_count, 8);
    assert_eq!(count_elements_with_class(&doc, "arrow"), 8);
}

#[test]
fn test_architecture_group_edges() {
    let input = r#"architecture-beta
        group left_group(cloud)[Left]
        group right_group(cloud)[Right]
        group top_group(cloud)[Top]
        group bottom_group(cloud)[Bottom]
        group center_group(cloud)[Center]

        service left_disk(disk)[Disk] in left_group
        service right_disk(disk)[Disk] in right_group
        service top_disk(disk)[Disk] in top_group
        service bottom_disk(disk)[Disk] in bottom_group
        service center_disk(disk)[Disk] in center_group

        left_disk{group}:R -- L:center_disk{group}
        right_disk{group}:L -- R:center_disk{group}
        top_disk{group}:B -- T:center_disk{group}
        bottom_disk{group}:T -- B:center_disk{group}
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);
    let doc = parse_svg(&svg);

    assert_eq!(structure.node_count, 5);
    assert_eq!(structure.edge_count, 4);
    assert!(has_id(&doc, "group-left_group"));
    assert!(has_id(&doc, "group-right_group"));
    assert!(has_id(&doc, "group-top_group"));
    assert!(has_id(&doc, "group-bottom_group"));
    assert!(has_id(&doc, "group-center_group"));
}

#[test]
fn test_architecture_edge_labels() {
    let input = r#"architecture-beta
        service servC(server)[Server 1]
        service servL(server)[Server 2]
        service servR(server)[Server 3]
        service servT(server)[Server 4]
        service servB(server)[Server 5]

        servC:L -[Label]- R:servL
        servC:R -[Label]- L:servR
        servC:T -[Label]- B:servT
        servC:B -[Label]- T:servB

        servL:T -[Label]- L:servT
        servL:B -[Label]- L:servB
        servR:T -[Label]- R:servT
        servR:B -[Label]- R:servB
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);

    assert_eq!(structure.node_count, 5);
    assert_eq!(structure.edge_count, 8);
    assert_labels(&structure, &["Label"]);
}

#[test]
fn test_architecture_simple_junction_edges() {
    let input = r#"architecture-beta
        service left_disk(disk)[Disk]
        service top_disk(disk)[Disk]
        service bottom_disk(disk)[Disk]
        service top_gateway(internet)[Gateway]
        service bottom_gateway(internet)[Gateway]
        junction juncC
        junction juncR

        left_disk:R -- L:juncC
        top_disk:B -- T:juncC
        bottom_disk:T -- B:juncC
        juncC:R -- L:juncR
        top_gateway:B -- T:juncR
        bottom_gateway:T -- B:juncR
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);

    assert_eq!(structure.node_count, 7);
    assert_eq!(structure.edge_count, 6);
    assert_labels(&structure, &["Gateway"]);
}

#[test]
fn test_architecture_complex_junction_edges() {
    let input = r#"architecture-beta
        group left
        group right
        service left_disk(disk)[Disk] in left
        service top_disk(disk)[Disk] in left
        service bottom_disk(disk)[Disk] in left
        service top_gateway(internet)[Gateway] in right
        service bottom_gateway(internet)[Gateway] in right
        junction juncC in left
        junction juncR in right

        left_disk:R -- L:juncC
        top_disk:B -- T:juncC
        bottom_disk:T -- B:juncC
        top_gateway:B -- T:juncR
        bottom_gateway:T -- B:juncR
        juncC{group}:R -- L:juncR{group}
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);
    let doc = parse_svg(&svg);

    assert_eq!(structure.node_count, 7);
    assert_eq!(structure.edge_count, 6);
    assert!(has_id(&doc, "group-left"));
    assert!(has_id(&doc, "group-right"));
}

#[test]
fn test_architecture_reasonable_height() {
    let input = r#"architecture-beta
        group federated(cloud)[Federated Environment]
            service server1(server)[System] in federated
            service edge(server)[Edge Device] in federated
            server1:R -- L:edge

        group on_prem(cloud)[Hub]
            service firewall(server)[Firewall Device] in on_prem
            service server(server)[Server] in on_prem
            firewall:R -- L:server

            service db1(database)[db1] in on_prem
            service db2(database)[db2] in on_prem
            service db3(database)[db3] in on_prem
            service db4(database)[db4] in on_prem
            service db5(database)[db5] in on_prem
            service db6(database)[db6] in on_prem

            junction mid in on_prem
            server:B -- T:mid

            junction 1Leftofmid in on_prem
            1Leftofmid:R -- L:mid
            1Leftofmid:B -- T:db1

            junction 2Leftofmid in on_prem
            2Leftofmid:R -- L:1Leftofmid
            2Leftofmid:B -- T:db2

            junction 3Leftofmid in on_prem
            3Leftofmid:R -- L:2Leftofmid
            3Leftofmid:B -- T:db3

            junction 1RightOfMid in on_prem
            mid:R -- L:1RightOfMid
            1RightOfMid:B -- T:db4

            junction 2RightOfMid in on_prem
            1RightOfMid:R -- L:2RightOfMid
            2RightOfMid:B -- T:db5

            junction 3RightOfMid in on_prem
            2RightOfMid:R -- L:3RightOfMid
            3RightOfMid:B -- T:db6

            edge:R -- L:firewall
    "#;

    let svg = render_architecture_svg(input);
    let structure = svg_structure(&svg);

    assert!(structure.height > 0.0);
    assert!(structure.height < 2000.0);
}
