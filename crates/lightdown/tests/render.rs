#[test]
fn renders_author_document_to_html() {
    let html = lightdown::render_html("# Hello\n\nLightdown is small.")
        .expect("author document renders");

    assert_eq!(html, "<h1>Hello</h1><p>Lightdown is small.</p>");
}

#[test]
fn renders_inline_markup_to_html() {
    let html = lightdown::render_html(
        "Use *simple* data and **explicit** rules with `lightdown build`.",
    )
    .expect("inline markup renders");

    assert_eq!(
        html,
        "<p>Use <em>simple</em> data and <strong>explicit</strong> rules with <code>lightdown build</code>.</p>"
    );
}

#[test]
fn renders_embedded_ir_sample() {
    let input = r#"Do you know \(a {:href "https://example.com"} [`lightdown`])?"#;
    let html = lightdown::render_html(input).expect("embedded ir renders");

    assert_eq!(
        html,
        "<p>Do you know <a href=\"https://example.com\"><code>lightdown</code></a>?</p>"
    );
}

#[test]
fn renders_block_embedded_ir_table() {
    let input = indoc::indoc! {r#"
        \(table
          (tr (th [Company]))
          (tr (td [Alfreds Futterkiste]))
        )
    "#};
    let html = lightdown::render_html(input).expect("block embedded ir renders");

    assert_eq!(
        html,
        "<table><thead><tr><th>Company</th></tr></thead><tbody><tr><td>Alfreds Futterkiste</td></tr></tbody></table>"
    );
}
