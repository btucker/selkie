#!/usr/bin/env node
/**
 * Performance benchmark comparing Selkie (Rust) vs Mermaid.js
 *
 * Tests Mermaid.js in two environments:
 * - JSDOM (Node.js, slower due to DOM emulation)
 * - Playwright (real Chromium browser, faster and fairer comparison)
 *
 * Usage: node benchmark.mjs
 */

import { execSync } from 'child_process';
import { JSDOM } from 'jsdom';

let chromium;
try {
  chromium = (await import('playwright')).chromium;
} catch (e) {
  chromium = null;
}

// Test diagrams of varying complexity
const testCases = [
  {
    name: 'Simple flowchart (5 nodes)',
    diagram: `flowchart LR
    A[Start] --> B{Decision}
    B -->|Yes| C[OK]
    B -->|No| D[Cancel]
    C --> E[End]
    D --> E`,
  },
  {
    name: 'Medium flowchart (15 nodes)',
    diagram: `flowchart TB
    A[Start] --> B{Check 1}
    B -->|Yes| C[Process 1]
    B -->|No| D[Process 2]
    C --> E{Check 2}
    D --> E
    E -->|Yes| F[Action 1]
    E -->|No| G[Action 2]
    F --> H[Step 1]
    G --> H
    H --> I{Check 3}
    I -->|Yes| J[Final 1]
    I -->|No| K[Final 2]
    J --> L[End]
    K --> L
    L --> M[Done]`,
  },
  {
    name: 'Sequence diagram (4 actors)',
    diagram: `sequenceDiagram
    participant A as Alice
    participant B as Bob
    participant C as Charlie
    participant D as Dave
    A->>B: Hello Bob!
    B-->>A: Hi Alice!
    A->>C: Hello Charlie
    C->>D: Forward to Dave
    D-->>C: Response
    C-->>A: Got response
    A->>B: Update Bob
    B-->>A: Acknowledged`,
  },
  {
    name: 'Class diagram (5 classes)',
    diagram: `classDiagram
    Animal <|-- Dog
    Animal <|-- Cat
    Animal <|-- Bird
    Animal : +String name
    Animal : +int age
    Animal: +eat()
    Animal: +sleep()
    class Dog{
        +String breed
        +bark()
        +fetch()
    }
    class Cat{
        +bool indoor
        +meow()
        +scratch()
    }
    class Bird{
        +float wingspan
        +fly()
        +sing()
    }`,
  },
  {
    name: 'State diagram (8 states)',
    diagram: `stateDiagram-v2
    [*] --> Idle
    Idle --> Starting : start
    Starting --> Running : initialized
    Running --> Paused : pause
    Paused --> Running : resume
    Running --> Stopping : stop
    Stopping --> Idle : stopped
    Running --> Error : error
    Error --> Idle : reset
    Error --> [*] : fatal`,
  },
  {
    name: 'ER diagram (4 entities)',
    diagram: `erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains
    PRODUCT ||--o{ LINE_ITEM : includes
    CUSTOMER {
        int id PK
        string name
        string email
        string address
    }
    ORDER {
        int id PK
        date created
        string status
        float total
    }
    PRODUCT {
        int id PK
        string name
        float price
        int stock
    }
    LINE_ITEM {
        int quantity
        float unit_price
    }`,
  },
  {
    name: 'Gantt chart (10 tasks)',
    diagram: `gantt
    title Project Timeline
    dateFormat YYYY-MM-DD
    section Planning
    Requirements :a1, 2024-01-01, 5d
    Design      :a2, after a1, 3d
    Review      :a3, after a2, 2d
    section Development
    Backend     :b1, after a3, 10d
    Frontend    :b2, after a3, 8d
    API         :b3, after b1, 5d
    section Testing
    Unit Tests  :c1, after b2, 3d
    Integration :c2, after b3, 4d
    QA          :c3, after c2, 5d
    Deploy      :c4, after c3, 1d`,
  },
  {
    name: 'Pie chart (6 segments)',
    diagram: `pie title Project Hours
    "Development" : 40
    "Testing" : 20
    "Documentation" : 15
    "Design" : 10
    "Meetings" : 10
    "Other" : 5`,
  },
  {
    name: 'Large flowchart (100 nodes)',
    diagram: generateLargeFlowchart(100, 200),
  },
];

