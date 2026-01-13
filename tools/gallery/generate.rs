//! Gallery generator - renders sample diagrams with selkie

use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("tools/gallery/output");
    fs::create_dir_all(output_dir)?;

    // Sample diagrams - complex examples to exercise more features
    let diagrams = vec![
        (
            "flowchart",
            r#"flowchart LR
    A[Start] --> B{Decision}
    B -->|Yes| C[Action 1]
    B -->|No| D[Action 2]
    C --> E[End]
    D --> E
    E --> F([Round])
    F --> G[[Subroutine]]
    G --> H[(Database)]
    H o--o I((Circle))"#,
        ),
        (
            "flowchart_full",
            r#"flowchart TB
    subgraph main [Main Flow]
        A[Rectangle] --> B(Rounded)
        B --> C{Diamond Decision}
        C -->|Yes| D([Stadium])
        C -->|No| E[[Subroutine]]
        D --> F[(Cylinder DB)]
        E --> F
    end
    subgraph shapes [All Shapes]
        G((Circle)) --> H>Asymmetric]
        H --> I[/Parallelogram/]
        I --> J[\Reverse Para\]
        J --> K[/Trapezoid\]
        K --> L[\Inv Trapezoid/]
        L --> M{{Hexagon}}
        M --> N(((Double Circle)))
    end
    subgraph edges [Edge Types]
        O --> P
        O --- Q
        O -.- R
        O -.-> S
        O ==> T
        O <--> U
        O x--x V
        O o--o W
    end
    F --> G
    N --> O"#,
        ),
        (
            "pie",
            r#"pie title Project Distribution
    "Development" : 40
    "Testing" : 25
    "Documentation" : 15
    "Design" : 20"#,
        ),
        (
            "sequence",
            r#"sequenceDiagram
    participant A as Alice
    participant B as Bob
    participant C as Server
    A->>B: Hello Bob!
    B-->>A: Hi Alice!
    Note over A,B: Authentication
    A->>+C: Login request
    C-->>-A: Token
    A->>B: How are you?
    B-->>A: I'm good, thanks!
    Note right of B: Bob thinks"#,
        ),
        (
            "class",
            r#"classDiagram
    Animal <|-- Duck
    Animal <|-- Fish
    Animal <|-- Zebra
    Animal : +int age
    Animal : +String gender
    Animal: +isMammal()
    Animal: +mate()
    class Duck{
        +String beakColor
        +swim()
        +quack()
    }
    class Fish{
        -int sizeInFeet
        -canEat()
    }
    class Zebra{
        +bool is_wild
        +run()
    }
    Duck "1" *-- "many" Egg : has"#,
        ),
        (
            "state",
            r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running : start
    Running --> Idle : stop
    Running --> Error : error
    Error --> Idle : reset
    Error --> [*]"#,
        ),
        (
            "er",
            r#"erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    PRODUCT ||--o{ LINE-ITEM : includes
    CUSTOMER {
        string name
        string email PK
        string address
    }
    ORDER {
        int orderNumber PK
        date orderDate
        string status
    }
    PRODUCT {
        int id PK
        string name
        float price
    }"#,
        ),
        (
            "gantt",
            r#"gantt
    title Project Timeline
    dateFormat YYYY-MM-DD
    section Planning
    Requirements :a1, 2024-01-01, 7d
    Design      :a2, after a1, 5d
    section Development
    Backend     :crit, b1, after a2, 10d
    Frontend    :b2, after a2, 8d
    API Integration :b3, after b1, 3d
    section Testing
    Unit Tests  :c1, after b2, 3d
    QA          :c2, after b3, 5d"#,
        ),
        // ========================================
        // Examples from mermaid.js documentation
        // https://mermaid.ai/open-source/syntax/examples.html
        // ========================================
        (
            "example_pie_netflix",
            r#"pie title NETFLIX
         "Time spent looking for movie" : 90
         "Time spent watching it" : 10"#,
        ),
        (
            "example_pie_voldemort",
            r#"pie title What Voldemort doesn't have?
         "FRIENDS" : 2
         "FAMILY" : 3
         "NOSE" : 45"#,
        ),
        (
            "example_sequence_basic",
            r#"sequenceDiagram
    Alice ->> Bob: Hello Bob, how are you?
    Bob-->>John: How about you John?
    Bob--x Alice: I am good thanks!
    Bob-x John: I am good thanks!
    Note right of John: Bob thinks a long<br/>long time, so long<br/>that the text does<br/>not fit on a row.

    Bob-->Alice: Checking with John...
    Alice->John: Yes... John, how are you?"#,
        ),
        (
            "example_flowchart_basic",
            r#"graph LR
    A[Square Rect] -- Link text --> B((Circle))
    A --> C(Round Rect)
    B --> D{Rhombus}
    C --> D"#,
        ),
        (
            "example_flowchart_styled",
            r#"graph TB
    sq[Square shape] --> ci((Circle shape))

    subgraph A
        od>Odd shape]-- Two line<br/>edge comment --> ro
        di{Diamond with <br/> line break} -.-> ro(Rounded<br>square<br>shape)
        di==>ro2(Rounded square shape)
    end

    e --> od3>Really long text with linebreak<br>in an Odd shape]

    e((Inner / circle<br>and some odd <br>special characters)) --> f(,.?!+-*ز)

    cyr[Cyrillic]-->cyr2((Circle shape Начало))

     classDef green fill:#9f6,stroke:#333,stroke-width:2px
     classDef orange fill:#f96,stroke:#333,stroke-width:4px
     class sq,e green
     class di orange"#,
        ),
        (
            "example_sequence_loops",
            r#"sequenceDiagram
    loop Daily query
        Alice->>Bob: Hello Bob, how are you?
        alt is sick
            Bob->>Alice: Not so good :(
        else is well
            Bob->>Alice: Feeling fresh like a daisy
        end

        opt Extra response
            Bob->>Alice: Thanks for asking
        end
    end"#,
        ),
        (
            "example_sequence_self_loop",
            r#"sequenceDiagram
    participant Alice
    participant Bob
    Alice->>John: Hello John, how are you?
    loop HealthCheck
        John->>John: Fight against hypochondria
    end
    Note right of John: Rational thoughts<br/>prevail...
    John-->>Alice: Great!
    John->>Bob: How about you?
    Bob-->>John: Jolly good!"#,
        ),
        (
            "example_sequence_blogging",
            r#"sequenceDiagram
    participant web as Web Browser
    participant blog as Blog Service
    participant account as Account Service
    participant mail as Mail Service
    participant db as Storage

    Note over web,db: The user must be logged in to submit blog posts
    web->>+account: Logs in using credentials
    account->>db: Query stored accounts
    db->>account: Respond with query result

    alt Credentials not found
        account->>web: Invalid credentials
    else Credentials found
        account->>-web: Successfully logged in

        Note over web,db: When the user is authenticated, they can now submit new posts
        web->>+blog: Submit new post
        blog->>db: Store post data

        par Notifications
            blog--)mail: Send mail to blog subscribers
            blog--)db: Store in-site notifications
        and Response
            blog-->>-web: Successfully posted
        end
    end"#,
        ),
    ];

    println!("Generating {} diagram SVGs...", diagrams.len());

    for (name, source) in &diagrams {
        let output_path = output_dir.join(format!("{}_rs.svg", name));

        match render_diagram(source) {
            Ok(svg) => {
                fs::write(&output_path, &svg)?;
                println!("  ✓ {}", name);
            }
            Err(e) => {
                println!("  ✗ {} - {}", name, e);
            }
        }
    }

    // Write diagram sources for the JS renderer
    let sources_path = output_dir.join("sources.json");
    let sources_json: Vec<_> = diagrams
        .iter()
        .map(|(name, source)| {
            serde_json::json!({
                "name": name,
                "source": source
            })
        })
        .collect();
    fs::write(&sources_path, serde_json::to_string_pretty(&sources_json)?)?;

    println!("\nDiagram sources written to {:?}", sources_path);
    println!("Run 'node tools/gallery/render_reference.mjs' to generate mermaid.js versions");

    Ok(())
}

fn render_diagram(source: &str) -> Result<String, String> {
    use mermaid::{parse, render};

    let diagram = parse(source).map_err(|e| e.to_string())?;
    render(&diagram).map_err(|e| e.to_string())
}
