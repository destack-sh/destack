//! Parse patterns.

use destack_language_token::TokenType;

use crate::{NodeId, ParseError, ParseResult, Parser, Pattern, RangeLiteral};

impl<'a> Parser<'a> {
    /// Eat a pattern.
    ///
    /// Examples:
    /// ```
    /// _
    /// 1
    /// 2 | 3
    /// 4..6
    /// (x, 0, ..)
    /// x, y
    /// y, x, ..
    /// Vector2 { x: 0, y, z: zed }
    /// ```
    pub fn eat_pattern(&mut self) -> ParseResult<NodeId<Pattern>> {
        let start = self.mark();

        // ------------------------------------------------------------
        // Primary patterns
        // ------------------------------------------------------------
        let pattern_id = {
            // wildcard
            if self.peek_identifier_char('_').is_ok() {
                self.bump();
                self.tree
                    .allocate(Pattern::Wildcard, self.get_span_from(start))
            }
            // rest
            else if self.peek_token(TokenType::Range).is_ok() {
                self.bump();
                self.tree.allocate(Pattern::Rest, self.get_span_from(start))
            }
            // scalar literal
            else if self.peek_scalar_literal().is_ok() {
                let literal_id = self.eat_scalar_literal()?;
                self.tree
                    .allocate(Pattern::Scalar(literal_id), self.get_span_from(start))
            }
            // tuple (explicit)
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.eat_token(TokenType::OpenParenthesis)?;
                let pattern_id = self.eat_pattern()?; // handle as implicit tuple
                self.eat_token(TokenType::CloseParenthesis)?;
                pattern_id
            }
            // error
            else {
                return Err(ParseError::UnexpectedToken(self.peek()?.span));
            }
        };

        // ------------------------------------------------------------
        // Postfix->Infix patterns
        // ------------------------------------------------------------

        // range
        if self.peek_token(TokenType::Range).is_ok() {
            self.bump(); // eat range
            let end_id = self.eat_pattern()?;
            let pattern = Pattern::Range {
                start: Some(pattern_id),
                end: Some(end_id),
                is_inclusive: false,
            };
            let pattern_id = self.tree.allocate(pattern, self.get_span_from(start));
            Ok(pattern_id)
        }
        // union
        else if self.peek_token(TokenType::BitwiseOr).is_ok() {
            todo!("union");
        }
        // tuple (implicit)
        else if self.peek_token(TokenType::Comma).is_ok() {
            todo!("tuple");
        }
        // no infix
        else {
            Ok(pattern_id)
        }
    }
}
