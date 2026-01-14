#!/usr/bin/env node
/**
 * Browser performance benchmark comparing Selkie WASM vs Mermaid.js
 *
 * Measures render time in a real Chromium browser for fair comparison.
 * Both libraries run in the same environment with identical test cases.
 *
 * Usage: npm run benchmark
 */

import { readFileSync, existsSync, writeFileSync } from 'fs';
import { chromium } from 'playwright';

const PKG_PATH = '../../pkg';
const WARMUP_RUNS = 2;
const BENCHMARK_RUNS = 10;

// Check if the wasm package has been built
if (!existsSync(`${PKG_PATH}/selkie.js`)) {
  console.error('Error: WASM package not found. Run: npm run build');
  console.error('Note: Requires rustup with wasm32-unknown-unknown target');
  process.exit(1);
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
    Idle --> [*] : shutdown`,
  },
  {
    name: 'Pie chart (5 slices)',
    diagram: `pie title Browser Market Share
    "Chrome" : 65
    "Safari" : 19
    "Firefox" : 8
    "Edge" : 5
    "Other" : 3`,
  },
];

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

function formatTime(ms) {
  if (ms < 1) return `${(ms * 1000).toFixed(0)}μs`;
  if (ms < 1000) return `${ms.toFixed(2)}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

async function runBenchmark() {
  console.log('Selkie.js vs Mermaid.js Browser Benchmark\n');
  console.log('='.repeat(60));
  console.log(`Warmup: ${WARMUP_RUNS} runs, Benchmark: ${BENCHMARK_RUNS} runs (median reported)`);
  console.log('='.repeat(60));

  const browser = await chromium.launch();
  const page = await browser.newPage();

  // Load selkie WASM
  const selkieJs = readFileSync(`${PKG_PATH}/selkie.js`, 'utf-8');
  const selkieWasm = readFileSync(`${PKG_PATH}/selkie_bg.wasm`);
  const wasmBase64 = selkieWasm.toString('base64');

  await page.setContent(`
    <!DOCTYPE html>
    <html>
    <head>
      <title>Benchmark</title>
      <script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"></script>
    </head>
    <body>
      <div id="mermaid-container"></div>
      <script type="module">
        ${selkieJs}

        // Load WASM from base64
        const wasmBytes = Uint8Array.from(atob('${wasmBase64}'), c => c.charCodeAt(0));

        window.benchmarkReady = (async () => {
          // Initialize selkie
          await __wbg_init(wasmBytes);
          window.selkie = { render_text };

          // Initialize mermaid
          mermaid.initialize({ startOnLoad: false });
          window.mermaidRender = async (id, diagram) => {
            const container = document.getElementById('mermaid-container');
            container.innerHTML = '';
            const { svg } = await mermaid.render(id, diagram);
            return svg;
          };
        })();
      </script>
    </body>
    </html>
  `);

  // Wait for both to be ready
  await page.waitForFunction(() => window.benchmarkReady, { timeout: 30000 });
  await page.evaluate(() => window.benchmarkReady);

  const results = [];

  for (const tc of testCases) {
    console.log(`\n${tc.name}:`);

    // Benchmark selkie
    const selkieTimes = [];
    for (let i = 0; i < WARMUP_RUNS + BENCHMARK_RUNS; i++) {
      const time = await page.evaluate((diagram) => {
        const start = performance.now();
        window.selkie.render_text(diagram);
        return performance.now() - start;
      }, tc.diagram);
      if (i >= WARMUP_RUNS) selkieTimes.push(time);
    }
    const selkieMedian = median(selkieTimes);

    // Benchmark mermaid
    const mermaidTimes = [];
    for (let i = 0; i < WARMUP_RUNS + BENCHMARK_RUNS; i++) {
      const time = await page.evaluate(async (diagram, idx) => {
        const start = performance.now();
        await window.mermaidRender(`bench-${idx}`, diagram);
        return performance.now() - start;
      }, tc.diagram, i);
      if (i >= WARMUP_RUNS) mermaidTimes.push(time);
    }
    const mermaidMedian = median(mermaidTimes);

    const speedup = mermaidMedian / selkieMedian;
    console.log(`  Selkie:   ${formatTime(selkieMedian)}`);
    console.log(`  Mermaid:  ${formatTime(mermaidMedian)}`);
    console.log(`  Speedup:  ${speedup.toFixed(1)}x`);

    results.push({
      name: tc.name,
      selkie: selkieMedian,
      mermaid: mermaidMedian,
      speedup,
    });
  }

  await browser.close();

  // Calculate bundle size
  const wasmSize = selkieWasm.length;
  const jsSize = selkieJs.length;
  const totalSize = wasmSize + jsSize;

  // Print summary table
  console.log('\n' + '='.repeat(60));
  console.log('Summary (Browser Benchmark)\n');

  console.log('| Diagram | Mermaid.js | Selkie | Speedup |');
  console.log('|---------|------------|--------|---------|');
  for (const r of results) {
    console.log(`| ${r.name} | ${formatTime(r.mermaid)} | ${formatTime(r.selkie)} | ${r.speedup.toFixed(1)}x |`);
  }

  console.log(`\nBundle Size:`);
  console.log(`  WASM:     ${(wasmSize / 1024).toFixed(1)} KB`);
  console.log(`  JS glue:  ${(jsSize / 1024).toFixed(1)} KB`);
  console.log(`  Total:    ${(totalSize / 1024).toFixed(1)} KB`);

  // Save results as JSON for README generation
  const output = {
    results,
    bundleSize: {
      wasm: wasmSize,
      js: jsSize,
      total: totalSize,
    },
    config: {
      warmupRuns: WARMUP_RUNS,
      benchmarkRuns: BENCHMARK_RUNS,
    },
    timestamp: new Date().toISOString(),
  };

  writeFileSync('benchmark-results.json', JSON.stringify(output, null, 2));
  console.log('\nResults saved to benchmark-results.json');

  console.log('='.repeat(60));
}

runBenchmark().catch(e => {
  console.error('Benchmark error:', e);
  process.exit(1);
});
