mod block_parser;
mod embedded_ir;
mod error;
mod inline_parser;
mod lower;
mod syntax;

pub use error::{ParseError, ParseErrorKind, RenderError};

pub fn parse(input: &str) -> Result<lightdown_ir::Module, ParseError> {
    let blocks = block_parser::parse_document(input)?;
    lower::lower_document(input, blocks)
}

pub fn render_html(input: &str) -> Result<String, RenderError> {
    let module = parse(input)?;
    let document = lightdown_ir::execute_document(&lightdown_ir::compile_module(&module)?)
        .map_err(RenderError::from)?;
    lightdown_html::render_document(&document).map_err(RenderError::from)
}
