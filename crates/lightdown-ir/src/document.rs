use crate::Span;
use crate::ast::Node;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub metadata: DocumentMetadata,
    pub blocks: Vec<Block>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentMetadata {
    pub version: String,
    pub span: Span,
}

pub type Block = Node<BlockKind>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlockKind {
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph(Vec<Inline>),
    List { ordered: bool, items: Vec<Block> },
    ListItem(Vec<Block>),
    BlockQuote(Vec<Block>),
    CodeBlock { lang: Option<String>, text: String },
    ThematicBreak,
    Table(Vec<TableChild>),
}

pub type Inline = Node<InlineKind>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InlineKind {
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Code(String),
    Link { href: String, children: Vec<Inline> },
    Image { src: String, alt: Option<String> },
    Break,
}

pub type TableChild = Node<TableChildKind>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TableChildKind {
    Head(Vec<TableRow>),
    Body(Vec<TableRow>),
}

pub type TableRow = Node<TableRowKind>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRowKind {
    pub cells: Vec<TableCell>,
}

pub type TableCell = Node<TableCellKind>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TableCellKind {
    Header(Vec<Inline>),
    Data(Vec<Inline>),
}
