use crate::parse::flags::ParserFlags;
use crate::parse::scope::ExpressionScope;
use crate::parse::{
    DeclarationHeader, PendingDecorators, is_declaration_keyword, is_type_relation_keyword,
};
use crate::{Parser, ParserResult, ParserSpanStart};
use destack_dir::{
    Asynchrony, Declaration, DependencyBinding, DependencyForm, DependencyItem, ExportKind,
    Expression, Keyword, LocalNodeId, NodeType, TokenLiteral, TokenType, TypeExpression,
};
use destack_source::Span;
use smallvec::SmallVec;

impl Parser {
    /// Insert one explicit `Expression::Type` wrapper.
    pub(crate) fn insert_type_expression_value(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<Expression> {
        let expression_id = self.insert_node(
            Expression::Type {
                value: type_expression_id,
            },
            self.tree.get_span(type_expression_id),
        );
        if let Some(span) = self.tree.get_main_span(type_expression_id) {
            self.tree.set_main_span(expression_id, span);
        }
        if let Some(span) = self.tree.get_head_span(type_expression_id) {
            self.tree.set_head_span(expression_id, span);
        }

        expression_id
    }

    /// Eat an expression with an explicit minimum infix precedence.
    ///
    /// Examples:
    /// ```ds
    /// left * right
    /// left as Type
    /// left satisfies Constraint
    /// ```
    pub(crate) fn eat_expression_at_precedence(
        &mut self,
        flags: ParserFlags,
        minimum_precedence: u16,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let scope = ExpressionScope::from_flags(flags).at_precedence(Some(minimum_precedence));

        self.with_recursive_descent(NodeType::Expression, |parser| {
            parser.eat_expression_scope(scope)
        })
    }

    /// Eat an expression using the given scope.
    ///
    /// Examples:
    /// ```ds
    /// call(argument)
    /// await load()
    /// match value { case => result }
    /// ```
    pub(crate) fn eat_expression(
        &mut self,
        flags: ParserFlags,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let scope = ExpressionScope::from_flags(flags);

        self.with_recursive_descent(NodeType::Expression, |parser| {
            parser.eat_expression_scope(scope)
        })
    }

    /// Eat one parenthesized expression and return its expression node.
    ///
    /// Examples:
    /// ```ds
    /// (value)
    /// (value + other)
    /// (condition ? yes : no)
    /// ```
    pub(crate) fn eat_parenthesized_expression(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let expression_id = self.eat_expression(self.flags)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(expression_id)
    }

    /// Parse a plain identifier expression without consuming unrelated syntax when present.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// namespace.value
    /// module { export const value = 1 }
    /// ```
    pub(crate) fn eat_plain_identifier_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if self.peek_token_type() != TokenType::Identifier || self.current_keyword().is_some() {
            return Ok(None);
        }

        if self.is_module_identifier() && self.next_token_type() == TokenType::OpenBrace {
            let declaration = self.eat_module(start)?;
            return Ok(Some(self.insert_node(
                Expression::Declaration(declaration),
                self.get_span_from(start),
            )));
        }

        if self.is_global_identifier() && self.next_token_type() == TokenType::OpenBrace {
            let header = DeclarationHeader {
                is_ambient: self.language.is_declaration(),
                ..DeclarationHeader::default()
            };
            let declaration = self.eat_global(start, header)?;
            return Ok(Some(self.insert_node(
                Expression::Declaration(declaration),
                self.get_span_from(start),
            )));
        }

        let expression_id = self.eat_identifier_expression_path(start)?;
        let expression_id = self.eat_expression_continuation(start, expression_id, false)?;

        Ok(Some(expression_id))
    }

    /// Eat one identifier path expression.
    ///
    /// Examples:
    /// ```ds
    /// namespace.value
    /// value
    /// module.value.member
    /// ```
    pub(crate) fn eat_identifier_expression_path(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let (name, name_span) = self.eat_identifier_with_span()?;
        let expression_id =
            self.insert_node(Expression::Identifier { name }, self.get_span_from(start));
        self.tree.set_main_span(expression_id, name_span);

        Ok(expression_id)
    }

    /// Parse expression continuation after an already parsed left value.
    ///
    /// Examples:
    /// ```ds
    /// .member(argument)
    /// [index]!
    /// <T>(argument)
    /// ```
    pub(crate) fn eat_expression_continuation(
        &mut self,
        start: &ParserSpanStart,
        left_expression_id: LocalNodeId<Expression>,
        left_is_parenthesized: bool,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let scope = ExpressionScope::from_flags(self.flags);
        let (left_expression_id, _) =
            self.eat_postfix(start, left_expression_id, left_is_parenthesized, scope)?;
        let left_expression_id = self.eat_binary_rest(start, left_expression_id, scope)?;
        let left_expression_id = self.eat_conditional_rest(start, left_expression_id, scope)?;
        let left_expression_id = self.eat_assignment_rest(start, left_expression_id, scope)?;

        self.eat_sequence_rest(start, left_expression_id, scope)
    }

