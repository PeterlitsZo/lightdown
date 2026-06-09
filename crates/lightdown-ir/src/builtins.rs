use crate::Span;
use crate::runtime::{
    BlockValue, CallableValue, DocumentValue, InlineValue, MetadataValue, NodeValue,
    TableCellValue, TableChildValue, TableRowValue, Value,
};
use crate::vm::VmError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Builtin {
    Doc,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    Ul,
    Ol,
    Li,
    BlockQuote,
    CodeBlock,
    Hr,
    Table,
    Thead,
    Tbody,
    Tr,
    Th,
    Td,
    Text,
    Em,
    Strong,
    Code,
    A,
    Img,
    Br,
    List,
    Map,
    Apply,
}

impl Builtin {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Doc => "doc",
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
            Self::H6 => "h6",
            Self::P => "p",
            Self::Ul => "ul",
            Self::Ol => "ol",
            Self::Li => "li",
            Self::BlockQuote => "blockquote",
            Self::CodeBlock => "codeblock",
            Self::Hr => "hr",
            Self::Table => "table",
            Self::Thead => "thead",
            Self::Tbody => "tbody",
            Self::Tr => "tr",
            Self::Th => "th",
            Self::Td => "td",
            Self::Text => "text",
            Self::Em => "em",
            Self::Strong => "strong",
            Self::Code => "code",
            Self::A => "a",
            Self::Img => "img",
            Self::Br => "br",
            Self::List => "list",
            Self::Map => "map",
            Self::Apply => "apply",
        }
    }
}

pub(crate) fn resolve_builtin(name: &str) -> Option<Builtin> {
    Some(match name {
        "doc" => Builtin::Doc,
        "h1" => Builtin::H1,
        "h2" => Builtin::H2,
        "h3" => Builtin::H3,
        "h4" => Builtin::H4,
        "h5" => Builtin::H5,
        "h6" => Builtin::H6,
        "p" => Builtin::P,
        "ul" => Builtin::Ul,
        "ol" => Builtin::Ol,
        "li" => Builtin::Li,
        "blockquote" => Builtin::BlockQuote,
        "codeblock" => Builtin::CodeBlock,
        "hr" => Builtin::Hr,
        "table" => Builtin::Table,
        "thead" => Builtin::Thead,
        "tbody" => Builtin::Tbody,
        "tr" => Builtin::Tr,
        "th" => Builtin::Th,
        "td" => Builtin::Td,
        "text" => Builtin::Text,
        "em" => Builtin::Em,
        "strong" => Builtin::Strong,
        "code" => Builtin::Code,
        "a" => Builtin::A,
        "img" => Builtin::Img,
        "br" => Builtin::Br,
        "list" => Builtin::List,
        "map" => Builtin::Map,
        "apply" => Builtin::Apply,
        _ => return None,
    })
}

pub(crate) fn call_callable(
    callable: CallableValue,
    args: Vec<Value>,
    span: Span,
    metadata: &MetadataValue,
) -> Result<Value, VmError> {
    match callable {
        CallableValue::Builtin(builtin) => call_builtin(builtin, args, span, metadata),
    }
}

