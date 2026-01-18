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
        Sample {
            name: "requirement",
            diagram_type: "requirement",
            source: r#"requirementDiagram

    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    functionalRequirement test_req2 {
    id: 1.1
    text: the second test text.
    risk: low
    verifymethod: inspection
    }

    performanceRequirement test_req3 {
    id: 1.2
    text: the third test text.
    risk: medium
    verifymethod: demonstration
    }

    element test_entity {
    type: simulation
    }

    element test_entity2 {
    type: word doc
    docRef: reqs/test_entity
    }

    test_entity - satisfies -> test_req2
    test_req - traces -> test_req2
    test_req - contains -> test_req3
    test_entity2 - verifies -> test_req"#,
        },
        Sample {
            name: "requirement_full",
            diagram_type: "requirement",
            source: r#"requirementDiagram

    requirement test_req {
    id: 1
    text: the test text.
    risk: high
    verifymethod: test
    }

    functionalRequirement test_req2 {
    id: 1.1
    text: the second test text.
    risk: low
    verifymethod: inspection
    }

    performanceRequirement test_req3 {
    id: 1.2
    text: the third test text.
    risk: medium
    verifymethod: demonstration
    }

    interfaceRequirement test_req4 {
    id: 1.2.1
    text: the fourth test text.
    risk: medium
    verifymethod: analysis
    }

    physicalRequirement test_req5 {
    id: 1.2.2
    text: the fifth test text.
    risk: medium
    verifymethod: analysis
    }

    designConstraint test_req6 {
    id: 1.2.3
    text: the sixth test text.
    risk: medium
    verifymethod: analysis
    }

    element test_entity {
    type: simulation
    }

    element test_entity2 {
    type: word doc
    docRef: reqs/test_entity
    }

    element test_entity3 {
    type: "test suite"
    docRef: github.com/all_the_tests
    }

    test_entity - satisfies -> test_req2
    test_req - traces -> test_req2
    test_req - contains -> test_req3
    test_req3 - contains -> test_req4
    test_req4 - derives -> test_req5
    test_req5 - refines -> test_req6
    test_entity3 - verifies -> test_req5"#,
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
        // Sankey diagrams
        Sample {
            name: "sankey_simple",
            diagram_type: "sankey",
            source: r#"sankey-beta

sourceNode,targetNode,10"#,
        },
        Sample {
            name: "sankey_chain",
            diagram_type: "sankey",
            source: r#"sankey-beta

a,b,8
b,c,8
c,d,8"#,
        },
        Sample {
            name: "sankey_branching",
            diagram_type: "sankey",
            source: r#"sankey-beta

a,b,8
b,c,8
c,d,8
d,e,8

x,c,4
c,y,4"#,
        },
        Sample {
            name: "sankey_energy",
            diagram_type: "sankey",
            source: r#"sankey-beta

Bio-conversion,Liquid,0.597
Bio-conversion,Losses,26.862
Bio-conversion,Solid,280.322
Bio-conversion,Gas,81.144"#,
        },
        Sample {
            name: "sankey_quoted",
            diagram_type: "sankey",
            source: r#"sankey-beta

"Biofuel imports",Liquid,35
"Heating and cooling",Residential,79.329
"District heating","Heating and cooling, commercial",22.505"#,
        },
        // Reference examples from mermaid documentation
        Sample {
            name: "sankey_empty_lines",
            diagram_type: "sankey",
            source: r#"sankey-beta

Bio-conversion,Losses,26.862

Bio-conversion,Solid,280.322

Bio-conversion,Gas,81.144"#,
        },
        Sample {
            name: "sankey_commas",
            diagram_type: "sankey",
            source: r#"sankey-beta

Pumped heat,"Heating and cooling, homes",193.026
Pumped heat,"Heating and cooling, commercial",70.672"#,
        },
        Sample {
            name: "sankey_double_quotes",
            diagram_type: "sankey",
            source: r#"sankey-beta

Pumped heat,"Heating and cooling, ""homes""",193.026
Pumped heat,"Heating and cooling, ""commercial""",70.672"#,
        },
        Sample {
            name: "sankey_energy_full",
            diagram_type: "sankey",
            source: r#"sankey-beta

Agricultural 'waste',Bio-conversion,124.729
Bio-conversion,Liquid,0.597
Bio-conversion,Losses,26.862
Bio-conversion,Solid,280.322
Bio-conversion,Gas,81.144
Biofuel imports,Liquid,35
Biomass imports,Solid,35
Coal imports,Coal,11.606
Coal reserves,Coal,63.965
Coal,Solid,75.571
District heating,Industry,10.639
District heating,Heating and cooling - commercial,22.505
District heating,Heating and cooling - homes,46.184
Electricity grid,Over generation / exports,104.453
Electricity grid,Heating and cooling - homes,113.726
Electricity grid,H2 conversion,27.14
Electricity grid,Industry,342.165
Electricity grid,Road transport,37.797
Electricity grid,Agriculture,4.412
Electricity grid,Heating and cooling - commercial,40.858
Electricity grid,Losses,56.691
Electricity grid,Rail transport,7.863
Electricity grid,Lighting & appliances - commercial,90.008
Electricity grid,Lighting & appliances - homes,93.494
Gas imports,Ngas,40.719
Gas reserves,Ngas,82.233
Gas,Heating and cooling - commercial,0.129
Gas,Losses,1.401
Gas,Thermal generation,151.891
Gas,Agriculture,2.096
Gas,Industry,48.58
Geothermal,Electricity grid,7.013
H2 conversion,H2,20.897
H2 conversion,Losses,6.242
H2,Road transport,20.897
Hydro,Electricity grid,6.995
Liquid,Industry,121.066
Liquid,International shipping,128.69
Liquid,Road transport,135.835
Liquid,Domestic aviation,14.458
Liquid,International aviation,206.267
Liquid,Agriculture,3.64
Liquid,National navigation,33.218
Liquid,Rail transport,4.413
Marine algae,Bio-conversion,4.375
Ngas,Gas,122.952
Nuclear,Thermal generation,839.978
Oil imports,Oil,504.287
Oil reserves,Oil,107.703
Oil,Liquid,611.99
Other waste,Solid,56.587
Other waste,Bio-conversion,77.81
Pumped heat,Heating and cooling - homes,193.026
Pumped heat,Heating and cooling - commercial,70.672
Solar PV,Electricity grid,59.901
Solar Thermal,Heating and cooling - homes,19.263
Solar,Solar Thermal,19.263
Solar,Solar PV,59.901
Solid,Agriculture,0.882
Solid,Thermal generation,400.12
Solid,Industry,46.477
Thermal generation,Electricity grid,525.531
Thermal generation,Losses,787.129
Thermal generation,District heating,79.329
Tidal,Electricity grid,9.452
UK land based bioenergy,Bio-conversion,182.01
Wave,Electricity grid,19.013
Wind,Electricity grid,289.366"#,
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
    } else if first_line.starts_with("requirementdiagram") {
        "requirement".to_string()
    } else if first_line.starts_with("timeline") {
        "timeline".to_string()
    } else if first_line.starts_with("journey") {
        "journey".to_string()
    } else if first_line.starts_with("architecture") {
        "architecture".to_string()
    } else if first_line.starts_with("c4context")
        || first_line.starts_with("c4container")
        || first_line.starts_with("c4component")
        || first_line.starts_with("c4dynamic")
        || first_line.starts_with("c4deployment")
    {
        "c4".to_string()
    } else if first_line.starts_with("sankey") {
        "sankey".to_string()
    } else if first_line.starts_with("quadrantchart") {
        "quadrant".to_string()
    } else if first_line.starts_with("treemap") {
        "treemap".to_string()
    } else if first_line.starts_with("xychart") {
        "xychart".to_string()
    } else if first_line.starts_with("block-beta") || first_line.starts_with("block") {
        "block".to_string()
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
        assert_eq!(
            detect_diagram_type("architecture-beta\n  service db(database)[DB]"),
            "architecture"
        );
        assert_eq!(detect_diagram_type("sankey-beta\n  a,b,10"), "sankey");
    }
}
