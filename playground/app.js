// Selkie Playground - Main Application

// Example diagrams
const examples = {
    // Simple examples
    'flowchart-simple': `flowchart TD
    A[Start] --> B{Is it working?}
    B -->|Yes| C[Great!]
    B -->|No| D[Debug]
    D --> B`,

    'sequence-simple': `sequenceDiagram
    Alice->>Bob: Hello Bob!
    Bob-->>Alice: Hi Alice!
    Alice->>Bob: How are you?
    Bob-->>Alice: I'm good, thanks!`,

    'class-simple': `classDiagram
    Animal <|-- Dog
    Animal <|-- Cat
    Animal : +String name
    Animal : +int age
    Animal : +makeSound()
    Dog : +fetch()
    Cat : +scratch()`,

    'state-simple': `stateDiagram-v2
    [*] --> Idle
    Idle --> Processing: Start
    Processing --> Completed: Success
    Processing --> Failed: Error
    Completed --> [*]
    Failed --> Idle: Retry`,

    'er-simple': `erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
    PRODUCT ||--o{ LINE-ITEM : "is in"`,

    'gantt-simple': `gantt
    title Project Timeline
    dateFormat YYYY-MM-DD
    section Planning
    Requirements :a1, 2024-01-01, 7d
    Design       :a2, after a1, 5d
    section Development
    Coding       :a3, after a2, 14d
    Testing      :a4, after a3, 7d`,

    'pie-simple': `pie title Browser Market Share
    "Chrome" : 65
    "Firefox" : 15
    "Safari" : 12
    "Edge" : 5
    "Other" : 3`,

    // Complex examples
    'flowchart-complex': `flowchart TB
    subgraph Frontend["Frontend Layer"]
        UI[Web Interface]
        Mobile[Mobile App]
        CLI[CLI Tool]
    end

    subgraph API["API Gateway"]
        Auth{Authentication}
        Rate[Rate Limiter]
        Cache[(Redis Cache)]
    end

    subgraph Services["Microservices"]
        UserSvc[User Service]
        OrderSvc[Order Service]
        PaymentSvc[Payment Service]
        NotifySvc[Notification Service]
    end

    subgraph Data["Data Layer"]
        DB[(PostgreSQL)]
        Search[(Elasticsearch)]
        Queue[(Message Queue)]
    end

    UI --> Auth
    Mobile --> Auth
    CLI --> Auth
    Auth -->|Valid| Rate
    Auth -->|Invalid| Reject[Reject Request]
    Rate --> Cache
    Cache -->|Cache Hit| Response[Return Response]
    Cache -->|Cache Miss| UserSvc

    UserSvc --> DB
    UserSvc --> Search
    OrderSvc --> DB
    OrderSvc --> Queue
    PaymentSvc --> DB
    PaymentSvc --> NotifySvc
    NotifySvc --> Queue

    Queue --> EmailWorker[Email Worker]
    Queue --> SMSWorker[SMS Worker]`,

    'sequence-complex': `sequenceDiagram
    autonumber
    participant User
    participant Browser
    participant API
    participant Auth
    participant DB
    participant Queue

    Note over User,Queue: E-Commerce Checkout Flow

    User->>Browser: Click Checkout
    activate Browser
    Browser->>+API: POST /checkout

    Note right of API: Validate cart items

    API->>+Auth: Verify session
    Auth->>DB: Query user
    DB-->>Auth: User record
    Auth-->>-API: Session valid

    alt Cart Empty
        API-->>Browser: Error: Empty cart
        Browser-->>User: Show error
    else Cart Valid
        API->>+DB: Reserve inventory

        par Process Payment
            API->>Queue: Queue payment job
            Queue-->>API: Job queued
        and Send Notifications
            API--)Queue: Queue email confirmation
        end

        DB-->>-API: Inventory reserved

        loop Retry up to 3 times
            API->>Queue: Check payment status
            Queue-->>API: Payment pending
        end

        Note over API,Queue: Payment confirmed

        API-->>-Browser: Order confirmed
        Browser-->>User: Show confirmation
    end
    deactivate Browser`,

    'class-complex': `classDiagram
    class Application {
        -config: Config
        -logger: Logger
        +start()
        +stop()
        +getStatus() Status
    }

    class Config {
        -settings: Map
        +get(key) any
        +set(key, value)
        +load(path)
    }

    class Logger {
        -level: LogLevel
        -outputs: Output[]
        +debug(msg)
        +info(msg)
        +warn(msg)
        +error(msg)
    }

    class Router {
        -routes: Route[]
        -middleware: Middleware[]
        +addRoute(route)
        +use(middleware)
        +handle(request) Response
    }

    class Route {
        +path: string
        +method: HttpMethod
        +handler: Handler
    }

    class Middleware {
        <<interface>>
        +process(req, next) Response
    }

    class AuthMiddleware {
        -tokenService: TokenService
        +process(req, next) Response
    }

    class RateLimitMiddleware {
        -limit: int
        -window: Duration
        +process(req, next) Response
    }

    class Handler {
        <<interface>>
        +handle(request) Response
    }

    class UserController {
        -userService: UserService
        +getUser(id) User
        +createUser(data) User
        +updateUser(id, data) User
        +deleteUser(id) void
    }

    class UserService {
        -repository: UserRepository
        -cache: Cache
        +findById(id) User
        +save(user) User
        +delete(id) void
    }

    class UserRepository {
        <<interface>>
        +find(id) User
        +save(user) User
        +delete(id) void
    }

    Application --> Config
    Application --> Logger
    Application --> Router
    Router --> Route
    Router --> Middleware
    AuthMiddleware ..|> Middleware
    RateLimitMiddleware ..|> Middleware
    Route --> Handler
    UserController ..|> Handler
    UserController --> UserService
    UserService --> UserRepository`,

    'state-complex': `stateDiagram-v2
    [*] --> Idle

    state Idle {
        [*] --> Ready
        Ready --> Processing: Start Job
    }

    state Processing {
        [*] --> Validating
        Validating --> Queued: Valid
        Validating --> Failed: Invalid
        Queued --> Running: Worker Available
        Running --> Completed: Success
        Running --> Failed: Error
        Running --> Paused: Pause Request

        state Running {
            [*] --> Initializing
            Initializing --> Executing
            Executing --> Finalizing
            Finalizing --> [*]
        }
    }

    state Paused {
        [*] --> WaitingResume
        WaitingResume --> Timeout: 1 hour
    }

    Paused --> Running: Resume
    Paused --> Cancelled: Cancel Request
    Timeout --> Cancelled

    Completed --> Idle: Reset
    Failed --> Idle: Retry
    Cancelled --> Idle: Reset

    Completed --> [*]
    Cancelled --> [*]`,

    'er-complex': `erDiagram
    CUSTOMER ||--o{ ORDER : places
    CUSTOMER {
        uuid id PK
        string email UK
        string name
        string phone
        date created_at
        boolean active
    }

    ORDER ||--|{ ORDER_ITEM : contains
    ORDER {
        uuid id PK
        uuid customer_id FK
        decimal total
        string status
        date ordered_at
        date shipped_at
    }

    ORDER_ITEM }|--|| PRODUCT : references
    ORDER_ITEM {
        uuid id PK
        uuid order_id FK
        uuid product_id FK
        int quantity
        decimal price
    }

    PRODUCT ||--o{ PRODUCT_CATEGORY : belongs_to
    PRODUCT {
        uuid id PK
        string sku UK
        string name
        text description
        decimal price
        int stock
        boolean available
    }

    CATEGORY ||--o{ PRODUCT_CATEGORY : contains
    CATEGORY {
        uuid id PK
        string name UK
        uuid parent_id FK
        int sort_order
    }

    PRODUCT_CATEGORY {
        uuid product_id PK,FK
        uuid category_id PK,FK
    }

    CUSTOMER ||--o{ ADDRESS : has
    ADDRESS {
        uuid id PK
        uuid customer_id FK
        string type
        string street
        string city
        string state
        string zip
        string country
    }

    ORDER ||--|| ADDRESS : ships_to
    ORDER ||--o| PAYMENT : paid_by
    PAYMENT {
        uuid id PK
        uuid order_id FK
        string method
        decimal amount
        string status
        date processed_at
    }`,

    'gantt-complex': `gantt
    title Product Launch Timeline
    dateFormat YYYY-MM-DD

    section Research
    Market Analysis      :done, research1, 2024-01-01, 14d
    User Interviews      :done, research2, 2024-01-08, 21d
    Competitor Review    :done, research3, after research1, 10d

    section Design
    Wireframes           :done, design1, after research2, 14d
    Visual Design        :done, design2, after design1, 21d
    Prototype            :active, design3, after design2, 14d
    User Testing         :design4, after design3, 10d

    section Development
    Backend API          :dev1, after design1, 42d
    Frontend MVP         :dev2, after design2, 35d
    Integration          :dev3, after dev1, 14d
    Performance Tuning   :dev4, after dev3, 7d

    section Testing
    Unit Tests           :test1, after dev2, 14d
    Integration Tests    :test2, after dev3, 10d
    UAT                  :test3, after test2, 14d
    Bug Fixes            :test4, after test3, 7d

    section Launch
    Beta Release         :milestone, launch1, after test3, 1d
    Marketing Prep       :launch2, after design4, 21d
    Public Launch        :crit, launch3, after test4, 1d
    Post-Launch Support  :launch4, after launch3, 30d`,

    'pie-complex': `pie showData
    title Cloud Infrastructure Costs
    "Compute (EC2/GKE)" : 35
    "Storage (S3/GCS)" : 18
    "Database (RDS)" : 22
    "Networking" : 8
    "CDN & Edge" : 6
    "Monitoring" : 5
    "Security" : 4
    "Other" : 2`,

    'requirement-simple': `requirementDiagram

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

    element test_entity {
    type: simulation
    }

    test_entity - satisfies -> test_req2
    test_req - traces -> test_req2`,

    'requirement-complex': `requirementDiagram

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
    test_entity2 - verifies -> test_req`,

    'mindmap-simple': `mindmap
  root((Central Topic))
    First Branch
      Sub-topic A
      Sub-topic B
    Second Branch
      Sub-topic C
      Sub-topic D
    Third Branch`,

    'mindmap-complex': `mindmap
  root((mindmap))
    Origins
      Long history
      ::icon(fa fa-book)
      Popularisation
        British popular psychology author Tony Buzan
    Research
      On effectiveness
      On Automatic creation
        Uses
          Creative techniques
          Strategic planning
          Argument mapping
    Tools
      Pen and paper
      Mermaid
        cloud)I am a cloud(
        bang))I am a bang((`,

    'quadrant-simple': `quadrantChart
    title Reach and Engagement
    x-axis Low Reach --> High Reach
    y-axis Low Engagement --> High Engagement
    quadrant-1 We should expand
    quadrant-2 Need to promote
    quadrant-3 Re-evaluate
    quadrant-4 May be improved
    Campaign A: [0.3, 0.6]
    Campaign B: [0.45, 0.23]
    Campaign C: [0.57, 0.69]
    Campaign D: [0.78, 0.34]`,

    'quadrant-complex': `quadrantChart
    title Analytics and Business Intelligence Platforms
    x-axis Completeness of Vision --> High Vision
    y-axis Ability to Execute --> High Execution
    quadrant-1 Leaders
    quadrant-2 Challengers
    quadrant-3 Niche Players
    quadrant-4 Visionaries
    Microsoft: [0.75, 0.75] radius: 10
    Salesforce: [0.55, 0.60] radius: 8
    SAP: [0.70, 0.65]
    IBM: [0.51, 0.40]
    Oracle: [0.65, 0.55]
    Qlik: [0.60, 0.45]
    Tableau: [0.68, 0.72]
    SAS: [0.45, 0.58]
    MicroStrategy: [0.50, 0.50]
    Amazon: [0.80, 0.68] color: #ff9900
    Google: [0.72, 0.60] color: #4285f4`,

    // Architecture diagrams
    'architecture-simple': `architecture-beta
    group api(cloud)[API]

    service db(database)[Database] in api
    service disk1(disk)[Storage] in api
    service server(server)[Server] in api
    service gateway(internet)[Gateway]

    db:L -- R:server
    disk1:T -- B:server
    gateway:R --> L:server`,

    'architecture-complex': `architecture-beta
    title Complex Architecture

    group edge(cloud)[Edge]
    group platform(server)[Platform]
    group data(database)[Data]
    group observability(disk)[Observability] in platform

    service gateway(internet)[Gateway] in edge
    service web(internet)[Web App] in edge
    service api(server)[API] in edge
    service auth(server)[Auth] in edge

    service core(server)[Core] in platform
    service cache(disk)[Cache] in platform
    service queue(server)[Queue] in platform
    junction hub in platform

    service db(database)[Main DB] in data
    service search(disk)[Search] in data

    service metrics(disk)[Metrics] in observability
    service logs(disk)[Logs] in observability

    gateway:R --> L:web
    web:R --> L:api
    api:R -- L:auth
    api{group}:B -[jwt]- T:core{group}
    core:L -- R:queue
    core:R -- L:cache
    core:B -- T:hub
    hub:R -- L:metrics
    metrics:R -- L:logs
    core{group}:R -[sql]- L:db{group}
    db:B -- T:search
    cache{group}:B -[replicate]- R:db{group}`,

    // Git Graph diagrams
    'git-simple': `gitGraph
    commit id:"A"
    commit id:"B"
    branch feature
    checkout feature
    commit id:"C"
    checkout main
    commit id:"D"
    merge feature`,

    'git-complex': `gitGraph
    commit id:"A"
    commit id:"B"
    branch feature
    checkout feature
    commit id:"C"
    commit id:"D"
    checkout main
    commit id:"E"
    merge feature
    branch hotfix
    checkout hotfix
    commit id:"F"
    checkout main
    merge hotfix
    commit id:"G"
    branch release
    checkout release
    commit id:"H"
    commit id:"I"
    checkout main
    merge release`,

    // Timeline diagrams
    'timeline-simple': `timeline
    title History of Social Media Platform
    2002 : LinkedIn
    2004 : Facebook : Google
    2005 : YouTube
    2006 : Twitter`,

    'timeline-complex': `timeline
    title England's History Timeline
    section Stone Age
      7600 BC : Britain's oldest known house was built in Orkney, Scotland
      6000 BC : Sea levels rise and Britain becomes an island.
    section Bronze Age
      2300 BC : People arrive from Europe and settle in Britain.
              : New styles of pottery and ways of burying the dead appear.
      2200 BC : The last major building works are completed at Stonehenge.
              : The first metal objects are made in Britain.`,

    // Sankey diagrams
    'sankey-simple': `sankey-beta

Revenue,Salaries,40
Revenue,Operations,25
Revenue,Marketing,15
Revenue,R&D,12
Revenue,Profit,8`,

    'sankey-complex': `sankey-beta

Website,Homepage,100
Homepage,Products,45
Homepage,Blog,25
Homepage,Pricing,20
Homepage,Bounce,10
Products,Add to Cart,30
Products,Exit,15
Blog,Subscribe,10
Blog,Exit,15
Pricing,Sign Up,15
Pricing,Exit,5
Add to Cart,Checkout,25
Add to Cart,Abandon,5
Checkout,Purchase,22
Checkout,Abandon,3`,

    // C4 diagrams
    'c4-simple': `C4Context
title System Context diagram for Internet Banking System

Enterprise_Boundary(b0, "BankBoundary") {
    Person(customer, "Banking Customer", "A customer of the bank")
    System(banking, "Internet Banking System", "Allows customers to view accounts")
}

System_Ext(email, "E-mail System", "External e-mail system")

Rel(customer, banking, "Uses")
Rel(banking, email, "Sends e-mails", "SMTP")`,

    'c4-complex': `C4Container
title Container diagram for Internet Banking System

System_Ext(email_system, "E-Mail System", "The internal Microsoft Exchange system")
Person(customer, "Customer", "A customer of the bank, with personal bank accounts")

Container_Boundary(c1, "Internet Banking") {
    Container(spa, "Single-Page App", "JavaScript, Angular", "Provides all the Internet banking functionality")
    Container(api, "API Application", "Java, Spring MVC", "Provides banking functionality via JSON/HTTPS API")
    ContainerDb(db, "Database", "Oracle", "Stores user data, accounts, transactions")
    ContainerQueue(queue, "Message Broker", "RabbitMQ", "Handles async messaging")
}

Rel(customer, spa, "Uses", "HTTPS")
Rel(spa, api, "Makes API calls to", "JSON/HTTPS")
Rel(api, db, "Reads from and writes to", "JDBC")
Rel(api, queue, "Sends messages to")
Rel(email_system, customer, "Sends e-mails to")`,

    // Journey diagrams
    'journey-simple': `journey
    title My working day
    section Go to work
      Make tea: 5: Me
      Go upstairs: 3: Me
      Do work: 1: Me, Cat
    section Go home
      Go downstairs: 5: Me
      Sit down: 3: Me`,

    'journey-complex': `journey
    title E-Commerce User Journey
    section Discovery
      Visit homepage: 5: Customer
      Search for product: 4: Customer
      Browse categories: 3: Customer
    section Selection
      View product details: 5: Customer
      Read reviews: 4: Customer
      Compare prices: 3: Customer
      Add to cart: 5: Customer
    section Checkout
      Review cart: 4: Customer
      Enter shipping info: 3: Customer, System
      Select payment method: 4: Customer
      Complete purchase: 5: Customer, System
    section Post-Purchase
      Receive confirmation: 5: System
      Track shipment: 4: Customer
      Receive delivery: 5: Customer`,

    // XY Chart diagrams
    'xychart-simple': `xychart-beta
    title "Monthly Sales"
    x-axis [Jan, Feb, Mar, Apr, May, Jun]
    y-axis "Sales (units)" 0 --> 100
    bar [20, 35, 45, 62, 78, 91]
    line [15, 30, 40, 55, 70, 85]`,

    'xychart-complex': `xychart-beta
    title "Website Analytics"
    x-axis [Mon, Tue, Wed, Thu, Fri, Sat, Sun]
    y-axis "Visitors (thousands)" 0 --> 50
    bar [12, 18, 25, 22, 30, 45, 42]
    line [10, 15, 20, 18, 25, 40, 38]
    line [8, 12, 18, 15, 22, 35, 30]`,

    // Radar diagrams
    'radar-simple': `radar-beta
    title Skills Assessment
    axis Coding, Testing, Design
    axis Review["Code Review"], Docs["Documentation"]

    curve TeamA["Team Alpha"]{
        Coding 4, Testing 3,
        Design 3, Review 4,
        Docs 2
    }
    curve TeamB["Team Beta"]{3, 4, 4, 3, 5}

    showLegend true
    ticks 5
    max 5
    graticule polygon`,

    'radar-complex': `radar-beta
    title Programming Language Comparison
    axis Performance, Ecosystem, Safety
    axis Learning["Learning Curve"], Tooling, Community

    curve rust["Rust"]{
        Performance 5, Ecosystem 4,
        Safety 5, Learning 2,
        Tooling 5, Community 4
    }
    curve python["Python"]{
        Performance 2, Ecosystem 5,
        Safety 3, Learning 5,
        Tooling 4, Community 5
    }
    curve go["Go"]{
        Performance 4, Ecosystem 4,
        Safety 4, Learning 4,
        Tooling 5, Community 4
    }
    curve cpp["C++"]{5, 5, 2, 1, 3, 4}

    showLegend true
    ticks 5
    max 5
    min 0
    graticule circle`,

    // Block diagrams
    'block-simple': `block-beta
  columns 2
  block
    id2["I am a wide one"]
    id1
  end
  id["Next row"]`,

    'block-complex': `block-beta
  columns 3
  A["Square Block"]
  B("Rounded Block")
  C{"Diamond"}

  block:container
    columns 2
    D["Nested 1"]
    E["Nested 2"]
  end
  space
  F(["Stadium"])

  G --> A
  B --> C
  D -- "labeled" --> E

  classDef blue fill:#66f,stroke:#333,stroke-width:2px;
  class A blue
  style B fill:#f9F,stroke:#333,stroke-width:4px`,

    // Packet diagrams
    'packet-simple': `packet
    title Simple Packet
    0-7: "Header"
    8-15: "Length"
    16-31: "Data"`,

    'packet-complex': `packet
    title TCP Packet Structure
    0-15: "Source Port"
    16-31: "Destination Port"
    32-63: "Sequence Number"
    64-95: "Acknowledgment Number"
    96-99: "Data Offset"
    100-105: "Reserved"
    106: "URG"
    107: "ACK"
    108: "PSH"
    109: "RST"
    110: "SYN"
    111: "FIN"
    112-127: "Window"
    128-143: "Checksum"
    144-159: "Urgent Pointer"
    160-191: "(Options and Padding)"
    192-255: "Data"`,

    // Treemap diagrams
    'treemap-simple': `treemap-beta
"Category A"
    "Item A1": 10
    "Item A2": 20
"Category B"
    "Item B1": 15
    "Item B2": 25`,

    'treemap-complex': `treemap-beta
"Company Budget"
    "Engineering":::engineering
        "Frontend": 300000
        "Backend": 400000
        "DevOps": 200000
    "Marketing":::marketing
        "Digital": 250000
        "Print": 100000
        "Events": 150000
    "Sales":::sales
        "Direct": 500000
        "Channel": 300000

classDef engineering fill:#6b9bc3,stroke:#333;
classDef marketing fill:#c36b9b,stroke:#333;
classDef sales fill:#c3a66b,stroke:#333;`,
};

