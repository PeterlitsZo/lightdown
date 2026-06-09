use std::collections::BTreeMap;

use crate::ast::{
    Block, BlockKind, Document, DocumentMetadata, Inline, InlineKind, Node, TableCell,
    TableCellKind, TableChild, TableChildKind, TableRow, TableRowKind,
};
use crate::{LexError, LexErrorKind, Lexer, Span, Token, TokenKind};

pub fn parse(input: &str) -> Result<Document, ParseError> {
    Parser::new(Lexer::new(input))?.parse_document()
}

pub fn parse_inline_fragment(input: &str) -> Result<Inline, ParseError> {
    Parser::new(Lexer::new(input))?.parse_inline_fragment()
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new<I>(tokens: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = Result<Token, LexError>>,
    {
        let tokens = tokens
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(ParseError::from)?;
        Ok(Self { tokens, cursor: 0 })
    }

    pub fn parse_document(mut self) -> Result<Document, ParseError> {
        let document = self.parse_doc_node()?;
        if let Some(token) = self.peek() {
            return Err(ParseError::new(
                ParseErrorKind::ExtraInput,
                Some(token.span),
            ));
        }
        Ok(document)
    }

    pub fn parse_inline_fragment(mut self) -> Result<Inline, ParseError> {
        let inline = self.parse_inline("fragment", false)?;
        if let Some(token) = self.peek() {
            return Err(ParseError::new(
                ParseErrorKind::ExtraInput,
                Some(token.span),
            ));
        }
        Ok(inline)
    }

    fn parse_doc_node(&mut self) -> Result<Document, ParseError> {
        let start = self.expect_kind(TokenShape::LParen)?.span.start;
        let name = self.expect_symbol()?;
        if name.value != "doc" {
            return Err(ParseError::new(
                ParseErrorKind::UnknownNode { node: name.value },
                Some(name.span),
            ));
        }

        let attributes = self.parse_required_attribute_map("doc", &["meta"])?;
        let metadata = self.parse_document_metadata(attributes)?;
        let mut blocks = Vec::new();

        while !self.at_shape(TokenShape::RParen) {
            blocks.push(self.parse_block("doc")?);
        }

        let end = self.expect_kind(TokenShape::RParen)?.span.end;
        Ok(Document {
            metadata,
            blocks,
            span: Span { start, end },
        })
    }

    fn parse_document_metadata(
        &mut self,
        mut attributes: AttributeMap,
    ) -> Result<DocumentMetadata, ParseError> {
        let entry = attributes.remove("meta").ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::MissingAttribute {
                    node: "doc".into(),
                    attribute: "meta".into(),
                },
                None,
            )
        })?;

        let mut metadata = match entry.value {
            MapValue::Map(metadata) => metadata,
            MapValue::String(_) => {
                return Err(ParseError::new(
                    ParseErrorKind::InvalidAttributeType {
                        node: "doc".into(),
                        attribute: "meta".into(),
                        expected: "map",
                    },
                    Some(entry.span),
                ));
            }
        };

        let version = self.take_string_attribute(&mut metadata, "meta", "version")?;
        self.reject_remaining_attributes("meta", &metadata)?;

        Ok(DocumentMetadata {
            version: version.value,
            span: entry.span,
        })
    }

    fn parse_block(&mut self, parent: &str) -> Result<Block, ParseError> {
        let token = self.peek().ok_or_else(|| {
            ParseError::new(ParseErrorKind::UnexpectedEof { expected: "block" }, None)
        })?;

        if !matches!(token.kind, TokenKind::LParen) {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    expected: "block",
                    found: token_description(&token.kind),
                },
                Some(token.span),
            ));
        }

        let start = self.expect_kind(TokenShape::LParen)?.span.start;
        let name = self.expect_symbol()?;
        let block = match name.value.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.reject_attribute_map_if_present(name.value.as_str())?;
                let level = name.value[1..]
                    .parse::<u8>()
                    .expect("heading level is valid");
                let inlines = self.parse_inlines_until_rparen(name.value.as_str(), false)?;
                BlockKind::Heading { level, inlines }
            }
            "p" => {
                self.reject_attribute_map_if_present("p")?;
                BlockKind::Paragraph(self.parse_inlines_until_rparen("p", false)?)
            }
            "ul" | "ol" => {
                self.reject_attribute_map_if_present(name.value.as_str())?;
                let mut items = Vec::new();
                while !self.at_shape(TokenShape::RParen) {
                    let item = self.parse_block(name.value.as_str())?;
                    if !matches!(item.kind, BlockKind::ListItem(_)) {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidChild {
                                parent: name.value.clone(),
                                child: block_name(&item.kind).into(),
                            },
                            Some(item.span),
                        ));
                    }
                    items.push(item);
                }
                if items.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::MissingChild {
                            parent: name.value.clone(),
                            expected: "li",
                        },
                        Some(Span {
                            start,
                            end: self.current_end(start),
                        }),
                    ));
                }
                BlockKind::List {
                    ordered: name.value == "ol",
                    items,
                }
            }
            "li" => {
                self.reject_attribute_map_if_present("li")?;
                let children = self.parse_blocks_until_rparen("li")?;
                if children.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::MissingChild {
                            parent: "li".into(),
                            expected: "block",
                        },
                        Some(Span {
                            start,
                            end: self.current_end(start),
                        }),
                    ));
                }
                BlockKind::ListItem(children)
            }
            "blockquote" => {
                self.reject_attribute_map_if_present("blockquote")?;
                let children = self.parse_blocks_until_rparen("blockquote")?;
                if children.is_empty() {
                    return Err(ParseError::new(
                        ParseErrorKind::MissingChild {
                            parent: "blockquote".into(),
                            expected: "block",
                        },
                        Some(Span {
                            start,
                            end: self.current_end(start),
                        }),
                    ));
                }
                BlockKind::BlockQuote(children)
            }
            "codeblock" => {
                let mut attributes = self.parse_optional_attribute_map("codeblock", &["lang"])?;
                let lang =
                    self.take_optional_string_attribute(&mut attributes, "codeblock", "lang")?;
                self.reject_remaining_attributes("codeblock", &attributes)?;
                let text = self.expect_string_child("codeblock")?;
                if !self.at_shape(TokenShape::RParen) {
                    let child = self.describe_next_child()?;
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidChild {
                            parent: "codeblock".into(),
                            child,
                        },
                        self.peek().map(|token| token.span),
                    ));
                }
                BlockKind::CodeBlock { lang, text }
            }
            "hr" => {
                self.reject_attribute_map_if_present("hr")?;
                if !self.at_shape(TokenShape::RParen) {
                    let child = self.describe_next_child()?;
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidChild {
                            parent: "hr".into(),
                            child,
                        },
                        self.peek().map(|token| token.span),
                    ));
                }
                BlockKind::ThematicBreak
            }
            "table" => {
                self.reject_attribute_map_if_present("table")?;
                BlockKind::Table(self.parse_table_children()?)
            }
            inline if is_inline_node(inline) => {
                return Err(ParseError::new(
                    ParseErrorKind::InvalidChild {
                        parent: parent.into(),
                        child: inline.into(),
                    },
                    Some(name.span),
                ));
            }
            unknown => {
                return Err(ParseError::new(
                    ParseErrorKind::UnknownNode {
                        node: unknown.into(),
                    },
                    Some(name.span),
                ));
            }
        };

        let end = self.expect_kind(TokenShape::RParen)?.span.end;
        Ok(Node::new(block, Span { start, end }))
    }

    fn parse_blocks_until_rparen(&mut self, parent: &str) -> Result<Vec<Block>, ParseError> {
        let mut blocks = Vec::new();
        while !self.at_shape(TokenShape::RParen) {
            blocks.push(self.parse_block(parent)?);
        }
        Ok(blocks)
    }

    fn parse_inline(&mut self, parent: &str, inside_link: bool) -> Result<Inline, ParseError> {
        let Some(token) = self.peek() else {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedEof { expected: "inline" },
                None,
            ));
        };

        match &token.kind {
            TokenKind::String(text) => {
                let span = token.span;
                let text = text.clone();
                self.cursor += 1;
                Ok(Node::new(InlineKind::Text(text), span))
            }
            TokenKind::LParen => {
                let start = self.expect_kind(TokenShape::LParen)?.span.start;
                let name = self.expect_symbol()?;
                let inline = match name.value.as_str() {
                    "em" => {
                        self.reject_attribute_map_if_present("em")?;
                        InlineKind::Emphasis(self.parse_inlines_until_rparen("em", inside_link)?)
                    }
                    "strong" => {
                        self.reject_attribute_map_if_present("strong")?;
                        InlineKind::Strong(self.parse_inlines_until_rparen("strong", inside_link)?)
                    }
                    "code" => {
                        self.reject_attribute_map_if_present("code")?;
                        let text = self.expect_string_child("code")?;
                        if !self.at_shape(TokenShape::RParen) {
                            let child = self.describe_next_child()?;
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidChild {
                                    parent: "code".into(),
                                    child,
                                },
                                self.peek().map(|token| token.span),
                            ));
                        }
                        InlineKind::Code(text)
                    }
                    "a" => {
                        if inside_link {
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidChild {
                                    parent: "a".into(),
                                    child: "a".into(),
                                },
                                Some(name.span),
                            ));
                        }
                        let mut attributes = self.parse_required_attribute_map("a", &["href"])?;
                        let href = self
                            .take_string_attribute(&mut attributes, "a", "href")?
                            .value;
                        self.reject_remaining_attributes("a", &attributes)?;
                        let children = self.parse_inlines_until_rparen("a", true)?;
                        InlineKind::Link { href, children }
                    }
                    "img" => {
                        let mut attributes =
                            self.parse_required_attribute_map("img", &["src", "alt"])?;
                        let src = self
                            .take_string_attribute(&mut attributes, "img", "src")?
                            .value;
                        let alt =
                            self.take_optional_string_attribute(&mut attributes, "img", "alt")?;
                        self.reject_remaining_attributes("img", &attributes)?;
                        if !self.at_shape(TokenShape::RParen) {
                            let child = self.describe_next_child()?;
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidChild {
                                    parent: "img".into(),
                                    child,
                                },
                                self.peek().map(|token| token.span),
                            ));
                        }
                        InlineKind::Image { src, alt }
                    }
                    "br" => {
                        self.reject_attribute_map_if_present("br")?;
                        if !self.at_shape(TokenShape::RParen) {
                            let child = self.describe_next_child()?;
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidChild {
                                    parent: "br".into(),
                                    child,
                                },
                                self.peek().map(|token| token.span),
                            ));
                        }
                        InlineKind::Break
                    }
                    block if is_block_node(block) => {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidChild {
                                parent: parent.into(),
                                child: block.into(),
                            },
                            Some(name.span),
                        ));
                    }
                    unknown => {
                        return Err(ParseError::new(
                            ParseErrorKind::UnknownNode {
                                node: unknown.into(),
                            },
                            Some(name.span),
                        ));
                    }
                };

                let end = self.expect_kind(TokenShape::RParen)?.span.end;
                Ok(Node::new(inline, Span { start, end }))
            }
            _ => Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    expected: "inline",
                    found: token_description(&token.kind),
                },
                Some(token.span),
            )),
        }
    }

    fn parse_inlines_until_rparen(
        &mut self,
        parent: &str,
        inside_link: bool,
    ) -> Result<Vec<Inline>, ParseError> {
        let mut inlines = Vec::new();
        while !self.at_shape(TokenShape::RParen) {
            inlines.push(self.parse_inline(parent, inside_link)?);
        }
        Ok(inlines)
    }

    fn parse_table_children(&mut self) -> Result<Vec<TableChild>, ParseError> {
        let mut children = Vec::new();
        let mut seen_head = false;
        let mut seen_body = false;
        let mut direct_rows = Vec::new();

        while !self.at_shape(TokenShape::RParen) {
            let start = self.expect_kind(TokenShape::LParen)?.span.start;
            let name = self.expect_symbol()?;
            let kind = match name.value.as_str() {
                "thead" => {
                    if seen_head || seen_body {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidChild {
                                parent: "table".into(),
                                child: "thead".into(),
                            },
                            Some(name.span),
                        ));
                    }
                    seen_head = true;
                    self.reject_attribute_map_if_present("thead")?;
                    TableChildKind::Head(self.parse_table_rows("thead")?)
                }
                "tbody" => {
                    if !direct_rows.is_empty() {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidChild {
                                parent: "table".into(),
                                child: "tbody".into(),
                            },
                            Some(name.span),
                        ));
                    }
                    if seen_body {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidChild {
                                parent: "table".into(),
                                child: "tbody".into(),
                            },
                            Some(name.span),
                        ));
                    }
                    seen_body = true;
                    self.reject_attribute_map_if_present("tbody")?;
                    TableChildKind::Body(self.parse_table_rows("tbody")?)
                }
                "tr" => {
                    if seen_head || seen_body {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidChild {
                                parent: "table".into(),
                                child: "tr".into(),
                            },
                            Some(name.span),
                        ));
                    }
                    let row = self.parse_table_row(start, "table")?;
                    direct_rows.push(row);
                    continue;
                }
                other => {
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidChild {
                            parent: "table".into(),
                            child: other.into(),
                        },
                        Some(name.span),
                    ));
                }
            };
            let end = self.expect_kind(TokenShape::RParen)?.span.end;
            children.push(Node::new(kind, Span { start, end }));
        }

        if !direct_rows.is_empty() {
            return Ok(normalize_direct_table_rows(direct_rows));
        }

        Ok(children)
    }

    fn parse_table_rows(&mut self, parent: &str) -> Result<Vec<TableRow>, ParseError> {
        let mut rows = Vec::new();
        while !self.at_shape(TokenShape::RParen) {
            let start = self.expect_kind(TokenShape::LParen)?.span.start;
            let name = self.expect_symbol()?;
            if name.value != "tr" {
                return Err(ParseError::new(
                    ParseErrorKind::InvalidChild {
                        parent: parent.into(),
                        child: name.value,
                    },
                    Some(name.span),
                ));
            }
            rows.push(self.parse_table_row(start, parent)?);
        }
        if rows.is_empty() {
            return Err(ParseError::new(
                ParseErrorKind::MissingChild {
                    parent: parent.into(),
                    expected: "tr",
                },
                None,
            ));
        }
        Ok(rows)
    }

    fn parse_table_row(&mut self, start: crate::Position, parent: &str) -> Result<TableRow, ParseError> {
        self.reject_attribute_map_if_present("tr")?;
        let cells = self.parse_table_cells()?;
        if cells.is_empty() {
            return Err(ParseError::new(
                ParseErrorKind::MissingChild {
                    parent: "tr".into(),
                    expected: "th or td",
                },
                Some(Span {
                    start,
                    end: self.current_end(start),
                }),
            ));
        }
        let end = self.expect_kind(TokenShape::RParen)?.span.end;
        let _ = parent;
        Ok(Node::new(TableRowKind { cells }, Span { start, end }))
    }

    fn parse_table_cells(&mut self) -> Result<Vec<TableCell>, ParseError> {
        let mut cells = Vec::new();
        while !self.at_shape(TokenShape::RParen) {
            let start = self.expect_kind(TokenShape::LParen)?.span.start;
            let name = self.expect_symbol()?;
            let kind = match name.value.as_str() {
                "th" => {
                    self.reject_attribute_map_if_present("th")?;
                    TableCellKind::Header(self.parse_inlines_until_rparen("th", false)?)
                }
                "td" => {
                    self.reject_attribute_map_if_present("td")?;
                    TableCellKind::Data(self.parse_inlines_until_rparen("td", false)?)
                }
                other => {
                    return Err(ParseError::new(
                        ParseErrorKind::InvalidChild {
                            parent: "tr".into(),
                            child: other.into(),
                        },
                        Some(name.span),
                    ));
                }
            };
            let end = self.expect_kind(TokenShape::RParen)?.span.end;
            cells.push(Node::new(kind, Span { start, end }));
        }
        Ok(cells)
    }

    fn parse_required_attribute_map(
        &mut self,
        node: &str,
        allowed: &[&str],
    ) -> Result<AttributeMap, ParseError> {
        if !self.at_shape(TokenShape::LBrace) {
            return Err(ParseError::new(
                ParseErrorKind::MissingAttribute {
                    node: node.into(),
                    attribute: allowed[0].into(),
                },
                self.peek().map(|token| token.span),
            ));
        }
        self.parse_attribute_map(node, allowed)
    }

    fn parse_optional_attribute_map(
        &mut self,
        node: &str,
        allowed: &[&str],
    ) -> Result<AttributeMap, ParseError> {
        if self.at_shape(TokenShape::LBrace) {
            self.parse_attribute_map(node, allowed)
        } else {
            Ok(AttributeMap::new())
        }
    }

    fn parse_attribute_map(
        &mut self,
        node: &str,
        allowed: &[&str],
    ) -> Result<AttributeMap, ParseError> {
        self.expect_kind(TokenShape::LBrace)?;
        let mut map = AttributeMap::new();

        while !self.at_shape(TokenShape::RBrace) {
            let key = self.expect_keyword()?;
            if !allowed.contains(&key.value.as_str()) {
                return Err(ParseError::new(
                    ParseErrorKind::UnknownAttribute {
                        node: node.into(),
                        attribute: key.value,
                    },
                    Some(key.span),
                ));
            }
            if map.contains_key(&key.value) {
                return Err(ParseError::new(
                    ParseErrorKind::DuplicateAttribute {
                        attribute: key.value,
                    },
                    Some(key.span),
                ));
            }
            let value = self.parse_map_value(node, key.value.as_str())?;
            map.insert(key.value, value);
        }

        self.expect_kind(TokenShape::RBrace)?;
        Ok(map)
    }

    fn parse_map_value(&mut self, node: &str, attribute: &str) -> Result<MapEntry, ParseError> {
        let token = self.peek().ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::UnexpectedEof {
                    expected: "attribute value",
                },
                None,
            )
        })?;

        match &token.kind {
            TokenKind::String(value) => {
                let span = token.span;
                let value = value.clone();
                self.cursor += 1;
                Ok(MapEntry {
                    value: MapValue::String(value),
                    span,
                })
            }
            TokenKind::LBrace => {
                let start = self.expect_kind(TokenShape::LBrace)?.span.start;
                let mut map = AttributeMap::new();
                let allowed = match (node, attribute) {
                    ("doc", "meta") => &["version"][..],
                    _ => {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidAttributeType {
                                node: node.into(),
                                attribute: attribute.into(),
                                expected: "string",
                            },
                            Some(Span {
                                start,
                                end: self.current_end(start),
                            }),
                        ));
                    }
                };

                while !self.at_shape(TokenShape::RBrace) {
                    let key = self.expect_keyword()?;
                    if !allowed.contains(&key.value.as_str()) {
                        return Err(ParseError::new(
                            ParseErrorKind::UnknownAttribute {
                                node: attribute.into(),
                                attribute: key.value,
                            },
                            Some(key.span),
                        ));
                    }
                    if map.contains_key(&key.value) {
                        return Err(ParseError::new(
                            ParseErrorKind::DuplicateAttribute {
                                attribute: key.value,
                            },
                            Some(key.span),
                        ));
                    }
                    let value = self.parse_map_value(attribute, key.value.as_str())?;
                    map.insert(key.value, value);
                }
                let end = self.expect_kind(TokenShape::RBrace)?.span.end;
                Ok(MapEntry {
                    value: MapValue::Map(map),
                    span: Span { start, end },
                })
            }
            _ => Err(ParseError::new(
                ParseErrorKind::InvalidAttributeType {
                    node: node.into(),
                    attribute: attribute.into(),
                    expected: "string",
                },
                Some(token.span),
            )),
        }
    }

    fn reject_attribute_map_if_present(&self, node: &str) -> Result<(), ParseError> {
        if let Some(token) = self.peek()
            && matches!(token.kind, TokenKind::LBrace)
        {
            return Err(ParseError::new(
                ParseErrorKind::UnknownAttribute {
                    node: node.into(),
                    attribute: "*".into(),
                },
                Some(token.span),
            ));
        }
        Ok(())
    }

    fn take_string_attribute(
        &self,
        attributes: &mut AttributeMap,
        node: &str,
        attribute: &str,
    ) -> Result<StringEntry, ParseError> {
        let entry = attributes.remove(attribute).ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::MissingAttribute {
                    node: node.into(),
                    attribute: attribute.into(),
                },
                None,
            )
        })?;
        match entry.value {
            MapValue::String(value) => Ok(StringEntry { value }),
            MapValue::Map(_) => Err(ParseError::new(
                ParseErrorKind::InvalidAttributeType {
                    node: node.into(),
                    attribute: attribute.into(),
                    expected: "string",
                },
                Some(entry.span),
            )),
        }
    }

    fn take_optional_string_attribute(
        &self,
        attributes: &mut AttributeMap,
        node: &str,
        attribute: &str,
    ) -> Result<Option<String>, ParseError> {
        if attributes.contains_key(attribute) {
            self.take_string_attribute(attributes, node, attribute)
                .map(|entry| Some(entry.value))
        } else {
            Ok(None)
        }
    }

    fn reject_remaining_attributes(
        &self,
        node: &str,
        attributes: &AttributeMap,
    ) -> Result<(), ParseError> {
        if let Some((attribute, entry)) = attributes.iter().next() {
            return Err(ParseError::new(
                ParseErrorKind::UnknownAttribute {
                    node: node.into(),
                    attribute: attribute.clone(),
                },
                Some(entry.span),
            ));
        }
        Ok(())
    }

    fn expect_string_child(&mut self, parent: &str) -> Result<String, ParseError> {
        let token = self.peek().ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::MissingChild {
                    parent: parent.into(),
                    expected: "string",
                },
                None,
            )
        })?;
        match &token.kind {
            TokenKind::String(value) => {
                let value = value.clone();
                self.cursor += 1;
                Ok(value)
            }
            TokenKind::RParen => Err(ParseError::new(
                ParseErrorKind::MissingChild {
                    parent: parent.into(),
                    expected: "string",
                },
                Some(token.span),
            )),
            _ => {
                let child = self.describe_next_child()?;
                Err(ParseError::new(
                    ParseErrorKind::InvalidChild {
                        parent: parent.into(),
                        child,
                    },
                    Some(token.span),
                ))
            }
        }
    }

    fn describe_next_child(&self) -> Result<String, ParseError> {
        let Some(token) = self.peek() else {
            return Ok("end of input".into());
        };
        if matches!(token.kind, TokenKind::LParen)
            && let Some(next) = self.tokens.get(self.cursor + 1)
            && let TokenKind::Symbol(name) = &next.kind
        {
            return Ok(name.clone());
        }
        Ok(token_description(&token.kind).into())
    }

    fn expect_symbol(&mut self) -> Result<SpannedString, ParseError> {
        let token = self.next_token("symbol")?;
        match token.kind {
            TokenKind::Symbol(value) => Ok(SpannedString {
                value,
                span: token.span,
            }),
            kind => Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    expected: "symbol",
                    found: token_description(&kind),
                },
                Some(token.span),
            )),
        }
    }

    fn expect_keyword(&mut self) -> Result<SpannedString, ParseError> {
        let token = self.next_token("keyword")?;
        match token.kind {
            TokenKind::Keyword(value) => Ok(SpannedString {
                value,
                span: token.span,
            }),
            kind => Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    expected: "keyword",
                    found: token_description(&kind),
                },
                Some(token.span),
            )),
        }
    }

    fn expect_kind(&mut self, shape: TokenShape) -> Result<Token, ParseError> {
        let expected = shape.description();
        let token = self.next_token(expected)?;
        if shape.matches(&token.kind) {
            Ok(token)
        } else {
            Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    expected,
                    found: token_description(&token.kind),
                },
                Some(token.span),
            ))
        }
    }

    fn next_token(&mut self, expected: &'static str) -> Result<Token, ParseError> {
        let token = self
            .tokens
            .get(self.cursor)
            .cloned()
            .ok_or_else(|| ParseError::new(ParseErrorKind::UnexpectedEof { expected }, None))?;
        self.cursor += 1;
        Ok(token)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn at_shape(&self, shape: TokenShape) -> bool {
        self.peek().is_some_and(|token| shape.matches(&token.kind))
    }

    fn current_end(&self, fallback: crate::Position) -> crate::Position {
        self.peek().map_or(fallback, |token| token.span.start)
    }
}

