/// A concrete position in the original input stream.
///
/// Offsets are zero-based byte indices, while line and column numbers are
/// one-based for human-facing diagnostics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Position {
    /// The zero-based byte offset from the beginning of the input.
    pub offset: usize,
    /// The one-based line number in the original input.
    pub line: usize,
    /// The one-based column number in the original input.
    pub column: usize,
}

impl Position {
    const fn start() -> Self {
        Self {
            offset: 0,
            line: 1,
            column: 1,
        }
    }
}

/// A half-open source range within the original input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    /// The inclusive start position of the range.
    pub start: Position,
    /// The exclusive end position of the range.
    pub end: Position,
}

impl Span {
    const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    LParen,
    RParen,
    LBrace,
    RBrace,
    String(String),
    Symbol(String),
    Keyword(String),
}

impl TokenKind {
    pub fn l_paren() -> Self {
        TokenKind::LParen
    }

    pub fn r_paren() -> Self {
        TokenKind::RParen
    }

    pub fn l_brace() -> Self {
        TokenKind::LBrace
    }

    pub fn r_brace() -> Self {
        TokenKind::RBrace
    }

    pub fn string<T: Into<String>>(s: T) -> Self {
        TokenKind::String(s.into())
    }

    pub fn symbol<T: Into<String>>(s: T) -> Self {
        TokenKind::Symbol(s.into())
    }

    pub fn keyword<T: Into<String>>(s: T) -> Self {
        TokenKind::Keyword(s.into())
    }
}

/// A single lexical token produced from Lightdown IR input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    /// The classified token payload.
    pub kind: TokenKind,
    /// The source range covered by this token.
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LexErrorKind {
    UnexpectedCharacter(char),
    UnexpectedColon,
    InvalidIdentifier(String),
    UnterminatedString,
    UnterminatedMultilineString,
    InvalidEscapeSequence(String),
}

/// A lexical error together with the source range where it occurred.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexError {
    /// The concrete failure kind.
    pub kind: LexErrorKind,
    /// The source range associated with the failure.
    pub span: Span,
}

