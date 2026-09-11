use crate::parse::expression::operator::ExpressionOperator;
use crate::parse::r#type::operator::TypePrefixOperator;
use crate::parse::{
    AwaitKeyword, DeclarationHeader, ExpressionPosition, ExpressionStop, TypePosition, TypeStop,
    YieldKeyword,
};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use destack_core::StringId;
use destack_dir::{
    BlockContext, Exclusivity, Expression, InferForm, Keyword, Literal, LocalNodeId, Mutability,
    NodeType, OperatorPrecedence, Path, RangeEnd, TokenLiteral, TokenType, TypeExpression,
    UnaryOperator, VarianceBound,
};
use destack_source::ByteRange;
use smallvec::{SmallVec, smallvec};

/// One consumed value prefix operation.
#[derive(Debug, Copy, Clone)]
enum ValuePrefix {
    /// One ordinary unary prefix.
    Unary {
        /// The unary operation.
        operator: UnaryOperator,
        /// The operator source range.
        range: ByteRange,
    },
    /// One borrow prefix.
    Borrow {
        /// The mutability modifier.
        mutability: Option<Mutability>,
        /// The exclusion modifier.
        exclusivity: Option<Exclusivity>,
        /// The variance modifier.
        variance: Option<VarianceBound>,
        /// The operator source range.
        range: ByteRange,
    },
}

impl Parser {
    /// Parse one identifier value without postfix operations.
    pub(crate) fn parse_identifier_expression(
        &mut self,
        start: &ParseStart,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let (name, range) = self.eat_identifier_with_range()?;

        Ok(self.insert_identifier_expression(start, name, range))
    }

    /// Parse one value operand through all prefix and postfix operations.
    #[inline(never)]
    pub(in crate::parse::expression) fn parse_expression_operand(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // parse prefixes only when the operand actually has one
        let prefixes = if let Some(first) = self.parse_value_prefix()? {
            let mut prefixes: SmallVec<[ValuePrefix; 4]> = smallvec![first];
            while let Some(prefix) = self.parse_value_prefix()? {
                prefixes.push(prefix);
            }

            Some(prefixes)
        } else {
            None
        };

        // parse every postfix around the primary expression
        let primary_start = self.mark_parse_start();
        let (expression, is_parenthesized) =
            self.parse_expression_primary(&primary_start, position, stop)?;
        let mut expression = self.parse_expression_postfix(
            &primary_start,
            expression,
            is_parenthesized,
            position,
            stop,
        )?;

        // fold consumed prefixes from the operand outward
        if let Some(prefixes) = prefixes {
            for prefix in prefixes.into_iter().rev() {
                expression = self.insert_value_prefix(prefix, expression);
            }
        }

        Ok(expression)
    }

    /// Parse one ordinary or reference value prefix when present.
    fn parse_value_prefix(&mut self) -> ParserResult<Option<ValuePrefix>> {
        // parse one ordinary unary prefix
        if let Some(operator) = self.peek_value_prefix_operator() {
            let range = self.peek_token().range();
            self.bump();

            return Ok(Some(ValuePrefix::Unary { operator, range }));
        }

        // leave non-borrow tokens to the primary expression
        if !matches!(
            self.peek_token_type(),
            TokenType::ElementwiseAnd | TokenType::LogicalAnd
        ) {
            return Ok(None);
        }

        // parse one borrow prefix
        let token = self.eat_reference_prefix_operator()?.token;
        let (mutability, exclusivity) = self.parse_borrow_qualifiers()?;
        let variance = self.parse_variance_bound_if_present();

        Ok(Some(ValuePrefix::Borrow {
            mutability,
            exclusivity,
            variance,
            range: token.range(),
        }))
    }

