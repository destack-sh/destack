use crate::parse::error::ParserResultExt;
use tspp_dir::{
    Expression, GenericArgument, GenericParameter, Keyword, LocalNodeId, NodeType, TokenType,
    TypeExpression, VarianceModifier,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::parse::expression::operator::ExpressionOperator;
use crate::parse::{ExpressionPosition, ExpressionStop, TokenMode, TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};

/// The closing-token policy of one generic argument list.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum GenericClose {
    /// Require one literal closing angle.
    Expression,
    /// Accept contextual type closing angles.
    Type,
}

impl Parser {
    /// Eat a generic argument close token or recover one missing `>`.
    fn eat_type_angle_close_or_recover_missing(&mut self, owner: NodeType) -> ParserResult<()> {
        if matches!(
            self.peek_token_type(),
            TokenType::GreaterThan
                | TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        ) {
            self.eat_type_angle_close()?;
            return Ok(());
        }

        if !self.peek_type_argument_close_recovery_boundary() {
            return Err(ParserError::expected(
                self.peek_token().range(),
                TokenType::GreaterThan,
            ));
        }

        self.report_expected_here(TokenType::GreaterThan, owner);

        Ok(())
    }

    /// Return whether type arguments may recover a missing closing angle here.
    fn peek_type_argument_close_recovery_boundary(&self) -> bool {
        let token_type = self.peek_token_type();

        // accept generic, expression, and source boundaries
        if token_type == TokenType::End
            || Self::is_type_angle_close_start(token_type)
            || matches!(token_type, TokenType::Colon | TokenType::ArrowWide)
            || self.peek_is_on_new_line()
        {
            return true;
        }

        // accept separators, postfix heads, and enclosing delimiters
        if matches!(
            token_type,
            TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Maybe
                | TokenType::TemplateString
                | TokenType::TemplateStringStart
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::Dot
        ) || Self::is_close_delimiter_token(token_type)
        {
            return true;
        }

        // accept declaration continuations after generic arguments
        if token_type == TokenType::OpenBrace
            || token_type == TokenType::Identifier
                && matches!(
                    self.peek_keyword(),
                    Some(Keyword::Implements | Keyword::With | Keyword::Where)
                )
        {
            return true;
        }

        // accept value operators that own the completed generic expression
        ExpressionOperator::from_token(token_type, self.peek_keyword()).is_some()
    }

    /// Parse one mixed generic argument node.
    fn parse_generic_argument(
        &mut self,
        start: &ParseStart,
    ) -> ParserResult<LocalNodeId<GenericArgument>> {
        let is_spread = self.peek_is(TokenType::Spread);
        if is_spread {
            self.eat_token(TokenType::Spread)?;
        }

        // spread call expressions are value packs
        if is_spread && self.peek_token_type_at(1) == TokenType::OpenParenthesis {
            return self.parse_static_value_argument(true, start);
        }

        // associated type refinement
        if !is_spread
            && self.peek_is_keyword(Keyword::Type)
            && self.peek_token_type_at(1) == TokenType::Identifier
            && self.peek_token_type_at(2) == TokenType::Assign
        {
            self.eat_keyword(Keyword::Type)?;
            let (name, name_range) = self.eat_identifier_with_range()?;
            self.eat_token(TokenType::Assign)?;
            let value = self.parse_type(TypePosition::Type, TypeStop::ANGLE_CLOSE)?;
            let argument = GenericArgument::AssociatedType { name, value };
            let argument_id = self.insert_node(argument, self.range_since(start));
            self.tree.set_main_range(argument_id, name_range);

            return Ok(argument_id);
        }

        // associated const refinement
        if !is_spread
            && self.peek_is_keyword(Keyword::Const)
            && self.peek_token_type_at(1) == TokenType::Identifier
            && self.peek_token_type_at(2) == TokenType::Assign
        {
            self.eat_keyword(Keyword::Const)?;
            let (name, name_range) = self.eat_identifier_with_range()?;
            self.eat_token(TokenType::Assign)?;
            let value = self.parse_type(TypePosition::Type, TypeStop::ANGLE_CLOSE)?;
            let argument = GenericArgument::AssociatedConst { name, value };
            let argument_id = self.insert_node(argument, self.range_since(start));
            self.tree.set_main_range(argument_id, name_range);

            return Ok(argument_id);
        }

        // explicit type-space argument
        if self.peek_is_keyword(Keyword::Type) {
            self.eat_keyword(Keyword::Type)?;
            let value = self.parse_type(TypePosition::Type, TypeStop::ANGLE_CLOSE)?;
            let argument = if is_spread {
                GenericArgument::SpreadType { value }
            } else {
                GenericArgument::Type { value }
            };

            return Ok(self.insert_node(argument, self.range_since(start)));
        }

        // classify the whole argument without speculative parsing
        if self.peek_generic_argument_type() {
            let value = self.parse_type(TypePosition::Type, TypeStop::ANGLE_CLOSE)?;
            let argument = if is_spread {
                GenericArgument::SpreadType { value }
            } else {
                GenericArgument::Type { value }
            };

            return Ok(self.insert_node(argument, self.range_since(start)));
        }

        // otherwise parse the argument in value space
        self.parse_static_value_argument(is_spread, start)
    }

