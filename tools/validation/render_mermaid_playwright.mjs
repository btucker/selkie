#!/usr/bin/env node
/**
 * Mermaid diagram renderer using Playwright for accurate rendering.
 *
 * This script uses a real browser to render mermaid diagrams, giving us
 * accurate getBBox measurements and proper text sizing.
 *
 * Usage:
 *   node render_mermaid_playwright.mjs <diagram_text>
 *   echo "flowchart LR\nA-->B" | node render_mermaid_playwright.mjs -
 *
 * Output:
 *   Raw SVG string
 */

import { readFileSync, mkdirSync, existsSync } from 'fs';
import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));

// HTML template with mermaid
const htmlTemplate = (diagramText) => `
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
  <style>
    body { margin: 0; padding: 20px; font-family: sans-serif; }
    #mermaid-container { display: inline-block; }
  </style>
</head>
<body>
  <div id="mermaid-container">
    <pre class="mermaid">
${diagramText}
    </pre>
  </div>
  <script>
    mermaid.initialize({
      startOnLoad: true,
      securityLevel: 'loose',
      theme: 'default',
      flowchart: {
        htmlLabels: false,
        useMaxWidth: false,
      }
    });

    // Signal when rendering is complete
    document.addEventListener('DOMContentLoaded', () => {
      // Wait for mermaid to process
      setTimeout(() => {
        window._mermaidDone = true;
      }, 500);
    });
  </script>
</body>
</html>
`;

async function renderDiagram(text) {
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();
    const page = await context.newPage();

    // Set page content with embedded diagram
    const html = htmlTemplate(text);
    await page.setContent(html, { waitUntil: 'networkidle' });

    // Wait for mermaid to finish rendering
    await page.waitForFunction(() => window._mermaidDone === true, { timeout: 10000 });

    // Give a bit more time for any final rendering
    await page.waitForTimeout(200);

    // Extract the SVG
    const svg = await page.evaluate(() => {
      const svgElement = document.querySelector('#mermaid-container svg');
      if (!svgElement) {
        throw new Error('No SVG found');
      }
      return svgElement.outerHTML;
    });

    return { success: true, svg };
  } catch (error) {
    return { success: false, error: error.message || String(error) };
  } finally {
    if (browser) {
      await browser.close();
    }
  }
}

async function main() {
  const args = process.argv.slice(2);
  let input;

  if (args.length === 0) {
    console.error('Usage: node render_mermaid_playwright.mjs <diagram_text_or_file_or_-_for_stdin>');
    process.exit(1);
  }

  const inputArg = args[0];

  if (inputArg === '-') {
    // Read from stdin
    input = readFileSync(0, 'utf-8');
  } else {
    // Try to read as file, fall back to treating as diagram text
    try {
      input = readFileSync(inputArg, 'utf-8');
    } catch {
      input = inputArg;
    }
  }

  const result = await renderDiagram(input);

  if (!result.success) {
    console.error(JSON.stringify({ success: false, error: result.error }));
    process.exit(1);
  }

  console.log(result.svg);
}

main();
