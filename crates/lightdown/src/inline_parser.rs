use lightdown_ir::{Position, Span};

use crate::error::{ParseError, ParseErrorKind};
use crate::syntax::{SourceInline, SourceInlineKind};

pub fn parse_inlines(input: &str, base_span: Span) -> Result<Vec<SourceInline>, ParseError> {
    InlineParser::new(input, base_span).parse_sequence()
}

pub fn parse_fragment(input: &str, base_span: Span) -> Result<Vec<SourceInline>, ParseError> {
    parse_inlines(input, base_span)
}

struct InlineParser<'a> {
    input: &'a str,
    base_span: Span,
    cursor: usize,
}

impl<'a> InlineParser<'a> {
    fn new(input: &'a str, base_span: Span) -> Self {
        Self {
            input,
            base_span,
            cursor: 0,
        }
    }

    fn parse_sequence(&mut self) -> Result<Vec<SourceInline>, ParseError> {
        let mut items = Vec::new();

        while !self.is_eof() {
            if self.at_str("**") {
                items.push(self.parse_strong()?);
            } else if self.at_char('*') {
                items.push(self.parse_emphasis()?);
            } else if self.at_char('`') {
                items.push(self.parse_code()?);
            } else if self.at_str("![") {
                items.push(self.parse_image()?);
            } else if self.at_char('[') {
                items.push(self.parse_link()?);
            } else if self.at_str(r"\(") {
                items.push(self.parse_embedded_ir()?);
            } else {
                items.push(self.parse_text_run());
            }
        }

        Ok(items)
    }

    fn parse_strong(&mut self) -> Result<SourceInline, ParseError> {
        let start = self.cursor;
        self.cursor += 2;
        let Some(close) = self.input[self.cursor..].find("**") else {
            return Err(self.error(ParseErrorKind::UnterminatedStrong, start, self.input.len()));
        };
        let inner_start = self.cursor;
        let inner_end = self.cursor + close;
        let children = parse_fragment(
            &self.input[inner_start..inner_end],
            self.span_for_range(inner_start, inner_end),
        )?;
        self.cursor = inner_end + 2;

        Ok(SourceInline {
            kind: SourceInlineKind::Strong(children),
            span: self.span_for_range(start, self.cursor),
        })
    }

    fn parse_emphasis(&mut self) -> Result<SourceInline, ParseError> {
        let start = self.cursor;
        self.cursor += 1;
        let Some(close) = self.input[self.cursor..].find('*') else {
            return Err(self.error(
                ParseErrorKind::UnterminatedEmphasis,
                start,
                self.input.len(),
            ));
        };
        let inner_start = self.cursor;
        let inner_end = self.cursor + close;
        let children = parse_fragment(
            &self.input[inner_start..inner_end],
            self.span_for_range(inner_start, inner_end),
        )?;
        self.cursor = inner_end + 1;

        Ok(SourceInline {
            kind: SourceInlineKind::Emphasis(children),
            span: self.span_for_range(start, self.cursor),
        })
    }

    fn parse_code(&mut self) -> Result<SourceInline, ParseError> {
        let start = self.cursor;
        self.cursor += 1;
        let Some(close) = self.input[self.cursor..].find('`') else {
            return Err(self.error(
                ParseErrorKind::UnterminatedInlineCode,
                start,
                self.input.len(),
            ));
        };
        let text_end = self.cursor + close;
        let text = self.input[self.cursor..text_end].to_string();
        self.cursor = text_end + 1;

        Ok(SourceInline {
            kind: SourceInlineKind::Code(text),
            span: self.span_for_range(start, self.cursor),
        })
    }

    fn parse_link(&mut self) -> Result<SourceInline, ParseError> {
        let start = self.cursor;
        self.cursor += 1;
        let Some(label_end) = self.input[self.cursor..].find(']') else {
            return Err(self.error(ParseErrorKind::UnterminatedLink, start, self.input.len()));
        };
        let label_start = self.cursor;
        let label_end = self.cursor + label_end;
        self.cursor = label_end + 1;
        if !self.at_char('(') {
            return Err(self.error(ParseErrorKind::UnterminatedLink, start, self.cursor));
        }
        self.cursor += 1;
        let Some(href_end) = self.input[self.cursor..].find(')') else {
            return Err(self.error(ParseErrorKind::UnterminatedLink, start, self.input.len()));
        };
        let href_start = self.cursor;
        let href_end = self.cursor + href_end;
        let href = self.input[href_start..href_end].to_string();
        self.cursor = href_end + 1;

        Ok(SourceInline {
            kind: SourceInlineKind::Link {
                href,
                children: parse_fragment(
                    &self.input[label_start..label_end],
                    self.span_for_range(label_start, label_end),
                )?,
            },
            span: self.span_for_range(start, self.cursor),
        })
    }

