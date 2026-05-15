use destack_source::{FileId, Span};

use crate::source::{Token, TokenType};

/// MIR lexer.
#[derive(Debug)]
pub struct Lexer<'a> {
    /// Source text.
    source: &'a str,
    /// Current byte offset.
    offset: usize,
}

impl<'a> Lexer<'a> {
    /// Create a lexer.
    pub const fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    /// Lex all tokens from one source.
    pub fn lex(file_id: FileId, source: &'a str) -> Vec<Token> {
        let mut lexer = Self::new(source);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next(file_id);
            let is_end = token.ty == TokenType::End;
            tokens.push(token);

            if is_end {
                break;
            }
        }

        tokens
    }

    /// Return remaining source.
    fn rest(&self) -> &'a str {
        &self.source[self.offset..]
    }

    /// Return the next character.
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Advance by one character.
    fn bump(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();

        Some(character)
    }

    /// Advance while a predicate accepts the next character.
    fn bump_while(&mut self, predicate: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&predicate) {
            self.bump();
        }
    }

    /// Lex the next token.
    fn next(&mut self, file_id: FileId) -> Token {
        let start = self.offset;
        let Some(character) = self.bump() else {
            return self.token(file_id, start, TokenType::End);
        };

        let ty = match character {
            ' ' | '\t' => {
                self.bump_while(|character| matches!(character, ' ' | '\t'));
                TokenType::Whitespace
            }
            '\n' => TokenType::Newline,
            '\r' => {
                if self.peek() == Some('\n') {
                    self.bump();
                }
                TokenType::Newline
            }
            '/' if self.peek() == Some('/') => {
                self.bump_while(|character| character != '\n');
                TokenType::Comment
            }
            '"' => self.string(),
            '\'' => self.character(),
            '-' if self.peek() == Some('>') => {
                self.bump();
                TokenType::Arrow
            }
            '=' if self.peek() == Some('>') => {
                self.bump();
                TokenType::FatArrow
            }
            character if character.is_ascii_digit() => self.number(),
            '-' if self
                .peek()
                .is_some_and(|character| character.is_ascii_digit()) =>
            {
                self.number()
            }
            character if is_identifier_start(character) => {
                self.bump_while(is_identifier_continue);
                TokenType::Identifier
            }
            character => symbol_ty(character).unwrap_or(TokenType::Unknown),
        };

        self.token(file_id, start, ty)
    }

    /// Create one token from the current offset.
    fn token(&self, file_id: FileId, start: usize, ty: TokenType) -> Token {
        let length = self.offset.saturating_sub(start);
        let span = Span::at(file_id, start as u32, length as u32);

        Token::new(ty, span)
    }

    /// Lex one string literal.
    fn string(&mut self) -> TokenType {
        loop {
            match self.peek() {
                Some('"') => {
                    self.bump();
                    return TokenType::String;
                }
                Some('\\') => {
                    self.bump();
                    self.bump();
                }
                Some('\n') | None => return TokenType::Unknown,
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    /// Lex one character literal.
    fn character(&mut self) -> TokenType {
        match self.peek() {
            Some('\\') => {
                self.bump();
                self.bump();
            }
            Some('\'') | Some('\n') | None => return TokenType::Unknown,
            Some(_) => {
                self.bump();
            }
        }

        if self.peek() == Some('\'') {
            self.bump();
            TokenType::Character
        } else {
            TokenType::Unknown
        }
    }

    /// Lex one numeric literal.
    fn number(&mut self) -> TokenType {
        self.bump_while(|character| character.is_ascii_digit() || character == '_');

        if self.peek() == Some('.') {
            self.bump();
            self.bump_while(|character| character.is_ascii_digit() || character == '_');
            self.bump_while(|character| character.is_ascii_alphanumeric());
            return TokenType::Float;
        }

        self.bump_while(|character| character.is_ascii_alphanumeric());
        TokenType::Integer
    }
}

/// Return the symbol type for one character.
fn symbol_ty(character: char) -> Option<TokenType> {
    let ty = match character {
        '@' => TokenType::At,
        '#' => TokenType::Hash,
        '(' => TokenType::OpenParenthesis,
        ')' => TokenType::CloseParenthesis,
        '{' => TokenType::OpenBrace,
        '}' => TokenType::CloseBrace,
        '[' => TokenType::OpenBracket,
        ']' => TokenType::CloseBracket,
        '<' => TokenType::LessThan,
        '>' => TokenType::GreaterThan,
        ':' => TokenType::Colon,
        ';' => TokenType::Semicolon,
        ',' => TokenType::Comma,
        '?' => TokenType::Question,
        '=' => TokenType::Equal,
        _ => return None,
    };

    Some(ty)
}

/// Return whether a character starts an identifier.
#[inline]
fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

/// Return whether a character continues an identifier.
#[inline]
fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '/' | '#' | '-')
}