    /// Eat one complete value expression.
    ///
    /// Examples:
    /// ```ds
    /// left ? then : else
    /// target = value
    /// first, second
    /// ```
    pub(super) fn eat_expression_body(
        &mut self,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        let mut decorators = if !self.flags.is_in_decorator() && self.peek_is(TokenType::At) {
            self.eat_decorators_maybe()?
        } else {
            SmallVec::new()
        };

        let expression_id = self.eat_assignment(&start, scope)?;
        let expression_id = self.eat_sequence_rest(&start, expression_id, scope)?;
        let expression_id = self.wrap_decorated_default_export(&start, expression_id, &decorators);
        self.attach_pending_decorators_to_expression(&mut decorators, expression_id);

        Ok(expression_id)
    }

    /// Eat one complete expression under an explicit parser scope.
    ///
    /// Examples:
    /// ```ds
    /// value + other
    /// target = value
    /// condition ? yes : no
    /// ```
    pub(super) fn eat_expression_scope(
        &mut self,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.with_flags(scope.flags, |parser| parser.eat_expression_body(scope))
    }

    /// Eat one operator operand under an explicit expression scope.
    ///
    /// Examples:
    /// ```ds
    /// right * other
    /// call(argument)
    /// value as Type
    /// ```
    pub(super) fn eat_value_operand(
        &mut self,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.with_flags(scope.flags, |parser| {
            let start = parser.span_start();

            parser.eat_conditional(&start, scope)
        })
    }

    /// Return whether object literal syntax is valid in statement position.
    pub(crate) fn can_parse_object_literal_in_statement_position(&mut self) -> bool {
        if !self.flags.is_in_statement_position() {
            return true;
        }

        if !self.language.is_destack() || self.flags.is_in_before_block() {
            return false;
        }

        let next_token = self.next_token();
        if next_token.token.ty() == TokenType::Spread {
            return true;
        }

        if next_token.token.ty() == TokenType::OpenBracket {
            return self.bracket_key_starts_statement_object();
        }

        if !self.token_can_start_statement_object_key(next_token.token.ty()) {
            return false;
        }

        if !self.literal_can_start_statement_object_key(
            next_token.token.ty(),
            next_token.token.literal(),
        ) {
            return false;
        }

        let after_key_token_type = self.token_type_at_offset(2);
        matches!(after_key_token_type, TokenType::Colon | TokenType::Maybe)
    }

    /// Return whether a bracket key starts a statement object literal.
    fn bracket_key_starts_statement_object(&mut self) -> bool {
        self.lookahead(|parser| {
            matches!(
                parser.scan_bracket_follow_token_at_offset(1),
                Some(TokenType::Colon | TokenType::Maybe)
            )
        })
    }

    /// Return whether a token can start a statement object key.
    fn token_can_start_statement_object_key(&self, token_type: TokenType) -> bool {
        matches!(token_type, TokenType::Identifier | TokenType::Literal)
    }

    /// Return whether a literal token can start a statement object key.
    fn literal_can_start_statement_object_key(
        &self,
        token_type: TokenType,
        literal: Option<TokenLiteral>,
    ) -> bool {
        if token_type != TokenType::Literal {
            return true;
        }

        matches!(
            literal,
            Some(
                TokenLiteral::String { .. } | TokenLiteral::Int { .. } | TokenLiteral::Float { .. }
            )
        )
    }

    /// Return true when a type-family keyword can begin a type form.
    pub(super) fn keyword_begins_type_form(
        &mut self,
        keyword: Keyword,
        next_token_type: TokenType,
        next_is_on_new_line: bool,
        next_keyword: Option<Keyword>,
        following_token_type: TokenType,
    ) -> bool {
        if keyword == Keyword::Type && next_is_on_new_line {
            return false;
        }

        if keyword == Keyword::Type
            && self.flags.is_in_for_each()
            && matches!(next_keyword, Some(Keyword::In | Keyword::Of))
        {
            return false;
        }

        let is_type_operator_value = keyword == Keyword::Type
            && is_type_relation_keyword(next_keyword)
            && !matches!(
                following_token_type,
                TokenType::Assign | TokenType::LessThan | TokenType::ShiftLeft
            );
        if is_type_operator_value {
            return false;
        }

        if !self.language.is_destack() {
            return next_token_type == TokenType::Identifier;
        }

        matches!(
            next_token_type,
            TokenType::Identifier
                | TokenType::OpenBrace
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::Literal
        )
    }

    /// Return true when a declaration descriptor starts here.
    pub(crate) fn should_parse_declaration_descriptor(&mut self) -> bool {
        self.current_keyword().is_some_and(is_declaration_keyword)
    }

    /// Return true when a using declaration is valid after modifiers.
    pub(crate) fn can_parse_using_declaration(
        &mut self,
        _header: &DeclarationHeader,
        asynchrony: Asynchrony,
    ) -> bool {
        let Some(token_type) = self.using_binding_head_token(asynchrony) else {
            return false;
        };

        self.token_can_start_using_binding_pattern(token_type)
    }

