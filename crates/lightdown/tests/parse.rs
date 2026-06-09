use lightdown_ir::{Expr, ExprKind};

#[test]
fn parses_minimal_author_document() {
    let module =
        lightdown::parse("# Hello\n\nLightdown is small.").expect("author document parses");

    assert_eq!(module.metadata.version, "0.1.0");
    assert_call_name(&module.body, "doc");

    let doc_args = call_args(&module.body);
    assert_eq!(doc_args.len(), 2);
    assert_call_name(&doc_args[0], "h1");
    assert_call_name(&doc_args[1], "p");
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

    let module = lightdown::parse(input).expect("document parses");

    let doc_args = call_args(&module.body);
    assert_eq!(doc_args.len(), 6);
    assert_call_name(&doc_args[0], "h1");
    assert_call_name(&doc_args[1], "ul");
    assert_call_name(&doc_args[2], "ol");
    assert_call_name(&doc_args[3], "blockquote");
    assert_call_name(&doc_args[4], "codeblock");
    assert_call_name(&doc_args[5], "hr");
}

#[test]
fn rejects_unterminated_code_fence() {
    let error =
        lightdown::parse("```rust\nfn main() {}\n").expect_err("unterminated fence is rejected");

    assert!(matches!(
        error.kind,
        lightdown::ParseErrorKind::UnterminatedCodeFence
    ));
}

#[test]
fn parses_inline_markup() {
    let input = "Use *simple* data and **explicit** rules with `lightdown build`.";
    let module = lightdown::parse(input).expect("inline markup parses");

    let paragraph = &call_args(&module.body)[0];
    let inline_args = call_args(paragraph);

    assert!(inline_args.iter().any(|expr| is_named_call(expr, "em")));
    assert!(inline_args.iter().any(|expr| is_named_call(expr, "strong")));
    assert!(inline_args.iter().any(|expr| is_named_call(expr, "code")));
}

#[test]
fn parses_links_and_images() {
    let input =
        "Read [the guide](https://example.com/guide) ![Alt text](https://example.com/a.png)";
    let module = lightdown::parse(input).expect("links and images parse");

    let paragraph = &call_args(&module.body)[0];
    let inline_args = call_args(paragraph);

    let link = inline_args
        .iter()
        .find(|expr| is_named_call(expr, "a"))
        .expect("link call exists");
    let image = inline_args
        .iter()
        .find(|expr| is_named_call(expr, "img"))
        .expect("image call exists");

    assert_string(&call_args(link)[0], "https://example.com/guide");
    assert_string(&call_args(image)[0], "https://example.com/a.png");
    assert_string(&call_args(image)[1], "Alt text");
}

#[test]
fn parses_embedded_ir_with_nested_lightdown_fragment() {
    let input = indoc::indoc! {r#"
        # Foobar

        ## Barfoo

        Do you know \(a {:href "https://example.com"} [`lightdown`])? `lightdown` is good.
    "#};

    let module = lightdown::parse(input).expect("embedded ir parses");
    let paragraph = &call_args(&module.body)[2];
    let link = call_args(paragraph)
        .iter()
        .find(|expr| is_named_call(expr, "a"))
        .expect("embedded link exists");
    let link_children = &call_args(link)[1];

    assert_string(&call_args(link)[0], "https://example.com");
    assert_call_name(link_children, "list");
    assert!(matches!(
        &call_args(link_children)[0].kind,
        ExprKind::Call { callee, .. } if matches!(&callee.kind, ExprKind::Symbol(name) if name == "code")
    ));
}

#[test]
fn parses_block_embedded_ir_table_with_list_map_apply() {
    let input = indoc::indoc! {r#"
        \(table
          (thead
            (apply tr (map th (list [Company] [Description]))))
        )
    "#};

    let module = lightdown::parse(input).expect("block embedded ir parses");
    let table = &call_args(&module.body)[0];
    let thead = &call_args(table)[0];
    let apply = &call_args(thead)[0];

    assert_call_name(apply, "apply");
    assert_symbol(&call_args(apply)[0], "tr");
    assert_call_name(&call_args(apply)[1], "map");
}

fn call_args(expr: &Expr) -> &[Expr] {
    let ExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    args
}

fn assert_call_name(expr: &Expr, expected: &str) {
    let ExprKind::Call { callee, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    assert_symbol(callee, expected);
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
        matches!(&expr.kind, ExprKind::String(text) if text == expected),
        "expected string {expected:?}, got {:?}",
        expr.kind
    );
}

fn is_named_call(expr: &Expr, expected: &str) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Call { callee, .. }
            if matches!(&callee.kind, ExprKind::Symbol(name) if name == expected)
    )
}