    /// Fold one consumed value prefix around its operand.
    fn insert_value_prefix(
        &mut self,
        prefix: ValuePrefix,
        right: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let (expression, operator_range) = match prefix {
            ValuePrefix::Unary { operator, range } => {
                (Expression::Unary { operator, right }, range)
            }
            ValuePrefix::Borrow {
                mutability,
                exclusivity,
                variance,
                range,
            } => (
                Expression::BorrowOf {
                    mutability,
                    exclusivity,
                    variance,
                    right,
                },
                range,
            ),
        };
        let right_range = self.tree.get_range(right);
        let source_range = ByteRange {
            start: operator_range.start,
            end: right_range.end,
        };
        let expression = self.insert_node(expression, source_range);
        self.tree.set_main_range(expression, operator_range);

        expression
    }

    /// Parse one primary expression without prefix or postfix operations.
    fn parse_expression_primary(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        let token = self.peek_token_type();

        // dispatch identifiers without repeating keyword classification
        if token == TokenType::Identifier {
            let expression = if let Some(keyword) = self.peek_keyword() {
                self.parse_keyword_primary(start, keyword, position, stop)?
            } else {
                self.parse_identifier_primary(start, position, stop)?
            };

            return Ok((expression, false));
        }

        self.parse_token_primary(start, token, position, stop)
    }