    /// Return whether current keyword starts a do-while statement.
    pub(crate) fn is_do_while_statement(&mut self, next_token_type: TokenType) -> bool {
        if !self.language.is_destack() {
            return true;
        }

        if next_token_type != TokenType::OpenBrace {
            return false;
        }

        self.lookahead(|parser| {
            parser
                .scan_brace_follow_token_at_offset(1)
                .is_some_and(|_| parser.current_keyword() == Some(Keyword::While))
        })
    }

    /// Return whether optional chaining starts after `?`.
    pub(super) fn is_optional_chain_after_question_mark(&mut self) -> bool {
        self.next_token_type() == TokenType::Dot
    }

    /// Return whether a type predicate asserts expression starts here.
    pub(crate) fn can_start_type_predicate_asserts(&mut self) -> bool {
        self.current_keyword() == Some(Keyword::Asserts)
            && (self.token_type_at_offset(1) == TokenType::Identifier
                || self.keyword_at_offset(1) == Some(Keyword::This))
    }

    /// Eat one member name and its span.
    ///
    /// Examples:
    /// ```ds
    /// member
    /// default
    /// true
    /// ```
    pub(crate) fn eat_member_name_with_span(
        &mut self,
    ) -> ParserResult<(destack_core::StringId, Span)> {
        if self.peek_is(TokenType::Literal)
            && matches!(
                self.current_token().token.literal(),
                Some(TokenLiteral::Boolean { .. })
            )
        {
            let span = self.peek()?.span;
            let name = self.strings.intern(self.get_span_str(span));
            self.bump();
            return Ok((name, span));
        }

        self.eat_identifier_with_span()
    }

    /// Return the head span for a type expression.
    pub(crate) fn type_expression_head_span(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> Span {
        self.tree
            .get_head_span(type_expression_id)
            .or_else(|| self.tree.get_main_span(type_expression_id))
            .unwrap_or_else(|| self.tree.get_span(type_expression_id))
    }

    /// Return the head span for an expression.
    pub(crate) fn expression_head_span(&self, expression_id: LocalNodeId<Expression>) -> Span {
        if let Some(span) = self.tree.get_head_span(expression_id) {
            return span;
        }

        match self.tree.get(expression_id) {
            Expression::Type { value } => self.type_expression_head_span(*value),
            Expression::Parenthesized { expression }
            | Expression::As { expression, .. }
            | Expression::Satisfies { expression, .. } => self.expression_head_span(*expression),
            _ => self
                .tree
                .get_main_span(expression_id)
                .unwrap_or_else(|| self.tree.get_span(expression_id)),
        }
    }

    /// Attach pending decorators to an expression.
    pub(in crate::parse::expression) fn attach_pending_decorators_to_expression(
        &mut self,
        decorators: &mut PendingDecorators,
        expression_id: LocalNodeId<Expression>,
    ) {
        if !decorators.is_empty() {
            let owner_id = match self.tree.get(expression_id) {
                Expression::Declaration(declaration_id) => declaration_id.id,
                _ => expression_id.id,
            };
            self.attach_decorators(owner_id, std::mem::take(decorators));
        }
    }

    /// Wrap a default export declaration when leading decorators need an export owner.
    fn wrap_decorated_default_export(
        &mut self,
        start: &ParserSpanStart,
        expression_id: LocalNodeId<Expression>,
        decorators: &PendingDecorators,
    ) -> LocalNodeId<Expression> {
        if decorators.is_empty() {
            return expression_id;
        }

        let Expression::Declaration(declaration_id) = self.tree.get(expression_id) else {
            return expression_id;
        };
        if self.tree.get(*declaration_id).export() != Some(ExportKind::Default) {
            return expression_id;
        }

        self.clear_declaration_export(*declaration_id);
        let item = self.insert_node(
            DependencyItem::Binding {
                binding: DependencyBinding::Default,
                form: Some(DependencyForm::Plain),
                name: None,
                alias: None,
                value: Some(expression_id),
            },
            self.get_span_from(start),
        );

        self.insert_node(
            Expression::Export {
                form: DependencyForm::Plain,
                target: None,
                items: vec![item],
                attributes: None,
            },
            self.get_span_from(start),
        )
    }

    /// Clear the inline export marker from one declaration.
    fn clear_declaration_export(&mut self, declaration_id: LocalNodeId<Declaration>) {
        match self.tree.get_mut(declaration_id) {
            Declaration::Type(declaration) => declaration.export = None,
            Declaration::Struct(declaration) => declaration.export = None,
            Declaration::Class(declaration) => declaration.export = None,
            Declaration::Enum(declaration) => declaration.export = None,
            Declaration::Interface(declaration) => declaration.export = None,
            Declaration::Extension(declaration) => declaration.export = None,
            Declaration::Function(declaration) => declaration.export = None,
            Declaration::Global(_) | Declaration::Module(_) => {}
        }
    }

    /// Attach pending decorators to a type expression.
    pub(crate) fn attach_pending_decorators_to_type_expression(
        &mut self,
        decorators: &mut PendingDecorators,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) {
        if !decorators.is_empty() {
            self.attach_decorators(type_expression_id.id, std::mem::take(decorators));
        }
    }
}
