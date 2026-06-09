use std::fmt;

use lightdown_ir::{
    CompileError as IrCompileError, ParseError as IrParseError, ParseErrorKind as IrParseErrorKind,
    Span, VmError as IrVmError,
};

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
        write!(
            formatter,
            "failed to parse Lightdown author syntax: {:?}",
            self.kind
        )
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
    Compile(IrCompileError),
    Vm(IrVmError),
}

impl From<ParseError> for RenderError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<IrCompileError> for RenderError {
    fn from(error: IrCompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<IrVmError> for RenderError {
    fn from(error: IrVmError) -> Self {
        Self::Vm(error)
    }
}

impl From<lightdown_html::RenderError> for RenderError {
    fn from(error: lightdown_html::RenderError) -> Self {
        match error {
            lightdown_html::RenderError::Parse(error) => Self::Parse(ParseError::from(error)),
            lightdown_html::RenderError::Compile(error) => Self::Compile(error),
            lightdown_html::RenderError::Vm(error) => Self::Vm(error),
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::Parse(error) => error.fmt(formatter),
            RenderError::Compile(error) => {
                write!(
                    formatter,
                    "failed to compile Lightdown IR to bytecode: {error:?}"
                )
            }
            RenderError::Vm(error) => {
                write!(formatter, "failed to execute Lightdown bytecode: {error:?}")
            }
        }
    }
}

impl std::error::Error for RenderError {}