type AttributeMap = BTreeMap<String, MapEntry>;

#[derive(Clone, Debug, Eq, PartialEq)]
struct MapEntry {
    value: MapValue,
    span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MapValue {
    String(String),
    Map(AttributeMap),
}

struct StringEntry {
    value: String,
}

struct SpannedString {
    value: String,
    span: Span,
}

#[derive(Clone, Copy)]
enum TokenShape {
    LParen,
    RParen,
    LBrace,
    RBrace,
}

impl TokenShape {
    fn matches(self, kind: &TokenKind) -> bool {
        matches!(
            (self, kind),
            (TokenShape::LParen, TokenKind::LParen)
                | (TokenShape::RParen, TokenKind::RParen)
                | (TokenShape::LBrace, TokenKind::LBrace)
                | (TokenShape::RBrace, TokenKind::RBrace)
        )
    }

    const fn description(self) -> &'static str {
        match self {
            TokenShape::LParen => "(",
            TokenShape::RParen => ")",
            TokenShape::LBrace => "{",
            TokenShape::RBrace => "}",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Option<Span>,
}

impl ParseError {
    fn new(kind: ParseErrorKind, span: Option<Span>) -> Self {
        Self { kind, span }
    }
}

impl From<LexError> for ParseError {
    fn from(error: LexError) -> Self {
        Self {
            kind: ParseErrorKind::Lex(error.kind),
            span: Some(error.span),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    Lex(LexErrorKind),
    UnexpectedEof {
        expected: &'static str,
    },
    UnexpectedToken {
        expected: &'static str,
        found: &'static str,
    },
    ExtraInput,
    UnknownNode {
        node: String,
    },
    InvalidChild {
        parent: String,
        child: String,
    },
    MissingChild {
        parent: String,
        expected: &'static str,
    },
    DuplicateAttribute {
        attribute: String,
    },
    UnknownAttribute {
        node: String,
        attribute: String,
    },
    MissingAttribute {
        node: String,
        attribute: String,
    },
    InvalidAttributeType {
        node: String,
        attribute: String,
        expected: &'static str,
    },
}

fn normalize_direct_table_rows(rows: Vec<TableRow>) -> Vec<TableChild> {
    let head_len = rows
        .iter()
        .take_while(|row| {
            row.kind
                .cells
                .iter()
                .all(|cell| matches!(cell.kind, TableCellKind::Header(_)))
        })
        .count();

    let mut children = Vec::new();
    if head_len > 0 {
        let span = Span {
            start: rows[0].span.start,
            end: rows[head_len - 1].span.end,
        };
        children.push(Node::new(
            TableChildKind::Head(rows[..head_len].to_vec()),
            span,
        ));
    }

    if head_len < rows.len() {
        let span = Span {
            start: rows[head_len].span.start,
            end: rows[rows.len() - 1].span.end,
        };
        children.push(Node::new(
            TableChildKind::Body(rows[head_len..].to_vec()),
            span,
        ));
    }

    children
}

fn is_block_node(name: &str) -> bool {
    matches!(
        name,
        "h1" | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "p"
            | "ul"
            | "ol"
            | "li"
            | "blockquote"
            | "codeblock"
            | "hr"
            | "table"
    )
}

fn is_inline_node(name: &str) -> bool {
    matches!(name, "em" | "strong" | "code" | "a" | "img" | "br")
}

fn block_name(kind: &BlockKind) -> &'static str {
    match kind {
        BlockKind::Heading { level, .. } => match level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            4 => "h4",
            5 => "h5",
            _ => "h6",
        },
        BlockKind::Paragraph(_) => "p",
        BlockKind::List { ordered, .. } => {
            if *ordered {
                "ol"
            } else {
                "ul"
            }
        }
        BlockKind::ListItem(_) => "li",
        BlockKind::BlockQuote(_) => "blockquote",
        BlockKind::CodeBlock { .. } => "codeblock",
        BlockKind::ThematicBreak => "hr",
        BlockKind::Table(_) => "table",
    }
}

fn token_description(kind: &TokenKind) -> &'static str {
    match kind {
        TokenKind::LParen => "(",
        TokenKind::RParen => ")",
        TokenKind::LBrace => "{",
        TokenKind::RBrace => "}",
        TokenKind::String(_) => "string",
        TokenKind::Symbol(_) => "symbol",
        TokenKind::Keyword(_) => "keyword",
    }
}