    /// Parse one value-shaped generic argument as a static type argument.
    fn parse_static_value_argument(
        &mut self,
        is_spread: bool,
        start: &ParseStart,
    ) -> ParserResult<LocalNodeId<GenericArgument>> {
        let expression_start = self.mark_parse_start();
        let expression =
            self.parse_expression(ExpressionPosition::Value, ExpressionStop::ANGLE_CLOSE)?;
        let value = self.insert_static_value_type(expression, &expression_start);
        let argument = if is_spread {
            GenericArgument::SpreadType { value }
        } else {
            GenericArgument::Type { value }
        };

        Ok(self.insert_node(argument, self.range_since(start)))
    }

    /// Carry one parsed value term in type space.
    fn insert_static_value_type(
        &mut self,
        expression: LocalNodeId<Expression>,
        start: &ParseStart,
    ) -> LocalNodeId<TypeExpression> {
        self.insert_node(
            TypeExpression::StaticValue { expression },
            self.range_since(start),
        )
    }

    /// Parse one generic parameter in a generic parameter list.
    fn parse_generic_parameter(&mut self) -> ParserResult<LocalNodeId<GenericParameter>> {
        let documentation = self.parse_documentation();
        let start = self.mark_parse_start();
        // generic parameters accept only the dedicated generic modifiers
        let mut variance = None;
        let mut is_const = false;

        loop {
            if self.peek_is_keyword(Keyword::In) {
                self.bump();
                variance = Some(match variance {
                    Some(VarianceModifier::Out) => VarianceModifier::InOut,
                    Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                    _ => VarianceModifier::In,
                });
                continue;
            }

            let is_out_modifier = self.peek_is(TokenType::Identifier)
                && self.peek_identifier_is("out")
                && self.peek_token_type_at(1) == TokenType::Identifier;
            if is_out_modifier {
                self.bump();
                variance = Some(match variance {
                    Some(VarianceModifier::In) => VarianceModifier::InOut,
                    Some(VarianceModifier::InOut) => VarianceModifier::InOut,
                    _ => VarianceModifier::Out,
                });
                continue;
            }

            if self.peek_is_keyword(Keyword::Const) {
                self.bump();
                is_const = true;
                continue;
            }

            break;
        }

        let is_variadic = self.peek_is(TokenType::Spread);
        if is_variadic {
            self.eat_token(TokenType::Spread)?;
        }

        // declare tick names as lifetime parameters
        if !is_const && !is_variadic && self.peek_is(TokenType::Lifetime) {
            let range = self.peek_token().range();
            let name = self.intern_range(range);
            self.bump();
            let parameter_id = self.insert_node(GenericParameter::Lifetime { name }, range);
            self.tree.set_main_range(parameter_id, range);
            self.attach_documentation(parameter_id, documentation);

            return Ok(parameter_id);
        }

        // generic parameters are always named
        let (name, name_range) = if self.peek_is(TokenType::Lifetime) {
            let range = self.peek_token().range();
            let name = self.intern_range(range);
            self.bump();

            (name, range)
        } else {
            self.eat_binding_identifier_with_range()?
        };

        let has_annotation = self.peek_is(TokenType::Colon);

        let (declared_type, declared_type_range) = if has_annotation {
            let type_start = self.mark_parse_start();
            self.bump();
            let declared_type = if self.peek_is(TokenType::Assign)
                || self.peek_is(TokenType::Comma)
                || self.peek_type_angle_close()
                || self.peek_is(TokenType::End)
            {
                self.recover_missing_type_expression_here(NodeType::GenericParameter)
            } else {
                self.parse_type_or_recover_missing(
                    TypePosition::Type,
                    TypeStop::ANGLE_CLOSE,
                    NodeType::GenericParameter,
                )?
            };
            (Some(declared_type), Some(self.range_since(&type_start)))
        } else {
            (None, None)
        };

        // every generic parameter defaults in type space, whichever modifier it carries
        let default = if self.peek_is(TokenType::Assign) {
            self.bump();

            let default_start = self.mark_parse_start();
            // a closing delimiter leaves no default to classify
            let value = if self.peek_is(TokenType::Comma)
                || self.peek_type_angle_close()
                || self.peek_is(TokenType::End)
            {
                self.recover_missing_type_expression_here(NodeType::GenericParameter)
            }
            // classify the default by its own shape, like a generic argument
            else if self.peek_generic_argument_type() {
                self.parse_type_or_recover_missing(
                    TypePosition::Type,
                    TypeStop::ANGLE_CLOSE,
                    NodeType::GenericParameter,
                )?
            }
            // every other default parses in value space and wraps as a static value type
            else {
                let expression = self.parse_parameter_default(
                    ExpressionPosition::Value,
                    ExpressionStop::ANGLE_CLOSE,
                    NodeType::GenericParameter,
                )?;

                self.insert_static_value_type(expression, &default_start)
            };
            Some(value)
        } else {
            None
        };

        let parameter = if is_variadic {
            GenericParameter::VariadicType {
                name,
                is_const,
                variance,
                constraint: declared_type,
                default,
            }
        } else {
            GenericParameter::Type {
                name,
                is_const,
                variance,
                constraint: declared_type,
                default,
            }
        };

        let parameter_id = self.insert_node(parameter, self.range_since(&start));
        self.tree.set_main_range(parameter_id, name_range);

        if let Some(range) = declared_type_range {
            self.tree.set_side_range(
                parameter_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                range,
            );
        }
        self.attach_documentation(parameter_id, documentation);

        Ok(parameter_id)
    }

