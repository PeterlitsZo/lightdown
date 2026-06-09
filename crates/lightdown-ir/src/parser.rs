use std::collections::BTreeMap;

use crate::ast::{Expr, ExprKind, Module, ModuleMetadata, Node};
use crate::{LexError, LexErrorKind, Lexer, Span, Token, TokenKind};

pub fn parse(input: &str) -> Result<Module, ParseError> {
    Parser::new(Lexer::new(input))?.parse_module()
}

pub fn parse_expr_fragment(input: &str) -> Result<Expr, ParseError> {
    Parser::new(Lexer::new(input))?.parse_expr_fragment()
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

    pub fn parse_module(mut self) -> Result<Module, ParseError> {
        let module = self.parse_doc_module()?;
        if let Some(token) = self.peek() {
            return Err(ParseError::new(
                ParseErrorKind::ExtraInput,
                Some(token.span),
            ));
        }
        Ok(module)
    }

    pub fn parse_expr_fragment(mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr()?;
        if let Some(token) = self.peek() {
            return Err(ParseError::new(
                ParseErrorKind::ExtraInput,
                Some(token.span),
            ));
        }
        Ok(expr)
    }

    fn parse_doc_module(&mut self) -> Result<Module, ParseError> {
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
        let mut args = Vec::new();

        while !self.at_shape(TokenShape::RParen) {
            args.push(self.parse_expr()?);
        }

        let end = self.expect_kind(TokenShape::RParen)?.span.end;
        let span = Span { start, end };

        Ok(Module {
            metadata,
            body: Node::new(
                ExprKind::Call {
                    callee: Box::new(Node::new(ExprKind::Symbol("doc".into()), name.span)),
                    args,
                },
                span,
            ),
            span,
        })
    }

    fn parse_document_metadata(
        &mut self,
        mut attributes: AttributeMap,
    ) -> Result<ModuleMetadata, ParseError> {
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

        Ok(ModuleMetadata {
            version: version.value,
            span: entry.span,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek().ok_or_else(|| {
            ParseError::new(
                ParseErrorKind::UnexpectedEof {
                    expected: "expression",
                },
                None,
            )
        })?;

        match &token.kind {
            TokenKind::String(value) => {
                let span = token.span;
                let value = value.clone();
                self.cursor += 1;
                Ok(Node::new(ExprKind::String(value), span))
            }
            TokenKind::Symbol(value) => {
                let span = token.span;
                let value = value.clone();
                self.cursor += 1;
                let kind = match value.as_str() {
                    "true" => ExprKind::Bool(true),
                    "false" => ExprKind::Bool(false),
                    _ => ExprKind::Symbol(value),
                };
                Ok(Node::new(kind, span))
            }
            TokenKind::LParen => self.parse_call(),
            kind => Err(ParseError::new(
                ParseErrorKind::UnexpectedToken {
                    expected: "expression",
                    found: token_description(kind),
                },
                Some(token.span),
            )),
        }
    }

    fn parse_call(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_kind(TokenShape::LParen)?.span.start;
        let name = self.expect_symbol()?;
        let mut args = self.parse_prefixed_args(name.value.as_str())?;
        while !self.at_shape(TokenShape::RParen) {
            args.push(self.parse_expr()?);
        }
        let end = self.expect_kind(TokenShape::RParen)?.span.end;

        Ok(Node::new(
            ExprKind::Call {
                callee: Box::new(Node::new(ExprKind::Symbol(name.value), name.span)),
                args,
            },
            Span { start, end },
        ))
    }

    fn parse_prefixed_args(&mut self, node: &str) -> Result<Vec<Expr>, ParseError> {
        if !self.at_shape(TokenShape::LBrace) {
            return Ok(Vec::new());
        }

        match node {
            "a" => {
                let mut attributes = self.parse_attribute_map("a", &["href"])?;
                let href = self.take_string_attribute(&mut attributes, "a", "href")?;
                self.reject_remaining_attributes("a", &attributes)?;
                Ok(vec![Node::new(ExprKind::String(href.value), href.span)])
            }
            "img" => {
                let mut attributes = self.parse_attribute_map("img", &["src", "alt"])?;
                let src = self.take_string_attribute(&mut attributes, "img", "src")?;
                let alt = self.take_optional_string_attribute(&mut attributes, "img", "alt")?;
                self.reject_remaining_attributes("img", &attributes)?;

                let mut args = vec![Node::new(ExprKind::String(src.value), src.span)];
                if let Some(alt) = alt {
                    args.push(Node::new(ExprKind::String(alt), src.span));
                }
                Ok(args)
            }
            "codeblock" => {
                let mut attributes = self.parse_attribute_map("codeblock", &["lang"])?;
                let lang =
                    self.take_optional_string_attribute(&mut attributes, "codeblock", "lang")?;
                self.reject_remaining_attributes("codeblock", &attributes)?;

                let mut args = Vec::new();
                if let Some(lang) = lang {
                    args.push(Node::new(ExprKind::String(lang), self.previous_span()));
                }
                Ok(args)
            }
            _ => Err(ParseError::new(
                ParseErrorKind::UnknownAttribute {
                    node: node.into(),
                    attribute: "*".into(),
                },
                self.peek().map(|token| token.span),
            )),
        }
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
            MapValue::String(value) => Ok(StringEntry {
                value,
                span: entry.span,
            }),
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

    fn previous_span(&self) -> Span {
        self.tokens[self.cursor - 1].span
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
    span: Span,
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
