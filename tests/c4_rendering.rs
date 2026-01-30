//! C4 diagram rendering tests - ported from Cypress tests
//!
//! These tests are ported from the mermaid.js Cypress test suite:
//! - cypress/e2e/rendering/c4.spec.js
//!
//! Test cases:
//! - C4.1: C4Context with enterprise boundaries, persons, systems, styling
//! - C4.2: C4Container with tags
//! - C4.3: C4Component with Rel_Back
//! - C4.4: C4Dynamic with ContainerDb
//! - C4.5: C4Deployment with Deployment_Node
//! - C4.6: C4Context with ComponentQueue_Ext

use selkie::{parse, render};

fn render_c4_svg(input: &str) -> Result<String, String> {
    let diagram = parse(input).map_err(|e| format!("Parse error: {}", e))?;
    render(&diagram).map_err(|e| format!("Render error: {}", e))
}

fn svg_contains_text(svg: &str, text: &str) -> bool {
    // Check for raw text or HTML entity encoded version
    svg.contains(text) || svg.contains(&text.replace('\'', "&apos;"))
}

// ============================================================================
// C4.1: C4Context with enterprise boundaries, persons, systems
// ============================================================================

#[test]
fn c4_1_should_render_simple_c4_context_diagram() {
    let input = r#"C4Context
      title System Context diagram for Internet Banking System

      Enterprise_Boundary(b0, "BankBoundary0") {
          Person(customerA, "Banking Customer A", "A customer of the bank, with personal bank accounts.")

          System(SystemAA, "Internet Banking System", "Allows customers to view information about their bank accounts, and make payments.")

          Enterprise_Boundary(b1, "BankBoundary") {
            System_Ext(SystemC, "E-mail system", "The internal Microsoft Exchange e-mail system.")
          }
        }

      BiRel(customerA, SystemAA, "Uses")
      Rel(SystemAA, SystemC, "Sends e-mails", "SMTP")
      Rel(SystemC, customerA, "Sends e-mails to")"#;

    let svg = render_c4_svg(input).expect("Failed to render C4Context diagram");

    // Verify SVG structure
    assert!(svg.contains("<svg"), "Should produce valid SVG");

    // Verify persons are rendered
    assert!(
        svg_contains_text(&svg, "Banking Customer A"),
        "Should contain person label 'Banking Customer A'"
    );

    // Verify systems are rendered
    assert!(
        svg_contains_text(&svg, "Internet Banking System"),
        "Should contain system label 'Internet Banking System'"
    );
    assert!(
        svg_contains_text(&svg, "E-mail system"),
        "Should contain external system label 'E-mail system'"
    );

    // Verify boundaries are rendered
    assert!(
        svg_contains_text(&svg, "BankBoundary0"),
        "Should contain boundary label 'BankBoundary0'"
    );
    assert!(
        svg_contains_text(&svg, "BankBoundary"),
        "Should contain nested boundary label 'BankBoundary'"
    );

    // Verify relationships are rendered
    assert!(
        svg_contains_text(&svg, "Uses"),
        "Should contain relationship label 'Uses'"
    );
    assert!(
        svg_contains_text(&svg, "Sends e-mails"),
        "Should contain relationship label 'Sends e-mails'"
    );
}

// ============================================================================
// C4.2: C4Container with tags
// ============================================================================

#[test]
fn c4_2_should_render_simple_c4_container_diagram() {
    let input = r#"C4Container
      title Container diagram for Internet Banking System

      System_Ext(email_system, "E-Mail System", "The internal Microsoft Exchange system", $tags="v1.0")
      Person(customer, Customer, "A customer of the bank, with personal bank accounts", $tags="v1.0")

      Container_Boundary(c1, "Internet Banking") {
          Container(spa, "Single-Page App", "JavaScript, Angular", "Provides all the Internet banking functionality to customers via their web browser")
      }

      Rel(customer, spa, "Uses", "HTTPS")
      Rel(email_system, customer, "Sends e-mails to")"#;

    let svg = render_c4_svg(input).expect("Failed to render C4Container diagram");

    // Verify SVG structure
    assert!(svg.contains("<svg"), "Should produce valid SVG");

    // Verify external system
    assert!(
        svg_contains_text(&svg, "E-Mail System"),
        "Should contain external system 'E-Mail System'"
    );

    // Verify person
    assert!(
        svg_contains_text(&svg, "Customer"),
        "Should contain person 'Customer'"
    );

    // Verify container boundary
    assert!(
        svg_contains_text(&svg, "Internet Banking"),
        "Should contain container boundary 'Internet Banking'"
    );

    // Verify container
    assert!(
        svg_contains_text(&svg, "Single-Page App"),
        "Should contain container 'Single-Page App'"
    );

    // Verify relationships
    assert!(
        svg_contains_text(&svg, "Uses"),
        "Should contain relationship 'Uses'"
    );
    assert!(
        svg_contains_text(&svg, "HTTPS"),
        "Should contain technology 'HTTPS'"
    );
}

