use lightdown_ir::{Position, Span};

use crate::error::{ParseError, ParseErrorKind};
use crate::inline_parser;
use crate::syntax::{SourceBlock, SourceBlockKind, SourceDocument};

pub fn parse_document(input: &str) -> Result<SourceDocument, ParseError> {
    let mut parser = BlockParser::new(input);
    parser.parse_document()
}

struct BlockParser<'a> {
    input: &'a str,
    lines: Vec<LineInfo<'a>>,
    cursor: usize,
}

impl<'a> BlockParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            lines: collect_lines(input),
            cursor: 0,
        }
    }

    fn parse_document(&mut self) -> Result<SourceDocument, ParseError> {
        let mut blocks = Vec::new();

        while self.cursor < self.lines.len() {
            if self.lines[self.cursor].text.trim().is_empty() {
                self.cursor += 1;
                continue;
            }

            if let Some(block) = self.parse_heading()? {
                blocks.push(block);
            } else if self.at_block_embedded_ir() {
                blocks.push(self.parse_block_embedded_ir()?);
            } else if self.at_unordered_list_item() || self.at_ordered_list_item() {
                blocks.push(self.parse_list()?);
            } else if self.at_block_quote() {
                blocks.push(self.parse_block_quote()?);
            } else if self.at_code_fence() {
                blocks.push(self.parse_code_fence()?);
            } else if self.at_thematic_break() {
                blocks.push(self.parse_thematic_break());
            } else {
                blocks.push(self.parse_paragraph()?);
            }
        }

        Ok(SourceDocument { blocks })
    }

    fn parse_heading(&mut self) -> Result<Option<SourceBlock>, ParseError> {
        let line = self.lines[self.cursor];
        let hashes = line.text.chars().take_while(|&ch| ch == '#').count();

        if !(1..=6).contains(&hashes) || !line.text[hashes..].starts_with(' ') {
            return Ok(None);
        }

        let span = span_for_range(self.input, line.start, line.end);
        let content_start = line.start + hashes + 1;
        let inline_span = span_for_range(self.input, content_start, line.end);
        let inlines = inline_parser::parse_inlines(&line.text[hashes + 1..], inline_span)?;
        self.cursor += 1;

        Ok(Some(SourceBlock {
            kind: SourceBlockKind::Heading {
                level: hashes as u8,
                inlines,
            },
            span,
        }))
    }

    fn parse_paragraph(&mut self) -> Result<SourceBlock, ParseError> {
        let start = self.lines[self.cursor].start;
        let mut end = self.lines[self.cursor].end;
        let mut parts = Vec::new();

        while self.cursor < self.lines.len() {
            let line = self.lines[self.cursor];
            if line.text.trim().is_empty() {
                break;
            }
            if !parts.is_empty() && self.is_block_start(line.text) {
                break;
            }

            parts.push(line.text.trim().to_string());
            end = line.end;
            self.cursor += 1;
        }

        let span = span_for_range(self.input, start, end);
        let inlines = inline_parser::parse_inlines(&parts.join(" "), span)?;
        Ok(SourceBlock {
            kind: SourceBlockKind::Paragraph(inlines),
            span,
        })
    }

    fn parse_block_embedded_ir(&mut self) -> Result<SourceBlock, ParseError> {
        let start = self.lines[self.cursor].start;
        let (source_end, end) = scan_embedded_ir_fragment(self.input, start)?;
        let span = span_for_range(self.input, start, end);
        let source = self.input[start + 2..source_end].to_string();

        while self.cursor < self.lines.len() && self.lines[self.cursor].end < end {
            self.cursor += 1;
        }
        if self.cursor < self.lines.len() {
            self.cursor += 1;
        }

        Ok(SourceBlock {
            kind: SourceBlockKind::EmbeddedIr(source),
            span,
        })
    }

    fn parse_list(&mut self) -> Result<SourceBlock, ParseError> {
        let ordered = self.at_ordered_list_item();
        let start = self.lines[self.cursor].start;
        let mut end = self.lines[self.cursor].end;
        let mut items = Vec::new();

        while self.cursor < self.lines.len() {
            let line = self.lines[self.cursor];
            if line.text.trim().is_empty() {
                break;
            }

            let content = if ordered {
                let Some(content) = ordered_list_item_content(line.text) else {
                    break;
                };
                content
            } else {
                let Some(content) = unordered_list_item_content(line.text) else {
                    break;
                };
                content
            };

            let item_span = span_for_range(self.input, line.start, line.end);
            let content_start = line.end.saturating_sub(content.len());
            let paragraph_span = span_for_range(self.input, content_start, line.end);
            let paragraph = SourceBlock {
                kind: SourceBlockKind::Paragraph(inline_parser::parse_inlines(
                    content,
                    paragraph_span,
                )?),
                span: paragraph_span,
            };

            items.push(SourceBlock {
                kind: SourceBlockKind::ListItem(vec![paragraph]),
                span: item_span,
            });
            end = line.end;
            self.cursor += 1;
        }

        Ok(SourceBlock {
            kind: SourceBlockKind::List { ordered, items },
            span: span_for_range(self.input, start, end),
        })
    }

    fn parse_block_quote(&mut self) -> Result<SourceBlock, ParseError> {
        let start = self.lines[self.cursor].start;
        let mut end = self.lines[self.cursor].end;
        let mut parts = Vec::new();

        while self.cursor < self.lines.len() {
            let line = self.lines[self.cursor];
            let Some(content) = block_quote_content(line.text) else {
                break;
            };
            if line.text.trim().is_empty() {
                break;
            }

            parts.push(content.trim().to_string());
            end = line.end;
            self.cursor += 1;
        }

        let span = span_for_range(self.input, start, end);
        let paragraph = SourceBlock {
            kind: SourceBlockKind::Paragraph(inline_parser::parse_inlines(
                &parts.join(" "),
                span,
            )?),
            span,
        };

        Ok(SourceBlock {
            kind: SourceBlockKind::BlockQuote(vec![paragraph]),
            span,
        })
    }

    fn parse_code_fence(&mut self) -> Result<SourceBlock, ParseError> {
        let opening = self.lines[self.cursor];
        let start = opening.start;
        let lang = fence_language(opening.text);
        self.cursor += 1;

        let mut body = Vec::new();
        let mut end = opening.end;

        while self.cursor < self.lines.len() {
            let line = self.lines[self.cursor];
            if line.text.trim() == "```" {
                end = line.end;
                self.cursor += 1;
                return Ok(SourceBlock {
                    kind: SourceBlockKind::CodeBlock {
                        lang,
                        text: body.join("\n"),
                    },
                    span: span_for_range(self.input, start, end),
                });
            }

            body.push(line.text.to_string());
            end = line.end;
            self.cursor += 1;
        }

        Err(ParseError::new(
            ParseErrorKind::UnterminatedCodeFence,
            Some(span_for_range(self.input, start, end)),
        ))
    }

    fn parse_thematic_break(&mut self) -> SourceBlock {
        let line = self.lines[self.cursor];
        self.cursor += 1;

        SourceBlock {
            kind: SourceBlockKind::ThematicBreak,
            span: span_for_range(self.input, line.start, line.end),
        }
    }

    fn is_block_start(&self, text: &str) -> bool {
        self.is_heading(text)
            || text.starts_with(r"\(")
            || unordered_list_item_content(text).is_some()
            || ordered_list_item_content(text).is_some()
            || block_quote_content(text).is_some()
            || text.trim() == "---"
            || text.starts_with("```")
    }

    fn is_heading(&self, text: &str) -> bool {
        let hashes = text.chars().take_while(|&ch| ch == '#').count();
        (1..=6).contains(&hashes) && text[hashes..].starts_with(' ')
    }

    fn at_unordered_list_item(&self) -> bool {
        unordered_list_item_content(self.lines[self.cursor].text).is_some()
    }

    fn at_ordered_list_item(&self) -> bool {
        ordered_list_item_content(self.lines[self.cursor].text).is_some()
    }

    fn at_block_quote(&self) -> bool {
        block_quote_content(self.lines[self.cursor].text).is_some()
    }

    fn at_code_fence(&self) -> bool {
        self.lines[self.cursor].text.starts_with("```")
    }

    fn at_thematic_break(&self) -> bool {
        self.lines[self.cursor].text.trim() == "---"
    }

    fn at_block_embedded_ir(&self) -> bool {
        self.lines[self.cursor].text.starts_with(r"\(")
    }
}