/// An iterator that tokenizes Lightdown IR source text.
///
/// The lexer skips insignificant whitespace, yields one token at a time, and
/// stops permanently after the first lexical error.
pub struct Lexer<'a> {
    input: &'a str,
    position: Position,
    finished: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: Position::start(),
            finished: false,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.bump_char();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>, LexError> {
        self.skip_whitespace();

        let start = self.position;
        let Some(ch) = self.peek_char() else {
            return Ok(None);
        };

        match ch {
            '(' => {
                self.bump_char();
                Ok(Some(Token {
                    kind: TokenKind::LParen,
                    span: Span::new(start, self.position),
                }))
            }
            ')' => {
                self.bump_char();
                Ok(Some(Token {
                    kind: TokenKind::RParen,
                    span: Span::new(start, self.position),
                }))
            }
            '{' => {
                self.bump_char();
                Ok(Some(Token {
                    kind: TokenKind::LBrace,
                    span: Span::new(start, self.position),
                }))
            }
            '}' => {
                self.bump_char();
                Ok(Some(Token {
                    kind: TokenKind::RBrace,
                    span: Span::new(start, self.position),
                }))
            }
            ':' => self.lex_keyword().map(Some),
            '"' if self.remaining().starts_with("\"\"\"") => self.lex_multiline_string().map(Some),
            '"' => self.lex_string().map(Some),
            ch if is_identifier_start(ch) => self.lex_symbol().map(Some),
            other => {
                self.bump_char();
                Err(LexError {
                    kind: LexErrorKind::UnexpectedCharacter(other),
                    span: Span::new(start, self.position),
                })
            }
        }
    }

    fn lex_symbol(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        let ident = self.consume_identifier()?;
        Ok(Token {
            kind: TokenKind::Symbol(ident),
            span: Span::new(start, self.position),
        })
    }

    fn lex_keyword(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        self.bump_char();

        let Some(next) = self.peek_char() else {
            return Err(LexError {
                kind: LexErrorKind::UnexpectedColon,
                span: Span::new(start, self.position),
            });
        };

        if !is_identifier_start(next) {
            return Err(LexError {
                kind: LexErrorKind::UnexpectedColon,
                span: Span::new(start, self.position),
            });
        }

        let ident = self.consume_identifier()?;
        Ok(Token {
            kind: TokenKind::Keyword(ident),
            span: Span::new(start, self.position),
        })
    }

    fn lex_string(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        self.bump_char();

        let mut value = String::new();

        loop {
            let Some(ch) = self.peek_char() else {
                return Err(LexError {
                    kind: LexErrorKind::UnterminatedString,
                    span: Span::new(start, self.position),
                });
            };

            match ch {
                '"' => {
                    self.bump_char();
                    return Ok(Token {
                        kind: TokenKind::String(value),
                        span: Span::new(start, self.position),
                    });
                }
                '\\' => {
                    let escape_start = self.position;
                    value.push(self.parse_escape_sequence(escape_start)?);
                }
                '\n' | '\r' => {
                    return Err(LexError {
                        kind: LexErrorKind::UnterminatedString,
                        span: Span::new(start, self.position),
                    });
                }
                ch if ch.is_control() => {
                    return Err(LexError {
                        kind: LexErrorKind::UnexpectedCharacter(ch),
                        span: Span::new(self.position, self.snapshot_after(ch)),
                    });
                }
                _ => {
                    self.bump_char();
                    value.push(ch);
                }
            }
        }
    }

    fn lex_multiline_string(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        self.consume_triple_quote();

        let mut raw = String::new();

        loop {
            if self.remaining().starts_with("\"\"\"") {
                self.consume_triple_quote();
                let normalized = normalize_multiline_string(&raw);
                return Ok(Token {
                    kind: TokenKind::String(normalized),
                    span: Span::new(start, self.position),
                });
            }

            let Some(ch) = self.peek_char() else {
                return Err(LexError {
                    kind: LexErrorKind::UnterminatedMultilineString,
                    span: Span::new(start, self.position),
                });
            };

            if ch == '\\' {
                let escape_start = self.position;
                raw.push(self.parse_escape_sequence(escape_start)?);
                continue;
            }

            self.bump_char();
            raw.push(ch);
        }
    }

    fn consume_identifier(&mut self) -> Result<String, LexError> {
        let start = self.position;
        let Some(first) = self.peek_char() else {
            return Err(LexError {
                kind: LexErrorKind::InvalidIdentifier(String::new()),
                span: Span::new(start, self.position),
            });
        };

        if !is_identifier_start(first) {
            return Err(LexError {
                kind: LexErrorKind::InvalidIdentifier(first.to_string()),
                span: Span::new(start, self.position),
            });
        }

        let mut ident = String::new();
        self.bump_char();
        ident.push(first);

        while let Some(ch) = self.peek_char() {
            if is_identifier_continue(ch) {
                self.bump_char();
                ident.push(ch);
            } else {
                break;
            }
        }

        Ok(ident)
    }

    fn parse_escape_sequence(&mut self, escape_start: Position) -> Result<char, LexError> {
        self.bump_char();

        let Some(ch) = self.peek_char() else {
            return Err(LexError {
                kind: LexErrorKind::InvalidEscapeSequence("\\".into()),
                span: Span::new(escape_start, self.position),
            });
        };

        let parsed = match ch {
            '"' => {
                self.bump_char();
                '"'
            }
            '\\' => {
                self.bump_char();
                '\\'
            }
            '/' => {
                self.bump_char();
                '/'
            }
            'b' => {
                self.bump_char();
                '\u{0008}'
            }
            'f' => {
                self.bump_char();
                '\u{000C}'
            }
            'n' => {
                self.bump_char();
                '\n'
            }
            'r' => {
                self.bump_char();
                '\r'
            }
            't' => {
                self.bump_char();
                '\t'
            }
            'u' => {
                self.bump_char();
                return self.parse_unicode_escape(escape_start);
            }
            other => {
                return Err(LexError {
                    kind: LexErrorKind::InvalidEscapeSequence(format!("\\{other}")),
                    span: Span::new(escape_start, self.position),
                });
            }
        };

        Ok(parsed)
    }

    fn parse_unicode_escape(&mut self, escape_start: Position) -> Result<char, LexError> {
        let first = self.consume_hex_code_unit(escape_start)?;

        if is_high_surrogate(first) {
            if self.peek_char() != Some('\\') {
                return Err(LexError {
                    kind: LexErrorKind::InvalidEscapeSequence(format!("\\u{first:04X}")),
                    span: Span::new(escape_start, self.position),
                });
            }

            let after_slash = self.snapshot_after('\\');
            self.bump_char();
            if self.peek_char() != Some('u') {
                return Err(LexError {
                    kind: LexErrorKind::InvalidEscapeSequence(format!("\\u{first:04X}")),
                    span: Span::new(escape_start, after_slash),
                });
            }

            self.bump_char();
            let second = self.consume_hex_code_unit(escape_start)?;
            if !is_low_surrogate(second) {
                return Err(LexError {
                    kind: LexErrorKind::InvalidEscapeSequence(format!(
                        "\\u{first:04X}\\u{second:04X}"
                    )),
                    span: Span::new(escape_start, self.position),
                });
            }

            let code_point =
                0x10000 + ((((first as u32) - 0xD800) << 10) | ((second as u32) - 0xDC00));
            return char::from_u32(code_point).ok_or_else(|| LexError {
                kind: LexErrorKind::InvalidEscapeSequence(format!("\\u{first:04X}\\u{second:04X}")),
                span: Span::new(escape_start, self.position),
            });
        }

        if is_low_surrogate(first) {
            return Err(LexError {
                kind: LexErrorKind::InvalidEscapeSequence(format!("\\u{first:04X}")),
                span: Span::new(escape_start, self.position),
            });
        }

        char::from_u32(first as u32).ok_or_else(|| LexError {
            kind: LexErrorKind::InvalidEscapeSequence(format!("\\u{first:04X}")),
            span: Span::new(escape_start, self.position),
        })
    }

    fn consume_hex_code_unit(&mut self, escape_start: Position) -> Result<u16, LexError> {
        let mut value = 0u16;
        let mut digits = String::new();

        for _ in 0..4 {
            let Some(ch) = self.peek_char() else {
                return Err(LexError {
                    kind: LexErrorKind::InvalidEscapeSequence(format!("\\u{digits}")),
                    span: Span::new(escape_start, self.position),
                });
            };

            let Some(digit) = ch.to_digit(16) else {
                return Err(LexError {
                    kind: LexErrorKind::InvalidEscapeSequence(format!("\\u{digits}{ch}")),
                    span: Span::new(escape_start, self.position),
                });
            };

            self.bump_char();
            digits.push(ch);
            value = (value << 4) | (digit as u16);
        }

        Ok(value)
    }

    fn consume_triple_quote(&mut self) {
        self.bump_char();
        self.bump_char();
        self.bump_char();
    }

    fn snapshot_after(&self, ch: char) -> Position {
        let mut position = self.position;
        position.offset += ch.len_utf8();
        if ch == '\n' {
            position.line += 1;
            position.column = 1;
        } else {
            position.column += 1;
        }
        position
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.position.offset..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.position.offset += ch.len_utf8();
        if ch == '\n' {
            self.position.line += 1;
            self.position.column = 1;
        } else {
            self.position.column += 1;
        }
        Some(ch)
    }
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.next_token() {
            Ok(Some(token)) => Some(Ok(token)),
            Ok(None) => {
                self.finished = true;
                None
            }
            Err(error) => {
                self.finished = true;
                Some(Err(error))
            }
        }
    }
}

