use lightdown_ir::BlockKind;

#[test]
fn parses_minimal_author_document() {
    let document = lightdown::parse("# Hello\n\nLightdown is small.")
        .expect("author document parses");

    assert_eq!(document.metadata.version, "0.1.0");
    assert!(matches!(
        document.blocks[0].kind,
        BlockKind::Heading { level: 1, .. }
    ));
    assert!(matches!(document.blocks[1].kind, BlockKind::Paragraph(_)));
}

#[test]
fn parses_lists_quotes_fences_and_thematic_breaks() {
    let input = indoc::indoc! {"
        # Title

        - Small
        - Predictable

        1. Parse
        2. Render

        > Programs should be written for people to read.

        ```rust
        fn main() {}
        ```

        ---
    "};

    let document = lightdown::parse(input).expect("document parses");

    assert_eq!(document.blocks.len(), 6);
    assert!(matches!(
        document.blocks[0].kind,
        BlockKind::Heading { level: 1, .. }
    ));
    assert!(matches!(
        document.blocks[1].kind,
        BlockKind::List { ordered: false, .. }
    ));
    assert!(matches!(
        document.blocks[2].kind,
        BlockKind::List { ordered: true, .. }
    ));
    assert!(matches!(document.blocks[3].kind, BlockKind::BlockQuote(_)));
    assert!(matches!(
        document.blocks[4].kind,
        BlockKind::CodeBlock { .. }
    ));
    assert!(matches!(
        document.blocks[5].kind,
        BlockKind::ThematicBreak
    ));
}

#[test]
fn rejects_unterminated_code_fence() {
    let error = lightdown::parse("```rust\nfn main() {}\n")
        .expect_err("unterminated fence is rejected");

    assert!(matches!(
        error.kind,
        lightdown::ParseErrorKind::UnterminatedCodeFence
    ));
}

#[test]
fn parses_inline_markup() {
    let input = "Use *simple* data and **explicit** rules with `lightdown build`.";
    let document = lightdown::parse(input).expect("inline markup parses");

    let BlockKind::Paragraph(inlines) = &document.blocks[0].kind else {
        panic!("expected paragraph");
    };

    assert!(matches!(
        &inlines[1].kind,
        lightdown_ir::InlineKind::Emphasis(_)
    ));
    assert!(matches!(
        &inlines[3].kind,
        lightdown_ir::InlineKind::Strong(_)
    ));
    assert!(matches!(
        &inlines[5].kind,
        lightdown_ir::InlineKind::Code(code) if code == "lightdown build"
    ));
}

#[test]
fn parses_links_and_images() {
    let input = "Read [the guide](https://example.com/guide) ![Alt text](https://example.com/a.png)";
    let document = lightdown::parse(input).expect("links and images parse");

    let BlockKind::Paragraph(inlines) = &document.blocks[0].kind else {
        panic!("expected paragraph");
    };

    assert!(matches!(
        &inlines[1].kind,
        lightdown_ir::InlineKind::Link { href, .. } if href == "https://example.com/guide"
    ));
    assert!(matches!(
        &inlines[3].kind,
        lightdown_ir::InlineKind::Image { src, alt }
            if src == "https://example.com/a.png" && alt.as_deref() == Some("Alt text")
    ));
}

#[test]
fn parses_embedded_ir_with_nested_lightdown_fragment() {
    let input = indoc::indoc! {r#"
        # Foobar

        ## Barfoo

        Do you know \(a {:href "https://example.com"} [`lightdown`])? `lightdown` is good.
    "#};

    let document = lightdown::parse(input).expect("embedded ir parses");
    let BlockKind::Paragraph(inlines) = &document.blocks[2].kind else {
        panic!("expected paragraph");
    };

    assert!(matches!(
        &inlines[1].kind,
        lightdown_ir::InlineKind::Link { href, children }
            if href == "https://example.com"
            && matches!(children[0].kind, lightdown_ir::InlineKind::Code(ref text) if text == "lightdown")
    ));
}

#[test]
fn rejects_invalid_embedded_ir() {
    let error = lightdown::parse(r#"Use \(a {:href "/"} (a {:href "/x"} "x"))"#)
        .expect_err("invalid embedded ir is rejected");

    assert!(matches!(error.kind, lightdown::ParseErrorKind::EmbeddedIr(_)));
}

#[test]
fn parses_block_embedded_ir_table() {
    let input = indoc::indoc! {r#"
        \(table
          (tr (th [Company]))
          (tr (td [Alfreds Futterkiste]))
        )
    "#};

    let document = lightdown::parse(input).expect("block embedded ir parses");

    assert_eq!(document.blocks.len(), 1);
    assert!(matches!(document.blocks[0].kind, BlockKind::Table(_)));
}
