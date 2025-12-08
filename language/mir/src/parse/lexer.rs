//! MIR lexer.

use super::token::{Token, TokenType};

/// Lexer for MIR text format.
#[derive(Debug)]
pub struct Lexer<'a> {
    /// The source string.
    source: &'a str,
    /// Current position in the source.
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer.
    pub fn new(source: &'a str) -> Self {
        Self { source, pos: 0 }
    }

    /// Lex all tokens from the source.
    pub fn lex(source: &'a str) -> Vec<Token<'a>> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            let is_end = token.ty == TokenType::End;
            tokens.push(token);
            if is_end {
                break;
            }
        }
        tokens
    }

    /// Get the remaining source.
    fn remaining(&self) -> &'a str {
        &self.source[self.pos..]
    }

    /// Peek the next character.
    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    /// Peek the second character.
    fn peek_next(&self) -> Option<char> {
        let mut chars = self.remaining().chars();
        chars.next();
        chars.next()
    }

    /// Advance by one character.
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Advance while predicate is true.
    fn advance_while(&mut self, predicate: impl Fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Get the next token.
    fn next_token(&mut self) -> Token<'a> {
        let start = self.pos;

        let Some(c) = self.advance() else {
            return Token::new(TokenType::End, "", start);
        };

        let ty = match c {
            // whitespace
            ' ' | '\t' => {
                self.advance_while(|c| c == ' ' || c == '\t');
                TokenType::Whitespace
            }

            // newline
            '\n' => TokenType::Newline,
            '\r' => {
                if self.peek() == Some('\n') {
                    self.advance();
                }
                TokenType::Newline
            }

            // comment
            '/' if self.peek() == Some('/') => {
                self.advance_while(|c| c != '\n');
                TokenType::Comment
            }

            // symbols
            '@' => TokenType::At,
            '(' => TokenType::OpenParen,
            ')' => TokenType::CloseParen,
            '{' => TokenType::OpenBrace,
            '}' => TokenType::CloseBrace,
            '[' => TokenType::OpenBracket,
            ']' => TokenType::CloseBracket,
            '<' => TokenType::LessThan,
            '>' => TokenType::GreaterThan,
            ':' => TokenType::Colon,
            ';' => TokenType::Semicolon,
            ',' => TokenType::Comma,
            '=' if self.peek() == Some('>') => {
                self.advance();
                TokenType::FatArrow
            }
            '=' => TokenType::Equals,
            '-' if self.peek() == Some('>') => {
                self.advance();
                TokenType::Arrow
            }

            // identifier or keyword
            c if is_ident_start(c) => self.eat_identifier(start),

            // number literal
            c if c.is_ascii_digit() => self.eat_number(start),
            '-' if self.peek().is_some_and(|c| c.is_ascii_digit()) => self.eat_number(start),

            _ => TokenType::Unknown,
        };

        let text = &self.source[start..self.pos];
        Token::new(ty, text, start)
    }

    /// Lex an identifier or keyword.
    fn eat_identifier(&mut self, start: usize) -> TokenType {
        self.advance_while(is_ident_continue);
        let text = &self.source[start..self.pos];

        // check for keywords
        match text {
            "extern" => TokenType::Extern,
            "function" => TokenType::Function,
            "global" => TokenType::Global,
            "return" => TokenType::Return,
            "jump" => TokenType::Jump,
            "branch" => TokenType::Branch,
            "switch" => TokenType::Switch,
            "unreachable" => TokenType::Unreachable,
            "void" => TokenType::Void,
            "bool" => TokenType::Bool,
            "rawptr" => TokenType::RawPtr,
            "ref" => {
                // check for ref? (nullable reference)
                if self.peek() == Some('?') {
                    self.advance();
                    TokenType::RefNullable
                } else {
                    TokenType::Ref
                }
            }
            "fn" => TokenType::Fn,
            "struct" => TokenType::Struct,
            "true" | "false" => TokenType::BoolLiteral,
            "owned" | "borrowed" | "copy" => TokenType::Ownership,
            "var" => TokenType::Var,
            "const" => TokenType::Const,
            _ => {
                // value
                if let Some(rest) = text.strip_prefix('v')
                    && rest.chars().all(|c| c.is_ascii_digit())
                {
                    TokenType::Value
                }
                // block
                else if let Some(rest) = text.strip_prefix("block")
                    && rest.chars().all(|c| c.is_ascii_digit())
                {
                    TokenType::BlockRefence
                }
                // local
                else if let Some(rest) = text.strip_prefix("local")
                    && rest.chars().all(|c| c.is_ascii_digit())
                {
                    TokenType::LocalReference
                }
                // type name
                else if is_type_name(text) {
                    TokenType::TypeName
                }
                // identifier
                else {
                    TokenType::Identifier
                }
            }
        }
    }

    /// Lex a number literal.
    fn eat_number(&mut self, start: usize) -> TokenType {
        // consume digits
        self.advance_while(|c| c.is_ascii_digit() || c == '_');

        // check for float
        if self.peek() == Some('.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.advance(); // consume '.'
            self.advance_while(|c| c.is_ascii_digit() || c == '_');
            // consume optional type suffix (f32, f64)
            self.advance_while(|c| c.is_ascii_alphanumeric());
            return TokenType::FloatLiteral;
        }

        // consume type suffix (i32, u64, f32, etc.)
        // type suffixes are like "i32", "u64", "f32" - letter followed by digits
        self.advance_while(|c| c.is_ascii_alphanumeric());

        let text = &self.source[start..self.pos];
        if text.contains('f') || text.contains('.') {
            TokenType::FloatLiteral
        } else {
            TokenType::IntLiteral
        }
    }
}