fn normalize_multiline_string(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(strip_margin_prefix)
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_margin_prefix(line: &str) -> &str {
    let Some(prefix_len) = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some((idx, ch)))
    else {
        return line;
    };

    let (idx, ch) = prefix_len;
    if ch != '|' {
        return line;
    }

    let rest = &line[idx + ch.len_utf8()..];
    if let Some(stripped) = rest.strip_prefix(' ') {
        stripped
    } else {
        rest
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == ':'
}

fn is_high_surrogate(value: u16) -> bool {
    (0xD800..=0xDBFF).contains(&value)
}

fn is_low_surrogate(value: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::{Lexer, Token, TokenKind};

    fn check_tokens_by_kinds(tokens: &[Token], kinds: &[TokenKind]) {
        assert_eq!(tokens.len(), kinds.len());
        for (token, kind) in tokens.iter().zip(kinds.iter()) {
            assert_eq!(token.kind, *kind);
        }
    }

    #[test]
    fn a_simple_document() {
        let input = indoc::indoc! { r#"
            (doc
              (meta :version "0.1.0")
              (codeblock {:lang "js"} """
                | import foobar from "./foobar";
                |
                | foobar()
                """))
        "# };

        let tokens = Lexer::new(input)
            .collect::<Result<Vec<_>, _>>()
            .expect("lexer should succeed");

        check_tokens_by_kinds(
            &tokens,
            &[
                TokenKind::l_paren(),
                TokenKind::symbol("doc"),
                TokenKind::l_paren(),
                TokenKind::symbol("meta"),
                TokenKind::keyword("version"),
                TokenKind::string("0.1.0"),
                TokenKind::r_paren(),
                TokenKind::l_paren(),
                TokenKind::symbol("codeblock"),
                TokenKind::l_brace(),
                TokenKind::keyword("lang"),
                TokenKind::string("js"),
                TokenKind::r_brace(),
                TokenKind::string("import foobar from \"./foobar\";\n\nfoobar()"),
                TokenKind::r_paren(),
                TokenKind::r_paren(),
            ],
        );
    }
}