// ============================================================================
// C4.3: C4Component with Rel_Back
// ============================================================================

#[test]
fn c4_3_should_render_simple_c4_component_diagram() {
    let input = r#"C4Component
      title Component diagram for Internet Banking System - API Application

      Container(spa, "Single Page Application", "javascript and angular", "Provides all the internet banking functionality to customers via their web browser.")

      Container_Boundary(api, "API Application") {
        Component(sign, "Sign In Controller", "MVC Rest Controller", "Allows users to sign in to the internet banking system")
      }

      Rel_Back(spa, sign, "Uses", "JSON/HTTPS")"#;

    let svg = render_c4_svg(input).expect("Failed to render C4Component diagram");

    // Verify SVG structure
    assert!(svg.contains("<svg"), "Should produce valid SVG");

    // Verify container
    assert!(
        svg_contains_text(&svg, "Single Page Application"),
        "Should contain container 'Single Page Application'"
    );

    // Verify component boundary
    assert!(
        svg_contains_text(&svg, "API Application"),
        "Should contain container boundary 'API Application'"
    );

    // Verify component
    assert!(
        svg_contains_text(&svg, "Sign In Controller"),
        "Should contain component 'Sign In Controller'"
    );

    // Verify relationship
    assert!(
        svg_contains_text(&svg, "Uses"),
        "Should contain relationship 'Uses'"
    );
    assert!(
        svg_contains_text(&svg, "JSON/HTTPS"),
        "Should contain technology 'JSON/HTTPS'"
    );
}

// ============================================================================
// C4.4: C4Dynamic with ContainerDb
// ============================================================================

#[test]
fn c4_4_should_render_simple_c4_dynamic_diagram() {
    let input = r#"C4Dynamic
      title Dynamic diagram for Internet Banking System - API Application

      ContainerDb(c4, "Database", "Relational Database Schema", "Stores user registration information, hashed authentication credentials, access logs, etc.")
      Container(c1, "Single-Page Application", "JavaScript and Angular", "Provides all of the Internet banking functionality to customers via their web browser.")
      Container_Boundary(b, "API Application") {
        Component(c3, "Security Component", "Spring Bean", "Provides functionality Related to signing in, changing passwords, etc.")
        Component(c2, "Sign In Controller", "Spring MVC Rest Controller", "Allows users to sign in to the Internet Banking System.")
      }
      Rel(c1, c2, "Submits credentials to", "JSON/HTTPS")
      Rel(c2, c3, "Calls isAuthenticated() on")
      Rel(c3, c4, "select * from users where username = ?", "JDBC")"#;

    let svg = render_c4_svg(input).expect("Failed to render C4Dynamic diagram");

    // Verify SVG structure
    assert!(svg.contains("<svg"), "Should produce valid SVG");

    // Verify database container
    assert!(
        svg_contains_text(&svg, "Database"),
        "Should contain ContainerDb 'Database'"
    );

    // Verify container
    assert!(
        svg_contains_text(&svg, "Single-Page Application"),
        "Should contain container 'Single-Page Application'"
    );

    // Verify components
    assert!(
        svg_contains_text(&svg, "Security Component"),
        "Should contain component 'Security Component'"
    );
    assert!(
        svg_contains_text(&svg, "Sign In Controller"),
        "Should contain component 'Sign In Controller'"
    );

    // Verify relationships
    assert!(
        svg_contains_text(&svg, "Submits credentials to"),
        "Should contain relationship 'Submits credentials to'"
    );
    assert!(
        svg_contains_text(&svg, "Calls isAuthenticated() on"),
        "Should contain relationship 'Calls isAuthenticated() on'"
    );
}