#[derive(Clone, Copy)]
struct LineInfo<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

fn collect_lines(input: &str) -> Vec<LineInfo<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;

    for (offset, ch) in input.char_indices() {
        if ch == '\n' {
            lines.push(LineInfo {
                text: &input[start..offset],
                start,
                end: offset,
            });
            start = offset + ch.len_utf8();
        }
    }

    if start < input.len() {
        lines.push(LineInfo {
            text: &input[start..],
            start,
            end: input.len(),
        });
    }

    lines
}

fn span_for_range(input: &str, start: usize, end: usize) -> Span {
    Span {
        start: position_for_offset(input, start),
        end: position_for_offset(input, end),
    }
}

fn position_for_offset(input: &str, offset: usize) -> Position {
    let bounded = offset.min(input.len());
    let mut position = Position {
        offset: 0,
        line: 1,
        column: 1,
    };

    for ch in input[..bounded].chars() {
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

fn unordered_list_item_content(text: &str) -> Option<&str> {
    text.strip_prefix("- ")
}

fn ordered_list_item_content(text: &str) -> Option<&str> {
    let digits = text.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }

    text.get(digits..)
        .filter(|rest| rest.starts_with(". "))
        .map(|rest| &rest[2..])
}

fn block_quote_content(text: &str) -> Option<&str> {
    text.strip_prefix("> ").or_else(|| text.strip_prefix('>'))
}

fn fence_language(text: &str) -> Option<String> {
    let rest = text.strip_prefix("```").unwrap_or_default().trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn scan_embedded_ir_fragment(input: &str, start: usize) -> Result<(usize, usize), ParseError> {
    let mut cursor = start + 2;
    let mut paren_depth = 1usize;
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    while cursor < input.len() {
        let ch = input[cursor..]
            .chars()
            .next()
            .expect("cursor points at a character");

        if in_string {
            cursor += ch.len_utf8();
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
                cursor += ch.len_utf8();
            }
            '[' => {
                bracket_depth += 1;
                cursor += ch.len_utf8();
            }
            ']' if bracket_depth > 0 => {
                bracket_depth -= 1;
                cursor += ch.len_utf8();
            }
            '(' if bracket_depth == 0 => {
                paren_depth += 1;
                cursor += ch.len_utf8();
            }
            ')' if bracket_depth == 0 => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    return Ok((cursor, cursor + ch.len_utf8()));
                }
                cursor += ch.len_utf8();
            }
            _ => cursor += ch.len_utf8(),
        }
    }

    Err(ParseError::new(
        ParseErrorKind::UnterminatedEmbeddedIr,
        Some(span_for_range(input, start, input.len())),
    ))
}