// Generate a large flowchart with N nodes and approximately M edges
function generateLargeFlowchart(nodeCount, edgeCount) {
  const lines = ['flowchart TB'];

  // Create nodes in groups (like layers in a neural network)
  const layerSize = 10;
  const layers = Math.ceil(nodeCount / layerSize);

  // Generate node definitions
  for (let i = 0; i < nodeCount; i++) {
    const layer = Math.floor(i / layerSize);
    const shapes = ['[', '(', '{', '([', '[['];
    const closeShapes = [']', ')', '}', '])', ']]'];
    const shapeIdx = i % shapes.length;
    lines.push(`    N${i}${shapes[shapeIdx]}Node ${i}${closeShapes[shapeIdx]}`);
  }

  // Generate edges - connect nodes between adjacent layers
  let edgesAdded = 0;

  // Connect each node to 2-3 nodes in the next layer
  for (let layer = 0; layer < layers - 1 && edgesAdded < edgeCount; layer++) {
    const layerStart = layer * layerSize;
    const layerEnd = Math.min(layerStart + layerSize, nodeCount);
    const nextLayerStart = (layer + 1) * layerSize;
    const nextLayerEnd = Math.min(nextLayerStart + layerSize, nodeCount);

    for (let i = layerStart; i < layerEnd && edgesAdded < edgeCount; i++) {
      // Connect to 2-3 nodes in the next layer
      const connections = 2 + (i % 2);
      for (let c = 0; c < connections && edgesAdded < edgeCount; c++) {
        const targetIdx = nextLayerStart + ((i - layerStart + c) % (nextLayerEnd - nextLayerStart));
        if (targetIdx < nodeCount) {
          const edgeTypes = ['-->', '---', '-.->', '===>', '-.->'];
          const edgeType = edgeTypes[edgesAdded % edgeTypes.length];
          lines.push(`    N${i} ${edgeType} N${targetIdx}`);
          edgesAdded++;
        }
      }
    }
  }

  // Add some cross-layer connections to reach edge target
  for (let i = 0; edgesAdded < edgeCount && i < nodeCount - 2; i++) {
    const target = Math.min(i + 2 + (i % 3), nodeCount - 1);
    if (target !== i) {
      lines.push(`    N${i} --> N${target}`);
      edgesAdded++;
    }
  }

  return lines.join('\n');
}

const WARMUP_RUNS = 3;
const BENCHMARK_RUNS = 10;
const SELKIE_PATH = process.env.SELKIE_PATH || './target/release/selkie';

// ============== JSDOM-based Mermaid.js ==============

async function setupJsdom() {
  const dom = new JSDOM('<!DOCTYPE html><html><body><div id="mermaid"></div></body></html>', {
    pretendToBeVisual: true,
  });
  global.window = dom.window;
  global.document = dom.window.document;
  Object.defineProperty(global, 'navigator', {
    value: dom.window.navigator,
    writable: true,
  });

  const mockBBox = { x: 0, y: 0, width: 100, height: 20 };
  const originalCreateElementNS = document.createElementNS.bind(document);
  document.createElementNS = function(namespaceURI, qualifiedName) {
    const element = originalCreateElementNS(namespaceURI, qualifiedName);
    if (namespaceURI === 'http://www.w3.org/2000/svg') {
      element.getBBox = () => mockBBox;
      element.getComputedTextLength = () => mockBBox.width;
      element.getBoundingClientRect = () => ({
        ...mockBBox,
        top: 0, left: 0, right: mockBBox.width, bottom: mockBBox.height,
        toJSON: () => mockBBox,
      });
    }
    return element;
  };
  global.window.getComputedStyle = () => ({
    getPropertyValue: () => '',
  });

  const mermaid = (await import('mermaid')).default;
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'loose',
    flowchart: { htmlLabels: false, useMaxWidth: false },
    theme: 'default',
  });

  return mermaid;
}

