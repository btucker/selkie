#!/usr/bin/env node
/**
 * CLI-to-CLI Benchmark: Selkie vs mermaid-cli (mmdc)
 *
 * This benchmark measures actual command-line execution time for both tools,
 * providing a fair comparison that includes startup overhead for both.
 *
 * Prerequisites:
 *   - Selkie: cargo build --release
 *   - mermaid-cli: npm install -g @mermaid-js/mermaid-cli
 *
 * Usage: node cli-benchmark.mjs
 */

import { execSync, spawnSync } from 'child_process';
import { writeFileSync, unlinkSync, existsSync, mkdirSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

// Test diagrams
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
    F --> H[Merge]
    G --> H
    H --> I{Final Check}
    I -->|Pass| J[Success]
    I -->|Fail| K[Retry]
    K --> B
    J --> L[Log]
    L --> M[End]`,
  },
  {
    name: 'Sequence diagram (4 actors)',
    diagram: `sequenceDiagram
    participant A as Alice
    participant B as Bob
    participant C as Server
    participant D as Database
    A->>B: Hello Bob!
    B->>C: Request data
    C->>D: Query
    D-->>C: Results
    C-->>B: Response
    B-->>A: Here you go!`,
  },
  {
    name: 'Class diagram (5 classes)',
    diagram: `classDiagram
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
    }`,
  },
  {
    name: 'Large flowchart (100 nodes)',
    diagram: generateLargeFlowchart(100, 200),
  },
];

function generateLargeFlowchart(nodeCount, edgeCount) {
  const lines = ['flowchart TB'];
  const layerSize = 10;
  const layers = Math.ceil(nodeCount / layerSize);

  for (let i = 0; i < nodeCount; i++) {
    const shapes = ['[', '(', '{', '([', '[['];
    const closeShapes = [']', ')', '}', '])', ']]'];
    const shapeIdx = i % shapes.length;
    lines.push(`    N${i}${shapes[shapeIdx]}Node ${i}${closeShapes[shapeIdx]}`);
  }

  let edgesAdded = 0;
  for (let layer = 0; layer < layers - 1 && edgesAdded < edgeCount; layer++) {
    const layerStart = layer * layerSize;
    const layerEnd = Math.min(layerStart + layerSize, nodeCount);
    const nextLayerStart = (layer + 1) * layerSize;
    const nextLayerEnd = Math.min(nextLayerStart + layerSize, nodeCount);

    for (let i = layerStart; i < layerEnd && edgesAdded < edgeCount; i++) {
      const connections = 2 + (i % 2);
      for (let c = 0; c < connections && edgesAdded < edgeCount; c++) {
        const targetIdx = nextLayerStart + ((i - layerStart + c) % (nextLayerEnd - nextLayerStart));
        if (targetIdx < nodeCount) {
          lines.push(`    N${i} --> N${targetIdx}`);
          edgesAdded++;
        }
      }
    }
  }

  return lines.join('\n');
}

const BENCHMARK_RUNS = 5;
const WARMUP_RUNS = 2;

// Paths
const SELKIE_PATH = process.env.SELKIE_PATH || '../../target/release/selkie';
const MMDC_PATH = process.env.MMDC_PATH || 'mmdc';

// Temp directory for test files
const tempDir = join(tmpdir(), 'selkie-benchmark');
if (!existsSync(tempDir)) {
  mkdirSync(tempDir, { recursive: true });
}

function checkCommand(cmd) {
  try {
    execSync(`which ${cmd}`, { stdio: 'ignore' });
    return true;
  } catch {
    // Try running directly (might be a path)
    try {
      execSync(`${cmd} --version`, { stdio: 'ignore', timeout: 5000 });
      return true;
    } catch {
      return false;
    }
  }
}

function timeCommand(cmd, args, inputFile, outputFile) {
  const start = process.hrtime.bigint();
  const result = spawnSync(cmd, args, {
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: 60000, // 60 second timeout
  });
  const end = process.hrtime.bigint();

  if (result.status !== 0) {
    const stderr = result.stderr?.toString() || '';
    return { success: false, error: stderr, timeMs: 0 };
  }

  const timeMs = Number(end - start) / 1_000_000;
  return { success: true, timeMs };
}

function benchmarkSelkie(diagram, inputFile, outputFile) {
  writeFileSync(inputFile, diagram);
  const times = [];

  // Warmup
  for (let i = 0; i < WARMUP_RUNS; i++) {
    timeCommand(SELKIE_PATH, ['-i', inputFile, '-o', outputFile], inputFile, outputFile);
  }

  // Benchmark
  for (let i = 0; i < BENCHMARK_RUNS; i++) {
    const result = timeCommand(SELKIE_PATH, ['-i', inputFile, '-o', outputFile], inputFile, outputFile);
    if (result.success) {
      times.push(result.timeMs);
    }
  }

  return times;
}

function benchmarkMmdc(diagram, inputFile, outputFile) {
  writeFileSync(inputFile, diagram);
  const times = [];

  // Warmup
  for (let i = 0; i < WARMUP_RUNS; i++) {
    timeCommand(MMDC_PATH, ['-i', inputFile, '-o', outputFile], inputFile, outputFile);
  }

  // Benchmark
  for (let i = 0; i < BENCHMARK_RUNS; i++) {
    const result = timeCommand(MMDC_PATH, ['-i', inputFile, '-o', outputFile], inputFile, outputFile);
    if (result.success) {
      times.push(result.timeMs);
    }
  }

  return times;
}

function median(arr) {
  if (arr.length === 0) return NaN;
  const sorted = [...arr].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

function mean(arr) {
  if (arr.length === 0) return NaN;
  return arr.reduce((a, b) => a + b, 0) / arr.length;
}

async function main() {
  console.log('Selkie vs mermaid-cli (mmdc) CLI Benchmark');
  console.log('='.repeat(60));
  console.log(`Warmup runs: ${WARMUP_RUNS}, Benchmark runs: ${BENCHMARK_RUNS}`);
  console.log('');

  // Check for available tools
  const hasSelkie = checkCommand(SELKIE_PATH);
  const hasMmdc = checkCommand(MMDC_PATH);

  console.log(`Selkie (${SELKIE_PATH}): ${hasSelkie ? 'found' : 'NOT FOUND'}`);
  console.log(`mmdc (${MMDC_PATH}): ${hasMmdc ? 'found' : 'NOT FOUND'}`);
  console.log('');

  if (!hasSelkie) {
    console.error('Error: Selkie not found. Build with: cargo build --release');
    process.exit(1);
  }

  if (!hasMmdc) {
    console.error('Error: mmdc not found. Install with: npm install -g @mermaid-js/mermaid-cli');
    console.error('Note: mermaid-cli requires a browser (Puppeteer). See: https://github.com/mermaid-js/mermaid-cli');
    process.exit(1);
  }

  const results = [];
  const inputFile = join(tempDir, 'input.mmd');
  const selkieOutput = join(tempDir, 'selkie-output.svg');
  const mmdcOutput = join(tempDir, 'mmdc-output.svg');

  for (const testCase of testCases) {
    process.stdout.write(`Benchmarking: ${testCase.name}... `);

    const selkieTimes = benchmarkSelkie(testCase.diagram, inputFile, selkieOutput);
    const mmdcTimes = benchmarkMmdc(testCase.diagram, inputFile, mmdcOutput);

    const selkieMedian = median(selkieTimes);
    const mmdcMedian = median(mmdcTimes);
    const speedup = mmdcMedian / selkieMedian;

    results.push({
      name: testCase.name,
      selkieMs: selkieMedian,
      mmdcMs: mmdcMedian,
      speedup,
    });

    console.log('done');
  }

  // Cleanup
  try {
    unlinkSync(inputFile);
    unlinkSync(selkieOutput);
    unlinkSync(mmdcOutput);
  } catch {}

  console.log('');
  console.log('Results (median execution time in milliseconds):');
  console.log('-'.repeat(75));
  console.log('| Diagram                          |   mmdc   |  Selkie  | Speedup |');
  console.log('|----------------------------------|----------|----------|---------|');

  for (const r of results) {
    const name = r.name.padEnd(32);
    const mmdc = isNaN(r.mmdcMs) ? '   N/A   ' : `${r.mmdcMs.toFixed(0).padStart(6)}ms `;
    const selkie = isNaN(r.selkieMs) ? '   N/A   ' : `${r.selkieMs.toFixed(0).padStart(6)}ms `;
    const speedup = isNaN(r.speedup) ? '  N/A  ' : `${r.speedup.toFixed(1).padStart(5)}x `;
    console.log(`| ${name} | ${mmdc}| ${selkie}| ${speedup}|`);
  }
  console.log('-'.repeat(75));

  const validSpeedups = results.map(r => r.speedup).filter(x => !isNaN(x) && isFinite(x));
  if (validSpeedups.length > 0) {
    const avgSpeedup = mean(validSpeedups);
    console.log(`\nAverage speedup: ${avgSpeedup.toFixed(1)}x faster than mmdc`);
  }

  // Markdown output
  console.log('\n\n=== Markdown for README ===\n');
  console.log('| Diagram | mmdc | Selkie | Speedup |');
  console.log('|---------|------|--------|---------|');
  for (const r of results) {
    const mmdc = isNaN(r.mmdcMs) ? 'N/A' : `${(r.mmdcMs/1000).toFixed(2)}s`;
    const selkie = isNaN(r.selkieMs) ? 'N/A' : `${r.selkieMs.toFixed(0)}ms`;
    const speedup = isNaN(r.speedup) ? 'N/A' : `**${r.speedup.toFixed(0)}x**`;
    console.log(`| ${r.name} | ${mmdc} | ${selkie} | ${speedup} |`);
  }
  console.log(`\n_CLI-to-CLI comparison. Median of ${BENCHMARK_RUNS} runs after ${WARMUP_RUNS} warmup runs._`);
}

main().catch(console.error);
