use crate::parse::{DeclarationNesting, ExpressionPosition, ExpressionStop};
use tspp_dir::{Argument, Expression, LocalNodeId, NodeType, TokenType};

use crate::{Parser, ParserError, ParserResult};

impl Parser {
    /// Parse one standalone argument fragment.
    ///
    /// Examples:
    /// ```tspp
    /// value
    /// ```
    pub fn parse_argument_fragment(&mut self) -> ParserResult<LocalNodeId<Argument>> {
        self.parse_positional_argument(ExpressionPosition::Value)
    }

    /// Return true when one recovered argument list should stop at the current statement boundary.
    fn peek_recovered_argument_list_end(&self, position: ExpressionPosition) -> bool {
        position.is_in_statement() && self.peek_is_on_new_line()
    }

    /// Return whether one argument was recovered as missing or malformed.
    fn has_recovered_argument_slot(&self, argument_id: LocalNodeId<Argument>) -> bool {
        match self.tree.get(argument_id) {
            Argument::Error => true,
            Argument::Elision => false,

            // missing and error values should stop newline led statement calls locally
            Argument::Positional { value } | Argument::Spread { value } => {
                matches!(
                    self.tree.get(*value),
                    Expression::Missing | Expression::Error
                )
            }
        }
    }

    /// Parse a positional or spread value argument.
    ///
    /// Examples:
    /// ```tspp
    /// 2
    /// foo()
    /// ...args
    /// ```
    #[inline]
    pub(crate) fn parse_positional_argument(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<Argument>> {
        let documentation = self.parse_documentation();

        // parse plain values without decorator or spread state
        if !self.peek_is(TokenType::At) && !self.peek_is(TokenType::Spread) {
            // recover empty arguments as list errors, not expression errors
            if Self::is_expression_slot_boundary_token(self.peek_token_type()) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let value = self.parse_expression(position.nested(), ExpressionStop::default())?;
            let value_range = self.tree.get_range(value);
            let argument_id = self.insert_node(Argument::Positional { value }, value_range);
            self.attach_documentation(argument_id, documentation);

            return Ok(argument_id);
        }

        let start = self.mark_parse_start();
        let decorators = self.parse_decorators();

        // spread argument
        if self.peek_is(TokenType::Spread) {
            self.bump();
            let value = self.parse_expression(position.nested(), ExpressionStop::default())?;

            // insert the spread argument
            let argument_id =
                self.insert_node(Argument::Spread { value }, self.range_since(&start));
            self.attach_documentation(argument_id, documentation);
            self.attach_decorators(argument_id.id, decorators);

            return Ok(argument_id);
        }

        // positional value expression
        let value = self.parse_expression(position.nested(), ExpressionStop::default())?;

        // insert the positional argument
        let argument_id =
            self.insert_node(Argument::Positional { value }, self.range_since(&start));
        self.attach_documentation(argument_id, documentation);
        self.attach_decorators(argument_id.id, decorators);

        Ok(argument_id)
    }

    /// Parse dynamic arguments (including the `(` and `)` tokens) if they exist.
    pub(crate) fn parse_arguments_if_present(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<Option<Vec<LocalNodeId<Argument>>>> {
        if self.peek_is(TokenType::OpenParenthesis) {
            return Ok(Some(self.parse_argument_list(position)?));
        }
        Ok(None)
    }

    /// Parse a positional dynamic argument list, including its parentheses.
    pub(crate) fn parse_argument_list(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<Vec<LocalNodeId<Argument>>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty dynamic arguments
        if self.peek_is(TokenType::CloseParenthesis) {
            self.bump();
            return Ok(vec![]);
        }

        // regular dynamic arguments
        let arguments =
            self.parse_argument_list_body(TokenType::CloseParenthesis, position.nested())?;

        self.eat_list_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::Expression,
        );

        Ok(arguments)
    }

    /// Parse one positional argument list body.
    ///
    /// Examples:
    /// ```tspp
    /// first, second
    /// first
    /// second
    /// broken, recovered
    /// ```
    #[inline]
    fn parse_argument_list_body(
        &mut self,
        terminator: TokenType,
        position: ExpressionPosition,
    ) -> ParserResult<Vec<LocalNodeId<Argument>>> {
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<Argument>; 4]>::new();

        while self.has_more_tokens() {
            if self.peek_is(terminator) {
                break;
            }

            // release exported declarations from incomplete statement calls
            if position.is_in_statement()
                && self.peek_declaration_boundary(DeclarationNesting::Expression)
            {
                break;
            }

            // eat one argument
            let argument_start = self.mark_parse_start();
            let is_recovered_argument;
            let argument = self.parse_positional_argument(position);
            let argument_id = match argument {
                Ok(argument_id) => {
                    is_recovered_argument = self.has_recovered_argument_slot(argument_id);
                    argument_id
                }
                Err(error) => {
                    self.recover_list_item(self.range_since(&argument_start), terminator, error);
                    let argument_id =
                        self.insert_node(Argument::Error, self.range_since(&argument_start));
                    is_recovered_argument = true;
                    argument_id
                }
            };

            arguments.push(argument_id);

            // continue regular lists after a real separator
            if self.peek_is(TokenType::Comma) {
                self.eat_token(TokenType::Comma)?;

                if !is_recovered_argument {
                    continue;
                }

                if self.peek_recovered_argument_list_end(position) {
                    break;
                }

                if !self.peek_recovered_list_continuation(terminator) {
                    break;
                }

                continue;
            }
            // recovered statement calls should stop before the next newline led statement
            else if !is_recovered_argument
                || self.peek_recovered_argument_list_end(position)
                || !self.peek_recovered_list_continuation(terminator)
            {
                break;
            }
        }

        Ok(arguments.into_vec())
    }
}
