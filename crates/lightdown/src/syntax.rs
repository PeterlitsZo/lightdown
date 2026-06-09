use lightdown_ir::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDocument {
    pub blocks: Vec<SourceBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceBlock {
    pub kind: SourceBlockKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceInline {
    pub kind: SourceInlineKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceBlockKind {
    Heading { level: u8, inlines: Vec<SourceInline> },
    Paragraph(Vec<SourceInline>),
    EmbeddedIr(String),
    List { ordered: bool, items: Vec<SourceBlock> },
    ListItem(Vec<SourceBlock>),
    BlockQuote(Vec<SourceBlock>),
    CodeBlock { lang: Option<String>, text: String },
    ThematicBreak,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceInlineKind {
    Text(String),
    Emphasis(Vec<SourceInline>),
    Strong(Vec<SourceInline>),
    Code(String),
    Link { href: String, children: Vec<SourceInline> },
    Image { src: String, alt: String },
    EmbeddedIr(String),
}
