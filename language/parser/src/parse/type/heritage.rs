use crate::{ParseError, ParseResult, Parser};

use destack_ast::{Expression, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

impl Parser {
    /// Return whether one optional heritage keyword is present at the current position.
    fn eat_heritage_keyword_maybe(&mut self, keyword: Keyword) -> ParseResult<bool> {
        if !self.is_keyword(keyword) {
            return Ok(false);
        }

        self.bump(); // eat heritage keyword
        Ok(true)
    }

    /// Return whether the next non-newline token terminates one heritage clause.
    fn newline_before_super_clause_terminator(&mut self, terminators: &[Keyword]) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.peek_is(TokenType::CloseParenthesis)
            || terminators
                .iter()
                .any(|terminator| self.is_keyword(*terminator))
    }

    /// Eat one heritage list with shared separator and recovery rules.
    fn eat_super_list<Item>(
        &mut self,
        terminators: &[Keyword],
        mut eat_item: impl FnMut(&mut Parser) -> ParseResult<Item>,
        mut finish_item: impl FnMut(&mut Parser, &Item, Span, bool) -> ParseResult<()>,
    ) -> ParseResult<Vec<Item>> {
        let mut items = Vec::new();
        let mut expect_item = true;

        // newline alone only separates heritage items in block-value mode
        let allow_newline_separator =
            !(self.language.is_javascript() || self.language.is_typescript());

        while self.has_more_tokens() {
            // clause boundary
            if self.is_super_clause_terminator(terminators) {
                if expect_item && !items.is_empty() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                break;
            }

            // newline separator or newline before the next clause
            if self.current_token_is_on_new_line() {
                if self.newline_before_super_clause_terminator(terminators) {
                    if expect_item && !items.is_empty() {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                    break;
                }

                if !allow_newline_separator && !expect_item {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                if !expect_item {
                    expect_item = true;
                }
            }

            // explicit comma separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop()?;
                expect_item = true;
                continue;
            }

            // generic item separator
            if self.is_item_stop() {
                if self.peek_is(TokenType::End) {
                    break;
                }

                self.eat_item_stop()?;
                expect_item = true;
                continue;
            }

            // next heritage item
            if !expect_item {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let item_start = self.span_start();
            let item_starts_with_parenthesis = self.peek_is(TokenType::OpenParenthesis);
            let item = eat_item(self)?;
            let item_span = self.get_span_from(&item_start);
            finish_item(self, &item, item_span, item_starts_with_parenthesis)?;

            items.push(item);
            expect_item = false;
        }

        Ok(items)
    }

    /// Eat one optional extends type clause.
    pub fn eat_extends_types_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        if !self.eat_heritage_keyword_maybe(Keyword::Extends)? {
            return Ok(None);
        }

        self.eat_super_type_list_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
    }

    /// Eat one optional extends expression clause.
    #[inline]
    pub fn eat_extends_expressions_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        if !self.eat_heritage_keyword_maybe(Keyword::Extends)? {
            return Ok(None);
        }

        self.eat_super_expression_list_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
    }

    /// Eat one optional implements type clause.
    #[inline]
    pub fn eat_implements_types_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        if !self.eat_heritage_keyword_maybe(Keyword::Implements)? {
            return Ok(None);
        }

        self.eat_super_type_list_maybe(&[Keyword::With, Keyword::Where])
    }

    /// Eat one optional type heritage clause body.
    #[inline]
    fn eat_super_type_list_maybe(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        let options = self
            .options
            .not_in_position()
            .in_super_type()
            .not_in_new_receiver()
            .in_type();
        let types = self.with_options(options, |parser| parser.eat_super_type_list(terminators))?;

        Ok(Some(types))
    }

    /// Eat one optional value heritage clause body.
    #[inline]
    fn eat_super_expression_list_maybe(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        let options = self
            .options
            .not_in_position()
            .in_super_type()
            .not_in_new_receiver()
            .not_in_type();
        let types = self.with_options(options, |parser| {
            parser.eat_super_expression_list(terminators)
        })?;

        Ok(Some(types))
    }

    /// Return true when the current token terminates one heritage clause.
    #[inline]
    fn is_super_clause_terminator(&mut self, terminators: &[Keyword]) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.peek_is(TokenType::CloseParenthesis)
            || terminators
                .iter()
                .any(|terminator| self.is_keyword(*terminator))
    }

    /// Normalize one type heritage entry.
    fn normalize_super_type_expression(
        &mut self,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        let mut expression_id = expression_id;

        // parenthesized declaration heads are not distinct in heritage lists
        while let TypeExpression::Parenthesized { expression } = self.tree.get(expression_id) {
            let inner_expression_id = *expression;

            if !matches!(
                self.tree.get(inner_expression_id),
                TypeExpression::Declaration { .. }
                    | TypeExpression::FunctionTypeDeclaration(_)
                    | TypeExpression::ConstructorTypeDeclaration(_)
            ) {
                break;
            }

            expression_id = inner_expression_id;
        }

        expression_id
    }

    /// Eat type heritage entries.
    fn eat_super_type_list(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Vec<LocalNodeId<TypeExpression>>> {
        self.eat_super_list(
            terminators,
            |parser| {
                let ty = parser.eat_type_expression_node_or_recover_missing(
                    parser.options.in_before_block().in_type(),
                    NodeType::Declaration,
                )?;

                Ok(parser.normalize_super_type_expression(ty))
            },
            |parser, ty, super_type_span, _| {
                parser.tree.set_side_span(
                    *ty,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    super_type_span,
                );

                Ok(())
            },
        )
    }

    /// Eat value heritage entries.
    fn eat_super_expression_list(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        self.eat_super_list(
            terminators,
            |parser| parser.eat_expression(parser.options.in_before_block()),
            |parser, ty, super_type_span, item_starts_with_parenthesis| {
                parser.tree.set_side_span(
                    *ty,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    super_type_span,
                );

                if !item_starts_with_parenthesis
                    && parser.super_type_has_invalid_unparenthesized_head(*ty)
                {
                    return Err(ParseError::unexpected(parser.tree.get_span(*ty)));
                }

                Ok(())
            },
        )
    }

    /// Return true when a heritage expression starts with an invalid unparenthesized head.
    fn super_type_has_invalid_unparenthesized_head(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        // unparenthesized lambdas are never valid here
        if self.is_unparenthesized_lambda_expression(expression_id) {
            return true;
        }

        // these heads require explicit parentheses
        matches!(
            self.tree.get(expression_id),
            Expression::Unary { .. }
                | Expression::Binary { .. }
                | Expression::If { .. }
                | Expression::Assign { .. }
                | Expression::SequenceExpression { .. }
        ) || self
            .wrapped_type_expression_maybe(expression_id)
            .is_some_and(|value| {
                matches!(
                    self.tree.get(value),
                    TypeExpression::Union { .. }
                        | TypeExpression::Intersection { .. }
                        | TypeExpression::Conditional { .. }
                )
            })
    }
}
