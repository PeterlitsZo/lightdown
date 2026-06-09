use lightdown_ir::{
    BlockKind, InlineKind, TableCellKind, TableChildKind, VmError, compile_module,
    execute_document, parse,
};

#[test]
fn evaluates_list_map_and_apply_into_document_nodes() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          (table
            (thead
              (apply tr (map th (list (text "Foo") (text "Bar")))))))
    "#})
    .expect("module parses");

    let document = execute_document(&compile_module(&module).expect("module compiles"))
        .expect("program executes");

    let BlockKind::Table(children) = &document.blocks[0].kind else {
        panic!("expected table");
    };
    let TableChildKind::Head(rows) = &children[0].kind else {
        panic!("expected head rows");
    };
    let cells = &rows[0].kind.cells;

    assert_eq!(cells.len(), 2);
    assert!(matches!(
        &cells[0].kind,
        TableCellKind::Header(inlines)
            if matches!(&inlines[0].kind, InlineKind::Text(text) if text == "Foo")
    ));
    assert!(matches!(
        &cells[1].kind,
        TableCellKind::Header(inlines)
            if matches!(&inlines[0].kind, InlineKind::Text(text) if text == "Bar")
    ));
}

#[test]
fn rejects_bare_strings_where_inline_nodes_are_required() {
    let module = parse(r#"(doc {:meta {:version "0.1.0"}} (table (thead (tr (th "Foo")))))"#)
        .expect("module parses");

    let error = execute_document(&compile_module(&module).expect("module compiles"))
        .expect_err("bare strings are rejected");

    assert!(matches!(
        error,
        VmError::BuiltinTypeMismatch {
            builtin,
            expected,
            found,
            ..
        } if builtin == "th" && expected == "inline" && found == "string"
    ));
}

#[test]
fn rejects_invalid_map_and_apply_arguments() {
    let invalid_apply = parse(r#"(doc {:meta {:version "0.1.0"}} (apply p (text "x")))"#)
        .expect("module parses");
    let invalid_apply_error =
        execute_document(&compile_module(&invalid_apply).expect("module compiles"))
            .expect_err("apply expects a list");
    assert!(matches!(
        invalid_apply_error,
        VmError::BuiltinTypeMismatch {
            builtin,
            expected,
            found,
            ..
        } if builtin == "apply" && expected == "list" && found == "inline"
    ));

    assert!(matches!(
        execute_document(
            &compile_module(
                &parse(r#"(doc {:meta {:version "0.1.0"}} (map (text "x") (list (text "y"))))"#)
                    .expect("module parses")
            )
            .expect("module compiles")
        )
        .expect_err("map expects a callable"),
        VmError::NonCallableValue { found: "inline", .. }
    ));
}
