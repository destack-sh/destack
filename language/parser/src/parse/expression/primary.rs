use crate::parse::DeclarationHeader;
use crate::parse::scope::ExpressionScope;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};
use destack_dir::{
    BinaryOperator, BlockContext, Expression, Keyword, LocalNodeId, Path, ScalarLiteral, TokenType,
    TypeExpression, UnaryOperator,
};
use smallvec::smallvec;

impl Parser {
    /// Eat value prefix operators or one primary expression.
    ///
    /// Examples:
    /// ```ds
    /// -value
    /// await load()
    /// { value: 1 }
    /// ```
    pub(in crate::parse::expression) fn eat_value_prefix_or_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let token_type = self.peek_token_type();

        // identifier and keyword families own the only hot ambiguous primary heads
        if token_type == TokenType::Identifier {
            let expression_id = self.eat_identifier_value_primary(start)?;

            return Ok((expression_id, false));
        }

        // value prefix
        if let Some(operator) = self.peek_unary_prefix_operator_maybe() {
            return self.eat_value_prefix(start, operator).map(|id| (id, false));
        }

        // contextual type keyword as value identifier
        if self.current_type_keyword_is_value_identifier() {
            return self.eat_identifier_primary(start).map(|id| (id, false));
        }

        // type prefix in value space
        if let Some(operator) = self.peek_type_unary_prefix_operator_maybe() {
            let type_expression_id = self.eat_type_prefix(start, operator)?;

            return Ok((self.insert_type_expression_value(type_expression_id), false));
        }

