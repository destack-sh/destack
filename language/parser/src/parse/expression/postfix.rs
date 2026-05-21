use crate::parse::scope::ExpressionScope;
use crate::parse::r#type::operator::TypeUnaryOperator;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};
use destack_dir::{
    Expression, LocalNodeId, NodeType, Path, PostfixPosition, ScalarLiteral, TokenType,
    TypeExpression, UnaryOperator,
};
use smallvec::smallvec;

impl Parser {
    /// Eat value postfix operators.
    ///
    /// Examples:
    /// ```ds
    /// value.member(argument)?
    /// call<T>(argument)
    /// value[index]!.member
    /// ```
    pub(in crate::parse::expression) fn eat_postfix(
        &mut self,
        start: &ParserSpanStart,
        mut left: LocalNodeId<Expression>,
        mut is_parenthesized: bool,
        scope: ExpressionScope,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        loop {
            let token_type = self.peek_token_type();
            let is_on_new_line = self.current_token_is_on_new_line();

            // decorator target boundary
            if scope.owns_decorator_line_boundary && is_on_new_line {
                break;
            }

            // statement boundary
            if scope.is_match_case_body && is_on_new_line {
                break;
            }

            // token dispatch
            let next = match token_type {
                TokenType::OpenBrace => self.eat_tagged_object_postfix(start, left, scope)?,
                TokenType::OpenParenthesis => {
                    self.eat_call_postfix(left, is_parenthesized, scope, is_on_new_line)?
                }
                TokenType::OpenBracket => Some(self.eat_index(left, PostfixPosition::Direct)?),
                TokenType::Dot => Some(self.eat_value_dot_postfix(start, left)?),
                TokenType::Maybe if self.current_question_starts_maybe_postfix() => {
                    Some(self.eat_maybe_postfix(start, left, PostfixPosition::Direct)?)
                }
                TokenType::Not if !is_on_new_line => {
                    Some(self.eat_must_postfix(start, left, PostfixPosition::Direct)?)
                }
                TokenType::Identifier
                    if self.peek_type_unary_postfix_operator_maybe()
                        == Some(TypeUnaryOperator::AsComptime) =>
                {
                    Some(self.eat_comptime_postfix(start, left))
                }
                TokenType::LessThan | TokenType::ShiftLeft => {
                    self.eat_generic_postfix(start, left, scope)?
                }
                TokenType::TemplateString | TokenType::TemplateStringStart
                    if self.tagged_template_tag_is_valid(left) =>
                {
                    Some(self.eat_tagged_template_postfix(start, left)?)
                }
                _ if !is_on_new_line => self.eat_current_unary_postfix(start, left),
                _ => None,
            };

            let Some(expression_id) = next else {
                break;
            };

            left = expression_id;
            is_parenthesized = false;
        }

        Ok((left, is_parenthesized))
    }