async function benchmarkMermaidJsdom(mermaid, diagram, runId) {
  const times = [];

  // Warmup
  for (let i = 0; i < WARMUP_RUNS; i++) {
    await mermaid.render(`jsdom-warmup-${runId}-${i}`, diagram);
  }

  // Benchmark
  for (let i = 0; i < BENCHMARK_RUNS; i++) {
    const start = performance.now();
    await mermaid.render(`jsdom-bench-${runId}-${i}`, diagram);
    const end = performance.now();
    times.push(end - start);
  }

  return times;
}

// ============== Playwright-based Mermaid.js (optional) ==============

async function setupPlaywright() {
  if (!chromium) return null;

  try {
    const browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();

    await page.setContent(`
      <!DOCTYPE html>
      <html>
      <head>
        <script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"></script>
      </head>
      <body>
        <div id="container"></div>
        <script>
          mermaid.initialize({
            startOnLoad: false,
            securityLevel: 'loose',
            flowchart: { htmlLabels: true, useMaxWidth: false },
            theme: 'default',
          });

          window.renderDiagram = async function(diagram, id) {
            const container = document.getElementById('container');
            container.innerHTML = '';
            const { svg } = await mermaid.render('diagram-' + id, diagram);
            return svg;
          };
        </script>
      </body>
      </html>
    `);

    await page.waitForFunction(() => typeof window.mermaid !== 'undefined');
    return { browser, page };
  } catch (e) {
    console.log('Playwright setup failed:', e.message);
    return null;
  }
}

async function benchmarkMermaidPlaywright(page, diagram, runId) {
  const times = [];

  for (let i = 0; i < WARMUP_RUNS; i++) {
    await page.evaluate(
      ({ d, id }) => window.renderDiagram(d, id),
      { d: diagram, id: `pw-warmup-${runId}-${i}` }
    );
  }

  for (let i = 0; i < BENCHMARK_RUNS; i++) {
    const start = performance.now();
    await page.evaluate(
      ({ d, id }) => window.renderDiagram(d, id),
      { d: diagram, id: `pw-bench-${runId}-${i}` }
    );
    const end = performance.now();
    times.push(end - start);
  }

  return times;
}

// ============== Selkie (Rust) ==============

function benchmarkSelkie(diagram) {
  const times = [];

  // Warmup
  for (let i = 0; i < WARMUP_RUNS; i++) {
    try {
      execSync(`echo '${diagram.replace(/'/g, "'\\''")}' | ${SELKIE_PATH} -i - -o -`, {
        encoding: 'utf-8',
        stdio: ['pipe', 'pipe', 'pipe'],
      });
    } catch (e) {
      // Ignore errors during warmup
    }
  }

  // Benchmark
  for (let i = 0; i < BENCHMARK_RUNS; i++) {
    const start = performance.now();
    try {
      execSync(`echo '${diagram.replace(/'/g, "'\\''")}' | ${SELKIE_PATH} -i - -o -`, {
        encoding: 'utf-8',
        stdio: ['pipe', 'pipe', 'pipe'],
      });
    } catch (e) {
      times.push(NaN);
      continue;
    }
    const end = performance.now();
    times.push(end - start);
  }

  return times;
}

// ============== Utilities ==============

