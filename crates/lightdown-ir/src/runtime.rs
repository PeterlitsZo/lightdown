use crate::Span;
use crate::ast::Node;
use crate::builtins::Builtin;
use crate::bytecode::FunctionId;
use crate::document::{
    Block, BlockKind, Document, DocumentMetadata, Inline, InlineKind, TableCell, TableCellKind,
    TableChild, TableChildKind, TableRow, TableRowKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    String(String),
    Bool(bool),
    List(Vec<Value>),
    Node(NodeValue),
    Callable(CallableValue),
    Unit,
}

impl Value {
    pub(crate) const fn kind_name(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Bool(_) => "bool",
            Self::List(_) => "list",
            Self::Node(node) => node.kind_name(),
            Self::Callable(_) => "callable",
            Self::Unit => "unit",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallableValue {
    Builtin(Builtin),
    Closure(ClosureValue),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureValue {
    pub function: FunctionId,
    pub captures: Vec<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeValue {
    Document(DocumentValue),
    Block(BlockValue),
    Inline(InlineValue),
    TableChild(TableChildValue),
    TableRow(TableRowValue),
    TableCell(TableCellValue),
}

impl NodeValue {
    pub(crate) const fn kind_name(&self) -> &'static str {
        match self {
            Self::Document(_) => "document",
            Self::Block(_) => "block",
            Self::Inline(_) => "inline",
            Self::TableChild(_) => "table child",
            Self::TableRow(_) => "table row",
            Self::TableCell(_) => "table cell",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentValue {
    pub metadata: MetadataValue,
    pub blocks: Vec<NodeValue>,
    pub span: Span,
}

impl DocumentValue {
    pub(crate) fn try_into_document(self) -> Result<Document, DecodeError> {
        Ok(Document {
            metadata: self.metadata.into_metadata(),
            blocks: decode_blocks(self.blocks)?,
            span: self.span,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataValue {
    pub version: String,
    pub span: Span,
}

impl MetadataValue {
    pub(crate) fn into_metadata(self) -> DocumentMetadata {
        DocumentMetadata {
            version: self.version,
            span: self.span,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlockValue {
    Heading {
        level: u8,
        inlines: Vec<NodeValue>,
        span: Span,
    },
    Paragraph {
        inlines: Vec<NodeValue>,
        span: Span,
    },
    List {
        ordered: bool,
        items: Vec<NodeValue>,
        span: Span,
    },
    ListItem {
        children: Vec<NodeValue>,
        span: Span,
    },
    BlockQuote {
        children: Vec<NodeValue>,
        span: Span,
    },
    CodeBlock {
        lang: Option<String>,
        text: String,
        span: Span,
    },
    ThematicBreak {
        span: Span,
    },
    Table {
        children: Vec<NodeValue>,
        span: Span,
    },
}

impl BlockValue {
    pub(crate) fn into_block(self) -> Result<Block, DecodeError> {
        let (kind, span) = match self {
            Self::Heading {
                level,
                inlines,
                span,
            } => (
                BlockKind::Heading {
                    level,
                    inlines: decode_inlines(inlines)?,
                },
                span,
            ),
            Self::Paragraph { inlines, span } => {
                (BlockKind::Paragraph(decode_inlines(inlines)?), span)
            }
            Self::List {
                ordered,
                items,
                span,
            } => (
                BlockKind::List {
                    ordered,
                    items: decode_blocks(items)?,
                },
                span,
            ),
            Self::ListItem { children, span } => {
                (BlockKind::ListItem(decode_blocks(children)?), span)
            }
            Self::BlockQuote { children, span } => {
                (BlockKind::BlockQuote(decode_blocks(children)?), span)
            }
            Self::CodeBlock { lang, text, span } => (BlockKind::CodeBlock { lang, text }, span),
            Self::ThematicBreak { span } => (BlockKind::ThematicBreak, span),
            Self::Table { children, span } => {
                (BlockKind::Table(decode_table_children(children)?), span)
            }
        };

        Ok(Node::new(kind, span))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InlineValue {
    Text {
        text: String,
        span: Span,
    },
    Emphasis {
        children: Vec<NodeValue>,
        span: Span,
    },
    Strong {
        children: Vec<NodeValue>,
        span: Span,
    },
    Code {
        text: String,
        span: Span,
    },
    Link {
        href: String,
        children: Vec<NodeValue>,
        span: Span,
    },
    Image {
        src: String,
        alt: Option<String>,
        span: Span,
    },
    Break {
        span: Span,
    },
}

impl InlineValue {
    pub(crate) fn into_inline(self) -> Result<Inline, DecodeError> {
        let (kind, span) = match self {
            Self::Text { text, span } => (InlineKind::Text(text), span),
            Self::Emphasis { children, span } => {
                (InlineKind::Emphasis(decode_inlines(children)?), span)
            }
            Self::Strong { children, span } => {
                (InlineKind::Strong(decode_inlines(children)?), span)
            }
            Self::Code { text, span } => (InlineKind::Code(text), span),
            Self::Link {
                href,
                children,
                span,
            } => (
                InlineKind::Link {
                    href,
                    children: decode_inlines(children)?,
                },
                span,
            ),
            Self::Image { src, alt, span } => (InlineKind::Image { src, alt }, span),
            Self::Break { span } => (InlineKind::Break, span),
        };

        Ok(Node::new(kind, span))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TableChildValue {
    Head { rows: Vec<NodeValue>, span: Span },
    Body { rows: Vec<NodeValue>, span: Span },
}

impl TableChildValue {
    pub(crate) fn into_table_child(self) -> Result<TableChild, DecodeError> {
        let (kind, span) = match self {
            Self::Head { rows, span } => (TableChildKind::Head(decode_table_rows(rows)?), span),
            Self::Body { rows, span } => (TableChildKind::Body(decode_table_rows(rows)?), span),
        };

        Ok(Node::new(kind, span))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRowValue {
    pub cells: Vec<NodeValue>,
    pub span: Span,
}

impl TableRowValue {
    pub(crate) fn into_table_row(self) -> Result<TableRow, DecodeError> {
        Ok(Node::new(
            TableRowKind {
                cells: decode_table_cells(self.cells)?,
            },
            self.span,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TableCellValue {
    Header {
        children: Vec<NodeValue>,
        span: Span,
    },
    Data {
        children: Vec<NodeValue>,
        span: Span,
    },
}

impl TableCellValue {
    pub(crate) fn into_table_cell(self) -> Result<TableCell, DecodeError> {
        let (kind, span) = match self {
            Self::Header { children, span } => {
                (TableCellKind::Header(decode_inlines(children)?), span)
            }
            Self::Data { children, span } => (TableCellKind::Data(decode_inlines(children)?), span),
        };

        Ok(Node::new(kind, span))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeError {
    ExpectedBlock,
    ExpectedInline,
    ExpectedTableChild,
    ExpectedTableRow,
    ExpectedTableCell,
}

fn decode_blocks(values: Vec<NodeValue>) -> Result<Vec<Block>, DecodeError> {
    values
        .into_iter()
        .map(|value| match value {
            NodeValue::Block(block) => block.into_block(),
            _ => Err(DecodeError::ExpectedBlock),
        })
        .collect()
}

fn decode_inlines(values: Vec<NodeValue>) -> Result<Vec<Inline>, DecodeError> {
    values
        .into_iter()
        .map(|value| match value {
            NodeValue::Inline(inline) => inline.into_inline(),
            _ => Err(DecodeError::ExpectedInline),
        })
        .collect()
}

fn decode_table_children(values: Vec<NodeValue>) -> Result<Vec<TableChild>, DecodeError> {
    values
        .into_iter()
        .map(|value| match value {
            NodeValue::TableChild(child) => child.into_table_child(),
            _ => Err(DecodeError::ExpectedTableChild),
        })
        .collect()
}

fn decode_table_rows(values: Vec<NodeValue>) -> Result<Vec<TableRow>, DecodeError> {
    values
        .into_iter()
        .map(|value| match value {
            NodeValue::TableRow(row) => row.into_table_row(),
            _ => Err(DecodeError::ExpectedTableRow),
        })
        .collect()
}

fn decode_table_cells(values: Vec<NodeValue>) -> Result<Vec<TableCell>, DecodeError> {
    values
        .into_iter()
        .map(|value| match value {
            NodeValue::TableCell(cell) => cell.into_table_cell(),
            _ => Err(DecodeError::ExpectedTableCell),
        })
        .collect()
}
