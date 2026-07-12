use crate::parse::context::{
    DecoratorContext, ExpressionContext, ExpressionMode, ExpressionStops, StatementPosition,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use destack_core::StringId;
use destack_dir::{
    Declaration, Expression, FunctionDeclaration, FunctionForm, GenericArgument, LocalNodeId,
    NodeType, Path, PostfixPosition, ScalarLiteral, TokenType, TypeExpression, UnaryOperator,
};
use destack_source::ByteRange;
use smallvec::smallvec;

/// One value expression head that can be promoted into static type space.
enum StaticTypeHead {
    /// One identifier expression.
    Identifier(StringId),
    /// One named member expression.
    Member {
        /// The member receiver.
        left: LocalNodeId<Expression>,
        /// The member name.
        name: StringId,
    },
    /// One generic instantiation expression.
    Instantiation {
        /// The instantiated expression.
        left: LocalNodeId<Expression>,
        /// The generic arguments.
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },
}

impl Parser {
    /// Return true when an expression is a lambda declaration without wrapping parentheses.
    fn is_unparenthesized_lambda_expression(&self, expression: LocalNodeId<Expression>) -> bool {
        matches!(
            self.tree.get(expression),
            Expression::Declaration(declaration)
                if matches!(
                    self.tree.get(*declaration),
                    Declaration::Function(FunctionDeclaration { signature, .. })
                        if signature.form == FunctionForm::Lambda
                )
        )
    }

    /// Return true when an expression can be used as an unparenthesized tagged template tag.
    fn is_valid_tagged_template_tag(&self, expression: LocalNodeId<Expression>) -> bool {
        !self.is_unparenthesized_lambda_expression(expression)
            && !matches!(self.tree.get(expression), Expression::Unary { .. })
    }

    /// Parse all postfix operations owned by one value expression.
    pub(in crate::parse::expression) fn parse_expression_postfix(
        &mut self,
        start: &ParseStart,
        mut left: LocalNodeId<Expression>,
        mut is_parenthesized: bool,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut is_first_postfix = true;
        let mut has_chain = false;
        loop {
            // stop before tokens that cannot continue one value operand
            let token_type = self.peek_token_type();
            let is_postfix_candidate = matches!(
                token_type,
                TokenType::OpenBrace
                    | TokenType::OpenParenthesis
                    | TokenType::OpenBracket
                    | TokenType::Dot
                    | TokenType::Maybe
                    | TokenType::Not
                    | TokenType::LessThan
                    | TokenType::ShiftLeft
                    | TokenType::TemplateString
                    | TokenType::TemplateStringStart
                    | TokenType::Increment
                    | TokenType::Decrement
            );
            if !is_postfix_candidate {
                break;
            }

            // classify newline ownership before dispatching the postfix
            let is_on_new_line = self.peek_is_on_new_line();
            let is_question_postfix =
                token_type == TokenType::Maybe && self.peek_question_postfix();
            let continues_postfix = token_type == TokenType::Dot || is_question_postfix;
            if is_on_new_line
                && !continues_postfix
                && (context.decorator != DecoratorContext::None
                    || context.stops.contains(ExpressionStops::MATCH_LINE)
                    || context.stops.contains(ExpressionStops::NEWLINE_CALL)
                    || context.statement == StatementPosition::Direct
                        && self.tree.get(left).ends_statement_on_newline())
            {
                break;
            }

            // parse one postfix operation without recursive descent
            let next = match token_type {
                TokenType::OpenBrace
                    if !is_on_new_line && !context.stops.contains(ExpressionStops::BODY_BRACE) =>
                {
                    self.parse_struct_postfix(start, left, context)?
                }
                TokenType::OpenParenthesis if context.mode != ExpressionMode::NewReceiver => {
                    if self.is_unparenthesized_lambda_expression(left) && !is_parenthesized {
                        return Err(ParserError::unexpected(self.peek_token_span()));
                    }
                    Some(self.parse_call(
                        left,
                        Vec::new(),
                        PostfixPosition::Direct,
                        context,
                        false,
                    )?)
                }
                TokenType::OpenBracket => {
                    Some(self.parse_index(left, PostfixPosition::Direct, context, false)?)
                }
                TokenType::Dot => {
                    Some(self.parse_dot_postfix(start, left, is_first_postfix, context, false)?)
                }
                // ?. makes the access it introduces optional
                TokenType::Maybe if self.peek_token_type_at(1) == TokenType::Dot => {
                    self.bump();
                    has_chain = true;

                    Some(self.parse_dot_postfix(start, left, is_first_postfix, context, true)?)
                }
                TokenType::Maybe if is_question_postfix => {
                    Some(self.parse_assertion_postfix(start, left, true, PostfixPosition::Direct))
                }
                TokenType::Not if !is_on_new_line => {
                    Some(self.parse_assertion_postfix(start, left, false, PostfixPosition::Direct))
                }
                TokenType::LessThan
                    if !matches!(
                        context.mode,
                        ExpressionMode::Tree | ExpressionMode::TypeofQuery
                    ) =>
                {
                    self.parse_generic_postfix(start, left, context, false)?
                }
                TokenType::ShiftLeft
                    if !matches!(
                        context.mode,
                        ExpressionMode::Tree | ExpressionMode::TypeofQuery
                    ) && self.peek_shift_left_generic_function_argument() =>
                {
                    self.parse_generic_postfix(start, left, context, false)?
                }
                TokenType::TemplateString | TokenType::TemplateStringStart
                    if self.is_valid_tagged_template_tag(left) =>
                {
                    Some(self.parse_tagged_template_postfix(start, left, Vec::new(), context)?)
                }
                _ if !is_on_new_line => UnaryOperator::from_postfix_token(token_type)
                    .map(|operator| self.parse_unary_postfix(start, left, operator)),
                _ => None,
            };
            let Some(next) = next else {
                break;
            };

            // continue from the newly wrapped expression
            left = next;
            is_first_postfix = false;
            is_parenthesized = false;
        }

        // a chain boundary reattaches the short-circuit arm
        if has_chain {
            left = self.insert_node(
                Expression::Chain { expression: left },
                self.range_since(start),
            );
        }

        Ok(left)
    }

    /// Parse one direct or indirect assertion postfix.
    fn parse_assertion_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<Expression>,
        is_maybe: bool,
        position: PostfixPosition,
    ) -> LocalNodeId<Expression> {
        let operator_range = self.peek_token().range();
        self.bump();
        let node = if is_maybe {
            Expression::Maybe { position, left }
        } else {
            Expression::Must { position, left }
        };
        let expression = self.insert_node(node, self.range_since(start));
        self.tree.set_main_range(expression, operator_range);

        expression
    }

    /// Parse one value unary postfix.
    fn parse_unary_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<Expression>,
        operator: UnaryOperator,
    ) -> LocalNodeId<Expression> {
        let operator_range = self.peek_token().range();
        self.bump();
        let expression = self.insert_node(
            Expression::Unary {
                operator,
                right: left,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(expression, operator_range);

        expression
    }

    /// Parse generic arguments as an instantiation or call postfix.
    fn parse_generic_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<Expression>,
        context: ExpressionContext,
        is_optional: bool,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if self.peek_is_on_new_line() || !self.peek_angle_group_expression_postfix() {
            return Ok(None);
        }

        let generic_arguments = self.parse_generic_argument_list(context)?;
        if self.peek_is(TokenType::OpenParenthesis) {
            return self
                .parse_call(
                    left,
                    generic_arguments,
                    PostfixPosition::Direct,
                    context,
                    is_optional,
                )
                .map(Some);
        }
        if matches!(
            self.peek_token_type(),
            TokenType::TemplateString | TokenType::TemplateStringStart
        ) {
            return self
                .parse_tagged_template_postfix(start, left, generic_arguments, context)
                .map(Some);
        }

        Ok(Some(self.insert_node(
            Expression::Instantiation {
                left,
                generic_arguments,
            },
            self.range_since(start),
        )))
    }

    /// Parse one tagged template continuation.
    fn parse_tagged_template_postfix(
        &mut self,
        start: &ParseStart,
        tag: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let value = self.parse_tagged_template_literal(context.function)?;

        Ok(self.insert_node(
            Expression::TaggedTemplateExpression {
                tag,
                generic_arguments,
                value,
            },
            self.range_since(start),
        ))
    }

    /// Parse one struct expression continuation when present.
    fn parse_struct_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<Expression>,
        context: ExpressionContext,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        let Some(ty) = self.promote_static_type_head(left) else {
            return Ok(None);
        };
        if !matches!(
            self.tree.get(ty),
            TypeExpression::Reference { .. }
                | TypeExpression::Member { .. }
                | TypeExpression::Function(_)
                | TypeExpression::Constructor(_)
        ) {
            return Err(ParserError::unexpected(self.tree.get_range(left)));
        }

        let properties = self.parse_object_literal(context.function)?;

        Ok(Some(self.insert_node(
            Expression::StructExpression { ty, properties },
            self.range_since(start),
        )))
    }

    /// Parse one dot member, indirect call, index, or assertion.
    fn parse_dot_postfix(
        &mut self,
        start: &ParseStart,
        left: LocalNodeId<Expression>,
        is_first_postfix: bool,
        context: ExpressionContext,
        is_optional: bool,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let dot_range = self.eat_token(TokenType::Dot)?.range();
        if is_first_postfix && self.is_decimal_integer_before_dot(left, dot_range) {
            return Err(ParserError::unexpected(dot_range));
        }

        // indirect call
        if self.peek_is(TokenType::OpenParenthesis) {
            return self.parse_call(
                left,
                Vec::new(),
                PostfixPosition::Indirect,
                context,
                is_optional,
            );
        }

        // indirect index
        if self.peek_is(TokenType::OpenBracket) {
            return self.parse_index(left, PostfixPosition::Indirect, context, is_optional);
        }

        // indirect assertion
        if self.peek_is(TokenType::Maybe) {
            return Ok(self.parse_assertion_postfix(start, left, true, PostfixPosition::Indirect));
        }
        if self.peek_is(TokenType::Not) {
            return Ok(self.parse_assertion_postfix(start, left, false, PostfixPosition::Indirect));
        }

        // indirect generic call or instantiation
        let is_generic_start =
            self.peek_is(TokenType::LessThan) || self.peek_shift_left_generic_function_argument();
        if is_generic_start
            && let Some(expression) =
                self.parse_generic_postfix(start, left, context, is_optional)?
        {
            return Ok(expression);
        }

        // recover a missing member name
        if !matches!(
            self.peek_token_type(),
            TokenType::Identifier | TokenType::Literal
        ) {
            self.report_unexpected_here(NodeType::Expression);

            return Ok(self.insert_node(
                Expression::Member {
                    left,
                    name: None,
                    is_optional,
                },
                self.range_since(start),
            ));
        }

        // named member
        let (name, name_range) = self.eat_member_name_with_range()?;
        let expression = self.insert_node(
            Expression::Member {
                left,
                name: Some(name),
                is_optional,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(expression, name_range);

        Ok(expression)
    }

    /// Create the static type head represented by a reference-shaped value.
    pub(in crate::parse::expression) fn promote_static_type_head(
        &mut self,
        expression: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<TypeExpression>> {
        // return an existing type expression without rebuilding it
        if let Expression::Type { value } = self.tree.get(expression) {
            return Some(*value);
        }

        // copy only fields from promotable value heads
        let head = match self.tree.get(expression) {
            Expression::Identifier { name } => StaticTypeHead::Identifier(*name),
            Expression::Member {
                left,
                name: Some(name),
                ..
            } => StaticTypeHead::Member {
                left: *left,
                name: *name,
            },
            Expression::Instantiation {
                left,
                generic_arguments,
            } => StaticTypeHead::Instantiation {
                left: *left,
                generic_arguments: generic_arguments.clone(),
            },
            _ => return None,
        };
        let range = self.tree.get_range(expression);

        // promote the complete head into type space
        match head {
            StaticTypeHead::Identifier(name) => Some(self.insert_node(
                TypeExpression::Reference {
                    path: Path {
                        segments: smallvec![name],
                    },
                    generic_arguments: Vec::new(),
                },
                range,
            )),
            StaticTypeHead::Member { left, name } => {
                let left = self.promote_static_type_head(left)?;
                let type_expression = match self.tree.get(left) {
                    TypeExpression::Reference {
                        path,
                        generic_arguments,
                    } if generic_arguments.is_empty() => {
                        let mut path = path.clone();
                        path.segments.push(name);

                        TypeExpression::Reference {
                            path,
                            generic_arguments: Vec::new(),
                        }
                    }
                    _ => TypeExpression::Member {
                        left,
                        name,
                        generic_arguments: Vec::new(),
                    },
                };

                Some(self.insert_node(type_expression, range))
            }
            StaticTypeHead::Instantiation {
                left,
                generic_arguments,
            } => self.promote_static_type_instantiation(range, left, generic_arguments),
        }
    }

    /// Create one instantiated static type head.
    fn promote_static_type_instantiation(
        &mut self,
        range: ByteRange,
        left: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    ) -> Option<LocalNodeId<TypeExpression>> {
        let ty = self.promote_static_type_head(left)?;
        let node = match self.tree.get(ty) {
            TypeExpression::Reference { path, .. } => TypeExpression::Reference {
                path: path.clone(),
                generic_arguments,
            },
            TypeExpression::Member { left, name, .. } => TypeExpression::Member {
                left: *left,
                name: *name,
                generic_arguments,
            },
            _ => return None,
        };

        Some(self.insert_node(node, range))
    }

    /// Return whether the current question mark touches its operand.
    fn peek_question_postfix(&self) -> bool {
        let question = self.peek_token().range();
        if self.peek_previous_token_end() == question.start {
            return true;
        }

        let next = self.peek_next_token();

        next.is(TokenType::Dot) && question.end == next.start()
    }

    /// Return whether decimal integer member access requires another dot.
    fn is_decimal_integer_before_dot(
        &self,
        left: LocalNodeId<Expression>,
        dot_range: ByteRange,
    ) -> bool {
        if !matches!(
            self.tree.get(left),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_))
        ) || self.tree.get_range(left).end != dot_range.start
        {
            return false;
        }

        let text = self.range_str(self.tree.get_range(left));
        !text.ends_with('n')
            && !matches!(text.get(..2), Some("0x" | "0X" | "0b" | "0B" | "0o" | "0O"))
    }
}
