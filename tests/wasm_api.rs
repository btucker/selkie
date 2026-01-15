#![cfg(all(feature = "wasm", target_arch = "wasm32"))]

use js_sys::{Function, Reflect};
use wasm_bindgen::JsValue;

use selkie::wasm::{initialize, parse, render, render_text};

#[test]
fn render_text_returns_svg() {
    let svg = render_text("flowchart TD; A-->B;").expect("render_text should succeed");
    assert!(svg.contains("<svg"), "expected SVG output");
}

#[test]
fn parse_validates_input() {
    parse("flowchart TD; A-->B;").expect("parse should succeed");
}

#[test]
fn render_matches_mermaid_shape() {
    initialize(JsValue::NULL);
    let value = render("diagram1", "flowchart TD; A-->B;").expect("render should succeed");
    let svg = Reflect::get(&value, &JsValue::from_str("svg"))
        .expect("should get svg")
        .as_string()
        .expect("svg should be a string");
    assert!(svg.contains("<svg"), "expected SVG output");

    let id = Reflect::get(&value, &JsValue::from_str("id"))
        .expect("should get id")
        .as_string()
        .expect("id should be a string");
    assert_eq!(id, "diagram1");

    let bind_functions = Reflect::get(&value, &JsValue::from_str("bindFunctions"))
        .expect("should get bindFunctions");
    assert!(
        bind_functions.is_function(),
        "bindFunctions should be a function"
    );
    let function: Function = bind_functions
        .dyn_into()
        .expect("bindFunctions should be callable");
    let _ = function;
}
