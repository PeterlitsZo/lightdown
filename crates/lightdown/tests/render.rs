#[test]
fn renders_author_document_to_html() {
    let html =
        lightdown::render_html("# Hello\n\nLightdown is small.").expect("author document renders");

    assert_eq!(html, "<h1>Hello</h1><p>Lightdown is small.</p>");
}

#[test]
fn renders_inline_markup_to_html() {
    let html =
        lightdown::render_html("Use *simple* data and **explicit** rules with `lightdown build`.")
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

#[test]
fn renders_list_map_and_apply_through_author_pipeline() {
    let input = indoc::indoc! {r#"
        \(table
          (thead
            (apply tr (map th (list [Foo] [Bar])))
          )
        )
    "#};
    let html = lightdown::render_html(input).expect("author pipeline renders map/apply");

    assert_eq!(
        html,
        "<table><thead><tr><th>Foo</th><th>Bar</th></tr></thead></table>"
    );
}

#[test]
fn ignores_whitespace_between_nested_lightdown_fragments_in_embedded_ir_lists() {
    let input = indoc::indoc! {r#"
        \(table
          (thead
            (apply tr (map th (list [Hello]     [World] [Do **YOU** know Lightdown?]))))
          (tbody
            (apply tr (map td (list [Peterlist] [Zo]    [Yes])))
            (apply tr (map td (list [Liu]       [Zilu]  [No])))))
    "#};
    let html =
        lightdown::render_html(input).expect("author pipeline renders whitespace-separated list");

    assert_eq!(
        html,
        "<table><thead><tr><th>Hello</th><th>World</th><th>Do <strong>YOU</strong> know Lightdown?</th></tr></thead><tbody><tr><td>Peterlist</td><td>Zo</td><td>Yes</td></tr><tr><td>Liu</td><td>Zilu</td><td>No</td></tr></tbody></table>"
    );
}
