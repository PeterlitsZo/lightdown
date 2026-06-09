use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = renderToHtml)]
pub fn render_to_html(input: &str) -> Result<String, JsValue> {
    lightdown_html::render(input).map_err(|error| JsValue::from_str(&error.to_string()))
}
