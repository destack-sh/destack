use destack_language_token::{TokenSpan, TokenType};

use crate::{ParseError, ParseResult, Parser};

/// A contextual keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    /// Mark the following item as public (with optional qualifier)
    Public,
    /// Define a Module (inline).
    Module,
    /// Define a Struct.
    Struct,
    /// Define an Enum.
    Enum,
    /// Define a Union.
    Union,
    /// Define a Trait.
    Trait,
    /// Define a Function.
    Function,
    /// Implement a type (perhaps for a Trait).
    Implement,
    /// Use an item in this context (like importing items from a module).
    Using,
    /// Alias or cast an item in this context.
    As,
    /// Where expression.
    Where,
    /// Let expression.
    Let,
    /// Var expression.
    Var,
    /// Conditional expression.
    If,
    /// Conditional expression.
    Else,
    /// Loop expression.
    While,
    /// Loop expression.
    For,
    /// Loop expression.
    In,
    /// Loop expression.
    Loop,
    /// Break expression.
    Break,
    /// Continue expression.
    Continue,
    /// Defer expression.
    Defer,
    /// Return expression.
    Return,
    /// Match expression.
    Match,
    /// Try expression.
    Try,
    /// Catch expression.
    Catch,
}

impl Keyword {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Keyword::Public => "public",
            Keyword::Module => "module",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Union => "union",
            Keyword::Trait => "trait",
            Keyword::Function => "function",
            Keyword::Implement => "implement",
            Keyword::Using => "using",
            Keyword::As => "as",
            Keyword::Where => "where",
            Keyword::Let => "let",
            Keyword::Var => "var",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::For => "for",
            Keyword::In => "in",
            Keyword::Loop => "loop",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Defer => "defer",
            Keyword::Return => "return",
            Keyword::Match => "match",
            Keyword::Try => "try",
            Keyword::Catch => "catch",
        }
    }
}

impl<'a> Parser<'a> {
    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        self.peek_keyword(keyword)?;
        self.eat_token(TokenType::Identifier)
    }

    /// Peek a keyword.
    pub fn peek_keyword(&self, keyword: Keyword) -> ParseResult<&TokenSpan> {
        let current = self.peek_next_token(TokenType::Identifier)?;
        if self.get_span_str(current.span) != keyword.as_str() {
            Err(ParseError::UnexpectedToken(current.span))
        } else {
            Ok(current)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Keyword, Parser};

    #[test]
    fn test_keyword() {
        let input = "public";
        let tokens = tokenize_semantic(input);
        let parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let result = parser.peek_keyword(Keyword::Public);
        assert!(result.is_ok());
        let result = parser.peek_keyword(Keyword::Module);
        assert!(result.is_err());
    }
}