    fn parse_image(&mut self) -> Result<SourceInline, ParseError> {
        let start = self.cursor;
        self.cursor += 2;
        let Some(alt_end) = self.input[self.cursor..].find(']') else {
            return Err(self.error(ParseErrorKind::UnterminatedImage, start, self.input.len()));
        };
        let alt_start = self.cursor;
        let alt_end = self.cursor + alt_end;
        self.cursor = alt_end + 1;
        if !self.at_char('(') {
            return Err(self.error(ParseErrorKind::UnterminatedImage, start, self.cursor));
        }
        self.cursor += 1;
        let Some(src_end) = self.input[self.cursor..].find(')') else {
            return Err(self.error(ParseErrorKind::UnterminatedImage, start, self.input.len()));
        };
        let src_start = self.cursor;
        let src_end = self.cursor + src_end;
        let src = self.input[src_start..src_end].to_string();
        let alt = self.input[alt_start..alt_end].to_string();
        self.cursor = src_end + 1;

        Ok(SourceInline {
            kind: SourceInlineKind::Image { src, alt },
            span: self.span_for_range(start, self.cursor),
        })
    }

    fn parse_embedded_ir(&mut self) -> Result<SourceInline, ParseError> {
        let start = self.cursor;
        self.cursor += 2;
        let source_start = self.cursor;
        let mut paren_depth = 1usize;
        let mut bracket_depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;

        while self.cursor < self.input.len() {
            let ch = self.current_char().expect("cursor points at a character");

            if in_string {
                self.cursor += ch.len_utf8();
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }

            match ch {
                '"' => {
                    in_string = true;
                    self.cursor += ch.len_utf8();
                }
                '[' => {
                    bracket_depth += 1;
                    self.cursor += ch.len_utf8();
                }
                ']' if bracket_depth > 0 => {
                    bracket_depth -= 1;
                    self.cursor += ch.len_utf8();
                }
                '(' if bracket_depth == 0 => {
                    paren_depth += 1;
                    self.cursor += ch.len_utf8();
                }
                ')' if bracket_depth == 0 => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        let source_end = self.cursor;
                        self.cursor += ch.len_utf8();
                        return Ok(SourceInline {
                            kind: SourceInlineKind::EmbeddedIr(
                                self.input[source_start..source_end].to_string(),
                            ),
                            span: self.span_for_range(start, self.cursor),
                        });
                    }
                    self.cursor += ch.len_utf8();
                }
                _ => self.cursor += ch.len_utf8(),
            }
        }

        Err(self.error(
            ParseErrorKind::UnterminatedEmbeddedIr,
            start,
            self.input.len(),
        ))
    }

    fn parse_text_run(&mut self) -> SourceInline {
        let start = self.cursor;
        while !self.is_eof()
            && !self.at_str("**")
            && !self.at_char('*')
            && !self.at_char('`')
            && !self.at_str("![")
            && !self.at_char('[')
            && !self.at_str(r"\(")
        {
            let ch = self.current_char().expect("cursor points at a character");
            self.cursor += ch.len_utf8();
        }

        SourceInline {
            kind: SourceInlineKind::Text(self.input[start..self.cursor].to_string()),
            span: self.span_for_range(start, self.cursor),
        }
    }

    fn error(&self, kind: ParseErrorKind, start: usize, end: usize) -> ParseError {
        ParseError::new(kind, Some(self.span_for_range(start, end)))
    }

    fn span_for_range(&self, start: usize, end: usize) -> Span {
        Span {
            start: advance_position(self.base_span.start, &self.input[..start]),
            end: advance_position(self.base_span.start, &self.input[..end]),
        }
    }

    fn current_char(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn at_char(&self, ch: char) -> bool {
        self.current_char() == Some(ch)
    }

    fn at_str(&self, pattern: &str) -> bool {
        self.input[self.cursor..].starts_with(pattern)
    }

    fn is_eof(&self) -> bool {
        self.cursor >= self.input.len()
    }
}

fn advance_position(start: Position, segment: &str) -> Position {
    let mut position = start;

    for ch in segment.chars() {
        position.offset += ch.len_utf8();
        if ch == '\n' {
            position.line += 1;
            position.column = 1;
        } else {
            position.column += 1;
        }
    }

    position
}
