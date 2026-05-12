use destack_source::{FileId, Span};

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
    pub fn lex(file_id: FileId, source: &'a str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token(file_id);
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
    fn next_token(&mut self, file_id: FileId) -> Token {
        let start = self.pos;

        let Some(c) = self.advance() else {
            return Token::new(TokenType::End, Span::at(file_id, start as u32, 0));
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
            '#' => TokenType::Hash,
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

            // string literal
            '"' => self.eat_string(),

            // char literal
            '\'' => self.eat_char(),

            // identifier or keyword
            c if is_ident_start(c) => self.eat_identifier(start),

            // number literal
            c if c.is_ascii_digit() => self.eat_number(start),
            '-' if self.peek().is_some_and(|c| c.is_ascii_digit()) => self.eat_number(start),

            _ => TokenType::Unknown,
        };

        let length = self.pos.saturating_sub(start);
        let span = Span::at(file_id, start as u32, length as u32);
        Token::new(ty, span)
    }

    /// Lex an identifier or keyword.
    fn eat_identifier(&mut self, start: usize) -> TokenType {
        self.advance_while(is_ident_continue);
        let text = &self.source[start..self.pos];

        // check for keywords
        match text {
            "extern" => TokenType::Extern,
            "export" => TokenType::Export,
            "function" => TokenType::Function,
            "global" => TokenType::Global,
            "type" => TokenType::Type,
            "local" => TokenType::Local,
            "return" => TokenType::Return,
            "jump" => TokenType::Jump,
            "branch" => TokenType::Branch,
            "check" => TokenType::Check,
            "switch" => TokenType::Switch,
            "yield" => TokenType::Yield,
            "call" => TokenType::Call,
            "trap.abort" => TokenType::Trap,
            "trap.panic" => TokenType::Trap,
            "unreachable" => TokenType::Unreachable,
            "tailCall" => TokenType::TailCall,
            "call.indirect" => TokenType::CallIndirect,
            "tailCall.indirect" => TokenType::TailCallIndirect,
            "call.class" => TokenType::CallClass,
            "tailCall.class" => TokenType::TailCallClass,
            "call.interface" => TokenType::CallInterface,
            "tailCall.interface" => TokenType::TailCallInterface,
            "void" => TokenType::Void,
            "boolean" => TokenType::Boolean,
            "ref" => {
                // check for ref? (nullable reference)
                if self.peek() == Some('?') {
                    self.advance();
                    TokenType::RefNullable
                } else {
                    TokenType::Ref
                }
            }
            "vector" => TokenType::Vector,
            "tensor" => TokenType::Tensor,
            "tensorView" => {
                if self.peek() == Some('?') {
                    self.advance();
                    TokenType::TensorViewNullable
                } else {
                    TokenType::TensorView
                }
            }
            "space" => TokenType::AddressSpace,
            "struct" => TokenType::Struct,
            "newtype" => TokenType::Newtype,
            "true" | "false" => TokenType::BooleanLiteral,
            "owned" | "borrowed" | "copy" => TokenType::Ownership,
            "readonly" => TokenType::Readonly,
            "const" => TokenType::Const,
            _ => {
                // value
                if let Some(rest) = text.strip_prefix('v')
                    && !rest.is_empty()
                    && rest.chars().all(|c| c.is_ascii_digit())
                {
                    TokenType::Value
                }
                // block
                else if let Some(rest) = text.strip_prefix('b')
                    && !rest.is_empty()
                    && rest.chars().all(|c| c.is_ascii_digit())
                {
                    TokenType::BlockRefence
                }
                // local
                else if let Some(rest) = text.strip_prefix("local")
                    && !rest.is_empty()
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

    /// Lex a string literal.
    fn eat_string(&mut self) -> TokenType {
        // opening " already consumed
        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    return TokenType::StringLiteral;
                }
                Some('\\') => {
                    // escape sequence - consume backslash and next char
                    self.advance();
                    self.advance();
                }
                Some('\n') | None => {
                    // unterminated string
                    return TokenType::Unknown;
                }
                Some(_) => {
                    self.advance();
                }
            }
        }
    }

    /// Lex a character literal.
    fn eat_char(&mut self) -> TokenType {
        // opening ' already consumed
        match self.peek() {
            Some('\\') => {
                // escape sequence
                self.advance();
                self.advance();
            }
            Some('\'') | Some('\n') | None => {
                // empty char or unterminated
                return TokenType::Unknown;
            }
            Some(_) => {
                self.advance();
            }
        }

        // expect closing '
        if self.peek() == Some('\'') {
            self.advance();
            TokenType::CharacterLiteral
        } else {
            TokenType::Unknown
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
            // consume optional type suffix (float32, float64)
            self.advance_while(|c| c.is_ascii_alphanumeric());
            return TokenType::FloatLiteral;
        }

        // consume type suffix (int32, uint64, float32, etc.)
        // type suffixes are an identifier tail after the numeric payload
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
/// Includes `.`, `/`, `#`, and the hyphen character to support module and symbol names.
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '#' | '-')
}

