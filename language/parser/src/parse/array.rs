use crate::parse::error::ParserResultExt;
use crate::parse::{ExpressionPosition, ExpressionStop};
use tspp_dir::{Argument, Expression, LocalNodeId, NodeType, TokenType};

use crate::{ParseStart, Parser, ParserResult};

impl Parser {
    /// Parse a bracket literal expression including the surrounding brackets.
    pub(crate) fn parse_bracket_literal(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_token(TokenType::OpenBracket)?;

        if self.peek_is(TokenType::CloseBracket) {
            self.bump();

            return Ok(self.insert_node(
                Expression::ArrayExpression { elements: vec![] },
                self.range_since(start),
            ));
        }

        if self.peek_is(TokenType::Comma) {
            let elements = self.parse_array_elements(None, position.nested())?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

            return Ok(self.insert_node(
                Expression::ArrayExpression { elements },
                self.range_since(start),
            ));
        }

        if self.peek_is(TokenType::Semicolon) {
            let value = self.recover_missing_expression_here(NodeType::Expression);
            self.bump();
            let length = self.parse_expression_or_recover_missing(
                position.nested(),
                ExpressionStop::default(),
                NodeType::Expression,
            )?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

            return Ok(self.insert_node(
                Expression::FixedArrayExpression { value, length },
                self.range_since(start),
            ));
        }

        let first = self.parse_positional_argument(position.nested())?;
        if self.peek_is(TokenType::Semicolon)
            && let Argument::Positional { value } = self.tree.get(first)
        {
            let value = *value;
            self.bump();
            let length = self.parse_expression_or_recover_missing(
                position.nested(),
                ExpressionStop::default(),
                NodeType::Expression,
            )?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

            return Ok(self.insert_node(
                Expression::FixedArrayExpression { value, length },
                self.range_since(start),
            ));
        }

        let elements = self.parse_array_elements(Some(first), position.nested())?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;

        Ok(self.insert_node(
            Expression::ArrayExpression { elements },
            self.range_since(start),
        ))
    }

    /// Parse an array literal (including the surrounding brackets).
    #[cfg(test)]
    pub(crate) fn parse_array_literal(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<Vec<LocalNodeId<Argument>>> {
        self.eat_token(TokenType::OpenBracket)?;
        let elements = if self.peek_is(TokenType::CloseBracket) {
            vec![]
        } else {
            self.parse_array_elements(None, position.nested())?
        };
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
        Ok(elements)
    }

    /// Parse array elements after the opening bracket.
    fn parse_array_elements(
        &mut self,
        first_element: Option<LocalNodeId<Argument>>,
        position: ExpressionPosition,
    ) -> ParserResult<Vec<LocalNodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        // track whether we expect an element (at start or after comma)
        let mut expect_element = first_element.is_none();
        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop at the closing token (trailing commas are allowed, no hole)
            if token_type == TokenType::CloseBracket {
                break;
            }

            // consume comma separators
            if token_type == TokenType::Comma {
                let start = self.mark_parse_start();

                // leading hole: if we expected an element but got separator instead
                if expect_element {
                    let hole = self.insert_node(Argument::Elision, self.range_since(&start));
                    elements.push(hole);
                }

                // consume optional newlines before comma and then the comma itself
                self.eat_token(TokenType::Comma)?;

                expect_element = true;
                continue;
            }

            // keep eating elements (positional/spread only)
            let element = self
                .parse_positional_argument(position)
                .in_node(NodeType::Argument)?;

            elements.push(element);
            expect_element = false;
        }
        Ok(elements)
    }
}
