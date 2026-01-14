use js_sys::{Function, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Mirror mermaid-js's initialize API (currently a no-op).
#[wasm_bindgen]
pub fn initialize(_config: JsValue) {}

/// Validate a Mermaid diagram and return an error on failure.
#[wasm_bindgen]
pub fn parse(input: &str) -> Result<(), JsValue> {
    crate::parse(input)
        .map(|_| ())
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Render Mermaid diagram text to SVG with a mermaid-js compatible return shape.
#[wasm_bindgen]
pub fn render(id: &str, input: &str) -> Result<JsValue, JsValue> {
    let svg =
        crate::render::render_text(input).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let result = Object::new();
    Reflect::set(&result, &JsValue::from_str("id"), &JsValue::from_str(id))?;
    Reflect::set(&result, &JsValue::from_str("svg"), &JsValue::from_str(&svg))?;
    let bind_functions = Function::new_no_args("");
    Reflect::set(
        &result,
        &JsValue::from_str("bindFunctions"),
        &bind_functions.into(),
    )?;
    Ok(result.into())
}

/// Render Mermaid diagram text to SVG (WASM-friendly).
#[wasm_bindgen]
pub fn render_text(input: &str) -> Result<String, JsValue> {
    crate::render::render_text(input).map_err(|err| JsValue::from_str(&err.to_string()))
}
