//! Built-in sample diagrams for evaluation.
//!
//! Samples can be loaded from docs/sources/*.mmd files or use embedded defaults.

use std::fs;
use std::path::Path;

/// A sample diagram for evaluation
#[derive(Debug, Clone)]
pub struct Sample {
    /// Name/identifier for the sample
    pub name: &'static str,
    /// Diagram type (flowchart, sequence, pie, etc.)
    pub diagram_type: &'static str,
    /// The mermaid diagram source
    pub source: &'static str,
}

/// An owned sample (for dynamically loaded files)
#[derive(Debug, Clone)]
pub struct OwnedSample {
    /// Name/identifier for the sample
    pub name: String,
    /// Diagram type (flowchart, sequence, pie, etc.)
    pub diagram_type: String,
    /// The mermaid diagram source
    pub source: String,
}

impl From<Sample> for OwnedSample {
    fn from(s: Sample) -> Self {
        OwnedSample {
            name: s.name.to_string(),
            diagram_type: s.diagram_type.to_string(),
            source: s.source.to_string(),
        }
    }
}

/// Load samples from docs/sources/ directory
pub fn load_from_docs_sources() -> Vec<OwnedSample> {
    let sources_dir = Path::new("docs/sources");
    if !sources_dir.exists() {
        return Vec::new();
    }

    let mut samples = Vec::new();

    if let Ok(entries) = fs::read_dir(sources_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "mmd").unwrap_or(false) {
                if let Ok(source) = fs::read_to_string(&path) {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let diagram_type = detect_diagram_type(&source);

                    samples.push(OwnedSample {
                        name,
                        diagram_type,
                        source,
                    });
                }
            }
        }
    }

    // Sort by name for consistent ordering
    samples.sort_by(|a, b| a.name.cmp(&b.name));
    samples
}

/// Get all samples: docs/sources/ files first, then embedded samples
pub fn all_samples_owned() -> Vec<OwnedSample> {
    let mut samples = load_from_docs_sources();

    // Add embedded samples
    for s in embedded_samples() {
        samples.push(s.into());
    }

    samples
}

/// Get embedded sample diagrams (the original hardcoded samples)
pub fn embedded_samples() -> Vec<Sample> {
    vec![
        // Basic diagram types
        Sample {
            name: "flowchart",
            diagram_type: "flowchart",
            source: r#"flowchart LR
    A[Start] --> B{Decision}
    B -->|Yes| C[Action 1]
    B -->|No| D[Action 2]
    C --> E[End]
    D --> E
    E --> F([Round])
    F --> G[[Subroutine]]
    G --> H[(Database)]
    H o--o I((Circle))"#,
        },
        Sample {
            name: "flowchart_full",
            diagram_type: "flowchart",
            source: r#"flowchart TB
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
        },
        Sample {
            name: "pie",
            diagram_type: "pie",
            source: r#"pie title Project Distribution
    "Development" : 40
    "Testing" : 25
    "Documentation" : 15
    "Design" : 20"#,
        },
        Sample {
            name: "sequence",
            diagram_type: "sequence",
            source: r#"sequenceDiagram
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
        },
        Sample {
            name: "class",
            diagram_type: "class",
            source: r#"classDiagram
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
        },
        Sample {
            name: "state",
            diagram_type: "state",
            source: r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running : start
    Running --> Idle : stop
    Running --> Error : error
    Error --> Idle : reset
    Error --> [*]"#,
        },
        Sample {
            name: "er",
            diagram_type: "er",
            source: r#"erDiagram
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
        },
        Sample {
            name: "gantt",
            diagram_type: "gantt",
            source: r#"gantt
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
        },
        // Examples from mermaid.js documentation
        Sample {
            name: "example_pie_netflix",
            diagram_type: "pie",
            source: r#"pie title NETFLIX
         "Time spent looking for movie" : 90
         "Time spent watching it" : 10"#,
        },
        Sample {
            name: "example_pie_voldemort",
            diagram_type: "pie",
            source: r#"pie title What Voldemort doesn't have?
         "FRIENDS" : 2
         "FAMILY" : 3
         "NOSE" : 45"#,
        },
        Sample {
            name: "example_sequence_basic",
            diagram_type: "sequence",
            source: r#"sequenceDiagram
    Alice ->> Bob: Hello Bob, how are you?
    Bob-->>John: How about you John?
    Bob--x Alice: I am good thanks!
    Bob-x John: I am good thanks!
    Note right of John: Bob thinks a long<br/>long time, so long<br/>that the text does<br/>not fit on a row.

    Bob-->Alice: Checking with John...
    Alice->John: Yes... John, how are you?"#,
        },
        Sample {
            name: "example_flowchart_basic",
            diagram_type: "flowchart",
            source: r#"graph LR
    A[Square Rect] -- Link text --> B((Circle))
    A --> C(Round Rect)
    B --> D{Rhombus}
    C --> D"#,
        },
        Sample {
            name: "example_flowchart_styled",
            diagram_type: "flowchart",
            source: r#"graph TB
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
        },
        Sample {
            name: "example_sequence_loops",
            diagram_type: "sequence",
            source: r#"sequenceDiagram
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
        },
        Sample {
            name: "example_sequence_self_loop",
            diagram_type: "sequence",
            source: r#"sequenceDiagram
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
        },
        Sample {
            name: "example_sequence_blogging",
            diagram_type: "sequence",
            source: r#"sequenceDiagram
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
        },
    ]
}

