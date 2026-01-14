const assert = require("assert/strict");
const fs = require("fs");
const path = require("path");
const { test } = require("node:test");

const pkgPath = path.resolve(__dirname, "../../pkg/selkie.js");
const hasPkg = fs.existsSync(pkgPath);

test(
  "selkie wasm package exposes mermaid-like API",
  { skip: !hasPkg },
  () => {
    const { initialize, parse, render, render_text } = require(pkgPath);

    initialize({ startOnLoad: false });
    parse("flowchart TD; A-->B;");
    const { svg, id, bindFunctions } = render("diagram1", "flowchart TD; A-->B;");
    const svgText = render_text("flowchart TD; A-->B;");

    assert.equal(id, "diagram1");
    assert.ok(svg.includes("<svg"));
    assert.ok(svgText.includes("<svg"));
    assert.equal(typeof bindFunctions, "function");
  }
);