fn call_builtin(
    builtin: Builtin,
    args: Vec<Value>,
    span: Span,
    metadata: &MetadataValue,
) -> Result<Value, VmError> {
    match builtin {
        Builtin::Doc => Ok(Value::Node(NodeValue::Document(DocumentValue {
            metadata: metadata.clone(),
            blocks: expect_blocks(builtin, args, span)?,
            span,
        }))),
        Builtin::H1 => heading(1, args, span, builtin),
        Builtin::H2 => heading(2, args, span, builtin),
        Builtin::H3 => heading(3, args, span, builtin),
        Builtin::H4 => heading(4, args, span, builtin),
        Builtin::H5 => heading(5, args, span, builtin),
        Builtin::H6 => heading(6, args, span, builtin),
        Builtin::P => Ok(Value::Node(NodeValue::Block(BlockValue::Paragraph {
            inlines: expect_inlines(builtin, args, span)?,
            span,
        }))),
        Builtin::Ul => list_block(false, args, span, builtin),
        Builtin::Ol => list_block(true, args, span, builtin),
        Builtin::Li => Ok(Value::Node(NodeValue::Block(BlockValue::ListItem {
            children: expect_blocks(builtin, args, span)?,
            span,
        }))),
        Builtin::BlockQuote => Ok(Value::Node(NodeValue::Block(BlockValue::BlockQuote {
            children: expect_blocks(builtin, args, span)?,
            span,
        }))),
        Builtin::CodeBlock => codeblock(args, span, builtin),
        Builtin::Hr => {
            expect_arity(builtin, &args, "0", span)?;
            Ok(Value::Node(NodeValue::Block(BlockValue::ThematicBreak {
                span,
            })))
        }
        Builtin::Table => table(args, span, builtin),
        Builtin::Thead => Ok(Value::Node(NodeValue::TableChild(TableChildValue::Head {
            rows: expect_rows(builtin, args, span)?,
            span,
        }))),
        Builtin::Tbody => Ok(Value::Node(NodeValue::TableChild(TableChildValue::Body {
            rows: expect_rows(builtin, args, span)?,
            span,
        }))),
        Builtin::Tr => Ok(Value::Node(NodeValue::TableRow(TableRowValue {
            cells: expect_cells(builtin, args, span)?,
            span,
        }))),
        Builtin::Th => Ok(Value::Node(NodeValue::TableCell(TableCellValue::Header {
            children: expect_inlines(builtin, args, span)?,
            span,
        }))),
        Builtin::Td => Ok(Value::Node(NodeValue::TableCell(TableCellValue::Data {
            children: expect_inlines(builtin, args, span)?,
            span,
        }))),
        Builtin::Text => {
            expect_arity(builtin, &args, "1", span)?;
            let text = expect_string(builtin, args.into_iter().next().expect("arity checked"), span)?;
            Ok(Value::Node(NodeValue::Inline(InlineValue::Text { text, span })))
        }
        Builtin::Em => Ok(Value::Node(NodeValue::Inline(InlineValue::Emphasis {
            children: expect_inlines(builtin, args, span)?,
            span,
        }))),
        Builtin::Strong => Ok(Value::Node(NodeValue::Inline(InlineValue::Strong {
            children: expect_inlines(builtin, args, span)?,
            span,
        }))),
        Builtin::Code => {
            expect_arity(builtin, &args, "1", span)?;
            let text = expect_string(builtin, args.into_iter().next().expect("arity checked"), span)?;
            Ok(Value::Node(NodeValue::Inline(InlineValue::Code { text, span })))
        }
        Builtin::A => link(args, span, builtin),
        Builtin::Img => image(args, span, builtin),
        Builtin::Br => {
            expect_arity(builtin, &args, "0", span)?;
            Ok(Value::Node(NodeValue::Inline(InlineValue::Break { span })))
        }
        Builtin::List => Ok(Value::List(args)),
        Builtin::Map => map_builtin(args, span, metadata),
        Builtin::Apply => apply_builtin(args, span, metadata),
    }
}

fn heading(level: u8, args: Vec<Value>, span: Span, builtin: Builtin) -> Result<Value, VmError> {
    Ok(Value::Node(NodeValue::Block(BlockValue::Heading {
        level,
        inlines: expect_inlines(builtin, args, span)?,
        span,
    })))
}

fn list_block(
    ordered: bool,
    args: Vec<Value>,
    span: Span,
    builtin: Builtin,
) -> Result<Value, VmError> {
    Ok(Value::Node(NodeValue::Block(BlockValue::List {
        ordered,
        items: expect_blocks(builtin, args, span)?,
        span,
    })))
}

fn codeblock(args: Vec<Value>, span: Span, builtin: Builtin) -> Result<Value, VmError> {
    match args.as_slice() {
        [text] => Ok(Value::Node(NodeValue::Block(BlockValue::CodeBlock {
            lang: None,
            text: expect_string(builtin, text.clone(), span)?,
            span,
        }))),
        [lang, text] => Ok(Value::Node(NodeValue::Block(BlockValue::CodeBlock {
            lang: Some(expect_string(builtin, lang.clone(), span)?),
            text: expect_string(builtin, text.clone(), span)?,
            span,
        }))),
        _ => Err(VmError::BuiltinArityMismatch {
            builtin: builtin.name(),
            expected: "1 or 2",
            actual: args.len(),
            span,
        }),
    }
}

fn table(args: Vec<Value>, span: Span, builtin: Builtin) -> Result<Value, VmError> {
    let mut children = Vec::new();
    let mut rows = Vec::new();

    for value in args {
        match value {
            Value::Node(NodeValue::TableChild(child)) => {
                if !rows.is_empty() {
                    return Err(VmError::BuiltinTypeMismatch {
                        builtin: builtin.name(),
                        expected: "table child",
                        found: "table row",
                        span,
                    });
                }
                children.push(NodeValue::TableChild(child));
            }
            Value::Node(NodeValue::TableRow(row)) => {
                if !children.is_empty() {
                    return Err(VmError::BuiltinTypeMismatch {
                        builtin: builtin.name(),
                        expected: "table row",
                        found: "table child",
                        span,
                    });
                }
                rows.push(NodeValue::TableRow(row));
            }
            other => {
                return Err(VmError::BuiltinTypeMismatch {
                    builtin: builtin.name(),
                    expected: "table child",
                    found: other.kind_name(),
                    span,
                });
            }
        }
    }

    let children = if !rows.is_empty() {
        normalize_direct_table_rows(rows)
    } else {
        children
    };

    Ok(Value::Node(NodeValue::Block(BlockValue::Table {
        children,
        span,
    })))
}

