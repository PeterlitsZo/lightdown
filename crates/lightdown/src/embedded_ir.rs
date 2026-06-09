use lightdown_ir::{Block, Inline, Span};

use crate::error::ParseError;
use crate::{inline_parser, lower};

pub fn lower_inline_fragment(source: String, span: Span) -> Result<Inline, ParseError> {
    let resolved = resolve_nested_lightdown_fragments(&source, span)?;
    let fragment = format!("({resolved})");
    let mut inline =
        lightdown_ir::parse_inline_fragment(&fragment).map_err(ParseError::from)?;
    inline.span = span;
    Ok(inline)
}

pub fn lower_block_fragment(source: String, span: Span) -> Result<Block, ParseError> {
    let resolved = resolve_nested_lightdown_fragments(&source, span)?;
    let document_source = format!(
        "(doc {{:meta {{:version \"0.1.0\"}}}}\n  ({resolved}))"
    );
    let mut document = lightdown_ir::parse(&document_source).map_err(ParseError::from)?;
    let mut block = document
        .blocks
        .pop()
        .expect("embedded block document contains exactly one block");
    block.span = span;
    Ok(block)
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

fn serialize_inline_sequence(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(serialize_inline)
        .collect::<Vec<_>>()
        .join(" ")
}

fn serialize_inline(inline: &Inline) -> String {
    match &inline.kind {
        lightdown_ir::InlineKind::Text(text) => serialize_string(text),
        lightdown_ir::InlineKind::Emphasis(children) => {
            format!("(em {})", serialize_inline_sequence(children))
        }
        lightdown_ir::InlineKind::Strong(children) => {
            format!("(strong {})", serialize_inline_sequence(children))
        }
        lightdown_ir::InlineKind::Code(text) => format!("(code {})", serialize_string(text)),
        lightdown_ir::InlineKind::Link { href, children } => {
            format!(
                "(a {{:href {}}} {})",
                serialize_string(href),
                serialize_inline_sequence(children)
            )
        }
        lightdown_ir::InlineKind::Image { src, alt } => {
            let mut attributes = format!(":src {}", serialize_string(src));
            if let Some(alt) = alt {
                attributes.push_str(" :alt ");
                attributes.push_str(&serialize_string(alt));
            }
            format!("(img {{{}}})", attributes)
        }
        lightdown_ir::InlineKind::Break => "(br)".to_string(),
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