// State
let selkie = null;
let currentZoom = 1;
let currentTheme = 'default';
let renderTimeout = null;
let lastSvg = '';

// Theme backgrounds (must match Rust theme definitions)
const themeBackgrounds = {
    'default': '#ffffff',
    'dark': '#1f2020',
    'forest': '#ffffff',
    'neutral': '#ffffff',
    'base': '#f4f4f4',
};

// DOM Elements
const editor = document.getElementById('editor');
const preview = document.getElementById('preview');
const errorDisplay = document.getElementById('error-display');
const renderTimeDisplay = document.getElementById('render-time');
const themeSelect = document.getElementById('theme-select');
const exampleSelect = document.getElementById('example-select');
const loadingOverlay = document.getElementById('loading-overlay');
const divider = document.getElementById('divider');
const previewContainer = document.getElementById('preview-container');

// Initialize the application
async function init() {
    try {
        await loadSelkie();
        setupEventListeners();
        loadFromUrl();
        loadingOverlay.classList.add('hidden');
    } catch (error) {
        console.error('Failed to initialize:', error);
        loadingOverlay.querySelector('p').textContent =
            `Failed to load: ${error.message}`;
    }
}

// Load Selkie WASM module
async function loadSelkie() {
    const { default: initWasm, initialize, parse, render, render_text } =
        await import('./pkg/selkie.js');

    await initWasm();
    initialize({ startOnLoad: false });

    selkie = { parse, render, render_text };
}