// ============================================================================
// C4.5: C4Deployment with Deployment_Node
// ============================================================================

#[test]
fn c4_5_should_render_simple_c4_deployment_diagram() {
    let input = r#"C4Deployment
      title Deployment Diagram for Internet Banking System - Live

      Deployment_Node(mob, "Customer's mobile device", "Apple IOS or Android"){
          Container(mobile, "Mobile App", "Xamarin", "Provides a limited subset of the Internet Banking functionality to customers via their mobile device.")
      }

      Deployment_Node(plc, "Big Bank plc", "Big Bank plc data center"){
          Deployment_Node(dn, "bigbank-api*** x8", "Ubuntu 16.04 LTS"){
              Deployment_Node(apache, "Apache Tomcat", "Apache Tomcat 8.x"){
                  Container(api, "API Application", "Java and Spring MVC", "Provides Internet Banking functionality via a JSON/HTTPS API.")
              }
          }
      }

      Rel(mobile, api, "Makes API calls to", "json/HTTPS")"#;

    let svg = render_c4_svg(input).expect("Failed to render C4Deployment diagram");

    // Verify SVG structure
    assert!(svg.contains("<svg"), "Should produce valid SVG");

    // Verify deployment nodes
    assert!(
        svg_contains_text(&svg, "Customer's mobile device"),
        "Should contain deployment node 'Customer's mobile device'"
    );
    assert!(
        svg_contains_text(&svg, "Big Bank plc"),
        "Should contain deployment node 'Big Bank plc'"
    );

    // Verify containers
    assert!(
        svg_contains_text(&svg, "Mobile App"),
        "Should contain container 'Mobile App'"
    );
    assert!(
        svg_contains_text(&svg, "API Application"),
        "Should contain container 'API Application'"
    );

    // Verify relationships
    assert!(
        svg_contains_text(&svg, "Makes API calls to"),
        "Should contain relationship 'Makes API calls to'"
    );
}

// ============================================================================
// C4.6: C4Context with ComponentQueue_Ext
// ============================================================================

#[test]
fn c4_6_should_render_c4_context_with_component_queue_ext() {
    let input = r#"C4Context
      title System Context diagram with ComponentQueue_Ext

      Enterprise_Boundary(b0, "BankBoundary0") {
          Person(customerA, "Banking Customer A", "A customer of the bank, with personal bank accounts.")

          System(SystemAA, "Internet Banking System", "Allows customers to view information about their bank accounts, and make payments.")

          Enterprise_Boundary(b1, "BankBoundary") {
            ComponentQueue_Ext(msgQueue, "Message Queue", "RabbitMQ", "External message queue system for processing banking transactions")
            System_Ext(SystemC, "E-mail system", "The internal Microsoft Exchange e-mail system.")
          }
        }

      BiRel(customerA, SystemAA, "Uses")
      Rel(SystemAA, msgQueue, "Sends messages to")
      Rel(SystemAA, SystemC, "Sends e-mails", "SMTP")"#;

    let svg = render_c4_svg(input).expect("Failed to render C4Context with ComponentQueue_Ext");

    // Verify SVG structure
    assert!(svg.contains("<svg"), "Should produce valid SVG");

    // Verify ComponentQueue_Ext
    assert!(
        svg_contains_text(&svg, "Message Queue"),
        "Should contain ComponentQueue_Ext 'Message Queue'"
    );
    assert!(
        svg_contains_text(&svg, "RabbitMQ"),
        "Should contain technology 'RabbitMQ'"
    );

    // Verify relationships to queue
    assert!(
        svg_contains_text(&svg, "Sends messages to"),
        "Should contain relationship 'Sends messages to'"
    );
}

// ============================================================================
// Additional parsing tests to ensure cypress test inputs parse correctly
// ============================================================================

mod parsing_tests {
    use super::*;

