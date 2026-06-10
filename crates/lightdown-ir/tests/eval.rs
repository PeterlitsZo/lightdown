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
    let invalid_apply =
        parse(r#"(doc {:meta {:version "0.1.0"}} (apply p (text "x")))"#).expect("module parses");
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
        VmError::NonCallableValue {
            found: "inline",
            ..
        }
    ));
}

#[test]
fn evaluates_let_and_lambda_with_multiple_body_expressions() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          (let
            ((headers (list (text "Hello") (text "World")))
             (make-row
               (lambda (cell-maker cells)
                 (text "ignored")
                 (apply tr (map cell-maker cells)))))
            (table
              (thead
                (make-row th headers))
              (tbody
                ((lambda (row)
                   (text "ignored")
                   (make-row td row))
                 (list (text "Peter") (text "Zo")))))))
    "#})
    .expect("module parses");

    let document = execute_document(&compile_module(&module).expect("module compiles"))
        .expect("program executes");

    let BlockKind::Table(children) = &document.blocks[0].kind else {
        panic!("expected table");
    };
    assert!(matches!(&children[0].kind, TableChildKind::Head(rows) if rows.len() == 1));
    assert!(matches!(&children[1].kind, TableChildKind::Body(rows) if rows.len() == 1));
}

#[test]
fn evaluates_ld_table_from_data_into_table_nodes() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          (ld::table::from-data
            (list (text "Foo") (text "Bar"))
            (list (text "Row 1") (text "Cell 1"))
            (list (text "Row 2") (text "Cell 2"))))
    "#})
    .expect("module parses");

    let document = execute_document(&compile_module(&module).expect("module compiles"))
        .expect("program executes");

    let BlockKind::Table(children) = &document.blocks[0].kind else {
        panic!("expected table");
    };
    let TableChildKind::Head(head_rows) = &children[0].kind else {
        panic!("expected head rows");
    };
    let TableChildKind::Body(body_rows) = &children[1].kind else {
        panic!("expected body rows");
    };

    assert_eq!(head_rows.len(), 1);
    assert_eq!(body_rows.len(), 2);
    assert!(matches!(
        &head_rows[0].kind.cells[0].kind,
        TableCellKind::Header(inlines)
            if matches!(&inlines[0].kind, InlineKind::Text(text) if text == "Foo")
    ));
    assert!(matches!(
        &body_rows[1].kind.cells[1].kind,
        TableCellKind::Data(inlines)
            if matches!(&inlines[0].kind, InlineKind::Text(text) if text == "Cell 2")
    ));
}