// Set up event listeners
function setupEventListeners() {
    // Editor input with debounce
    editor.addEventListener('input', () => {
        clearTimeout(renderTimeout);
        renderTimeout = setTimeout(renderDiagram, 150);
        updateUrl();
    });

    // Tab key support in editor
    editor.addEventListener('keydown', (e) => {
        if (e.key === 'Tab') {
            e.preventDefault();
            const start = editor.selectionStart;
            const end = editor.selectionEnd;
            editor.value = editor.value.substring(0, start) + '  ' + editor.value.substring(end);
            editor.selectionStart = editor.selectionEnd = start + 2;
        }
    });

    // Example selector
    exampleSelect.addEventListener('change', (e) => {
        const exampleKey = e.target.value;
        if (exampleKey && examples[exampleKey]) {
            editor.value = examples[exampleKey];
            renderDiagram();
            updateUrl();
        }
    });

    // Theme selector
    themeSelect.addEventListener('change', (e) => {
        currentTheme = e.target.value;
        updatePreviewBackground();
        renderDiagram();
        updateUrl();
    });

    // Zoom controls
    document.getElementById('zoom-in').addEventListener('click', () => {
        currentZoom = Math.min(currentZoom + 0.25, 3);
        applyZoom();
    });

    document.getElementById('zoom-out').addEventListener('click', () => {
        currentZoom = Math.max(currentZoom - 0.25, 0.25);
        applyZoom();
    });

    document.getElementById('zoom-reset').addEventListener('click', () => {
        currentZoom = 1;
        applyZoom();
    });

    // Download SVG
    document.getElementById('download-svg').addEventListener('click', downloadSvg);

    // Resizable divider
    setupDividerDrag();
}