    /// Eat a direct call postfix when this expression owns it.
    ///
    /// Examples:
    /// ```ds
    /// value()
    /// value(argument)
    /// value<T>(argument)
    /// ```
    fn eat_call_postfix(
        &mut self,
        left: LocalNodeId<Expression>,
        is_parenthesized: bool,
        scope: ExpressionScope,
        is_on_new_line: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if self.call_belongs_to_outer_scope(left, scope, is_on_new_line) {
            return Ok(None);
        }

        if self.is_unparenthesized_lambda_expression(left) && !is_parenthesized {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        self.eat_call(left, Vec::new().into(), PostfixPosition::Direct)
            .map(Some)
    }

    /// Return whether a direct call is owned by an outer parser.
    fn call_belongs_to_outer_scope(
        &self,
        left: LocalNodeId<Expression>,
        scope: ExpressionScope,
        is_on_new_line: bool,
    ) -> bool {
        if scope.is_new_receiver {
            return true;
        }

        if scope.owns_newline_call_boundary && is_on_new_line {
            return true;
        }

        is_on_new_line
            && (self.tree.get(left).ends_statement_on_newline()
                || !self.current_token_is_on_new_line())
    }

    /// Parse one maybe postfix.
    ///
    /// Examples:
    /// ```ds
    /// value?
    /// value.?
    /// value?[index]
    /// ```
    fn eat_maybe_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        position: PostfixPosition,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_span = self.peek()?.span;
        self.bump();
        let expression_id = self.insert_node(
            Expression::Maybe { position, left },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Parse one must postfix.
    ///
    /// Examples:
    /// ```ds
    /// value!
    /// value.!
    /// call()!
    /// ```
    fn eat_must_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        position: PostfixPosition,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_span = self.peek()?.span;
        self.bump();
        let expression_id = self.insert_node(
            Expression::Must { position, left },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }

    /// Parse one comptime postfix.
    ///
    /// Examples:
    /// ```ds
    /// value as comptime
    /// call() as comptime
    /// Namespace.value as comptime
    /// ```
    fn eat_comptime_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let operator_start = self.span_start();
        self.bump();
        self.bump();
        let expression_id = self.insert_node(
            Expression::Comptime { body: left },
            self.get_span_from(start),
        );
        self.tree
            .set_main_span(expression_id, self.get_span_from(&operator_start));

        expression_id
    }

    /// Parse one unary postfix.
    ///
    /// Examples:
    /// ```ds
    /// value++
    /// value--
    /// object.field++
    /// ```
    fn eat_unary_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        operator: UnaryOperator,
    ) -> LocalNodeId<Expression> {
        let operator_start = self.span_start();
        self.bump();
        let expression_id = self.insert_node(
            Expression::Unary {
                operator,
                right: left,
            },
            self.get_span_from(start),
        );
        self.tree
            .set_main_span(expression_id, self.get_span_from(&operator_start));

        expression_id
    }

    /// Eat a value unary postfix when present.
    ///
    /// Examples:
    /// ```ds
    /// value++
    /// value--
    /// object.field++
    /// ```
    fn eat_current_unary_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<Expression>> {
        let Some(operator) = UnaryOperator::from_postfix_token(self.peek_token_type()) else {
            return None;
        };

        Some(self.eat_unary_postfix(start, left, operator))
    }

    /// Parse value generic postfix syntax when present.
    ///
    /// Examples:
    /// ```ds
    /// value<T>
    /// value<T>(argument)
    /// value<<T>() => T>
    /// ```
    fn eat_generic_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        scope: ExpressionScope,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if self.generic_belongs_to_outer_scope(scope)? {
            return Ok(None);
        }

        let checkpoint = self.checkpoint();
        let mark = self.tree.next_id();
        let has_leading_gap = self.generic_arguments_have_leading_gap(left);
        let Some(generic_arguments) = self.eat_generic_arguments_if_valid(true) else {
            return Ok(None);
        };

        if has_leading_gap && !self.spaced_generic_arguments_have_postfix_anchor() {
            self.restore(checkpoint, mark);
            return Ok(None);
        }

        if self.peek_is(TokenType::OpenParenthesis) {
            let call = self.eat_call(left, Some(generic_arguments), PostfixPosition::Direct)?;

            return Ok(Some(call));
        }

