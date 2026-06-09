mod block_parser;
mod embedded_ir;
mod error;
mod inline_parser;
mod lower;
mod syntax;

pub use error::{ParseError, ParseErrorKind, RenderError};

pub fn parse(input: &str) -> Result<lightdown_ir::Document, ParseError> {
    let blocks = block_parser::parse_document(input)?;
    lower::lower_document(input, blocks)
}

pub fn render_html(input: &str) -> Result<String, RenderError> {
    let document = parse(input)?;
    Ok(lightdown_html::render_document(&document))
}