    /// Parse generic parameters, including the `<` and `>` tokens, if they exist.
    ///
    /// `is_empty_allowed` accepts `<>` as an empty parameter list.
    pub(crate) fn parse_generic_parameters_if_present(
        &mut self,
        is_empty_allowed: bool,
    ) -> ParserResult<Option<Vec<LocalNodeId<GenericParameter>>>> {
        if !self.peek_is(TokenType::LessThan) {
            return Ok(None);
        }

        Ok(Some(self.parse_generic_parameter_list(is_empty_allowed)?))
    }

    /// Parse generic parameters, including the `<` and `>` tokens.
    ///
    /// `is_empty_allowed` accepts `<>` as an empty parameter list.
    pub(crate) fn parse_generic_parameter_list(
        &mut self,
        is_empty_allowed: bool,
    ) -> ParserResult<Vec<LocalNodeId<GenericParameter>>> {
        let start = self.mark_parse_start();
        self.eat_token(TokenType::LessThan)?;

        // optionally accept empty generic parameters
        if self.peek_type_angle_close() {
            self.eat_type_angle_close()?;
            if is_empty_allowed {
                return Ok(vec![]);
            }
            return Err(ParserError::expected(
                self.range_since(&start),
                TokenType::Identifier,
            ));
        }

        // regular generic parameters
        let mut parameters = Vec::new();
        while self.has_more_tokens() {
            if self.peek_type_angle_close() {
                break;
            }

            let parameter = self
                .parse_generic_parameter()
                .in_node(NodeType::GenericParameter)?;
            parameters.push(parameter);

            if self.peek_type_angle_close() {
                break;
            }

            if self.peek_is(TokenType::Comma) {
                self.bump();
                continue;
            }

            break;
        }

        let container_start = self.range_since(&start).start;
        if let Some(first_parameter_id) = parameters.first().copied() {
            self.set_node_leading_range(first_parameter_id, container_start);
        }

        self.eat_type_angle_close_or_recover_missing(NodeType::Expression)?;
        Ok(parameters)
    }

    /// Parse type generic arguments, including the angle tokens.
    ///
    /// Examples:
    /// ```tspp
    /// <T>
    /// <K, V>
    /// <<T>() => T>
    /// ```
    pub(crate) fn parse_type_generic_arguments(
        &mut self,
    ) -> ParserResult<Vec<LocalNodeId<GenericArgument>>> {
        let start = self.mark_parse_start();

        self.eat_generic_angle_open()?;

        let first_argument_leading_start = self.peek_previous_token_end();
        let generic_arguments = if self.peek_type_angle_close() {
            vec![self.recover_empty_generic_argument(&start)]
        } else {
            self.parse_generic_argument_list_body(first_argument_leading_start)?
        };

        self.eat_type_angle_close_or_recover_missing(NodeType::Expression)?;

        Ok(generic_arguments)
    }