fn link(args: Vec<Value>, span: Span, builtin: Builtin) -> Result<Value, VmError> {
    if args.is_empty() {
        return Err(VmError::BuiltinArityMismatch {
            builtin: builtin.name(),
            expected: "at least 1",
            actual: 0,
            span,
        });
    }

    let mut args = args.into_iter();
    let href = expect_string(builtin, args.next().expect("length checked"), span)?;
    let children = expect_inlines(builtin, args.collect(), span)?;
    Ok(Value::Node(NodeValue::Inline(InlineValue::Link {
        href,
        children,
        span,
    })))
}

fn image(args: Vec<Value>, span: Span, builtin: Builtin) -> Result<Value, VmError> {
    match args.as_slice() {
        [src] => Ok(Value::Node(NodeValue::Inline(InlineValue::Image {
            src: expect_string(builtin, src.clone(), span)?,
            alt: None,
            span,
        }))),
        [src, alt] => Ok(Value::Node(NodeValue::Inline(InlineValue::Image {
            src: expect_string(builtin, src.clone(), span)?,
            alt: Some(expect_string(builtin, alt.clone(), span)?),
            span,
        }))),
        _ => Err(VmError::BuiltinArityMismatch {
            builtin: builtin.name(),
            expected: "1 or 2",
            actual: args.len(),
            span,
        }),
    }
}

fn map_builtin(args: Vec<Value>, span: Span, metadata: &MetadataValue) -> Result<Value, VmError> {
    expect_named_arity("map", &args, 2, span)?;
    let mut args = args.into_iter();
    let callable = expect_callable("map", args.next().expect("arity checked"), span)?;
    let list = expect_list("map", args.next().expect("arity checked"), span)?;

    let mut mapped = Vec::with_capacity(list.len());
    for item in list {
        mapped.push(call_callable(callable.clone(), vec![item], span, metadata)?);
    }
    Ok(Value::List(mapped))
}

fn apply_builtin(
    args: Vec<Value>,
    span: Span,
    metadata: &MetadataValue,
) -> Result<Value, VmError> {
    expect_named_arity("apply", &args, 2, span)?;
    let mut args = args.into_iter();
    let callable = expect_callable("apply", args.next().expect("arity checked"), span)?;
    let values = expect_list("apply", args.next().expect("arity checked"), span)?;
    call_callable(callable, values, span, metadata)
}

fn expect_arity(
    builtin: Builtin,
    args: &[Value],
    expected: &'static str,
    span: Span,
) -> Result<(), VmError> {
    if args.len().to_string() == expected {
        Ok(())
    } else {
        Err(VmError::BuiltinArityMismatch {
            builtin: builtin.name(),
            expected,
            actual: args.len(),
            span,
        })
    }
}

fn expect_named_arity(
    builtin: &'static str,
    args: &[Value],
    expected: usize,
    span: Span,
) -> Result<(), VmError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(VmError::BuiltinArityMismatch {
            builtin,
            expected: match expected {
                0 => "0",
                1 => "1",
                2 => "2",
                _ => "n",
            },
            actual: args.len(),
            span,
        })
    }
}

fn expect_string(builtin: Builtin, value: Value, span: Span) -> Result<String, VmError> {
    match value {
        Value::String(text) => Ok(text),
        other => Err(VmError::BuiltinTypeMismatch {
            builtin: builtin.name(),
            expected: "string",
            found: other.kind_name(),
            span,
        }),
    }
}

fn expect_list(
    builtin: &'static str,
    value: Value,
    span: Span,
) -> Result<Vec<Value>, VmError> {
    match value {
        Value::List(values) => Ok(values),
        other => Err(VmError::BuiltinTypeMismatch {
            builtin,
            expected: "list",
            found: other.kind_name(),
            span,
        }),
    }
}

fn expect_callable(
    builtin: &'static str,
    value: Value,
    span: Span,
) -> Result<CallableValue, VmError> {
    match value {
        Value::Callable(callable) => Ok(callable),
        other => Err(VmError::NonCallableValue {
            found: other.kind_name(),
            builtin: Some(builtin),
            span: value_span(&other).or(Some(span)),
        }),
    }
}

fn expect_blocks(builtin: Builtin, args: Vec<Value>, span: Span) -> Result<Vec<NodeValue>, VmError> {
    args.into_iter()
        .map(|value| expect_node_kind(builtin, value, "block", span))
        .collect()
}

