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
};

// State
let selkie = null;
let currentZoom = 1;
let renderTimeout = null;
let lastSvg = '';

// DOM Elements
const editor = document.getElementById('editor');
const preview = document.getElementById('preview');
const errorDisplay = document.getElementById('error-display');
const renderTimeDisplay = document.getElementById('render-time');
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

    try {
        const startTime = performance.now();
        const result = selkie.render('diagram', input);
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
        history.replaceState(null, '', `#${encoded}`);
    } else {
        history.replaceState(null, '', window.location.pathname);
    }
}

function loadFromUrl() {
    const hash = window.location.hash.slice(1);
    if (hash) {
        try {
            const decoded = decodeURIComponent(atob(hash));
            editor.value = decoded;
            renderDiagram();
            return;
        } catch (e) {
            console.warn('Failed to decode URL hash:', e);
        }
    }

    // Load default example
    editor.value = examples['flowchart-simple'];
    renderDiagram();
}

// Start the application
init();
