use tspp_source::{FileId, Span};

use crate::{Token, TokenType};

/// Bytecode lexer.
#[derive(Debug)]
pub struct Lexer<'a> {
    /// Source text.
    source: &'a str,
    /// Current byte offset.
    offset: usize,
}

impl<'a> Lexer<'a> {
    /// Create one lexer.
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

    /// Return the remaining source.
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

    /// Advance while one predicate accepts the next character.
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
            '-' if self.peek() == Some('>') => {
                self.bump();
                TokenType::Arrow
            }
            '-' if self.peek().is_some_and(Self::is_identifier_start) => {
                self.bump_while(Self::is_identifier_continue);
                TokenType::Identifier
            }
            '=' if self.peek() == Some('>') => {
                self.bump();
                TokenType::FatArrow
            }
            character if character.is_ascii_digit() => self.number(),
            '-' if self.peek().is_some_and(|next| next.is_ascii_digit()) => self.number(),
            character if Self::is_identifier_start(character) => {
                self.bump_while(Self::is_identifier_continue);
                TokenType::Identifier
            }
            character => match Self::symbol(character) {
                Some(ty) => ty,
                None => TokenType::Unknown,
            },
        };

        self.token(file_id, start, ty)
    }

    /// Create one token ending at the current offset.
    fn token(&self, file_id: FileId, start: usize, ty: TokenType) -> Token {
        let byte_len = self.offset - start;
        let span = Span::at(file_id, start as u32, byte_len as u32);

        Token::new(ty, span)
    }

    /// Lex one numeric literal.
    fn number(&mut self) -> TokenType {
        self.bump_digits();

        let has_fraction = if self.peek() == Some('.') {
            self.bump();
            self.bump_digits();

            true
        } else {
            false
        };

        // keep a signed decimal exponent in the same floating point token
        let has_exponent = if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            self.bump_digits();

            true
        } else {
            false
        };

        if has_fraction || has_exponent {
            TokenType::Float
        } else {
            self.bump_while(|character| character.is_ascii_alphanumeric());

            TokenType::Integer
        }
    }

    /// Advance over decimal digits and separators.
    fn bump_digits(&mut self) {
        self.bump_while(|character| character.is_ascii_digit() || character == '_');
    }

    /// Return one symbol token category.
    fn symbol(character: char) -> Option<TokenType> {
        let ty = match character {
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
            '|' => TokenType::Pipe,
            '@' => TokenType::At,
            '*' => TokenType::Star,
            '=' => TokenType::Equal,
            _ => return None,
        };

        Some(ty)
    }

    /// Return whether one character starts an identifier.
    fn is_identifier_start(character: char) -> bool {
        character.is_ascii_alphabetic() || character == '_'
    }

    /// Return whether one character continues an identifier.
    fn is_identifier_continue(character: char) -> bool {
        character.is_ascii_alphanumeric() || matches!(character, '_' | '.')
    }
}
