use lightdown_ir::{BlockKind, InlineKind, ParseErrorKind, TableCellKind, TableChildKind, parse};

#[test]
fn parses_minimal_document_with_nested_metadata_map() {
    let document = parse(r#"(doc {:meta {:version "0.1.0"}})"#).expect("document parses");

    assert_eq!(document.metadata.version, "0.1.0");
    assert!(document.blocks.is_empty());
    assert_eq!(document.span.start.line, 1);
    assert_eq!(document.span.start.column, 1);
}

#[test]
fn parses_representative_document() {
    let input = indoc::indoc! { r#"
        (doc {:meta {:version "0.1.0"}}
          (h1 "Lightdown")
          (p
            "Read "
            (a {:href "/guide.html"} "the guide")
            ".")
          (ul
            (li (p "Headings"))
            (li (p "Code blocks")))
          (codeblock {:lang "javascript"} """
            | console.log('hello');
            """)
          (table
            (thead
              (tr
                (th "Name")
                (th "Description")))
            (tbody
              (tr
                (td "Lightdown")
                (td "A lightweight document language")))))
    "# };

    let document = parse(input).expect("document parses");

    assert_eq!(document.metadata.version, "0.1.0");
    assert_eq!(document.blocks.len(), 5);

    assert!(matches!(
        document.blocks[0].kind,
        BlockKind::Heading { level: 1, .. }
    ));

    let BlockKind::Paragraph(inlines) = &document.blocks[1].kind else {
        panic!("expected paragraph");
    };
    assert!(matches!(&inlines[1].kind, InlineKind::Link { href, .. } if href == "/guide.html"));

    let BlockKind::CodeBlock { lang, text } = &document.blocks[3].kind else {
        panic!("expected code block");
    };
    assert_eq!(lang.as_deref(), Some("javascript"));
    assert_eq!(text, "console.log('hello');");

    let BlockKind::Table(children) = &document.blocks[4].kind else {
        panic!("expected table");
    };
    assert!(matches!(children[0].kind, TableChildKind::Head(_)));
    assert!(matches!(children[1].kind, TableChildKind::Body(_)));
    let TableChildKind::Head(rows) = &children[0].kind else {
        unreachable!();
    };
    assert!(matches!(
        rows[0].kind.cells[0].kind,
        TableCellKind::Header(_)
    ));
}

#[test]
fn rejects_old_meta_child_syntax() {
    let error = parse(r#"(doc (meta :version "0.1.0"))"#).expect_err("old syntax is rejected");

    assert!(matches!(
        error.kind,
        ParseErrorKind::MissingAttribute { node, attribute } if node == "doc" && attribute == "meta"
    ));
}

#[test]
fn rejects_malformed_document_metadata() {
    let error = parse(r#"(doc {:meta {:version "0.1.0" :extra "x"}})"#)
        .expect_err("unknown metadata keys are rejected");

    assert!(matches!(
        error.kind,
        ParseErrorKind::UnknownAttribute { node, attribute } if node == "meta" && attribute == "extra"
    ));
}

#[test]
fn rejects_duplicate_attributes() {
    let error =
        parse(r#"(doc {:meta {:version "0.1.0"}} (codeblock {:lang "js" :lang "ts"} "x"))"#)
            .expect_err("duplicate attributes are rejected");

    assert!(matches!(
        error.kind,
        ParseErrorKind::DuplicateAttribute { attribute } if attribute == "lang"
    ));
}

#[test]
fn rejects_invalid_child_placement() {
    let error = parse(r#"(doc {:meta {:version "0.1.0"}} (p (ul (li (p "x")))))"#)
        .expect_err("block inside inline context is rejected");

    assert!(matches!(
        error.kind,
        ParseErrorKind::InvalidChild { parent, child } if parent == "p" && child == "ul"
    ));
}

#[test]
fn rejects_invalid_codeblock_shape() {
    let error = parse(r#"(doc {:meta {:version "0.1.0"}} (codeblock "a" "b"))"#)
        .expect_err("codeblock requires exactly one text child");

    assert!(matches!(
        error.kind,
        ParseErrorKind::UnexpectedToken { .. } | ParseErrorKind::InvalidChild { .. }
    ));
}

#[test]
fn rejects_nested_links() {
    let error =
        parse(r#"(doc {:meta {:version "0.1.0"}} (p (a {:href "/"} (a {:href "/x"} "x"))))"#)
            .expect_err("nested links are rejected");

    assert!(matches!(
        error.kind,
        ParseErrorKind::InvalidChild { parent, child } if parent == "a" && child == "a"
    ));
}

#[test]
fn propagates_lexer_errors() {
    let error = parse("(doc @)").expect_err("lexer error is propagated");

    assert!(matches!(error.kind, ParseErrorKind::Lex(_)));
}
