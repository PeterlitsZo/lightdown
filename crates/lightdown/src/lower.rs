use lightdown_ir::{Expr, ExprKind, Module, ModuleMetadata, Node, Position, Span};

use crate::embedded_ir;
use crate::error::ParseError;
use crate::syntax::{SourceBlock, SourceBlockKind, SourceDocument, SourceInline, SourceInlineKind};

pub fn lower_document(_input: &str, document: SourceDocument) -> Result<Module, ParseError> {
    let metadata_span = zero_span();
    let args = lower_blocks(document.blocks)?;
    let end = args.last().map_or(metadata_span.end, |expr| expr.span.end);
    let span = Span {
        start: metadata_span.start,
        end,
    };

    Ok(Module {
        metadata: ModuleMetadata {
            version: "0.1.0".to_string(),
            span: metadata_span,
        },
        body: call_expr("doc", args, span),
        span,
    })
}

pub fn lower_blocks(blocks: Vec<SourceBlock>) -> Result<Vec<Expr>, ParseError> {
    blocks.into_iter().map(lower_block).collect()
}

fn lower_block(block: SourceBlock) -> Result<Expr, ParseError> {
    let span = block.span;
    match block.kind {
        SourceBlockKind::Heading { level, inlines } => Ok(call_expr(
            format!("h{level}"),
            lower_inlines(inlines)?,
            span,
        )),
        SourceBlockKind::Paragraph(inlines) => Ok(call_expr("p", lower_inlines(inlines)?, span)),
        SourceBlockKind::EmbeddedIr(source) => embedded_ir::lower_block_fragment(source, span),
        SourceBlockKind::List { ordered, items } => {
            let name = if ordered { "ol" } else { "ul" };
            Ok(call_expr(name, lower_blocks(items)?, span))
        }
        SourceBlockKind::ListItem(blocks) => Ok(call_expr("li", lower_blocks(blocks)?, span)),
        SourceBlockKind::BlockQuote(blocks) => {
            Ok(call_expr("blockquote", lower_blocks(blocks)?, span))
        }
        SourceBlockKind::CodeBlock { lang, text } => {
            let mut args = Vec::new();
            if let Some(lang) = lang {
                args.push(string_expr(lang, span));
            }
            args.push(string_expr(text, span));
            Ok(call_expr("codeblock", args, span))
        }
        SourceBlockKind::ThematicBreak => Ok(call_expr("hr", Vec::new(), span)),
    }
}

pub fn lower_inlines(inlines: Vec<SourceInline>) -> Result<Vec<Expr>, ParseError> {
    inlines.into_iter().map(lower_inline).collect()
}

pub fn lower_inline_vec(inlines: Vec<SourceInline>) -> Result<Vec<Expr>, ParseError> {
    lower_inlines(inlines)
}

fn lower_inline(inline: SourceInline) -> Result<Expr, ParseError> {
    let span = inline.span;
    match inline.kind {
        SourceInlineKind::Text(text) => Ok(call_expr("text", vec![string_expr(text, span)], span)),
        SourceInlineKind::Emphasis(children) => Ok(call_expr("em", lower_inlines(children)?, span)),
        SourceInlineKind::Strong(children) => {
            Ok(call_expr("strong", lower_inlines(children)?, span))
        }
        SourceInlineKind::Code(text) => Ok(call_expr("code", vec![string_expr(text, span)], span)),
        SourceInlineKind::Link { href, children } => {
            let mut args = vec![string_expr(href, span)];
            args.extend(lower_inlines(children)?);
            Ok(call_expr("a", args, span))
        }
        SourceInlineKind::Image { src, alt } => Ok(call_expr(
            "img",
            vec![string_expr(src, span), string_expr(alt, span)],
            span,
        )),
        SourceInlineKind::EmbeddedIr(source) => embedded_ir::lower_inline_fragment(source, span),
    }
}

fn call_expr(name: impl Into<String>, args: Vec<Expr>, span: Span) -> Expr {
    Node {
        kind: ExprKind::Call {
            callee: Box::new(symbol_expr(name, span)),
            args,
        },
        span,
    }
}

fn symbol_expr(name: impl Into<String>, span: Span) -> Expr {
    Node {
        kind: ExprKind::Symbol(name.into()),
        span,
    }
}

fn string_expr(text: impl Into<String>, span: Span) -> Expr {
    Node {
        kind: ExprKind::String(text.into()),
        span,
    }
}

fn zero_span() -> Span {
    let position = Position {
        offset: 0,
        line: 1,
        column: 1,
    };
    Span {
        start: position,
        end: position,
    }
}