// Render the current diagram
function renderDiagram() {
    const input = editor.value.trim();

    if (!input) {
        preview.innerHTML = '<p style="color: var(--text-secondary)">Enter a diagram to preview</p>';
        errorDisplay.classList.add('hidden');
        renderTimeDisplay.textContent = '';
        return;
    }

    // Prepend theme directive if not using default theme
    let diagramInput = input;
    if (currentTheme !== 'default') {
        // Check if the diagram already has a theme directive
        const hasThemeDirective = /%%\{.*"theme"\s*:/i.test(input);
        if (!hasThemeDirective) {
            diagramInput = `%%{init: {"theme": "${currentTheme}"}}%%\n${input}`;
        }
    }

    try {
        const startTime = performance.now();
        const result = selkie.render('diagram', diagramInput);
        const endTime = performance.now();

        lastSvg = result.svg;
        preview.innerHTML = result.svg;
        errorDisplay.classList.add('hidden');

        const renderTime = (endTime - startTime).toFixed(2);
        renderTimeDisplay.textContent = `Rendered in ${renderTime}ms`;

        applyZoom();
    } catch (error) {
        errorDisplay.textContent = error.message || String(error);
        errorDisplay.classList.remove('hidden');
        renderTimeDisplay.textContent = '';
    }
}

// Apply zoom to preview
function applyZoom() {
    preview.style.transform = `scale(${currentZoom})`;
    document.getElementById('zoom-reset').textContent = `${Math.round(currentZoom * 100)}%`;
}

// Update preview background to match theme
function updatePreviewBackground() {
    const bgColor = themeBackgrounds[currentTheme] || themeBackgrounds['default'];
    previewContainer.style.backgroundColor = bgColor;
}

// Download SVG file
function downloadSvg() {
    if (!lastSvg) return;

    const blob = new Blob([lastSvg], { type: 'image/svg+xml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'diagram.svg';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

// Setup divider drag functionality
function setupDividerDrag() {
    let isDragging = false;
    const editorPane = document.querySelector('.editor-pane');
    const previewPane = document.querySelector('.preview-pane');

    divider.addEventListener('mousedown', (e) => {
        isDragging = true;
        divider.classList.add('dragging');
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
    });

    document.addEventListener('mousemove', (e) => {
        if (!isDragging) return;

        const containerRect = document.querySelector('main').getBoundingClientRect();
        const percentage = ((e.clientX - containerRect.left) / containerRect.width) * 100;

        if (percentage > 20 && percentage < 80) {
            editorPane.style.flex = `0 0 ${percentage}%`;
            previewPane.style.flex = `0 0 ${100 - percentage}%`;
        }
    });

    document.addEventListener('mouseup', () => {
        if (isDragging) {
            isDragging = false;
            divider.classList.remove('dragging');
            document.body.style.cursor = '';
            document.body.style.userSelect = '';
        }
    });
}

// URL state management
function updateUrl() {
    const code = editor.value;
    if (code) {
        const encoded = btoa(encodeURIComponent(code));
        // Include theme in URL if not default
        const themePrefix = currentTheme !== 'default' ? `${currentTheme}:` : '';
        history.replaceState(null, '', `#${themePrefix}${encoded}`);
    } else {
        history.replaceState(null, '', window.location.pathname);
    }
}

function loadFromUrl() {
    const hash = window.location.hash.slice(1);
    if (hash) {
        try {
            // Check for theme prefix (format: "theme:base64code" or just "base64code")
            let theme = 'default';
            let codeHash = hash;

            const colonIndex = hash.indexOf(':');
            if (colonIndex > 0 && colonIndex < 10) {
                // Potential theme prefix (themes are short names)
                const potentialTheme = hash.substring(0, colonIndex);
                if (themeBackgrounds[potentialTheme]) {
                    theme = potentialTheme;
                    codeHash = hash.substring(colonIndex + 1);
                }
            }

            const decoded = decodeURIComponent(atob(codeHash));
            editor.value = decoded;
            currentTheme = theme;
            themeSelect.value = theme;
            updatePreviewBackground();
            renderDiagram();
            return;
        } catch (e) {
            console.warn('Failed to decode URL hash:', e);
        }
    }

    // Load default example
    editor.value = examples['flowchart-simple'];
    updatePreviewBackground();
    renderDiagram();
}

// Start the application
init();