    #[test]
    fn parse_c4_1_context() {
        let input = r#"C4Context
      title System Context diagram for Internet Banking System

      Enterprise_Boundary(b0, "BankBoundary0") {
          Person(customerA, "Banking Customer A", "A customer of the bank, with personal bank accounts.")

          System(SystemAA, "Internet Banking System", "Allows customers to view information about their bank accounts, and make payments.")

          Enterprise_Boundary(b1, "BankBoundary") {
            System_Ext(SystemC, "E-mail system", "The internal Microsoft Exchange e-mail system.")
          }
        }

      BiRel(customerA, SystemAA, "Uses")
      Rel(SystemAA, SystemC, "Sends e-mails", "SMTP")
      Rel(SystemC, customerA, "Sends e-mails to")"#;

        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse C4.1: {:?}", result.err());
    }

    #[test]
    fn parse_c4_2_container() {
        let input = r#"C4Container
      title Container diagram for Internet Banking System

      System_Ext(email_system, "E-Mail System", "The internal Microsoft Exchange system", $tags="v1.0")
      Person(customer, Customer, "A customer of the bank, with personal bank accounts", $tags="v1.0")

      Container_Boundary(c1, "Internet Banking") {
          Container(spa, "Single-Page App", "JavaScript, Angular", "Provides all the Internet banking functionality to customers via their web browser")
      }

      Rel(customer, spa, "Uses", "HTTPS")
      Rel(email_system, customer, "Sends e-mails to")"#;

        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse C4.2: {:?}", result.err());
    }

    #[test]
    fn parse_c4_3_component() {
        let input = r#"C4Component
      title Component diagram for Internet Banking System - API Application

      Container(spa, "Single Page Application", "javascript and angular", "Provides all the internet banking functionality to customers via their web browser.")

      Container_Boundary(api, "API Application") {
        Component(sign, "Sign In Controller", "MVC Rest Controller", "Allows users to sign in to the internet banking system")
      }

      Rel_Back(spa, sign, "Uses", "JSON/HTTPS")"#;

        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse C4.3: {:?}", result.err());
    }

    #[test]
    fn parse_c4_4_dynamic() {
        let input = r#"C4Dynamic
      title Dynamic diagram for Internet Banking System - API Application

      ContainerDb(c4, "Database", "Relational Database Schema", "Stores user registration information, hashed authentication credentials, access logs, etc.")
      Container(c1, "Single-Page Application", "JavaScript and Angular", "Provides all of the Internet banking functionality to customers via their web browser.")
      Container_Boundary(b, "API Application") {
        Component(c3, "Security Component", "Spring Bean", "Provides functionality Related to signing in, changing passwords, etc.")
        Component(c2, "Sign In Controller", "Spring MVC Rest Controller", "Allows users to sign in to the Internet Banking System.")
      }
      Rel(c1, c2, "Submits credentials to", "JSON/HTTPS")
      Rel(c2, c3, "Calls isAuthenticated() on")
      Rel(c3, c4, "select * from users where username = ?", "JDBC")"#;

        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse C4.4: {:?}", result.err());
    }

    #[test]
    fn parse_c4_6_component_queue_ext() {
        let input = r#"C4Context
      title System Context diagram with ComponentQueue_Ext

      Enterprise_Boundary(b0, "BankBoundary0") {
          Person(customerA, "Banking Customer A", "A customer of the bank, with personal bank accounts.")

          System(SystemAA, "Internet Banking System", "Allows customers to view information about their bank accounts, and make payments.")

          Enterprise_Boundary(b1, "BankBoundary") {
            ComponentQueue_Ext(msgQueue, "Message Queue", "RabbitMQ", "External message queue system for processing banking transactions")
            System_Ext(SystemC, "E-mail system", "The internal Microsoft Exchange e-mail system.")
          }
        }

      BiRel(customerA, SystemAA, "Uses")
      Rel(SystemAA, msgQueue, "Sends messages to")
      Rel(SystemAA, SystemC, "Sends e-mails", "SMTP")"#;

        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse C4.6: {:?}", result.err());
    }
}

// ============================================================================
// Shape rendering tests
// ============================================================================

mod shape_tests {
    use super::*;