    /// Parse one non-keyword identifier primary.
    fn parse_identifier_primary(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if self.peek_identifier_is("delete") {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        let (name, name_range) = self.eat_identifier_with_range()?;

        // classify an identifier-shaped expression from one consumed head
        match self.peek_token_type() {
            // an applied hole heads an inferred call
            TokenType::OpenParenthesis if self.range_str(name_range) == "_" => {
                let expression = self.insert_node(
                    Expression::Infer {
                        form: InferForm::Hole,
                        name: None,
                    },
                    name_range,
                );

                Ok(expression)
            }
            TokenType::ArrowWide => {
                let declaration =
                    self.parse_bare_lambda(start, name, name_range, DeclarationHeader::default())?;

                Ok(self.insert_declaration_expression(start, declaration))
            }
            TokenType::Colon if self.peek_label_body(stop) => {
                let body = self.parse_label_body()?;

                // attach the label to its loop, the only valid target
                if let Expression::While { label, .. }
                | Expression::ForEach { label, .. }
                | Expression::For { label, .. }
                | Expression::Loop { label, .. } = self.tree.get_mut(body)
                {
                    *label = Some(name);
                }
                self.tree.set_range(body, self.range_since(start));
                self.tree.set_main_range(body, name_range);

                Ok(body)
            }
            TokenType::OpenBrace => {
                self.parse_identifier_object_primary(start, name, name_range, position, stop)
            }
            _ => Ok(self.insert_identifier_expression(start, name, name_range)),
        }
    }

    /// Parse one identifier followed by a brace body.
    fn parse_identifier_object_primary(
        &mut self,
        start: &ParseStart,
        name: StringId,
        name_range: ByteRange,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let name_text = self.strings.get(name);

        // parse module { body }
        if name_text == "module" {
            let declaration = self.parse_module(start, name_range)?;

            return Ok(self.insert_declaration_expression(start, declaration));
        }

        // parse global { body }
        if name_text == "global" {
            let header = DeclarationHeader {
                is_ambient: self.is_ambient,
                ..DeclarationHeader::default()
            };
            let declaration = self.parse_global(start, name_range, header)?;

            return Ok(self.insert_declaration_expression(start, declaration));
        }

        // leave an identifier followed by an enclosing control body
        if position == ExpressionPosition::Block
            || stop.has(ExpressionStop::BODY_BRACE)
            || self.peek_is_on_new_line()
        {
            return Ok(self.insert_identifier_expression(start, name, name_range));
        }

        // promote the identifier once before parsing its object value
        let ty = if name_text == "_" {
            TypeExpression::Infer {
                form: InferForm::Hole,
                name: None,
                constraint: None,
            }
        } else {
            TypeExpression::Reference {
                path: Path {
                    segments: smallvec![name],
                },
                generic_arguments: Vec::new(),
            }
        };
        let ty = self.insert_node(ty, name_range);
        self.tree.set_main_range(ty, name_range);
        let properties = self.parse_object_literal()?;

        Ok(self.insert_node(
            Expression::StructExpression { ty, properties },
            self.range_since(start),
        ))
    }

    /// Parse one keyword primary.
    fn parse_keyword_primary(
        &mut self,
        start: &ParseStart,
        keyword: Keyword,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // parse @keyword as an identifier decorator head except for this and super
        if position == ExpressionPosition::DecoratorHead
            && !matches!(keyword, Keyword::This | Keyword::Super)
        {
            return self.parse_identifier_expression(start);
        }

        // parse declaration keyword ...
        if self.peek_declaration_primary(position) {
            return self.parse_declaration_primary(start, position);
        }

        // parse type declarations and first-class structural type values
        if self.peek_type_keyword_value(keyword, stop) {
            let value = self.parse_type_declaration(start, TypeStop::default())?;

            return Ok(self.insert_type_expression_value(value));
        }

        // parse keyof T, readonly T, local T, or shared T as a value
        if TypePrefixOperator::from_token(TokenType::Identifier, Some(keyword)).is_some()
            && self.peek_type_operand_start_at(1)
        {
            let value = self.parse_type(TypePosition::Type, TypeStop::default())?;

            return Ok(self.insert_type_expression_value(value));
        }

        match keyword {
            Keyword::Function | Keyword::Typeof | Keyword::Void => {
                Err(ParserError::unexpected(self.peek_token_span()))
            }
            Keyword::This => {
                let keyword_range = self.peek_token().range();
                self.bump();
                let expression = self.insert_node(Expression::This, self.range_since(start));
                self.tree.set_main_range(expression, keyword_range);

                Ok(expression)
            }
            Keyword::Super => {
                let keyword_range = self.peek_token().range();
                self.bump();
                let expression = self.insert_node(Expression::Super, self.range_since(start));
                self.tree.set_main_range(expression, keyword_range);

                Ok(expression)
            }
            Keyword::Null => {
                self.bump();
                Ok(self.insert_node(Expression::Literal(Literal::Null), self.range_since(start)))
            }
            Keyword::Undefined => {
                self.bump();
                Ok(self.insert_node(
                    Expression::Literal(Literal::Undefined),
                    self.range_since(start),
                ))
            }
            Keyword::Do if self.peek_do_block_expression() => {
                let block = self.parse_block(BlockContext::Expression)?;

                Ok(self.insert_node(Expression::Block(block), self.range_since(start)))
            }
            Keyword::Debugger => {
                self.bump();
                Ok(self.insert_node(Expression::Debugger, self.range_since(start)))
            }
            Keyword::If => self.parse_if(),
            Keyword::While | Keyword::Do => self.parse_while(),
            Keyword::For => self.parse_for(),
            Keyword::Loop => self.parse_loop(),
            Keyword::Try => self.parse_try(),
            Keyword::Match => self.parse_match(),
            Keyword::Switch => self.parse_switch(),
            Keyword::Break => self.parse_break(),
            Keyword::Continue => self.parse_continue(),
            Keyword::Return => self.parse_return(),
            Keyword::Yield if self.keywords.yield_keyword == YieldKeyword::Forbidden => {
                Err(ParserError::unexpected(self.peek_token_span()))
            }
            Keyword::Yield if self.keywords.yield_keyword == YieldKeyword::Expression => {
                self.parse_yield()
            }
            Keyword::Await if self.keywords.await_keyword == AwaitKeyword::Forbidden => {
                Err(ParserError::unexpected(self.peek_token_span()))
            }
            Keyword::Await => self.parse_await(),
            Keyword::Async if self.peek_async_lambda() => self
                .parse_function(start, DeclarationHeader::default(), position)
                .map(|declaration| self.insert_declaration_expression(start, declaration)),
            Keyword::Const => self.parse_const_evaluation(),
            Keyword::New => self.parse_new(position),
            Keyword::Import if self.peek_import_statement() => self.parse_import(),
            Keyword::Import if self.peek_next_token_type() == TokenType::Dot => {
                self.parse_import_meta(start)
            }
            Keyword::Export => self.parse_export(),
            _ => self.parse_identifier_expression(start),
        }
    }

    /// Return whether a type-family keyword starts a type value expression.
    fn peek_type_keyword_value(&self, keyword: Keyword, stop: ExpressionStop) -> bool {
        // reject keywords outside the type family
        if !matches!(
            keyword,
            Keyword::Type | Keyword::Readonly | Keyword::Newtype
        ) {
            return false;
        }

        // leave line-leading type to declaration parsing
        let next = self.peek_next_token();
        let next_keyword = self.token_keyword(next);
        if keyword == Keyword::Type && next.is_on_new_line() {
            return false;
        }

        // keep type as the binding identifier in for (type of value)
        if keyword == Keyword::Type
            && stop.has(ExpressionStop::FOR_EACH)
            && next_keyword == Some(Keyword::Of)
        {
            return false;
        }

        // keep type extends|implements T as a relation unless an alias head follows
        let following = self.peek_token_type_at(2);
        if keyword == Keyword::Type
            && matches!(next_keyword, Some(Keyword::Extends | Keyword::Implements))
            && !matches!(
                following,
                TokenType::Assign | TokenType::LessThan | TokenType::ShiftLeft
            )
        {
            return false;
        }

        // explicit type markers claim symbolic memory type prefixes
        if keyword == Keyword::Type && self.peek_memory_type_prefix_at(1) {
            return true;
        }

        // keep type followed by any other value operator in value space
        if keyword == Keyword::Type
            && ExpressionOperator::from_token(next.ty(), next_keyword).is_some()
        {
            return false;
        }

        self.peek_type_operand_start_at(1)
    }

    /// Parse one non-identifier token primary.
    fn parse_token_primary(
        &mut self,
        start: &ParseStart,
        token: TokenType,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        match token {
            TokenType::OpenParenthesis => self.parse_parenthesized_primary(start, position, stop),
            TokenType::OpenBracket => self
                .parse_bracket_literal(start, position)
                .map(|expression| (expression, false)),
            TokenType::OpenBrace => self
                .parse_brace_primary(start, position)
                .map(|expression| (expression, false)),
            TokenType::LessThan if self.peek_generic_lambda() => self
                .parse_function(start, DeclarationHeader::default(), position)
                .map(|declaration| {
                    (
                        self.insert_declaration_expression(start, declaration),
                        false,
                    )
                }),
            TokenType::LessThan
                if position != ExpressionPosition::Constructor
                    && self.peek_tree_literal_start() =>
            {
                self.parse_tree_literal()
                    .map(|expression| (expression, false))
            }
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.peek_template_literal_start() =>
            {
                let value = self.parse_template_literal()?;
                let expression = self.insert_node(
                    Expression::TemplateExpression { value },
                    self.range_since(start),
                );

                Ok((expression, false))
            }
            TokenType::Divide | TokenType::DivideAssign => {
                let literal = self.parse_regex_literal()?;
                let expression =
                    self.insert_node(Expression::Literal(literal), self.range_since(start));

                Ok((expression, false))
            }
            TokenType::Literal if self.peek_scalar_literal_start() => {
                let literal = self.parse_scalar_literal()?;
                let expression =
                    self.insert_node(Expression::Literal(literal), self.range_since(start));

                Ok((expression, false))
            }
            TokenType::Range | TokenType::RangeInclusive => {
                let end_kind = if token == TokenType::RangeInclusive {
                    RangeEnd::Inclusive
                } else {
                    RangeEnd::Open
                };
                self.bump();
                let end = if self.peek_expression_range_end_omitted() {
                    if end_kind == RangeEnd::Open {
                        None
                    } else {
                        Some(self.recover_missing_expression_here(NodeType::Expression))
                    }
                } else {
                    Some(self.parse_expression_at(
                        position.right(),
                        stop,
                        OperatorPrecedence::Range,
                    )?)
                };
                let expression = self.insert_node(
                    Expression::RangeExpression {
                        start: None,
                        end,
                        end_kind,
                    },
                    self.range_since(start),
                );

                Ok((expression, false))
            }
            _ if TypePrefixOperator::from_token(token, None).is_some() => {
                let ty = self.parse_type(TypePosition::Type, TypeStop::default())?;

                Ok((self.insert_type_expression_value(ty), false))
            }
            _ => Err(ParserError::unexpected(self.peek_token_span())),
        }
    }

    /// Parse an object or block expression from one opening brace.
    fn parse_brace_primary(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if position == ExpressionPosition::Block
            || position.is_statement() && !self.peek_statement_object()
        {
            let block = self.parse_block(BlockContext::Expression)?;

            return Ok(self.insert_node(Expression::Block(block), self.range_since(start)));
        }
        let properties = self.parse_object_literal()?;

        Ok(self.insert_node(
            Expression::ObjectExpression { properties },
            self.range_since(start),
        ))
    }

    /// Parse `import.meta` or `import.source`.
    fn parse_import_meta(&mut self, start: &ParseStart) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_keyword(Keyword::Import)?;
        self.eat_token(TokenType::Dot)?;
        if self.peek_identifier_is("meta") {
            self.bump();

            return Ok(self.insert_node(Expression::ImportMeta, self.range_since(start)));
        }
        if self.peek_identifier_is("source") {
            self.bump();

            return Ok(self.insert_node(Expression::ImportSource, self.range_since(start)));
        }

        Err(ParserError::unexpected(self.peek_token_span()))
    }