function median(arr) {
  const sorted = [...arr].filter(x => !isNaN(x)).sort((a, b) => a - b);
  if (sorted.length === 0) return NaN;
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

function mean(arr) {
  const valid = arr.filter(x => !isNaN(x));
  if (valid.length === 0) return NaN;
  return valid.reduce((a, b) => a + b, 0) / valid.length;
}

// ============== Main ==============

async function main() {
  console.log('Selkie vs Mermaid.js Performance Benchmark');
  console.log('='.repeat(70));
  console.log(`Warmup runs: ${WARMUP_RUNS}, Benchmark runs: ${BENCHMARK_RUNS}`);
  console.log('');

  // Set up JSDOM (always available)
  console.log('Setting up JSDOM environment...');
  const mermaidJsdom = await setupJsdom();

  // Try Playwright (optional, requires browser)
  console.log('Setting up Playwright (Chromium)...');
  const playwright = await setupPlaywright();
  const usePlaywright = playwright !== null;

  if (usePlaywright) {
    console.log('Using Playwright for Mermaid.js benchmarks (real browser)');
  } else {
    console.log('Playwright not available, using JSDOM for Mermaid.js benchmarks');
  }

  console.log('');

  const results = [];

  for (let idx = 0; idx < testCases.length; idx++) {
    const testCase = testCases[idx];
    process.stdout.write(`Benchmarking: ${testCase.name}... `);

    let mermaidTimes;
    if (usePlaywright) {
      mermaidTimes = await benchmarkMermaidPlaywright(playwright.page, testCase.diagram, idx);
    } else {
      mermaidTimes = await benchmarkMermaidJsdom(mermaidJsdom, testCase.diagram, idx);
    }
    const selkieTimes = benchmarkSelkie(testCase.diagram);

    const mermaidMedian = median(mermaidTimes);
    const selkieMedian = median(selkieTimes);
    const speedup = mermaidMedian / selkieMedian;

    results.push({
      name: testCase.name,
      mermaidMs: mermaidMedian,
      selkieMs: selkieMedian,
      speedup,
    });

    console.log('done');
  }

  if (usePlaywright) {
    await playwright.browser.close();
  }

  console.log('');
  console.log('Results (median render time in milliseconds):');
  console.log('-'.repeat(75));
  console.log('| Diagram                          | Mermaid.js |  Selkie  | Speedup |');
  console.log('|----------------------------------|------------|----------|---------|');

  for (const r of results) {
    const name = r.name.padEnd(32);
    const mermaid = isNaN(r.mermaidMs) ? 'N/A'.padStart(10) : `${r.mermaidMs.toFixed(1)} ms`.padStart(10);
    const selkie = isNaN(r.selkieMs) ? 'N/A'.padStart(8) : `${r.selkieMs.toFixed(1)} ms`.padStart(8);
    const speedup = isNaN(r.speedup) ? 'N/A'.padStart(7) : `${r.speedup.toFixed(1)}x`.padStart(7);
    console.log(`| ${name} | ${mermaid} | ${selkie} | ${speedup} |`);
  }

  console.log('-'.repeat(75));

  // Overall average speedup
  const validSpeedups = results.map(r => r.speedup).filter(x => !isNaN(x) && isFinite(x));
  if (validSpeedups.length > 0) {
    const avgSpeedup = mean(validSpeedups);
    console.log(`\nAverage speedup: ${avgSpeedup.toFixed(1)}x faster than Mermaid.js`);
  }

  // Output as markdown table for README
  const env = usePlaywright ? 'Chromium browser' : 'Node.js/JSDOM';
  console.log('\n\n=== Markdown for README ===\n');
  console.log('| Diagram | Mermaid.js | Selkie | Speedup |');
  console.log('|---------|------------|--------|---------|');
  for (const r of results) {
    const name = r.name;
    const mermaid = isNaN(r.mermaidMs) ? 'N/A' : `${r.mermaidMs.toFixed(1)}ms`;
    const selkie = isNaN(r.selkieMs) ? 'N/A' : `${r.selkieMs.toFixed(1)}ms`;
    const speedup = isNaN(r.speedup) ? 'N/A' : `**${r.speedup.toFixed(1)}x**`;
    console.log(`| ${name} | ${mermaid} | ${selkie} | ${speedup} |`);
  }

  const avgSpeedup = mean(validSpeedups);
  console.log(`\n_Mermaid.js v11 in ${env}. Median of ${BENCHMARK_RUNS} runs after ${WARMUP_RUNS} warmup._`);
}

main().catch(console.error);
