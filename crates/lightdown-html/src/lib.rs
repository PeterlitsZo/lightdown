use std::fmt;

use lightdown_ir::{
    Block, BlockKind, CompileError, Document, Inline, InlineKind, ParseError, TableCell,
    TableCellKind, TableChild, TableChildKind, TableRow, VmError, compile_module,
    execute_document, parse,
};

pub fn render(input: &str) -> Result<String, RenderError> {
    let module = parse(input).map_err(RenderError::from)?;
    let document = execute_document(&compile_module(&module).map_err(RenderError::from)?)
        .map_err(RenderError::from)?;
    render_document(&document)
}

pub fn render_document(document: &Document) -> Result<String, RenderError> {
    Ok(render_resolved_document(document))
}

fn render_resolved_document(document: &Document) -> String {
    let mut output = String::new();
    render_blocks(&document.blocks, &mut output);
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderError {
    Parse(ParseError),
    Compile(CompileError),
    Vm(VmError),
}

impl From<ParseError> for RenderError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<CompileError> for RenderError {
    fn from(error: CompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<VmError> for RenderError {
    fn from(error: VmError) -> Self {
        Self::Vm(error)
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::Parse(error) => {
                write!(formatter, "failed to parse Lightdown IR: {error:?}")
            }
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

fn render_blocks(blocks: &[Block], output: &mut String) {
    for block in blocks {
        render_block(block, output);
    }
}

fn render_block(block: &Block, output: &mut String) {
    match &block.kind {
        BlockKind::Heading { level, inlines } => {
            output.push_str("<h");
            output.push(char::from(b'0' + *level));
            output.push('>');
            render_inlines(inlines, output);
            output.push_str("</h");
            output.push(char::from(b'0' + *level));
            output.push('>');
        }
        BlockKind::Paragraph(inlines) => {
            output.push_str("<p>");
            render_inlines(inlines, output);
            output.push_str("</p>");
        }
        BlockKind::List { ordered, items } => {
            if *ordered {
                output.push_str("<ol>");
            } else {
                output.push_str("<ul>");
            }
            render_blocks(items, output);
            if *ordered {
                output.push_str("</ol>");
            } else {
                output.push_str("</ul>");
            }
        }
        BlockKind::ListItem(blocks) => {
            output.push_str("<li>");
            render_blocks(blocks, output);
            output.push_str("</li>");
        }
        BlockKind::BlockQuote(blocks) => {
            output.push_str("<blockquote>");
            render_blocks(blocks, output);
            output.push_str("</blockquote>");
        }
        BlockKind::CodeBlock { lang, text } => {
            output.push_str("<pre><code");
            if let Some(lang) = lang {
                output.push_str(r#" class="language-"#);
                escape_attribute(lang, output);
                output.push('"');
            }
            output.push('>');
            escape_text(text, output);
            output.push_str("</code></pre>");
        }
        BlockKind::ThematicBreak => output.push_str("<hr>"),
        BlockKind::Table(children) => {
            output.push_str("<table>");
            for child in children {
                render_table_child(child, output);
            }
            output.push_str("</table>");
        }
    }
}

fn render_inlines(inlines: &[Inline], output: &mut String) {
    for inline in inlines {
        render_inline(inline, output);
    }
}

fn render_inline(inline: &Inline, output: &mut String) {
    match &inline.kind {
        InlineKind::Text(text) => escape_text(text, output),
        InlineKind::Emphasis(inlines) => {
            output.push_str("<em>");
            render_inlines(inlines, output);
            output.push_str("</em>");
        }
        InlineKind::Strong(inlines) => {
            output.push_str("<strong>");
            render_inlines(inlines, output);
            output.push_str("</strong>");
        }
        InlineKind::Code(text) => {
            output.push_str("<code>");
            escape_text(text, output);
            output.push_str("</code>");
        }
        InlineKind::Link { href, children } => {
            output.push_str(r#"<a href=""#);
            escape_attribute(href, output);
            output.push_str(r#"">"#);
            render_inlines(children, output);
            output.push_str("</a>");
        }
        InlineKind::Image { src, alt } => {
            output.push_str(r#"<img src=""#);
            escape_attribute(src, output);
            output.push('"');
            if let Some(alt) = alt {
                output.push_str(r#" alt=""#);
                escape_attribute(alt, output);
                output.push('"');
            }
            output.push('>');
        }
        InlineKind::Break => output.push_str("<br>"),
    }
}

fn render_table_child(child: &TableChild, output: &mut String) {
    match &child.kind {
        TableChildKind::Head(rows) => {
            output.push_str("<thead>");
            render_table_rows(rows, output);
            output.push_str("</thead>");
        }
        TableChildKind::Body(rows) => {
            output.push_str("<tbody>");
            render_table_rows(rows, output);
            output.push_str("</tbody>");
        }
    }
}

fn render_table_rows(rows: &[TableRow], output: &mut String) {
    for row in rows {
        output.push_str("<tr>");
        for cell in &row.kind.cells {
            render_table_cell(cell, output);
        }
        output.push_str("</tr>");
    }
}

fn render_table_cell(cell: &TableCell, output: &mut String) {
    match &cell.kind {
        TableCellKind::Header(inlines) => {
            output.push_str("<th>");
            render_inlines(inlines, output);
            output.push_str("</th>");
        }
        TableCellKind::Data(inlines) => {
            output.push_str("<td>");
            render_inlines(inlines, output);
            output.push_str("</td>");
        }
    }
}

fn escape_text(input: &str, output: &mut String) {
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(ch),
        }
    }
}

fn escape_attribute(input: &str, output: &mut String) {
    escape_text(input, output);
}