/// Detect diagram type from source content
fn detect_diagram_type(source: &str) -> String {
    let source_lower = source.to_lowercase();
    let first_line = source_lower.lines().next().unwrap_or("");

    if first_line.starts_with("flowchart") || first_line.starts_with("graph ") {
        "flowchart".to_string()
    } else if first_line.starts_with("sequencediagram") {
        "sequence".to_string()
    } else if first_line.starts_with("classdiagram") {
        "class".to_string()
    } else if first_line.starts_with("statediagram") {
        "state".to_string()
    } else if first_line.starts_with("erdiagram") {
        "er".to_string()
    } else if first_line.starts_with("gantt") {
        "gantt".to_string()
    } else if first_line.starts_with("pie") {
        "pie".to_string()
    } else if first_line.starts_with("gitgraph") {
        "git".to_string()
    } else if first_line.starts_with("mindmap") {
        "mindmap".to_string()
    } else if first_line.starts_with("timeline") {
        "timeline".to_string()
    } else if first_line.starts_with("journey") {
        "journey".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Get samples filtered by diagram type
pub fn samples_by_type(diagram_type: &str) -> Vec<OwnedSample> {
    all_samples_owned()
        .into_iter()
        .filter(|s| s.diagram_type == diagram_type)
        .collect()
}

/// Get available diagram types
pub fn available_types() -> Vec<String> {
    let mut types: Vec<String> = all_samples_owned()
        .iter()
        .map(|s| s.diagram_type.clone())
        .collect();
    types.sort();
    types.dedup();
    types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_samples_not_empty() {
        assert!(!embedded_samples().is_empty());
    }

    #[test]
    fn test_available_types() {
        let types = available_types();
        assert!(types.contains(&"flowchart".to_string()));
        assert!(types.contains(&"sequence".to_string()));
        assert!(types.contains(&"pie".to_string()));
    }

    #[test]
    fn test_samples_by_type() {
        let flowcharts = samples_by_type("flowchart");
        assert!(!flowcharts.is_empty());
        assert!(flowcharts.iter().all(|s| s.diagram_type == "flowchart"));
    }

    #[test]
    fn test_detect_diagram_type() {
        assert_eq!(detect_diagram_type("flowchart LR\n  A-->B"), "flowchart");
        assert_eq!(detect_diagram_type("graph TD\n  A-->B"), "flowchart");
        assert_eq!(
            detect_diagram_type("sequenceDiagram\n  A->>B: Hi"),
            "sequence"
        );
        assert_eq!(detect_diagram_type("pie\n  \"A\": 50"), "pie");
        assert_eq!(detect_diagram_type("stateDiagram-v2\n  [*]-->A"), "state");
    }
}
