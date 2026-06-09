use std::fmt;

use lightdown_ir::{ParseError as IrParseError, ParseErrorKind as IrParseErrorKind, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Option<Span>,
}

impl ParseError {
    pub(crate) fn new(kind: ParseErrorKind, span: Option<Span>) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    UnterminatedCodeFence,
    UnterminatedInlineCode,
    UnterminatedEmphasis,
    UnterminatedStrong,
    UnterminatedLink,
    UnterminatedImage,
    UnterminatedEmbeddedIr,
    EmbeddedIr(IrParseErrorKind),
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "failed to parse Lightdown author syntax: {:?}", self.kind)
    }
}

impl std::error::Error for ParseError {}

impl From<IrParseError> for ParseError {
    fn from(error: IrParseError) -> Self {
        Self {
            kind: ParseErrorKind::EmbeddedIr(error.kind),
            span: error.span,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderError {
    Parse(ParseError),
}

impl From<ParseError> for RenderError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::Parse(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RenderError {}
