use crate::{Error, LimitKind, MAX_DEPTH, MAX_NODES, MAX_SOURCE_BYTES, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TokenKind {
    Ident(String),
    Num(f64),
    String(String),
    Bool(bool),
    Nil,
    If,
    Else,
    Exists,
    Not,
    And,
    Or,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Otherwise,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Power,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Eof,
}

pub(crate) fn lex(source: &str) -> Result<Vec<Token>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(Error::limit(LimitKind::SourceBytes, MAX_SOURCE_BYTES));
    }
    let mut lexer = Lexer {
        source,
        position: 0,
        tokens: Vec::new(),
        depth: 0,
    };
    while lexer.position < source.len() {
        lexer.skip_whitespace();
        if lexer.position < source.len() {
            lexer.token()?;
        }
        if lexer.tokens.len() > MAX_NODES * 4 {
            return Err(Error::limit(LimitKind::SyntaxNodes, MAX_NODES));
        }
    }
    if lexer.depth != 0 {
        return Err(Error::parse(source.len(), "unclosed delimiter"));
    }
    lexer.tokens.push(Token {
        kind: TokenKind::Eof,
        offset: source.len(),
    });
    Ok(lexer.tokens)
}

struct Lexer<'a> {
    source: &'a str,
    position: usize,
    tokens: Vec<Token>,
    depth: usize,
}

impl Lexer<'_> {
    fn token(&mut self) -> Result<()> {
        let offset = self.position;
        let byte = self.bytes()[self.position];
        let kind = match byte {
            b'(' => self.open(TokenKind::LeftParen)?,
            b')' => self.close(TokenKind::RightParen)?,
            b'[' => self.open(TokenKind::LeftBracket)?,
            b']' => self.close(TokenKind::RightBracket)?,
            b'{' => self.open(TokenKind::LeftBrace)?,
            b'}' => self.close(TokenKind::RightBrace)?,
            b',' => self.single(TokenKind::Comma),
            b'+' => self.single(TokenKind::Plus),
            b'-' => self.single(TokenKind::Minus),
            b'%' => self.single(TokenKind::Percent),
            b'.' if self.peek(1).is_some_and(|byte| byte.is_ascii_digit()) => self.number()?,
            b'.' => self.single(TokenKind::Dot),
            b'*' if self.peek(1) == Some(b'*') => self.double(TokenKind::Power),
            b'*' => self.single(TokenKind::Star),
            b'/' => self.single(TokenKind::Slash),
            b'!' if self.peek(1) == Some(b'=') => self.double(TokenKind::NotEqual),
            b'=' if self.peek(1) == Some(b'=') => self.double(TokenKind::EqualEqual),
            b'<' if self.peek(1) == Some(b'=') => self.double(TokenKind::LessEqual),
            b'<' => self.single(TokenKind::Less),
            b'>' if self.peek(1) == Some(b'=') => self.double(TokenKind::GreaterEqual),
            b'>' => self.single(TokenKind::Greater),
            b'"' => self.string()?,
            byte if byte.is_ascii_digit() => self.number()?,
            byte if is_ident_start(byte) => self.ident(),
            _ => return Err(Error::parse(offset, "unexpected character")),
        };
        self.tokens.push(Token { kind, offset });
        Ok(())
    }

    fn bytes(&self) -> &[u8] {
        self.source.as_bytes()
    }

    fn peek(&self, ahead: usize) -> Option<u8> {
        self.bytes().get(self.position + ahead).copied()
    }

    fn single(&mut self, kind: TokenKind) -> TokenKind {
        self.position += 1;
        kind
    }

    fn double(&mut self, kind: TokenKind) -> TokenKind {
        self.position += 2;
        kind
    }

    fn open(&mut self, kind: TokenKind) -> Result<TokenKind> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(Error::limit(LimitKind::NestingDepth, MAX_DEPTH));
        }
        self.position += 1;
        Ok(kind)
    }

    fn close(&mut self, kind: TokenKind) -> Result<TokenKind> {
        if self.depth == 0 {
            return Err(Error::parse(self.position, "unmatched closing delimiter"));
        }
        self.depth -= 1;
        self.position += 1;
        Ok(kind)
    }

    fn skip_whitespace(&mut self) {
        while self.peek(0).is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.position += 1;
        }
    }

    fn ident(&mut self) -> TokenKind {
        let start = self.position;
        self.position += 1;
        while self.peek(0).is_some_and(is_ident_continue) {
            self.position += 1;
        }
        match &self.source[start..self.position] {
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            "nil" => TokenKind::Nil,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "exists" => TokenKind::Exists,
            "not" => TokenKind::Not,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "otherwise" => TokenKind::Otherwise,
            name => TokenKind::Ident(name.to_owned()),
        }
    }

    fn number(&mut self) -> Result<TokenKind> {
        let start = self.position;
        self.digits_and_separators()?;
        if self.peek(0) == Some(b'.') && self.peek(1) != Some(b'.') {
            self.position += 1;
            if self.peek(0) == Some(b'_') {
                return Err(Error::parse(
                    self.position,
                    "underscore must separate digits",
                ));
            }
            self.digits_and_separators()?;
        }
        let text = self.source[start..self.position].replace('_', "");
        let value = text
            .parse::<f64>()
            .map_err(|_| Error::parse(start, "invalid number literal"))?;
        if !value.is_finite() {
            return Err(Error::parse(start, "number literal must be finite"));
        }
        Ok(TokenKind::Num(value))
    }

    fn digits_and_separators(&mut self) -> Result<()> {
        let mut digit = false;
        while let Some(byte) = self.peek(0) {
            if byte.is_ascii_digit() {
                digit = true;
                self.position += 1;
            } else if byte == b'_' {
                if !digit || !self.peek(1).is_some_and(|next| next.is_ascii_digit()) {
                    return Err(Error::parse(
                        self.position,
                        "underscore must separate digits",
                    ));
                }
                digit = false;
                self.position += 1;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn string(&mut self) -> Result<TokenKind> {
        let offset = self.position;
        self.position += 1;
        let mut value = String::new();
        while let Some(byte) = self.peek(0) {
            if byte == b'"' {
                self.position += 1;
                return Ok(TokenKind::String(value));
            }
            if byte != b'\\' {
                let character = self.source[self.position..]
                    .chars()
                    .next()
                    .ok_or_else(|| Error::parse(offset, "unclosed string literal"))?;
                value.push(character);
                self.position += character.len_utf8();
                continue;
            }
            self.position += 1;
            let escaped = self
                .peek(0)
                .ok_or_else(|| Error::parse(offset, "unclosed escape sequence"))?;
            self.position += 1;
            match escaped {
                b'\\' => value.push('\\'),
                b'"' => value.push('"'),
                b'n' => value.push('\n'),
                b'r' => value.push('\r'),
                b't' => value.push('\t'),
                b'u' => {
                    let end = self.position.saturating_add(4);
                    let digits = self
                        .source
                        .get(self.position..end)
                        .ok_or_else(|| Error::parse(offset, "short unicode escape"))?;
                    let scalar = u32::from_str_radix(digits, 16)
                        .map_err(|_| Error::parse(offset, "invalid unicode escape"))?;
                    let character = char::from_u32(scalar)
                        .ok_or_else(|| Error::parse(offset, "invalid unicode scalar"))?;
                    value.push(character);
                    self.position = end;
                }
                _ => return Err(Error::parse(offset, "invalid escape sequence")),
            }
        }
        Err(Error::parse(offset, "unclosed string literal"))
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
