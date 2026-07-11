use crate::parse::DeclarationHeader;
use crate::parse::context::{
    AwaitContext, BraceContext, DecoratorContext, ExpressionContext, ExpressionStops,
    StatementPosition, TypeContext, YieldContext,
};
use crate::parse::expression::operator::ExpressionOperator;
use crate::parse::r#type::operator::TypePrefixOperator;
use crate::{ParseStart, Parser, ParserError, ParserResult};
use destack_core::StringId;
use destack_dir::{
    BlockContext, Expression, InferForm, Keyword, LocalNodeId, Mutability, NodeType,
    OperatorPrecedence, Path, RangeEnd, ScalarLiteral, TokenLiteral, TokenType, TypeExpression,
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
        /// The variance modifier.
        variance: Option<VarianceBound>,
        /// The operator source range.
        range: ByteRange,
    },
    /// One move prefix.
    Move {
        /// The mutability modifier.
        mutability: Option<Mutability>,
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
        context: ExpressionContext,
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
            self.parse_expression_primary(&primary_start, context)?;
        let mut expression =
            self.parse_expression_postfix(&primary_start, expression, is_parenthesized, context)?;

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

        // leave non-reference tokens to the primary expression
        if !matches!(
            self.peek_token_type(),
            TokenType::ElementwiseAnd | TokenType::ElementwiseXor | TokenType::LogicalAnd
        ) {
            return Ok(None);
        }

        // parse one borrow or move prefix
        let token = self.eat_reference_prefix_operator()?.token;
        let mutability = Some(self.parse_reference_mutability());
        let variance = self.parse_variance_bound_if_present();
        let prefix = if token.is(TokenType::ElementwiseAnd) {
            ValuePrefix::Borrow {
                mutability,
                variance,
                range: token.range(),
            }
        } else {
            ValuePrefix::Move {
                mutability,
                variance,
                range: token.range(),
            }
        };

        Ok(Some(prefix))
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
                variance,
                range,
            } => (
                Expression::BorrowOf {
                    mutability,
                    variance,
                    right,
                },
                range,
            ),
            ValuePrefix::Move {
                mutability,
                variance,
                range,
            } => (
                Expression::MoveOf {
                    mutability,
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
        context: ExpressionContext,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        let token = self.peek_token_type();

        // dispatch identifiers without repeating keyword classification
        if token == TokenType::Identifier {
            let expression = if let Some(keyword) = self.peek_keyword() {
                self.parse_keyword_primary(start, keyword, context)?
            } else {
                self.parse_identifier_primary(start, context)?
            };

            return Ok((expression, false));
        }

        self.parse_token_primary(start, token, context)
    }

    /// Parse one non-keyword identifier primary.
    fn parse_identifier_primary(
        &mut self,
        start: &ParseStart,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if self.peek_identifier_is("delete") {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // primitive type literals are first-class values outside value postfix syntax
        if !self.peek_identifier_value_postfix() && self.peek_intrinsic_type_literal().is_some() {
            let value = self.parse_type(TypeContext {
                function: context.function,
                ..TypeContext::default()
            })?;

            return Ok(self.insert_type_expression_value(value));
        }

        let (name, name_range) = self.eat_identifier_with_range()?;

        // classify identifier-shaped grammar from one consumed head
        match self.peek_token_type() {
            TokenType::ArrowWide => {
                let declaration = self.parse_bare_lambda(
                    start,
                    name,
                    name_range,
                    DeclarationHeader::default(),
                    context.function,
                )?;

                Ok(self.insert_declaration_expression(start, declaration))
            }
            TokenType::Colon if self.peek_label_body(context) => {
                let body = self.parse_label_body(context)?;
                let expression = self.insert_node(
                    Expression::Label { label: name, body },
                    self.range_since(start),
                );
                self.tree.set_main_range(expression, name_range);

                Ok(expression)
            }
            TokenType::OpenBrace => {
                self.parse_identifier_object_primary(start, name, name_range, context)
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
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let name_text = self.strings.get(name);

        // parse module Name { body }
        if name_text == "module" {
            let declaration = self.parse_module(start, context.function)?;

            return Ok(self.insert_declaration_expression(start, declaration));
        }

        // parse global { body }
        if name_text == "global" {
            let header = DeclarationHeader {
                is_ambient: self.is_ambient,
                ..DeclarationHeader::default()
            };
            let declaration = self.parse_global(start, header, context.function)?;

            return Ok(self.insert_declaration_expression(start, declaration));
        }

        // leave an identifier followed by an enclosing control body
        if context.brace == BraceContext::Block
            || context.stops.contains(ExpressionStops::BODY_BRACE)
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
        let properties = self.parse_object_literal(context.function)?;

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
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // parse @keyword as an identifier decorator head except for this and super
        if context.decorator == DecoratorContext::Head
            && !matches!(keyword, Keyword::This | Keyword::Super)
        {
            return self.parse_identifier_expression(start);
        }

        // parse declaration keyword ...
        if self.peek_declaration_primary(context) {
            return self.parse_declaration_primary(start, context);
        }

        // parse type declarations and first-class structural type values
        if self.peek_type_keyword_value(keyword, context) {
            let value = self.parse_type_declaration(
                start,
                TypeContext {
                    function: context.function,
                    ..TypeContext::default()
                },
            )?;

            return Ok(self.insert_type_expression_value(value));
        }

        // parse keyof T, readonly T, local T, or shared T as a value
        if TypePrefixOperator::from_token(TokenType::Identifier, Some(keyword)).is_some()
            && self.peek_type_prefix_value()
        {
            let value = self.parse_type(TypeContext {
                function: context.function,
                ..TypeContext::default()
            })?;

            return Ok(self.insert_type_expression_value(value));
        }

        match keyword {
            Keyword::Function => Err(ParserError::unexpected(self.peek_token_span())),
            Keyword::This => {
                self.bump();
                Ok(self.insert_node(Expression::This, self.range_since(start)))
            }
            Keyword::Super => {
                self.bump();
                Ok(self.insert_node(Expression::Super, self.range_since(start)))
            }
            Keyword::Null => {
                self.bump();
                Ok(self.insert_node(
                    Expression::ScalarLiteral(ScalarLiteral::Null),
                    self.range_since(start),
                ))
            }
            Keyword::Undefined => {
                self.bump();
                Ok(self.insert_node(
                    Expression::ScalarLiteral(ScalarLiteral::Undefined),
                    self.range_since(start),
                ))
            }
            Keyword::Do if self.peek_do_block_expression() => {
                let block = self.parse_block(BlockContext::Expression, context.function)?;

                Ok(self.insert_node(Expression::Block(block), self.range_since(start)))
            }
            Keyword::Debugger => {
                self.bump();
                Ok(self.insert_node(Expression::Debugger, self.range_since(start)))
            }
            Keyword::If => self.parse_if(context.function),
            Keyword::While | Keyword::Do => self.parse_while(context.function),
            Keyword::For => self.parse_for(context.function),
            Keyword::Loop => self.parse_loop(context.function),
            Keyword::Try => self.parse_try(context.function),
            Keyword::Switch | Keyword::Match => self.parse_match(context.function),
            Keyword::Break => self.parse_break(context.function),
            Keyword::Continue => self.parse_continue(),
            Keyword::Throw => self.parse_throw(context.function),
            Keyword::Return => self.parse_return(context.function),
            Keyword::Yield if context.function.yield_context == YieldContext::Forbidden => {
                Err(ParserError::unexpected(self.peek_token_span()))
            }
            Keyword::Yield if context.function.yield_context == YieldContext::Expression => {
                self.parse_yield(context.function)
            }
            Keyword::Await if context.function.await_context == AwaitContext::Forbidden => {
                Err(ParserError::unexpected(self.peek_token_span()))
            }
            Keyword::Await => self.parse_await(context.function),
            Keyword::Async if self.peek_async_lambda() => self
                .parse_function(start, DeclarationHeader::default(), context)
                .map(|declaration| self.insert_declaration_expression(start, declaration)),
            Keyword::Comptime => self.parse_comptime(context.function),
            Keyword::New => self.parse_new(context),
            Keyword::Import if self.peek_import_statement() => self.parse_import(context.function),
            Keyword::Import if self.peek_next_token_type() == TokenType::Dot => {
                self.parse_import_meta(start)
            }
            Keyword::Export => self.parse_export(context.function),
            _ => self.parse_identifier_expression(start),
        }
    }

    /// Return whether the current identifier is followed by a value postfix.
    fn peek_identifier_value_postfix(&self) -> bool {
        let next = self.peek_next_token_type();

        next == TokenType::Dot
            || next == TokenType::Maybe && self.peek_token_type_at(2) == TokenType::Dot
    }

    /// Return whether a type-prefix keyword starts a first-class type value.
    fn peek_type_prefix_value(&self) -> bool {
        let next = self.peek_next_token_type();

        match next {
            TokenType::Identifier
            | TokenType::OpenParenthesis
            | TokenType::OpenBracket
            | TokenType::OpenBrace
            | TokenType::LessThan
            | TokenType::ElementwiseAnd
            | TokenType::ElementwiseXor
            | TokenType::LogicalAnd
            | TokenType::Multiply
            | TokenType::ElementwiseOr
            | TokenType::TemplateString
            | TokenType::TemplateStringStart
            | TokenType::Literal
            | TokenType::Range
            | TokenType::RangeInclusive => true,
            TokenType::Add | TokenType::Subtract => {
                self.peek_token_type_at(2) == TokenType::Literal
            }
            _ => false,
        }
    }

    /// Return whether a type-family keyword starts a type value expression.
    fn peek_type_keyword_value(&self, keyword: Keyword, context: ExpressionContext) -> bool {
        // reject keywords outside the type family
        if !matches!(
            keyword,
            Keyword::Type | Keyword::Readonly | Keyword::Newtype
        ) {
            return false;
        }

        // leave line-leading type to declaration parsing
        let next = self.peek_next_token();
        if keyword == Keyword::Type && next.is_on_new_line() {
            return false;
        }

        // keep type followed by a value operator in expression grammar
        if keyword == Keyword::Type
            && ExpressionOperator::from_token(next.ty(), next.keyword()).is_some()
        {
            return false;
        }

        // keep type as the binding identifier in for (type in|of value)
        if keyword == Keyword::Type
            && context.stops.contains(ExpressionStops::FOR_EACH)
            && matches!(next.keyword(), Some(Keyword::In | Keyword::Of))
        {
            return false;
        }

        // keep type extends|implements T as a relation unless an alias head follows
        let following = self.peek_token_type_at(2);
        if keyword == Keyword::Type
            && matches!(next.keyword(), Some(Keyword::Extends | Keyword::Implements))
            && !matches!(
                following,
                TokenType::Assign | TokenType::LessThan | TokenType::ShiftLeft
            )
        {
            return false;
        }

        matches!(
            next.ty(),
            TokenType::Identifier
                | TokenType::OpenBrace
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::Literal
        )
    }

    /// Parse one non-identifier token primary.
    fn parse_token_primary(
        &mut self,
        start: &ParseStart,
        token: TokenType,
        context: ExpressionContext,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        match token {
            TokenType::OpenParenthesis => self.parse_parenthesized_primary(start, context),
            TokenType::OpenBracket => self
                .parse_bracket_literal(start, context)
                .map(|expression| (expression, false)),
            TokenType::OpenBrace => self
                .parse_brace_primary(start, context)
                .map(|expression| (expression, false)),
            TokenType::LessThan if self.peek_generic_lambda() => self
                .parse_function(start, DeclarationHeader::default(), context)
                .map(|declaration| {
                    (
                        self.insert_declaration_expression(start, declaration),
                        false,
                    )
                }),
            TokenType::LessThan if self.peek_tree_literal_start() => self
                .parse_tree_literal(context.function)
                .map(|expression| (expression, false)),
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.peek_template_literal_start() =>
            {
                let value = self.parse_template_literal(context.function)?;
                let expression = self.insert_node(
                    Expression::TemplateExpression { value },
                    self.range_since(start),
                );

                Ok((expression, false))
            }
            TokenType::Divide | TokenType::DivideAssign => {
                let literal = self.parse_regex_literal()?;
                let expression =
                    self.insert_node(Expression::ScalarLiteral(literal), self.range_since(start));

                Ok((expression, false))
            }
            TokenType::Literal if self.peek_scalar_literal_start() => {
                let literal = self.parse_scalar_literal()?;
                let expression =
                    self.insert_node(Expression::ScalarLiteral(literal), self.range_since(start));

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
                    Some(self.parse_expression(context.right(OperatorPrecedence::Range))?)
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
                let ty = self.parse_type(TypeContext {
                    function: context.function,
                    ..TypeContext::default()
                })?;

                Ok((self.insert_type_expression_value(ty), false))
            }
            _ => Err(ParserError::unexpected(self.peek_token_span())),
        }
    }

    /// Parse an object or block expression from one opening brace.
    fn parse_brace_primary(
        &mut self,
        start: &ParseStart,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if context.brace == BraceContext::Block
            || context.statement == StatementPosition::Direct && !self.peek_statement_object()
        {
            let block = self.parse_block(BlockContext::Expression, context.function)?;

            return Ok(self.insert_node(Expression::Block(block), self.range_since(start)));
        }
        let properties = self.parse_object_literal(context.function)?;

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
        if self.peek_is(TokenType::Identifier) {
            return self
                .peek_keyword()
                .and_then(UnaryOperator::from_prefix_keyword);
        }

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

        // computed fields require a colon after their balanced key
        if first.is(TokenType::OpenBracket) {
            return self
                .peek_token_after_group(1, TokenType::OpenBracket, TokenType::CloseBracket)
                .is_some_and(|token| token.is(TokenType::Colon));
        }

        // named fields require an object key followed by a colon
        let is_key = first.is(TokenType::Identifier)
            || first.is(TokenType::Literal)
                && matches!(
                    first.literal(),
                    Some(
                        TokenLiteral::String { .. }
                            | TokenLiteral::Int { .. }
                            | TokenLiteral::Float { .. }
                    )
                );

        is_key && self.peek_token_type_at(2) == TokenType::Colon
    }

    /// Return whether `do {` begins a block expression rather than a do-while loop.
    fn peek_do_block_expression(&self) -> bool {
        if self.peek_next_token_type() != TokenType::OpenBrace {
            return false;
        }

        self.peek_token_after_group(1, TokenType::OpenBrace, TokenType::CloseBrace)
            .is_none_or(|token| token.keyword() != Some(Keyword::While))
    }
}