/// Check if character can start an identifier.
fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Check if character can continue an identifier.
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Check if string is a type name (i8, i16, i32, i64, u8, u16, u32, u64, f32, f64).
fn is_type_name(s: &str) -> bool {
    matches!(
        s,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "i256"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "u256"
            | "f32"
            | "f64"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_simple_function() {
        let source = "function @function0() -> void {";
        let tokens = Lexer::lex(source);
        let types: Vec<_> = tokens.iter().map(|t| t.ty).collect();
        assert_eq!(
            types,
            vec![
                TokenType::Function,
                TokenType::Whitespace,
                TokenType::At,
                TokenType::Identifier, // function0
                TokenType::OpenParen,
                TokenType::CloseParen,
                TokenType::Whitespace,
                TokenType::Arrow,
                TokenType::Whitespace,
                TokenType::Void,
                TokenType::Whitespace,
                TokenType::OpenBrace,
                TokenType::End,
            ]
        );
    }

    #[test]
    fn test_lex_value_and_type() {
        let source = "v0: i32";
        let tokens = Lexer::lex(source);
        let types: Vec<_> = tokens.iter().map(|t| t.ty).collect();
        assert_eq!(
            types,
            vec![
                TokenType::Value,
                TokenType::Colon,
                TokenType::Whitespace,
                TokenType::TypeName,
                TokenType::End,
            ]
        );
    }

    #[test]
    fn test_lex_instruction() {
        let source = "v2 = iadd v0, v1";
        let tokens = Lexer::lex(source);
        let types: Vec<_> = tokens.iter().map(|t| t.ty).collect();
        assert_eq!(
            types,
            vec![
                TokenType::Value,
                TokenType::Whitespace,
                TokenType::Equals,
                TokenType::Whitespace,
                TokenType::Identifier, // iadd
                TokenType::Whitespace,
                TokenType::Value,
                TokenType::Comma,
                TokenType::Whitespace,
                TokenType::Value,
                TokenType::End,
            ]
        );
    }

    #[test]
    fn test_lex_int_literal() {
        let source = "42i32 -5i64 0u8";
        let tokens = Lexer::lex(source);
        let types: Vec<_> = tokens
            .iter()
            .filter(|t| !t.ty.is_trivia())
            .map(|t| t.ty)
            .collect();
        assert_eq!(
            types,
            vec![
                TokenType::IntLiteral,
                TokenType::IntLiteral,
                TokenType::IntLiteral,
                TokenType::End,
            ]
        );
    }

    #[test]
    fn test_lex_block_ref() {
        let source = "block0 block123";
        let tokens = Lexer::lex(source);
        let types: Vec<_> = tokens
            .iter()
            .filter(|t| !t.ty.is_trivia())
            .map(|t| t.ty)
            .collect();
        assert_eq!(
            types,
            vec![
                TokenType::BlockRefence,
                TokenType::BlockRefence,
                TokenType::End
            ]
        );
    }
}