fn expect_inlines(
    builtin: Builtin,
    args: Vec<Value>,
    span: Span,
) -> Result<Vec<NodeValue>, VmError> {
    let mut inlines = Vec::new();
    for value in args {
        collect_inline_nodes(builtin, value, span, &mut inlines)?;
    }
    Ok(inlines)
}

fn expect_rows(builtin: Builtin, args: Vec<Value>, span: Span) -> Result<Vec<NodeValue>, VmError> {
    args.into_iter()
        .map(|value| expect_node_kind(builtin, value, "table row", span))
        .collect()
}

fn expect_cells(
    builtin: Builtin,
    args: Vec<Value>,
    span: Span,
) -> Result<Vec<NodeValue>, VmError> {
    args.into_iter()
        .map(|value| expect_node_kind(builtin, value, "table cell", span))
        .collect()
}

fn expect_node_kind(
    builtin: Builtin,
    value: Value,
    expected: &'static str,
    span: Span,
) -> Result<NodeValue, VmError> {
    match value {
        Value::Node(node) if node.kind_name() == expected => Ok(node),
        Value::Node(node) => Err(VmError::BuiltinTypeMismatch {
            builtin: builtin.name(),
            expected,
            found: node.kind_name(),
            span,
        }),
        other => Err(VmError::BuiltinTypeMismatch {
            builtin: builtin.name(),
            expected,
            found: other.kind_name(),
            span,
        }),
    }
}

fn collect_inline_nodes(
    builtin: Builtin,
    value: Value,
    span: Span,
    output: &mut Vec<NodeValue>,
) -> Result<(), VmError> {
    match value {
        Value::Node(NodeValue::Inline(inline)) => {
            output.push(NodeValue::Inline(inline));
            Ok(())
        }
        Value::List(values) => {
            for value in values {
                collect_inline_nodes(builtin, value, span, output)?;
            }
            Ok(())
        }
        Value::Node(node) => Err(VmError::BuiltinTypeMismatch {
            builtin: builtin.name(),
            expected: "inline",
            found: node.kind_name(),
            span,
        }),
        other => Err(VmError::BuiltinTypeMismatch {
            builtin: builtin.name(),
            expected: "inline",
            found: other.kind_name(),
            span,
        }),
    }
}

fn normalize_direct_table_rows(rows: Vec<NodeValue>) -> Vec<NodeValue> {
    let head_len = rows
        .iter()
        .take_while(|row| match row {
            NodeValue::TableRow(row) => row.cells.iter().all(|cell| {
                matches!(cell, NodeValue::TableCell(TableCellValue::Header { .. }))
            }),
            _ => false,
        })
        .count();

    let mut children = Vec::new();
    if head_len > 0 {
        let span = node_span(&rows[0]).expect("table row has span");
        let end = node_span(&rows[head_len - 1]).expect("table row has span").end;
        children.push(NodeValue::TableChild(TableChildValue::Head {
            rows: rows[..head_len].to_vec(),
            span: Span {
                start: span.start,
                end,
            },
        }));
    }

    if head_len < rows.len() {
        let span = node_span(&rows[head_len]).expect("table row has span");
        let end = node_span(rows.last().expect("table row exists"))
            .expect("table row has span")
            .end;
        children.push(NodeValue::TableChild(TableChildValue::Body {
            rows: rows[head_len..].to_vec(),
            span: Span {
                start: span.start,
                end,
            },
        }));
    }

    children
}

fn node_span(node: &NodeValue) -> Option<Span> {
    match node {
        NodeValue::Document(document) => Some(document.span),
        NodeValue::Block(block) => match block {
            BlockValue::Heading { span, .. }
            | BlockValue::Paragraph { span, .. }
            | BlockValue::List { span, .. }
            | BlockValue::ListItem { span, .. }
            | BlockValue::BlockQuote { span, .. }
            | BlockValue::CodeBlock { span, .. }
            | BlockValue::ThematicBreak { span }
            | BlockValue::Table { span, .. } => Some(*span),
        },
        NodeValue::Inline(inline) => match inline {
            InlineValue::Text { span, .. }
            | InlineValue::Emphasis { span, .. }
            | InlineValue::Strong { span, .. }
            | InlineValue::Code { span, .. }
            | InlineValue::Link { span, .. }
            | InlineValue::Image { span, .. }
            | InlineValue::Break { span } => Some(*span),
        },
        NodeValue::TableChild(child) => match child {
            TableChildValue::Head { span, .. } | TableChildValue::Body { span, .. } => Some(*span),
        },
        NodeValue::TableRow(row) => Some(row.span),
        NodeValue::TableCell(cell) => match cell {
            TableCellValue::Header { span, .. } | TableCellValue::Data { span, .. } => Some(*span),
        },
    }
}

fn value_span(value: &Value) -> Option<Span> {
    match value {
        Value::Node(node) => node_span(node),
        _ => None,
    }
}