    /// Return the current value prefix operation.
    fn peek_value_prefix_operator(&self) -> Option<UnaryOperator> {
        UnaryOperator::from_prefix_token(self.peek_token_type())
    }

    /// Insert one already consumed identifier expression.
    pub(in crate::parse) fn insert_identifier_expression(
        &mut self,
        start: &ParseStart,
        name: StringId,
        name_range: ByteRange,
    ) -> LocalNodeId<Expression> {
        let expression = self.insert_node(Expression::Identifier { name }, self.range_since(start));
        self.tree.set_main_range(expression, name_range);

        expression
    }

    /// Insert one type expression as a value expression.
    pub(in crate::parse) fn insert_type_expression_value(
        &mut self,
        value: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<Expression> {
        let expression = self.insert_node(Expression::Type { value }, self.tree.get_range(value));
        if let Some(range) = self.tree.get_main_range(value) {
            self.tree.set_main_range(expression, range);
        }
        if let Some(range) = self.tree.get_head_range(value) {
            self.tree.set_head_range(expression, range);
        }

        expression
    }

    /// Return whether a statement brace begins an object expression.
    fn peek_statement_object(&self) -> bool {
        let first = self.peek_token_at(1);

        // spread fields unambiguously begin an object value
        if first.is(TokenType::Spread) {
            return true;
        }

        // named fields require an object name followed by a colon
        let is_name = first.is(TokenType::Identifier)
            || first.is(TokenType::Literal)
                && matches!(
                    first.literal(),
                    Some(
                        TokenLiteral::String { .. }
                            | TokenLiteral::Int { .. }
                            | TokenLiteral::Float { .. }
                    )
                );

        is_name && self.peek_token_type_at(2) == TokenType::Colon
    }

    /// Return whether `do {` begins a block expression rather than a do-while loop.
    fn peek_do_block_expression(&self) -> bool {
        if self.peek_next_token_type() != TokenType::OpenBrace {
            return false;
        }

        self.peek_token_after_group(1, TokenType::OpenBrace, TokenType::CloseBrace)
            .is_none_or(|token| self.token_keyword(token) != Some(Keyword::While))
    }
}
