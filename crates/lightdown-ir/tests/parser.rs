use lightdown_ir::{Expr, ExprKind, ParseErrorKind, parse, parse_expr_fragment};

#[test]
fn parses_minimal_document_module() {
    let module = parse(r#"(doc {:meta {:version "0.1.0"}})"#).expect("module parses");

    assert_eq!(module.metadata.version, "0.1.0");

    let ExprKind::Call { callee, args } = &module.body.kind else {
        panic!("expected doc call");
    };
    assert_symbol(callee, "doc");
    assert!(args.is_empty());
    assert_eq!(module.span.start.line, 1);
    assert_eq!(module.span.start.column, 1);
}

#[test]
fn parses_representative_document_as_call_tree() {
    let input = indoc::indoc! { r#"
        (doc {:meta {:version "0.1.0"}}
          (h1 (text "Lightdown"))
          (p
            (text "Read ")
            (a {:href "/guide.html"} (text "the guide"))
            (text "."))
          (ul
            (li (p (text "Headings")))
            (li (p (text "Code blocks"))))
          (codeblock {:lang "javascript"} """
            | console.log('hello');
            """)
          (table
            (thead
              (tr
                (th (text "Name"))
                (th (text "Description"))))
            (tbody
              (tr
                (td (text "Lightdown"))
                (td (text "A lightweight document language"))))))
    "# };

    let module = parse(input).expect("module parses");

    assert_eq!(module.metadata.version, "0.1.0");

    let ExprKind::Call { callee, args } = &module.body.kind else {
        panic!("expected doc call");
    };
    assert_symbol(callee, "doc");
    assert_eq!(args.len(), 5);

    assert_call_name(&args[0], "h1");
    assert_call_name(&args[1], "p");
    assert_call_name(&args[2], "ul");
    assert_call_name(&args[3], "codeblock");
    assert_call_name(&args[4], "table");

    let heading_args = call_args(&args[0]);
    assert_eq!(heading_args.len(), 1);
    assert_text_call(&heading_args[0], "Lightdown");

    let paragraph_args = call_args(&args[1]);
    assert_eq!(paragraph_args.len(), 3);
    assert_text_call(&paragraph_args[0], "Read ");
    assert_call_name(&paragraph_args[1], "a");
    assert_text_call(&paragraph_args[2], ".");

    let link_args = call_args(&paragraph_args[1]);
    assert_eq!(link_args.len(), 2);
    assert_string(&link_args[0], "/guide.html");
    assert_text_call(&link_args[1], "the guide");

    let codeblock_args = call_args(&args[3]);
    assert_eq!(codeblock_args.len(), 2);
    assert_string(&codeblock_args[0], "javascript");
    assert_string(&codeblock_args[1], "console.log('hello');");
}

#[test]
fn parses_list_map_and_apply_calls() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          (table
            (thead
              (apply tr (map th (list (text "Foo") (text "Bar")))))))
    "#})
    .expect("module parses");

    let table = &call_args(&module.body)[0];
    let thead = &call_args(table)[0];
    let apply = &call_args(thead)[0];

    assert_call_name(apply, "apply");
    let apply_args = call_args(apply);
    assert_eq!(apply_args.len(), 2);
    assert_symbol(&apply_args[0], "tr");

    let map = &apply_args[1];
    assert_call_name(map, "map");
    let map_args = call_args(map);
    assert_eq!(map_args.len(), 2);
    assert_symbol(&map_args[0], "th");

    let list = &map_args[1];
    assert_call_name(list, "list");
    let list_args = call_args(list);
    assert_eq!(list_args.len(), 2);
    assert_text_call(&list_args[0], "Foo");
    assert_text_call(&list_args[1], "Bar");
}