/// Check if string is a type name.
fn is_type_name(s: &str) -> bool {
    matches!(
        s,
        "int8"
            | "int16"
            | "int32"
            | "int64"
            | "int128"
            | "int256"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uint128"
            | "uint256"
            | "float32"
            | "float64"
            | "isize"
            | "usize"
            | "typeDescriptor"
            | "typeId"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_simple_function() {
        let source = "function function0(): void {";
        let tokens = Lexer::lex(FileId::new(0), source);
        let types: Vec<_> = tokens.iter().map(|t| t.ty).collect();
        assert_eq!(
            types,
            vec![
                TokenType::Function,
                TokenType::Whitespace,
                TokenType::Identifier, // function0
                TokenType::OpenParen,
                TokenType::CloseParen,
                TokenType::Colon,
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
        let source = "v0: int32";
        let tokens = Lexer::lex(FileId::new(0), source);
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
        let source = "v2 = int.add v0, v1";
        let tokens = Lexer::lex(FileId::new(0), source);
        let types: Vec<_> = tokens.iter().map(|t| t.ty).collect();
        assert_eq!(
            types,
            vec![
                TokenType::Value,
                TokenType::Whitespace,
                TokenType::Equals,
                TokenType::Whitespace,
                TokenType::Identifier, // int.add
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
        let source = "42int32 -5int64 0uint8";
        let tokens = Lexer::lex(FileId::new(0), source);
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
        let source = "b0 b123";
        let tokens = Lexer::lex(FileId::new(0), source);
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

    #[test]
    fn test_lex_string_literal() {
        let source = r#""hello" "world""#;
        let tokens = Lexer::lex(FileId::new(0), source);
        let types: Vec<_> = tokens
            .iter()
            .filter(|t| !t.ty.is_trivia())
            .map(|t| t.ty)
            .collect();
        assert_eq!(
            types,
            vec![
                TokenType::StringLiteral,
                TokenType::StringLiteral,
                TokenType::End
            ]
        );
    }

    #[test]
    fn test_lex_string_with_escapes() {
        let source = r#""hello\nworld" "tab\there""#;
        let tokens = Lexer::lex(FileId::new(0), source);
        let types: Vec<_> = tokens
            .iter()
            .filter(|t| !t.ty.is_trivia())
            .map(|t| t.ty)
            .collect();
        assert_eq!(
            types,
            vec![
                TokenType::StringLiteral,
                TokenType::StringLiteral,
                TokenType::End
            ]
        );
    }

    #[test]
    fn test_lex_char_literal() {
        let source = "'a' 'b' '\\n'";
        let tokens = Lexer::lex(FileId::new(0), source);
        let types: Vec<_> = tokens
            .iter()
            .filter(|t| !t.ty.is_trivia())
            .map(|t| t.ty)
            .collect();
        assert_eq!(
            types,
            vec![
                TokenType::CharacterLiteral,
                TokenType::CharacterLiteral,
                TokenType::CharacterLiteral,
                TokenType::End
            ]
        );
    }
}
