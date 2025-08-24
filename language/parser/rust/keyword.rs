use destack_language_lexer::{SemanticToken, Span, Token, TokenType};

use crate::{ParseError, ParseResult, Parser};

/// A contextual keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    /// Mark the following item as public (with optional qualifier)
    Public,
    /// Define a Module.
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
    /// Implement a Trait.
    Implement,
    /// Use an item in this context (like importing items from a module).
    Using,
    /// Alias or cast an item in this context.
    As,
    /// Define a constant.
    Const,
    /// Define a variable.
    Let,
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

impl<'a> Parser<'a> {
    /// Eat a keyword.
    pub fn eat_keyword(&mut self, keyword: Keyword) -> ParseResult<'a, ()> {
        let Some(current) = self.tokens.get(self.pos) else {
            return Err(ParseError::SyntaxError(SemanticToken {
                token: Token {
                    r#type: TokenType::EndOfInput,
                    len: 0,
                },
                span: Span::default(),
            }));
        };

        if current.token.r#type == TokenType::Identifier {
            self.pos = self.pos.saturating_add(1);
            Ok(())
        } else {
            Err(ParseError::SyntaxError(*current))
        }
    }

    /// Peek a keyword.
    pub fn peek_keyword(&self, keyword: Keyword) -> ParseResult<'a, ()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use destack_language_lexer::tokenize_semantic;

    use crate::{Keyword, Parser};

    #[test]
    fn test_keyword() {
        let input = "public";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(&tokens);
        parser.eat_keyword(Keyword::Public).unwrap();
    }
}