        // token primary
        self.eat_token_value_primary(start)
    }

    /// Eat an identifier or keyword primary in value space.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// label: value
    /// async value => value
    /// ```
    fn eat_identifier_value_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if let Some(keyword) = self.current_keyword() {
            return self.eat_keyword_value_family_primary(start, keyword);
        }

        if let Some(expression_id) = self.eat_module_or_global_identifier_primary(start)? {
            return Ok(expression_id);
        }

        if let Some(expression_id) = self.eat_identifier_tagged_object_primary(start)? {
            return Ok(expression_id);
        }

        let next_token_type = self.next_token_type();

        if matches!(next_token_type, TokenType::ArrowWide) {
            if let Some(expression_id) = self.eat_lambda_expression(start)? {
                return Ok(expression_id);
            }
        }

        if next_token_type == TokenType::Colon {
            return self.eat_label_primary(start);
        }

        self.eat_identifier_primary(start)
    }

    /// Eat a direct single-name tagged object expression.
    ///
    /// Examples:
    /// ```ds
    /// Type { field: value }
    /// Row { id: 1, name }
    /// Item { ...defaults }
    /// ```
    fn eat_identifier_tagged_object_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !self.language.is_destack() || self.next_token_type() != TokenType::OpenBrace {
            return Ok(None);
        }

        if self.flags.is_in_super_type()
            || self.flags.is_in_before_block()
            || self.flags.is_in_for_each()
            || self.next_token().token.is_on_new_line
        {
            return Ok(None);
        }

        let (name, name_span) = self.eat_identifier_with_span()?;
        let path = Path {
            segments: smallvec![name],
        };
        let ty = self.insert_node(
            TypeExpression::Reference {
                path,
                generic_arguments: Vec::new(),
            },
            name_span,
        );
        self.tree.set_main_span(ty, name_span);

        let properties = self.with_flags(self.flags.not_in_position(), |parser| {
            parser.eat_object_literal()
        })?;
        let expression_id = self.insert_node(
            Expression::StructExpression { ty, properties },
            self.get_span_from(start),
        );

        Ok(Some(expression_id))
    }

    /// Eat one keyword family primary in value space.
    ///
    /// Examples:
    /// ```ds
    /// if ready { value }
    /// export class Value {}
    /// keyof Type
    /// ```
    fn eat_keyword_value_family_primary(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if !self.language.is_destack() && keyword == Keyword::Shared {
            return self.eat_identifier_primary(start);
        }

        if let Some(expression_id) = self.eat_declaration_prefix_primary(start, keyword)? {
            return Ok(expression_id);
        }

        if let Some(expression_id) = self.eat_keyword_expression(start, keyword)? {
            return Ok(expression_id);
        }

        if let Some(operator) = self.peek_unary_prefix_operator_maybe() {
            return self.eat_value_prefix(start, operator);
        }

        if self.current_type_keyword_is_value_identifier() {
            return self.eat_identifier_primary(start);
        }

        if let Some(operator) = self.peek_type_unary_prefix_operator_maybe() {
            let type_expression_id = self.eat_type_prefix(start, operator)?;

            return Ok(self.insert_type_expression_value(type_expression_id));
        }

        self.eat_identifier_primary(start)
    }

    /// Eat a module or global primary declaration when present.
    ///
    /// Examples:
    /// ```ds
    /// module name {}
    /// global {}
    /// module name { export const value = 1 }
    /// ```
    fn eat_module_or_global_identifier_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if self.is_module_identifier() && self.next_token_type() == TokenType::OpenBrace {
            let declaration = self.eat_module(start)?;
            let expression_id = self.insert_node(
                Expression::Declaration(declaration),
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        if self.is_global_identifier() && self.next_token_type() == TokenType::OpenBrace {
            let header = DeclarationHeader {
                is_ambient: self.language.is_declaration(),
                ..DeclarationHeader::default()
            };
            let declaration = self.eat_global(start, header)?;
            let expression_id = self.insert_node(
                Expression::Declaration(declaration),
                self.get_span_from(start),
            );

            return Ok(Some(expression_id));
        }

        Ok(None)
    }

    /// Return whether the current type keyword is a value identifier here.
    fn current_type_keyword_is_value_identifier(&mut self) -> bool {
        self.type_keyword_starts_value_member_path()
            || self.current_type_prefix_keyword_without_operand_is_value_identifier()
    }

    /// Eat one token primary in value space.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// 42
    /// `hello ${name}`
    /// ```
    fn eat_token_value_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<(LocalNodeId<Expression>, bool)> {
        let token_type = self.peek_token_type();
        match token_type {
            TokenType::OpenParenthesis => {
                if let Some(expression_id) = self.eat_lambda_expression(start)? {
                    return Ok((expression_id, false));
                }

                self.eat_parenthesized_value(start)
            }
            TokenType::OpenBracket => self
                .eat_bracket_literal_expression(start)
                .map(|id| (id, false)),
            TokenType::OpenBrace => self.eat_value_brace_primary(start).map(|id| (id, false)),
            TokenType::LessThan if self.can_start_generic_arrow_expression() => {
                let function_id = self.eat_function(start, DeclarationHeader::default())?;
                Ok((
                    self.insert_node(
                        Expression::Declaration(function_id),
                        self.get_span_from(start),
                    ),
                    false,
                ))
            }
            TokenType::LessThan if self.can_start_tree_literal() => {
                let flags = self.flags.not_in_position();
                self.with_flags(flags, |parser| parser.eat_tree_literal())
                    .map(|id| (id, false))
            }
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.is_template_literal_start() =>
            {
                let value = self.eat_template_literal()?;
                Ok((
                    self.insert_node(
                        Expression::TemplateExpression { value },
                        self.get_span_from(start),
                    ),
                    false,
                ))
            }
            TokenType::Divide | TokenType::DivideAssign => {
                let literal = self.eat_regex_literal()?;
                Ok((
                    self.insert_node(
                        Expression::ScalarLiteral(literal),
                        self.get_span_from(start),
                    ),
                    false,
                ))
            }
            TokenType::Literal if self.is_scalar_literal_start() => {
                let literal = self.eat_scalar_literal()?;
                Ok((
                    self.insert_node(
                        Expression::ScalarLiteral(literal),
                        self.get_span_from(start),
                    ),
                    false,
                ))
            }
            TokenType::ElementwiseOr if self.language.is_destack() => self
                .eat_value_leading_binary_list(start, BinaryOperator::ElementwiseOr)
                .map(|id| (id, false)),
            TokenType::Range | TokenType::RangeInclusive if self.language.is_destack() => {
                self.eat_value_startless_range(start).map(|id| (id, false))
            }
            TokenType::ElementwiseAnd | TokenType::ElementwiseXor if self.language.is_destack() => {
                self.eat_value_reference_operator(start, token_type)
                    .map(|id| (id, false))
            }
            TokenType::Hash if self.token_type_at_offset(1) == TokenType::Identifier => {
                if self.language.is_destack() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                self.bump();
                let (name, name_span) = self.eat_identifier_with_span()?;
                let id = self.insert_node(
                    Expression::PrivateIdentifier { name },
                    self.get_span_from(start),
                );
                self.tree.set_main_span(id, name_span);
                Ok((id, false))
            }
            _ => Err(ParseError::unexpected(self.peek()?.span)),
        }
    }

    /// Parse a value list with a leading binary separator.
    ///
    /// Examples:
    /// ```ds
    /// | A | B
    /// | A
    /// & A & B
    /// ```
    fn eat_value_leading_binary_list(
        &mut self,
        start: &ParserSpanStart,
        operator: BinaryOperator,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let mut left: Option<LocalNodeId<Expression>> = None;
        let minimum_precedence = operator.precedence();

        while self.peek_token_type() == TokenType::ElementwiseOr {
            self.bump();
            let value_scope = ExpressionScope::from_flags(self.flags.not_in_position())
                .at_precedence(Some(minimum_precedence));
            let right = self.eat_value_operand(value_scope)?;

            // append the next value to the growing left associative expression
            left = Some(if let Some(left) = left {
                self.insert_node(
                    Expression::Binary {
                        left,
                        operator,
                        right,
                    },
                    self.get_span_from(start),
                )
            } else {
                right
            });
        }

        left.ok_or_else(|| ParseError::unexpected(self.anchor_span_here()))
    }

    /// Parse one value prefix operator.
    ///
    /// Examples:
    /// ```ds
    /// !value
    /// await call()
    /// &mut value
    /// ```
    fn eat_value_prefix(
        &mut self,
        start: &ParserSpanStart,
        operator: UnaryOperator,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_start = self.span_start();
        self.bump();
        let operator_span = self.get_span_from(&operator_start);
        let right_scope = ExpressionScope::from_flags(self.flags.not_in_position())
            .at_precedence(Some(operator.precedence()));
        let right = self.eat_value_operand(right_scope)?;

        let id = self.insert_node(
            Expression::Unary { operator, right },
            self.get_span_from(start),
        );
        self.tree.set_main_span(id, operator_span);

        Ok(id)
    }

    /// Parse one identifier primary in value space.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// namespace.value
    /// Type<T> { field: value }
    /// ```
    fn eat_identifier_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.current_keyword() == Some(Keyword::This) {
            self.bump();
            return Ok(self.insert_node(Expression::This, self.get_span_from(start)));
        }
        if self.current_keyword() == Some(Keyword::Super) {
            self.bump();
            return Ok(self.insert_node(Expression::Super, self.get_span_from(start)));
        }
        if self.current_keyword() == Some(Keyword::Const) && self.flags.is_in_type() {
            self.bump();
            let type_id = self.insert_node(TypeExpression::Const, self.get_span_from(start));
            return Ok(self.insert_type_expression_value(type_id));
        }

        self.eat_identifier_expression_path(start)
    }

    /// Parse one labeled expression primary.
    ///
    /// Examples:
    /// ```ds
    /// label: value
    /// outer: while ready {}
    /// case: call()
    /// ```
    fn eat_label_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if !self.can_parse_label_expression() {
            return self.eat_identifier_primary(start);
        }

        let (label, label_span, body) = self.eat_label_expression_shell()?;
        let id = self.insert_node(Expression::Label { label, body }, self.get_span_from(start));
        self.tree.set_main_span(id, label_span);

        Ok(id)
    }

    /// Return whether a type keyword is the head of a value member path.
    fn type_keyword_starts_value_member_path(&mut self) -> bool {
        let keyword = self.current_keyword();
        let is_contextual_type_keyword =
            matches!(keyword, Some(Keyword::Keyof | Keyword::Readonly))
                || self.language.is_destack() && keyword == Some(Keyword::Shared);
        if !is_contextual_type_keyword {
            return false;
        }

        matches!(
            self.token_type_at_offset(1),
            TokenType::Dot | TokenType::Maybe
        )
    }

    /// Return whether a contextual type prefix keyword is a value identifier here.
    fn current_type_prefix_keyword_without_operand_is_value_identifier(&mut self) -> bool {
        let keyword = self.current_keyword();
        let is_contextual_type_prefix = matches!(keyword, Some(Keyword::Keyof | Keyword::Readonly))
            || self.language.is_destack()
                && matches!(keyword, Some(Keyword::Local | Keyword::Shared));
        if !is_contextual_type_prefix {
            return false;
        }

        let next_token_can_start_operand = match self.token_type_at_offset(1) {
            TokenType::Identifier => true,
            TokenType::Literal => true,
            TokenType::OpenParenthesis | TokenType::OpenBracket => true,
            TokenType::LessThan | TokenType::Not => true,
            TokenType::Multiply | TokenType::ElementwiseAnd | TokenType::ElementwiseXor => true,
            _ => false,
        };

        !next_token_can_start_operand
    }

    /// Parse keyword expression dispatch.
    ///
    /// Examples:
    /// ```ds
    /// if value {}
    /// return value
    /// match value { case => result }
    /// ```
    pub(crate) fn eat_keyword_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        self.eat_keyword_expression_with_header(start, keyword, DeclarationHeader::default())
    }

    /// Parse keyword expression dispatch with a declaration header.
    ///
    /// Examples:
    /// ```ds
    /// declare function value()
    /// export class Value {}
    /// declare namespace Value {}
    /// ```
    pub(in crate::parse::expression) fn eat_keyword_expression_with_header(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
        header: DeclarationHeader,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // preserve decorator member paths
        let is_decorator_identifier = self.flags.is_in_decorator()
            && !matches!(
                keyword,
                Keyword::Function
                    | Keyword::Struct
                    | Keyword::Class
                    | Keyword::Enum
                    | Keyword::Interface
                    | Keyword::Extension
                    | Keyword::Const
                    | Keyword::Newtype
            );
        if is_decorator_identifier {
            return self.eat_identifier_expression_path(start).map(Some);
        }

        // declarations
        if let Some(expression) = self.eat_keyword_declaration_expression(start, keyword, header)? {
            return Ok(Some(expression));
        }

        // control expressions
        if let Some(expression) = self.eat_keyword_control_expression(start, keyword)? {
            return Ok(Some(expression));
        }

        // scalar and meta primaries
        if let Some(expression) = self.eat_keyword_value_primary(start, keyword)? {
            return Ok(Some(expression));
        }

        // type forms in value space
        if let Some(expression) = self.eat_keyword_type_value(start, keyword, header)? {
            return Ok(Some(expression));
        }

        Ok(None)
    }

    /// Parse control keyword expressions.
    ///
    /// Examples:
    /// ```ds
    /// while ready {}
    /// if ready { value } else { fallback }
    /// try work() catch error
    /// ```
    fn eat_keyword_control_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        match keyword {
            Keyword::If => self.eat_if().map(Some),
            Keyword::While => self.eat_while().map(Some),
            Keyword::Do => {
                let next_token_type = self.next_token_type();
                if self.is_do_while_statement(next_token_type) {
                    self.eat_while().map(Some)
                } else if next_token_type == TokenType::OpenBrace {
                    let block = self.eat_block(BlockContext::Expression)?;
                    let expression =
                        self.insert_node(Expression::Block(block), self.get_span_from(start));

                    Ok(Some(expression))
                } else {
                    Ok(None)
                }
            }
            Keyword::For => self.eat_for().map(Some),
            Keyword::Loop if self.language.is_destack() => self.eat_loop().map(Some),
            Keyword::Try => self.eat_try().map(Some),
            Keyword::Switch => self.eat_match().map(Some),
            Keyword::Match if self.language.is_destack() => self.eat_match().map(Some),
            Keyword::Break => self.eat_break().map(Some),
            Keyword::Continue => self.eat_continue().map(Some),
            Keyword::Throw => self.eat_throw().map(Some),
            Keyword::Return => self.eat_return().map(Some),
            Keyword::Yield if self.flags.is_in_generator() => self.eat_yield().map(Some),
            Keyword::Await => self.eat_await().map(Some),
            Keyword::Comptime if self.language.is_destack() => self.eat_comptime().map(Some),
            _ => Ok(None),
        }
    }

    /// Parse scalar and meta keyword primaries.
    ///
    /// Examples:
    /// ```ds
    /// null
    /// true
    /// this
    /// ```
    fn eat_keyword_value_primary(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        match keyword {
            Keyword::Debugger => {
                self.bump();
                let expression = self.insert_node(Expression::Debugger, self.get_span_from(start));

                Ok(Some(expression))
            }
            Keyword::Null => {
                self.bump();
                let literal = Expression::ScalarLiteral(ScalarLiteral::Null);
                let expression = self.insert_node(literal, self.get_span_from(start));

                Ok(Some(expression))
            }
            Keyword::New if self.next_token_type() == TokenType::Dot => {
                self.eat_new_target(start).map(Some)
            }
            Keyword::New => self.eat_new().map(Some),
            Keyword::Async => self.eat_lambda_expression(start),
            Keyword::Import if self.can_start_import_statement() => self.eat_import().map(Some),
            Keyword::Import if self.next_token_type() == TokenType::Dot => {
                self.eat_import_meta_or_source(start).map(Some)
            }
            _ => Ok(None),
        }
    }

    /// Parse type keyword forms that produce value expressions.
    ///
    /// Examples:
    /// ```ds
    /// type Value = string
    /// interface Shape {}
    /// typeof value
    /// ```
    fn eat_keyword_type_value(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
        header: DeclarationHeader,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !matches!(
            keyword,
            Keyword::Type | Keyword::Newtype | Keyword::Readonly
        ) {
            return Ok(None);
        }

        let next_token = self.next_token();
        let next_token_type = next_token.token.ty;
        let next_is_on_new_line = next_token.token.is_on_new_line;
        let next_keyword = self.next_keyword();
        let following_token_type = self.token_type_at_offset(2);
        if !self.keyword_begins_type_form(
            keyword,
            next_token_type,
            next_is_on_new_line,
            next_keyword,
            following_token_type,
        ) {
            return Ok(None);
        }

        let type_expression = self.eat_type(start, header)?;
        let expression = self.expression_from_type_value(type_expression);

        Ok(Some(expression))
    }

    /// Build a value expression from a parsed type form.
    fn expression_from_type_value(
        &mut self,
        type_expression: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<Expression> {
        if let TypeExpression::Declaration { declaration } = self.tree.get(type_expression) {
            return self.insert_node(
                Expression::Declaration(*declaration),
                self.tree.get_span(type_expression),
            );
        }

        self.insert_type_expression_value(type_expression)
    }

    /// Parse `import.meta` or `import.source`.
    ///
    /// Examples:
    /// ```ds
    /// import.meta
    /// import.source("module")
    /// import("module")
    /// ```
    fn eat_import_meta_or_source(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let import_span = self.peek_keyword(Keyword::Import)?.span;
        self.eat_keyword(Keyword::Import)?;
        self.eat_token(TokenType::Dot)?;
        if self.current_identifier_str_is("meta") {
            self.bump();
            return Ok(self.insert_node(Expression::ImportMeta, self.get_span_from(start)));
        }
        if self.current_identifier_str_is("source") {
            let import_name = self.strings.intern("import");
            let (source_name, source_span) = self.eat_identifier_with_span()?;
            let path = Path {
                segments: smallvec![import_name, source_name],
            };
            let id = self.insert_node(
                Expression::QualifiedReference {
                    path,
                    generic_arguments: Vec::new(),
                },
                self.get_span_from(start),
            );
            self.set_path_expression_spans(id, &[import_span, source_span])?;
            return Ok(id);
        }

        Err(ParseError::unexpected(self.peek()?.span))
    }

    /// Parse `new.target`.
    ///
    /// Examples:
    /// ```ds
    /// new.target
    /// new.target.name
    /// new.target?.name
    /// ```
    fn eat_new_target(&mut self, start: &ParserSpanStart) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_keyword(Keyword::New)?;
        self.eat_token(TokenType::Dot)?;
        self.eat_identifier_str("target")?;

        Ok(self.insert_node(Expression::NewTarget, self.get_span_from(start)))
    }

    /// Parse value brace primary.
    ///
    /// Examples:
    /// ```ds
    /// { value: 1 }
    /// { [key]: value }
    /// { ...other }
    /// ```
    fn eat_value_brace_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if self.flags.is_in_before_block() && !self.flags.is_in_for_each()
            || self.flags.is_in_statement_position()
                && !self.can_parse_object_literal_in_statement_position()
        {
            let block = self.eat_block(BlockContext::Expression)?;
            return Ok(self.insert_node(Expression::Block(block), self.get_span_from(start)));
        }
        let properties = self.with_flags(self.flags.not_in_position(), |parser| {
            parser.eat_object_literal()
        })?;

        Ok(self.insert_node(
            Expression::ObjectExpression { properties },
            self.get_span_from(start),
        ))
    }
}
