use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = renderToHtml)]
pub fn render_to_html(input: &str) -> Result<String, JsValue> {
    render_author_syntax(input).map_err(|error| JsValue::from_str(&error.to_string()))
}

fn render_author_syntax(input: &str) -> Result<String, lightdown::RenderError> {
    lightdown::render_html(input)
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_author_syntax_from_wasm_entrypoint() {
        let html =
            super::render_author_syntax("# Hello\n\n`lightdown`").expect("wasm entrypoint renders");

        assert_eq!(html, "<h1>Hello</h1><p><code>lightdown</code></p>");
    }
}
