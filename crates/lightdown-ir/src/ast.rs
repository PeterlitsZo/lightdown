use crate::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node<T> {
    pub kind: T,
    pub span: Span,
}

impl<T> Node<T> {
    pub(crate) const fn new(kind: T, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    pub metadata: ModuleMetadata,
    pub body: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleMetadata {
    pub version: String,
    pub span: Span,
}

pub type Expr = Node<ExprKind>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprKind {
    String(String),
    Bool(bool),
    Symbol(String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Lambda {
        params: Vec<String>,
        body: Vec<Expr>,
    },
}