    /// Eat one generic argument opening angle and split `<<` when needed.
    fn eat_generic_angle_open(&mut self) -> ParserResult<GenericClose> {
        if self.peek_is(TokenType::LessThan) {
            self.bump_with_mode(TokenMode::Ordinary);

            return Ok(GenericClose::Expression);
        }

        if self.peek_is(TokenType::ShiftLeft) {
            if !self.re_lex_generic_angle_open() {
                return Err(ParserError::expected(
                    self.peek_token().range(),
                    TokenType::LessThan,
                ));
            }

            self.bump_with_mode(TokenMode::Ordinary);

            return Ok(GenericClose::Type);
        }

        Err(ParserError::expected(
            self.peek_token().range(),
            TokenType::LessThan,
        ))
    }

    /// Recover one empty generic argument list as an error argument.
    fn recover_empty_generic_argument(
        &mut self,
        start: &ParseStart,
    ) -> LocalNodeId<GenericArgument> {
        let error = ParserError::expected(self.range_since(start), TokenType::Identifier);
        self.report_error(error);

        let argument_start = self.mark_parse_start();

        self.insert_node(GenericArgument::Error, self.range_since(&argument_start))
    }

    /// Parse top-level generic arguments.
    #[inline]
    fn parse_generic_argument_list_body(
        &mut self,
        mut next_argument_leading_start: u32,
    ) -> ParserResult<Vec<LocalNodeId<GenericArgument>>> {
        let mut arguments = smallvec::SmallVec::<[LocalNodeId<GenericArgument>; 4]>::new();

        while self.has_more_tokens() {
            // stop on closing `>`
            if self.peek_type_angle_close() {
                break;
            }

            // parse one argument
            let documentation = self.parse_documentation();
            let argument_start = self.mark_parse_start();
            let mut is_recovered_argument = false;
            let argument_id = match self.parse_generic_argument(&argument_start) {
                Ok(argument) => argument,
                Err(error) => {
                    is_recovered_argument = true;
                    self.recover_list_item(
                        self.range_since(&argument_start),
                        TokenType::GreaterThan,
                        error,
                    );

                    self.insert_node(GenericArgument::Error, self.range_since(&argument_start))
                }
            };
            self.set_node_leading_range(argument_id, next_argument_leading_start);
            if !matches!(self.tree.get(argument_id), GenericArgument::Error) {
                self.attach_documentation(argument_id, documentation);
            }

            arguments.push(argument_id);

            // continue through separators
            if self.peek_is(TokenType::Comma) {
                self.bump();
                next_argument_leading_start = self.peek_previous_token_end();
            }
            // let recovered arguments continue across newline separators only
            else if !is_recovered_argument
                || !self.peek_recovered_list_continuation(TokenType::GreaterThan)
            {
                break;
            }
        }

        Ok(arguments.into_vec())
    }

    /// Parse type or value generic arguments, including their angle tokens.
    pub(crate) fn parse_generic_argument_list(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<Vec<LocalNodeId<GenericArgument>>> {
        let start = self.mark_parse_start();
        let generic_close = self.eat_generic_angle_open()?;
        let first_argument_leading_start = self.peek_previous_token_end();

        let is_decorator = position.is_decorator();
        let uses_type_close = is_decorator || generic_close == GenericClose::Type;
        let generic_arguments = if is_decorator {
            if self.peek_type_angle_close() {
                vec![self.recover_empty_generic_argument(&start)]
            } else {
                self.parse_generic_argument_list_body(first_argument_leading_start)?
            }
        } else if self.peek_expression_type_angle_close() {
            return Err(ParserError::expected(
                self.range_since(&start),
                TokenType::Identifier,
            ));
        } else {
            self.parse_generic_argument_list_body(first_argument_leading_start)?
        };

        // type closes can consume glued right angle tails
        if uses_type_close {
            if is_decorator {
                self.eat_type_angle_close_or_recover_missing(NodeType::Expression)?;
            } else {
                self.eat_type_angle_close()?;
            }
        } else {
            self.eat_expression_type_angle_close()?;
        }
        Ok(generic_arguments)
    }
}