#[test]
fn normalizes_attribute_sugar_into_positional_calls() {
    let module = parse(indoc::indoc! {r#"
        (doc {:meta {:version "0.1.0"}}
          (p
            (a {:href "/"} (text "home"))
            (img {:src "/logo.png" :alt "Logo"})
            (codeblock {:lang "rust"} "fn main() {}")))
    "#})
    .expect("module parses");

    let paragraph = &call_args(&module.body)[0];
    let paragraph_args = call_args(paragraph);

    let link = &paragraph_args[0];
    assert_call_name(link, "a");
    let link_args = call_args(link);
    assert_eq!(link_args.len(), 2);
    assert_string(&link_args[0], "/");
    assert_text_call(&link_args[1], "home");

    let image = &paragraph_args[1];
    assert_call_name(image, "img");
    let image_args = call_args(image);
    assert_eq!(image_args.len(), 2);
    assert_string(&image_args[0], "/logo.png");
    assert_string(&image_args[1], "Logo");

    let codeblock = &paragraph_args[2];
    assert_call_name(codeblock, "codeblock");
    let codeblock_args = call_args(codeblock);
    assert_eq!(codeblock_args.len(), 2);
    assert_string(&codeblock_args[0], "rust");
    assert_string(&codeblock_args[1], "fn main() {}");
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
fn propagates_lexer_errors() {
    let error = parse("(doc @)").expect_err("lexer error is propagated");

    assert!(matches!(error.kind, ParseErrorKind::Lex(_)));
}

#[test]
fn parses_expression_fragment() {
    let expr = parse_expr_fragment(r#"(code "lightdown")"#).expect("expression fragment parses");

    assert_call_name(&expr, "code");
    let args = call_args(&expr);
    assert_eq!(args.len(), 1);
    assert_string(&args[0], "lightdown");
}

#[test]
fn parses_lambda_with_multiple_body_expressions() {
    let expr = parse_expr_fragment(r#"(lambda (x y) (text "ignored") (list x y))"#)
        .expect("lambda parses");

    let ExprKind::Lambda { params, body } = &expr.kind else {
        panic!("expected lambda expression");
    };
    assert_eq!(params, &["x".to_string(), "y".to_string()]);
    assert_eq!(body.len(), 2);
    assert_text_call(&body[0], "ignored");
    assert_call_name(&body[1], "list");
}

#[test]
fn parses_namespaced_symbols() {
    let expr = parse_expr_fragment(r#"(ld::table::from-data (list (text "Foo")))"#)
        .expect("expression fragment parses");

    assert_call_name(&expr, "ld::table::from-data");
}

#[test]
fn lowers_let_into_lambda_call() {
    let expr = parse_expr_fragment(r#"(let ((x (text "Foo")) (y (text "Bar"))) (list x y))"#)
        .expect("let parses");

    let ExprKind::Call { callee, args } = &expr.kind else {
        panic!("expected lowered call");
    };
    assert_eq!(args.len(), 2);
    assert_text_call(&args[0], "Foo");
    assert_text_call(&args[1], "Bar");

    let ExprKind::Lambda { params, body } = &callee.kind else {
        panic!("expected let to lower into lambda");
    };
    assert_eq!(params, &["x".to_string(), "y".to_string()]);
    assert_eq!(body.len(), 1);
    assert_call_name(&body[0], "list");
}

#[test]
fn rejects_extra_input_after_expression_fragment() {
    let error = parse_expr_fragment(r#"(code "x") "y""#).expect_err("extra input is rejected");

    assert!(matches!(error.kind, ParseErrorKind::ExtraInput));
}

fn call_args(expr: &Expr) -> &[Expr] {
    let ExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    args
}

fn assert_call_name(expr: &Expr, name: &str) {
    let ExprKind::Call { callee, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    assert_symbol(callee, name);
}

fn assert_symbol(expr: &Expr, expected: &str) {
    assert!(
        matches!(&expr.kind, ExprKind::Symbol(name) if name == expected),
        "expected symbol {expected:?}, got {:?}",
        expr.kind
    );
}

fn assert_string(expr: &Expr, expected: &str) {
    assert!(
        matches!(&expr.kind, ExprKind::String(value) if value == expected),
        "expected string {expected:?}, got {:?}",
        expr.kind
    );
}

fn assert_text_call(expr: &Expr, expected: &str) {
    assert_call_name(expr, "text");
    let args = call_args(expr);
    assert_eq!(args.len(), 1);
    assert_string(&args[0], expected);
}