        Ok(Some(self.insert_node(
            Expression::Instantiation {
                left,
                generic_arguments,
            },
            self.get_span_from(start),
        )))
    }

    /// Return whether value generic postfix is owned by an outer parser.
    fn generic_belongs_to_outer_scope(&mut self, scope: ExpressionScope) -> ParseResult<bool> {
        if self.peek_is(TokenType::ShiftLeft) && !self.shift_left_can_start_generic_arguments() {
            return Ok(true);
        }

        if scope.is_new_receiver || scope.is_tree_literal || scope.is_typeof_query {
            return Ok(true);
        }

        if self.current_token_is_on_new_line() && self.can_start_tree_literal() {
            if scope.is_statement_position {
                return Ok(true);
            }

            return Err(ParseError::unexpected(self.peek()?.span));
        }

        Ok(false)
    }

    /// Parse tagged template postfix.
    ///
    /// Examples:
    /// ```ds
    /// tag`value`
    /// tag`hello ${name}`
    /// namespace.tag`value`
    /// ```
    fn eat_tagged_template_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let value = self.eat_tagged_template_literal()?;

        Ok(self.insert_node(
            Expression::TaggedTemplateExpression {
                tag: left,
                generic_arguments: Vec::new(),
                value,
            },
            self.get_span_from(start),
        ))
    }

    /// Return whether generic arguments are separated from their receiver.
    fn generic_arguments_have_leading_gap(&mut self, left: LocalNodeId<Expression>) -> bool {
        self.tree.get_span(left).end < self.anchor_span_here().start
    }

    /// Return whether spaced generic arguments have a strong postfix follow.
    fn spaced_generic_arguments_have_postfix_anchor(&mut self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::Dot
                | TokenType::Maybe
                | TokenType::TemplateString
                | TokenType::TemplateStringStart
        )
    }

    /// Return whether `<<` can be a generic argument start.
    fn shift_left_can_start_generic_arguments(&mut self) -> bool {
        self.lookahead(|parser| parser.scan_shift_left_can_start_generic_arguments())
    }

    /// Scan whether `<<` can be a generic argument start.
    fn scan_shift_left_can_start_generic_arguments(&mut self) -> bool {
        self.bump();
        let mut angle_depth = 1usize;

        loop {
            let token_type = self.peek_token_type();

            // statement and expression boundaries make this a shift operator
            if matches!(
                token_type,
                TokenType::End
                    | TokenType::Comma
                    | TokenType::Semicolon
                    | TokenType::CloseParenthesis
                    | TokenType::CloseBracket
                    | TokenType::CloseBrace
            ) {
                return false;
            }

            // the inner generic head must close before a function shaped type
            if token_type == TokenType::GreaterThan && angle_depth == 1 {
                self.bump();

                return matches!(
                    self.peek_token_type(),
                    TokenType::OpenParenthesis | TokenType::ArrowWide
                );
            }

            match token_type {
                TokenType::LessThan => angle_depth += 1,
                TokenType::ShiftLeft => angle_depth += 2,
                TokenType::GreaterThan => {
                    if angle_depth == 0 {
                        return false;
                    }

                    angle_depth -= 1;
                }
                TokenType::ShiftRight => {
                    if angle_depth < 2 {
                        return false;
                    }

                    angle_depth -= 2;
                }
                TokenType::UnsignedShiftRight => {
                    if angle_depth < 3 {
                        return false;
                    }

                    angle_depth -= 3;
                }
                _ => {}
            }

            self.bump();
        }
    }

    /// Return whether `?` starts a postfix expression here.
    fn current_question_starts_maybe_postfix(&mut self) -> bool {
        if self.is_optional_chain_after_question_mark() {
            return true;
        }

        // plain postfix maybe is Destack syntax, TS keeps `?` for ternaries
        if !self.language.is_destack() {
            return false;
        }

        if self.current_token_is_on_new_line() {
            return false;
        }

        self.question_can_end_postfix()
    }

    /// Parse a Destack tagged object literal postfix when present.
    ///
    /// Examples:
    /// ```ds
    /// Type { field: value }
    /// Namespace.Type { field: value }
    /// Type<T> { field: value }
    /// ```
    fn eat_tagged_object_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        scope: ExpressionScope,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !self.language.is_destack()
            || !self.peek_is(TokenType::OpenBrace)
            || self.current_token_is_on_new_line()
        {
            return Ok(None);
        }

        // let heritage and for each recovery own the following block
        if self.flags.is_in_super_type() || scope.owns_for_each_boundary {
            return Ok(None);
        }

        let left = self.without_parentheses_expression(left);
        let Some(ty) = self.static_type_head_from_expression(left) else {
            return Ok(None);
        };

        if !self.can_start_tagged_object_literal_type(ty) {
            return Err(ParseError::unexpected(self.tree.get_span(left)));
        }

        let properties = self.with_flags(self.flags.not_in_position(), |parser| {
            parser.eat_object_literal()
        })?;
        let expression_id = self.insert_node(
            Expression::StructExpression { ty, properties },
            self.get_span_from(start),
        );

        Ok(Some(expression_id))
    }

    /// Build the static type head represented by reference-shaped value syntax.
    ///
    /// Examples:
    /// ```ds
    /// Type
    /// Namespace.Type
    /// Type<T>
    /// ```
    pub(in crate::parse::expression) fn static_type_head_from_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<TypeExpression>> {
        match self.tree.get(expression_id) {
            Expression::Type { value } => Some(*value),
            Expression::Identifier { name } => {
                let path = Path {
                    segments: smallvec![*name],
                };
                let ty = TypeExpression::Reference {
                    path,
                    generic_arguments: Vec::new(),
                };

                Some(self.insert_node(ty, self.tree.get_span(expression_id)))
            }
            Expression::QualifiedReference {
                path,
                generic_arguments,
            } => {
                let ty = TypeExpression::Reference {
                    path: path.clone(),
                    generic_arguments: generic_arguments.clone(),
                };

                Some(self.insert_node(ty, self.tree.get_span(expression_id)))
            }
            Expression::Member {
                left,
                name: Some(name),
            } => {
                let left = *left;
                let name = *name;
                let left = self.without_parentheses_expression(left);
                let ty = self.static_type_head_from_expression(left)?;
                let ty = self.static_type_head_with_member(ty, name);

                Some(self.insert_node(ty, self.tree.get_span(expression_id)))
            }
            Expression::Instantiation {
                left,
                generic_arguments,
            } => self.static_type_head_from_instantiation(
                expression_id,
                *left,
                generic_arguments.clone(),
            ),
            _ => None,
        }
    }

    /// Return a static type head from a generic instantiation expression.
    fn static_type_head_from_instantiation(
        &mut self,
        expression: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<destack_dir::GenericArgument>>,
    ) -> Option<LocalNodeId<TypeExpression>> {
        let left = self.without_parentheses_expression(left);
        let ty = self.static_type_head_from_expression(left)?;

        match self.tree.get(ty) {
            TypeExpression::Reference { path, .. } => {
                let ty = TypeExpression::Reference {
                    path: path.clone(),
                    generic_arguments,
                };

                Some(self.insert_node(ty, self.tree.get_span(expression)))
            }
            TypeExpression::Member { left, name, .. } => {
                let ty = TypeExpression::Member {
                    left: *left,
                    name: *name,
                    generic_arguments,
                };

                Some(self.insert_node(ty, self.tree.get_span(expression)))
            }
            _ => None,
        }
    }

    /// Add one member segment to a static type head.
    fn static_type_head_with_member(
        &self,
        ty: LocalNodeId<TypeExpression>,
        name: destack_core::StringId,
    ) -> TypeExpression {
        match self.tree.get(ty) {
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
                left: ty,
                name,
                generic_arguments: Vec::new(),
            },
        }
    }

    /// Return whether `?` can finish one postfix expression here.
    fn question_can_end_postfix(&mut self) -> bool {
        if self.current_token_is_on_new_line() {
            return false;
        }

        if self.next_token_type() == TokenType::OpenParenthesis {
            return false;
        }

        if self.is_next_any_stop() || self.is_next_any_close_parenthesis() {
            return true;
        }

        if matches!(
            self.next_token_type(),
            TokenType::Dot | TokenType::OpenBracket
        ) {
            return !self.next_bracket_is_conditional_branch();
        }

        if self.next_token_type() == TokenType::LessThan
            && self.allow_tree_literals()
            && self.is_tree_literal_start_at_offset(1)
        {
            return false;
        }

        self.peek_infix_operator_at_offset_maybe(1).is_some()
    }

    /// Return whether `?[...]` starts a ternary then branch.
    fn next_bracket_is_conditional_branch(&mut self) -> bool {
        if self.next_token_type() != TokenType::OpenBracket {
            return false;
        }

        self.lookahead(|parser| {
            parser.scan_bracket_follow_token_at_offset(1) == Some(TokenType::Colon)
        })
    }

    /// Parse a value dot postfix.
    ///
    /// Examples:
    /// ```ds
    /// value.member
    /// value.(argument)
    /// value.[index]
    /// ```
    fn eat_value_dot_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let dot_span = self.eat_token(TokenType::Dot)?.span;
        if self.expression_is_decimal_integer_before_dot(left, dot_span) {
            return Err(ParseError::unexpected(dot_span));
        }

        // indirect call and index
        if self.peek_is(TokenType::OpenParenthesis) {
            return self.eat_call(left, None, PostfixPosition::Indirect);
        }

        if self.peek_is(TokenType::OpenBracket) {
            return self.eat_index(left, PostfixPosition::Indirect);
        }

        // indirect assertions
        if self.peek_is(TokenType::Maybe) {
            return self.eat_maybe_postfix(start, left, PostfixPosition::Indirect);
        }

        if self.peek_is(TokenType::Not) {
            return self.eat_must_postfix(start, left, PostfixPosition::Indirect);
        }

        // indirect generic postfix
        if matches!(
            self.peek_token_type(),
            TokenType::LessThan | TokenType::ShiftLeft
        ) {
            return self.eat_indirect_generic_postfix(start, left);
        }

        // private member
        if self.peek_is(TokenType::Hash) {
            return self.eat_private_member_postfix(start, left);
        }

        // named or missing member
        self.eat_named_member_postfix(start, left)
    }

    /// Parse an indirect generic postfix.
    ///
    /// Examples:
    /// ```ds
    /// value.<T>
    /// value.<T>(argument)
    /// value.<<T>() => T>
    /// ```
    fn eat_indirect_generic_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let Some(generic_arguments) = self.eat_generic_arguments_if_valid(true) else {
            return Err(ParseError::unexpected(self.peek()?.span));
        };

        if self.peek_is(TokenType::OpenParenthesis) {
            return self.eat_call(left, Some(generic_arguments), PostfixPosition::Indirect);
        }

        Ok(self.insert_node(
            Expression::Instantiation {
                left,
                generic_arguments,
            },
            self.get_span_from(start),
        ))
    }

    /// Parse a private member postfix.
    ///
    /// Examples:
    /// ```ds
    /// value.#member
    /// this.#member
    /// value.#method()
    /// ```
    fn eat_private_member_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let hash_span = self.peek()?.span;
        self.bump();

        let name = if self.peek_is(TokenType::Identifier) {
            Some(self.eat_identifier()?)
        } else {
            self.report_unexpected_for_here(NodeType::Expression);
            None
        };

        let expression = self.insert_node(
            Expression::PrivateMember { left, name },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression, hash_span);

        Ok(expression)
    }

    /// Parse a named member or recover a missing member name.
    ///
    /// Examples:
    /// ```ds
    /// value.member
    /// value.default
    /// value.true
    /// ```
    fn eat_named_member_postfix(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if !self.current_token_starts_member_name() {
            self.report_unexpected_for_here(NodeType::Expression);
            let expression = self.insert_node(
                Expression::Member { left, name: None },
                self.get_span_from(start),
            );

            return Ok(expression);
        }

        let (name, name_span) = self.eat_member_name_with_span()?;
        let expression = self.insert_node(
            Expression::Member {
                left,
                name: Some(name),
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression, name_span);

        Ok(expression)
    }

    /// Return whether the current token starts a member name.
    fn current_token_starts_member_name(&mut self) -> bool {
        self.peek_is(TokenType::Identifier) || self.peek_is(TokenType::Literal)
    }

    /// Return true when a decimal integer needs a separator before member access.
    fn expression_is_decimal_integer_before_dot(
        &self,
        left: LocalNodeId<Expression>,
        dot_span: destack_source::Span,
    ) -> bool {
        if !matches!(
            self.tree.get(left),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_))
        ) {
            return false;
        }

        let left_span = self.tree.get_span(left);
        if left_span.end != dot_span.start {
            return false;
        }

        let source_text = self.get_span_str(left_span);
        !source_text.ends_with('n')
            && !source_text.starts_with("0x")
            && !source_text.starts_with("0X")
            && !source_text.starts_with("0b")
            && !source_text.starts_with("0B")
            && !source_text.starts_with("0o")
            && !source_text.starts_with("0O")
    }
}
