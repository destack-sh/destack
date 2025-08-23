use destack_language_lexer::{SemanticToken, Span, Token, TokenType};

use crate::{ParseError, ParseResult, Parser};

/// A contextual keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Public,
    Module,
    Struct,
    Enum,
    Union,
    Trait,
    Function,
    Implement,
    Using,
    Const,
    Let,
    If,
    Else,
    While,
    For,
    Loop,
    Break,
    Continue,
    Return,
    Match,
    Try,
    Catch,
}

impl<'a> Parser<'a> {
    /// Eat a keyword.
    pub fn eat_keyword(&mut self, _keyword: Keyword) -> ParseResult<'a, ()> {
        let Some(current) = self.tokens.get(self.pos) else {
            return Err(ParseError::UnexpectedToken(SemanticToken {
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
            Err(ParseError::UnexpectedToken(*current))
        }
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