    #[test]
    fn render_person_shape() {
        let input = r#"C4Context
Person(user, "User", "A system user")"#;

        let svg = render_c4_svg(input).expect("Failed to render person");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "User"),
            "Should contain person label"
        );
    }

    #[test]
    fn render_system_shape() {
        let input = r#"C4Context
System(sys, "My System", "Description of system")"#;

        let svg = render_c4_svg(input).expect("Failed to render system");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "My System"),
            "Should contain system label"
        );
    }

    #[test]
    fn render_container_shape() {
        let input = r#"C4Container
Container(api, "API", "Node.js", "REST API")"#;

        let svg = render_c4_svg(input).expect("Failed to render container");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "API"),
            "Should contain container label"
        );
    }

    #[test]
    fn render_component_shape() {
        let input = r#"C4Component
Component(auth, "Auth", "Spring", "Authentication service")"#;

        let svg = render_c4_svg(input).expect("Failed to render component");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Auth"),
            "Should contain component label"
        );
    }

    #[test]
    fn render_database_shapes() {
        let input = r#"C4Container
ContainerDb(db, "Database", "PostgreSQL", "Stores data")"#;

        let svg = render_c4_svg(input).expect("Failed to render database");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Database"),
            "Should contain database label"
        );
    }

    #[test]
    fn render_queue_shapes() {
        let input = r#"C4Container
ContainerQueue(queue, "Message Queue", "RabbitMQ", "Handles async messages")"#;

        let svg = render_c4_svg(input).expect("Failed to render queue");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Message Queue"),
            "Should contain queue label"
        );
    }

    #[test]
    fn render_external_system() {
        let input = r#"C4Context
System_Ext(ext, "External System", "Third party service")"#;

        let svg = render_c4_svg(input).expect("Failed to render external system");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "External System"),
            "Should contain external system label"
        );
    }
}

// ============================================================================
// Boundary rendering tests
// ============================================================================

mod boundary_tests {
    use super::*;

    #[test]
    fn render_enterprise_boundary() {
        let input = r#"C4Context
Enterprise_Boundary(eb, "Enterprise") {
    System(sys, "System", "")
}"#;

        let svg = render_c4_svg(input).expect("Failed to render enterprise boundary");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Enterprise"),
            "Should contain boundary label"
        );
    }

    #[test]
    fn render_system_boundary() {
        let input = r#"C4Container
System_Boundary(sb, "System Boundary") {
    Container(c, "Container", "Tech", "")
}"#;

        let svg = render_c4_svg(input).expect("Failed to render system boundary");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "System Boundary"),
            "Should contain boundary label"
        );
    }

    #[test]
    fn render_container_boundary() {
        let input = r#"C4Component
Container_Boundary(cb, "Container Boundary") {
    Component(comp, "Component", "Tech", "")
}"#;

        let svg = render_c4_svg(input).expect("Failed to render container boundary");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Container Boundary"),
            "Should contain boundary label"
        );
    }

    #[test]
    fn render_nested_boundaries() {
        let input = r#"C4Context
Enterprise_Boundary(eb, "Enterprise") {
    Enterprise_Boundary(inner, "Inner Boundary") {
        System(sys, "Nested System", "")
    }
}"#;

        let svg = render_c4_svg(input).expect("Failed to render nested boundaries");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Enterprise"),
            "Should contain outer boundary"
        );
        assert!(
            svg_contains_text(&svg, "Inner Boundary"),
            "Should contain inner boundary"
        );
        assert!(
            svg_contains_text(&svg, "Nested System"),
            "Should contain nested system"
        );
    }
}

// ============================================================================
// Relationship rendering tests
// ============================================================================

mod relationship_tests {
    use super::*;

