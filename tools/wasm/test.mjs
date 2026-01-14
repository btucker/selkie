#!/usr/bin/env node
/**
 * JavaScript tests for selkie.js WebAssembly bindings
 *
 * Tests the browser-compatible API that mirrors mermaid-js:
 * - initialize(config) - Configure selkie (currently a no-op)
 * - parse(input) - Validate diagram syntax
 * - render(id, input) - Render to SVG with mermaid-compatible return shape
 * - render_text(input) - Render to SVG string directly
 *
 * Usage: npm test
 */

import { readFileSync, existsSync } from 'fs';
import { chromium } from 'playwright';

const PKG_PATH = '../../pkg';

// Check if the wasm package has been built
if (!existsSync(`${PKG_PATH}/selkie.js`)) {
  console.error('Error: WASM package not found. Run: npm run build');
  console.error('Note: Requires rustup with wasm32-unknown-unknown target');
  process.exit(1);
}

// Test cases for various diagram types
const testCases = [
  {
    name: 'Simple flowchart',
    input: 'flowchart TD; A-->B;',
    expectSvg: true,
  },
  {
    name: 'Flowchart with labels',
    input: `flowchart LR
      A[Start] --> B{Decision}
      B -->|Yes| C[OK]
      B -->|No| D[Cancel]`,
    expectSvg: true,
  },
  {
    name: 'Sequence diagram',
    input: `sequenceDiagram
      Alice->>Bob: Hello
      Bob-->>Alice: Hi`,
    expectSvg: true,
  },
  {
    name: 'Class diagram',
    input: `classDiagram
      Animal <|-- Dog
      Animal : +String name`,
    expectSvg: true,
  },
  {
    name: 'State diagram',
    input: `stateDiagram-v2
      [*] --> Active
      Active --> [*]`,
    expectSvg: true,
  },
  {
    name: 'Pie chart',
    input: `pie title Languages
      "Rust" : 45
      "TypeScript" : 30
      "Python" : 25`,
    expectSvg: true,
  },
  {
    name: 'ER diagram',
    input: `erDiagram
      CUSTOMER ||--o{ ORDER : places
      ORDER ||--|{ LINE-ITEM : contains`,
    expectSvg: true,
  },
  {
    name: 'Gantt chart',
    input: `gantt
      title A Gantt Diagram
      section Section
      Task :a1, 2024-01-01, 7d`,
    expectSvg: true,
  },
];

const invalidCases = [
  {
    name: 'Invalid syntax',
    input: 'not a valid diagram',
    expectError: true,
  },
  {
    name: 'Empty input',
    input: '',
    expectError: true,
  },
];

async function runTests() {
  console.log('Selkie.js WebAssembly Tests\n');
  console.log('='.repeat(50));

  const browser = await chromium.launch();
  const page = await browser.newPage();

  // Set up the HTML page with selkie.js
  const selkieJs = readFileSync(`${PKG_PATH}/selkie.js`, 'utf-8');
  const selkieWasm = readFileSync(`${PKG_PATH}/selkie_bg.wasm`);
  const wasmBase64 = selkieWasm.toString('base64');

  await page.setContent(`
    <!DOCTYPE html>
    <html>
    <head><title>Selkie Tests</title></head>
    <body>
      <script type="module">
        ${selkieJs}

        // Load WASM from base64
        const wasmBytes = Uint8Array.from(atob('${wasmBase64}'), c => c.charCodeAt(0));

        window.selkieReady = (async () => {
          await __wbg_init(wasmBytes);
          window.selkie = { initialize, parse, render, render_text };
        })();
      </script>
    </body>
    </html>
  `);

  // Wait for selkie to be ready
  await page.waitForFunction(() => window.selkieReady);
  await page.evaluate(() => window.selkieReady);

  let passed = 0;
  let failed = 0;

  // Test initialize
  console.log('\nTest: initialize()');
  try {
    await page.evaluate(() => {
      window.selkie.initialize({ startOnLoad: false });
    });
    console.log('  PASS - initialize accepts config object');
    passed++;
  } catch (e) {
    console.log(`  FAIL - ${e.message}`);
    failed++;
  }

  // Test valid diagrams
  console.log('\nTest: Valid diagram parsing and rendering');
  for (const tc of testCases) {
    try {
      const result = await page.evaluate((input) => {
        // Test parse
        window.selkie.parse(input);

        // Test render_text
        const svg = window.selkie.render_text(input);
        if (!svg.includes('<svg')) throw new Error('No SVG in render_text output');

        // Test render (mermaid-compatible)
        const renderResult = window.selkie.render('test-id', input);
        if (!renderResult.svg.includes('<svg')) throw new Error('No SVG in render output');
        if (renderResult.id !== 'test-id') throw new Error('Wrong id in render output');
        if (typeof renderResult.bindFunctions !== 'function') throw new Error('bindFunctions not a function');

        return { svg: svg.substring(0, 100), svgLength: svg.length };
      }, tc.input);

      console.log(`  PASS - ${tc.name} (${result.svgLength} chars)`);
      passed++;
    } catch (e) {
      console.log(`  FAIL - ${tc.name}: ${e.message}`);
      failed++;
    }
  }

  // Test invalid diagrams
  console.log('\nTest: Invalid diagram handling');
  for (const tc of invalidCases) {
    try {
      await page.evaluate((input) => {
        window.selkie.parse(input);
      }, tc.input);
      console.log(`  FAIL - ${tc.name}: Should have thrown error`);
      failed++;
    } catch (e) {
      console.log(`  PASS - ${tc.name}: Correctly rejected`);
      passed++;
    }
  }

  // Test render without parse (should work)
  console.log('\nTest: render without prior parse');
  try {
    const result = await page.evaluate(() => {
      const { svg } = window.selkie.render('direct', 'flowchart TD; X-->Y;');
      return svg.includes('<svg');
    });
    if (result) {
      console.log('  PASS - render works independently');
      passed++;
    } else {
      throw new Error('No SVG output');
    }
  } catch (e) {
    console.log(`  FAIL - ${e.message}`);
    failed++;
  }

  await browser.close();

  // Summary
  console.log('\n' + '='.repeat(50));
  console.log(`Results: ${passed} passed, ${failed} failed`);
  console.log('='.repeat(50));

  process.exit(failed > 0 ? 1 : 0);
}

runTests().catch(e => {
  console.error('Test runner error:', e);
  process.exit(1);
});
