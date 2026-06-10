use lightdown_ir::{Expr, ExprKind, Span};

use crate::error::ParseError;
use crate::{inline_parser, lower};

pub fn lower_inline_fragment(source: String, span: Span) -> Result<Expr, ParseError> {
    let resolved = resolve_nested_lightdown_fragments(&source, span)?;
    let fragment = format!("({resolved})");
    let mut expr = lightdown_ir::parse_expr_fragment(&fragment).map_err(ParseError::from)?;
    expr.span = span;
    Ok(expr)
}

pub fn lower_block_fragment(source: String, span: Span) -> Result<Expr, ParseError> {
    let resolved = resolve_nested_lightdown_fragments(&source, span)?;
    let document_source = format!("(doc {{:meta {{:version \"0.1.0\"}}}} ({resolved}))");
    let module = lightdown_ir::parse(&document_source).map_err(ParseError::from)?;
    let ExprKind::Call { args, .. } = module.body.kind else {
        unreachable!("doc module body is always a call");
    };
    let mut args = args;
    let mut expr = args
        .pop()
        .expect("embedded block document contains exactly one block");
    expr.span = span;
    Ok(expr)
}

fn resolve_nested_lightdown_fragments(source: &str, span: Span) -> Result<String, ParseError> {
    let mut output = String::new();
    let mut cursor = 0;

    while let Some(relative_start) = source[cursor..].find('[') {
        let start = cursor + relative_start;
        let end = find_matching_bracket(source, start, span)?;
        output.push_str(&source[cursor..start]);

        let fragment = &source[start + 1..end];
        let inlines = inline_parser::parse_fragment(fragment, span)?;
        let ir_inlines = lower::lower_inline_vec(inlines)?;
        output.push_str(&serialize_inline_sequence(&ir_inlines));

        cursor = end + 1;
    }

    output.push_str(&source[cursor..]);
    Ok(output)
}

fn find_matching_bracket(source: &str, start: usize, span: Span) -> Result<usize, ParseError> {
    let mut cursor = start + 1;
    let mut depth = 0usize;
    let mut in_code = false;

    while cursor < source.len() {
        let ch = source[cursor..]
            .chars()
            .next()
            .expect("cursor points at a character");

        if in_code {
            cursor += ch.len_utf8();
            if ch == '`' {
                in_code = false;
            }
            continue;
        }

        match ch {
            '`' => {
                in_code = true;
                cursor += ch.len_utf8();
            }
            '\\' => {
                cursor += ch.len_utf8();
                if cursor < source.len() {
                    let next = source[cursor..]
                        .chars()
                        .next()
                        .expect("cursor points at a character");
                    cursor += next.len_utf8();
                }
            }
            '[' => {
                depth += 1;
                cursor += ch.len_utf8();
            }
            ']' => {
                if depth == 0 {
                    return Ok(cursor);
                }
                depth -= 1;
                cursor += ch.len_utf8();
            }
            _ => cursor += ch.len_utf8(),
        }
    }

    Err(ParseError::new(
        crate::ParseErrorKind::UnterminatedEmbeddedIr,
        Some(span),
    ))
}

fn serialize_inline_sequence(inlines: &[Expr]) -> String {
    let items = inlines
        .iter()
        .map(serialize_expr)
        .collect::<Vec<_>>()
        .join(" ");
    format!("(list {items})")
}

fn serialize_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::String(text) => serialize_string(text),
        ExprKind::Bool(true) => "true".to_string(),
        ExprKind::Bool(false) => "false".to_string(),
        ExprKind::Symbol(name) => name.clone(),
        ExprKind::Lambda { params, body } => {
            let params = params.join(" ");
            let body = body
                .iter()
                .map(serialize_expr)
                .collect::<Vec<_>>()
                .join(" ");
            format!("(lambda ({params}) {body})")
        }
        ExprKind::Call { callee, args } => {
            let callee = serialize_expr(callee);
            if args.is_empty() {
                format!("({callee})")
            } else {
                let args = args
                    .iter()
                    .map(serialize_expr)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("({callee} {args})")
            }
        }
    }
}

fn serialize_string(text: &str) -> String {
    let mut output = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}