    #[test]
    fn render_basic_relationship() {
        let input = r#"C4Context
Person(user, "User", "")
System(sys, "System", "")
Rel(user, sys, "Uses")"#;

        let svg = render_c4_svg(input).expect("Failed to render relationship");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Uses"),
            "Should contain relationship label"
        );
    }

    #[test]
    fn render_bidirectional_relationship() {
        let input = r#"C4Context
Person(user, "User", "")
System(sys, "System", "")
BiRel(user, sys, "Communicates")"#;

        let svg = render_c4_svg(input).expect("Failed to render bidirectional relationship");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Communicates"),
            "Should contain BiRel label"
        );
        // BiRel should have arrows on both ends
        assert!(
            svg.contains("marker-start"),
            "BiRel should have marker-start for bidirectional arrow"
        );
        assert!(
            svg.contains("marker-end"),
            "BiRel should have marker-end for bidirectional arrow"
        );
    }

    #[test]
    fn render_relationship_with_technology() {
        let input = r#"C4Context
Person(user, "User", "")
System(sys, "System", "")
Rel(user, sys, "Calls API", "HTTPS")"#;

        let svg = render_c4_svg(input).expect("Failed to render relationship with tech");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
        assert!(
            svg_contains_text(&svg, "Calls API"),
            "Should contain relationship label"
        );
        assert!(
            svg_contains_text(&svg, "HTTPS"),
            "Should contain technology"
        );
    }

    #[test]
    fn render_directional_relationships() {
        let input = r#"C4Context
Person(a, "A", "")
Person(b, "B", "")
Person(c, "C", "")
Person(d, "D", "")
Rel_Up(a, b, "Up")
Rel_Down(a, c, "Down")
Rel_Left(a, d, "Left")"#;

        let svg = render_c4_svg(input).expect("Failed to render directional relationships");
        assert!(svg.contains("<svg"), "Should produce valid SVG");
    }

    #[test]
    fn render_relationship_stroke_color_matches_reference() {
        // mermaid.js uses #444444 for relationship stroke color (svgDraw.js default)
        let input = r#"C4Context
Person(a, "Alice", "A person")
System(b, "System B", "A system")
Rel(a, b, "Uses")"#;

        let svg = render_c4_svg(input).expect("Failed to render");
        assert!(
            svg.contains(r##"stroke="#444444""##),
            "Relationship stroke should be #444444 to match mermaid.js reference"
        );
        assert!(
            !svg.contains(r##"stroke="#666666""##),
            "Relationship stroke should NOT be #666666"
        );
    }

    #[test]
    fn render_relationship_label_fill_matches_reference() {
        // Relationship labels use #444444 fill (COLOR_REL), matching mermaid.js
        let input = r#"C4Context
Person(a, "Alice", "A person")
System(b, "System B", "A system")
Rel(a, b, "Uses", "HTTP")"#;

        let svg = render_c4_svg(input).expect("Failed to render");
        // Relationship label text should use #444444 fill
        assert!(
            svg.contains(r##"fill="#444444""##),
            "Relationship label fill should be #444444 to match mermaid.js reference"
        );
        // Arrow marker paths should NOT have explicit fill (inherit black per mermaid.js)
        let marker_section = svg
            .split(r#"<marker id="c4-arrow""#)
            .nth(1)
            .and_then(|s| s.split("</marker>").next())
            .unwrap_or("");
        assert!(
            !marker_section.contains("fill=\"#"),
            "Arrow marker path should not have explicit fill color — inherits black per mermaid.js"
        );
    }
}

mod symbol_tests {
    use super::*;

    #[test]
    fn render_c4_includes_symbol_defs() {
        // mermaid.js C4 includes symbol definitions for computer, database, clock icons
        let input = r#"C4Context
Person(a, "Alice", "A person")
System(b, "System B", "A system")
Rel(a, b, "Uses")"#;

        let svg = render_c4_svg(input).expect("Failed to render");
        assert!(
            svg.contains(r##"<symbol id="computer""##),
            "Should include computer symbol definition"
        );
        assert!(
            svg.contains(r##"<symbol id="database""##),
            "Should include database symbol definition"
        );
        assert!(
            svg.contains(r##"<symbol id="clock""##),
            "Should include clock symbol definition"
        );
    }

    #[test]
    fn render_c4_includes_all_marker_types() {
        // mermaid.js has 4 marker types: arrowhead, arrowend, crosshead, filled-head
        let input = r#"C4Context
Person(a, "Alice", "A person")
System(b, "System B", "A system")
Rel(a, b, "Uses")"#;

        let svg = render_c4_svg(input).expect("Failed to render");
        assert!(
            svg.contains(r##"id="c4-arrow""##),
            "Should include forward arrow marker"
        );
        assert!(
            svg.contains(r##"id="c4-arrow-reverse""##),
            "Should include reverse arrow marker"
        );
        assert!(
            svg.contains(r##"id="c4-crosshead""##),
            "Should include crosshead marker"
        );
        assert!(
            svg.contains(r##"id="c4-filled-head""##),
            "Should include filled-head marker"
        );
    }
}
