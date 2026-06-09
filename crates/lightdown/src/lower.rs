use lightdown_ir::{
    Block, BlockKind, Document, DocumentMetadata, Inline, InlineKind, Node, Position, Span,
};

use crate::embedded_ir;
use crate::error::ParseError;
use crate::syntax::{SourceBlock, SourceBlockKind, SourceDocument, SourceInline, SourceInlineKind};

pub fn lower_document(_input: &str, document: SourceDocument) -> Result<Document, ParseError> {
    let metadata_span = zero_span();
    let blocks = lower_blocks(document.blocks)?;
    let end = blocks.last().map_or(metadata_span.end, |block| block.span.end);

    Ok(Document {
        metadata: DocumentMetadata {
            version: "0.1.0".to_string(),
            span: metadata_span,
        },
        blocks,
        span: Span {
            start: metadata_span.start,
            end,
        },
    })
}

pub fn lower_blocks(blocks: Vec<SourceBlock>) -> Result<Vec<Block>, ParseError> {
    blocks.into_iter().map(lower_block).collect()
}

fn lower_block(block: SourceBlock) -> Result<Block, ParseError> {
    let span = block.span;
    let kind = match block.kind {
        SourceBlockKind::Heading { level, inlines } => BlockKind::Heading {
            level,
            inlines: lower_inlines(inlines)?,
        },
        SourceBlockKind::Paragraph(inlines) => BlockKind::Paragraph(lower_inlines(inlines)?),
        SourceBlockKind::EmbeddedIr(source) => {
            return embedded_ir::lower_block_fragment(source, span);
        }
        SourceBlockKind::List { ordered, items } => BlockKind::List {
            ordered,
            items: lower_blocks(items)?,
        },
        SourceBlockKind::ListItem(blocks) => BlockKind::ListItem(lower_blocks(blocks)?),
        SourceBlockKind::BlockQuote(blocks) => BlockKind::BlockQuote(lower_blocks(blocks)?),
        SourceBlockKind::CodeBlock { lang, text } => BlockKind::CodeBlock { lang, text },
        SourceBlockKind::ThematicBreak => BlockKind::ThematicBreak,
    };

    Ok(Node { kind, span })
}

pub fn lower_inlines(inlines: Vec<SourceInline>) -> Result<Vec<Inline>, ParseError> {
    inlines.into_iter().map(lower_inline).collect()
}

pub fn lower_inline_vec(inlines: Vec<SourceInline>) -> Result<Vec<Inline>, ParseError> {
    lower_inlines(inlines)
}

fn lower_inline(inline: SourceInline) -> Result<Inline, ParseError> {
    let span = inline.span;
    let kind = match inline.kind {
        SourceInlineKind::Text(text) => InlineKind::Text(text),
        SourceInlineKind::Emphasis(children) => InlineKind::Emphasis(lower_inlines(children)?),
        SourceInlineKind::Strong(children) => InlineKind::Strong(lower_inlines(children)?),
        SourceInlineKind::Code(text) => InlineKind::Code(text),
        SourceInlineKind::Link { href, children } => InlineKind::Link {
            href,
            children: lower_inlines(children)?,
        },
        SourceInlineKind::Image { src, alt } => InlineKind::Image {
            src,
            alt: Some(alt),
        },
        SourceInlineKind::EmbeddedIr(source) => {
            return embedded_ir::lower_inline_fragment(source, span);
        }
    };

    Ok(Node { kind, span })
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
