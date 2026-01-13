#!/usr/bin/env node
/**
 * Renders diagrams using mermaid-cli for comparison
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const outputDir = join(__dirname, 'output');

async function renderDiagram(name, source) {
  const inputPath = join(outputDir, `${name}_input.mmd`);
  const outputPath = join(outputDir, `${name}_ref.svg`);

  try {
    // Write the diagram source to a temp file
    writeFileSync(inputPath, source);

    // Run mermaid-cli
    execSync(`npx mmdc -i "${inputPath}" -o "${outputPath}" -b transparent`, {
      cwd: __dirname,
      stdio: 'pipe',
    });

    return true;
  } catch (error) {
    console.error(`  ✗ ${name} - ${error.message.split('\n')[0]}`);
    return false;
  }
}

async function main() {
  const sourcesPath = join(outputDir, 'sources.json');

  if (!existsSync(sourcesPath)) {
    console.error('Error: sources.json not found. Run `cargo run --bin gallery-generate` first.');
    process.exit(1);
  }

  const sources = JSON.parse(readFileSync(sourcesPath, 'utf-8'));
  console.log(`Rendering ${sources.length} diagrams with mermaid-cli...`);

  for (const { name, source } of sources) {
    const success = await renderDiagram(name, source);
    if (success) {
      console.log(`  ✓ ${name}`);
    }
  }

  // Generate HTML gallery
  generateGallery(sources);
}

function generateGallery(sources) {
  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Mermaid-RS Rendering Gallery</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #f5f5f5;
      padding: 20px;
    }
    h1 {
      text-align: center;
      margin-bottom: 30px;
      color: #333;
    }
    .diagram-section {
      background: white;
      border-radius: 12px;
      box-shadow: 0 2px 8px rgba(0,0,0,0.1);
      margin-bottom: 30px;
      overflow: hidden;
    }
    .diagram-header {
      background: #6366f1;
      color: white;
      padding: 15px 20px;
      font-size: 18px;
      font-weight: 600;
      text-transform: capitalize;
    }
    .diagram-content {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 20px;
      padding: 20px;
    }
    .diagram-panel {
      border: 1px solid #e5e7eb;
      border-radius: 8px;
      overflow: hidden;
    }
    .panel-header {
      background: #f9fafb;
      padding: 10px 15px;
      font-weight: 500;
      color: #374151;
      border-bottom: 1px solid #e5e7eb;
    }
    .panel-header.rs { background: #fef3c7; color: #92400e; }
    .panel-header.ref { background: #dbeafe; color: #1e40af; }
    .panel-content {
      padding: 15px;
      min-height: 200px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: white;
    }
    .panel-content svg {
      max-width: 100%;
      height: auto;
    }
    .panel-content img {
      max-width: 100%;
      height: auto;
    }
    .source-code {
      background: #1f2937;
      color: #e5e7eb;
      padding: 15px;
      font-family: 'Fira Code', 'Monaco', monospace;
      font-size: 12px;
      white-space: pre-wrap;
      overflow-x: auto;
      border-top: 1px solid #374151;
    }
    .error {
      color: #dc2626;
      font-style: italic;
    }
    .legend {
      display: flex;
      justify-content: center;
      gap: 30px;
      margin-bottom: 20px;
    }
    .legend-item {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .legend-color {
      width: 16px;
      height: 16px;
      border-radius: 4px;
    }
    .legend-color.rs { background: #fef3c7; }
    .legend-color.ref { background: #dbeafe; }
  </style>
</head>
<body>
  <h1>Selkie Rendering Gallery</h1>

  <div class="legend">
    <div class="legend-item">
      <div class="legend-color rs"></div>
      <span>selkie (Rust)</span>
    </div>
    <div class="legend-item">
      <div class="legend-color ref"></div>
      <span>mermaid.js (Reference)</span>
    </div>
  </div>

  ${sources.map(({ name, source }) => `
  <div class="diagram-section">
    <div class="diagram-header">${name} Diagram</div>
    <div class="diagram-content">
      <div class="diagram-panel">
        <div class="panel-header rs">selkie</div>
        <div class="panel-content">
          <object type="image/svg+xml" data="${name}_rs.svg" width="100%">
            <p class="error">SVG not available</p>
          </object>
        </div>
      </div>
      <div class="diagram-panel">
        <div class="panel-header ref">mermaid.js</div>
        <div class="panel-content">
          <object type="image/svg+xml" data="${name}_ref.svg" width="100%">
            <p class="error">SVG not available</p>
          </object>
        </div>
      </div>
    </div>
    <div class="source-code">${escapeHtml(source)}</div>
  </div>
  `).join('')}

</body>
</html>`;

  const galleryPath = join(outputDir, 'index.html');
  writeFileSync(galleryPath, html);
  console.log(`\n✨ Gallery generated: ${galleryPath}`);
}

function escapeHtml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

main().catch(console.error);
