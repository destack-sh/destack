use crate::{ParseError, ParseResult, Parser};

use destack_ast::{Expression, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::NodeSpanType;

impl Parser {
    /// Eat one optional extends type clause.
    pub fn eat_extends_types_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        // look ahead to `extends`
        let start_index = self.pos_index();
        let extends_index = self.next_non_newline_index_from(start_index);
        if self.token_type_at(extends_index) != TokenType::Identifier
            || self.keyword_for_index(extends_index) != Some(Keyword::Extends)
        {
            return Ok(None);
        }

        // consume newlines before `extends`
        if extends_index != start_index {
            self.eat_newlines_maybe()?;
        }

        // type heritage after `extends`
        self.bump(); // eat extends
        self.eat_super_type_list_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
    }

    /// Eat one optional extends expression clause.
    /// Used by JS and TS class heritage where extends accepts value expressions.
    #[inline]
    pub fn eat_extends_expressions_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Expression>>>> {
        // look ahead to `extends`
        let start_index = self.pos_index();
        let extends_index = self.next_non_newline_index_from(start_index);
        if self.token_type_at(extends_index) != TokenType::Identifier
            || self.keyword_for_index(extends_index) != Some(Keyword::Extends)
        {
            return Ok(None);
        }

        // consume newlines before `extends`
        if extends_index != start_index {
            self.eat_newlines_maybe()?;
        }

        // value heritage after `extends`
        self.bump(); // eat extends
        self.eat_super_expression_list_maybe(&[Keyword::Implements, Keyword::With, Keyword::Where])
    }

    /// Eat one optional implements type clause.
    #[inline]
    pub fn eat_implements_types_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        // look ahead to `implements`
        let start_index = self.pos_index();
        let implements_index = self.next_non_newline_index_from(start_index);
        if self.token_type_at(implements_index) != TokenType::Identifier
            || self.keyword_for_index(implements_index) != Some(Keyword::Implements)
        {
            return Ok(None);
        }

        // consume newlines before `implements`
        if implements_index != start_index {
            self.eat_newlines_maybe()?;
        }

        // type heritage after `implements`
        self.bump(); // eat implements
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
    fn is_super_type_clause_terminator(&mut self, terminators: &[Keyword]) -> bool {
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
        let mut types: Vec<LocalNodeId<TypeExpression>> = Vec::new();
        let mut expect_type = true;

        // JS and TS require explicit separators here
        let allow_newline_separator =
            !(self.language.is_javascript() || self.language.is_typescript());

        while self.has_more_tokens() {
            // clause boundary
            if self.is_super_type_clause_terminator(terminators) {
                if expect_type && !types.is_empty() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                break;
            }

            // newline separator or newline before the next clause
            if self.peek_is(TokenType::Newline) {
                let current_index = self.pos_index();
                let next_index = self.next_non_newline_index_from(current_index);
                let is_terminator_after_newline = self.token_type_at(next_index)
                    == TokenType::OpenBrace
                    || self.token_type_at(next_index) == TokenType::CloseParenthesis
                    || terminators
                        .iter()
                        .any(|terminator| self.keyword_for_index(next_index) == Some(*terminator));
                if is_terminator_after_newline {
                    if expect_type && !types.is_empty() {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                    break;
                }

                self.eat_newlines_maybe()?;

                if !allow_newline_separator && !expect_type {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                if !expect_type {
                    expect_type = true;
                }
                continue;
            }

            // explicit comma separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop_with_newlines()?;
                expect_type = true;
                continue;
            }

            // generic item separator
            if self.is_item_stop() {
                if self.peek_is(TokenType::End) {
                    break;
                }

                self.eat_item_stop_with_newlines()?;
                expect_type = true;
                continue;
            }

            // next heritage type
            if !expect_type {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let type_start = self.mark_span();
            let ty = self.eat_type_expression_node_or_recover_missing(
                self.options.in_before_block().in_type(),
                NodeType::Declaration,
            )?;
            let ty = self.normalize_super_type_expression(ty);
            let super_type_span = self.get_span_from(&type_start);

            self.tree
                .set_side_span(ty, NodeSpanType::Type, super_type_span);

            types.push(ty);
            expect_type = false;
        }

        Ok(types)
    }

    /// Eat value heritage entries.
    fn eat_super_expression_list(
        &mut self,
        terminators: &[Keyword],
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        let mut types: Vec<LocalNodeId<Expression>> = Vec::new();
        let mut expect_type = true;

        // JS and TS require explicit separators here
        let allow_newline_separator =
            !(self.language.is_javascript() || self.language.is_typescript());

        while self.has_more_tokens() {
            // clause boundary
            if self.is_super_type_clause_terminator(terminators) {
                if expect_type && !types.is_empty() {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                break;
            }

            // newline separator or newline before the next clause
            if self.peek_is(TokenType::Newline) {
                let current_index = self.pos_index();
                let next_index = self.next_non_newline_index_from(current_index);
                let is_terminator_after_newline = self.token_type_at(next_index)
                    == TokenType::OpenBrace
                    || self.token_type_at(next_index) == TokenType::CloseParenthesis
                    || terminators
                        .iter()
                        .any(|terminator| self.keyword_for_index(next_index) == Some(*terminator));
                if is_terminator_after_newline {
                    if expect_type && !types.is_empty() {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                    break;
                }

                self.eat_newlines_maybe()?;

                if !allow_newline_separator && !expect_type {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
                if !expect_type {
                    expect_type = true;
                }
                continue;
            }

            // explicit comma separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop_with_newlines()?;
                expect_type = true;
                continue;
            }

            // generic item separator
            if self.is_item_stop() {
                if self.peek_is(TokenType::End) {
                    break;
                }

                self.eat_item_stop_with_newlines()?;
                expect_type = true;
                continue;
            }

            // next heritage expression
            if !expect_type {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let type_start = self.mark_span();
            let ty = self.eat_expression(self.options.in_before_block())?;
            let super_type_span = self.get_span_from(&type_start);
            if self.super_type_has_invalid_unparenthesized_head(ty) {
                return Err(ParseError::unexpected(self.tree.get_span(ty)));
            }

            self.tree
                .set_side_span(ty, NodeSpanType::Type, super_type_span);

            types.push(ty);
            expect_type = false;
        }

        Ok(types)
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
        ) || matches!(
            self.tree.get(expression_id),
            Expression::Type { value }
                if matches!(
                    self.tree.get(*value),
                    TypeExpression::Union { .. }
                        | TypeExpression::Intersection { .. }
                        | TypeExpression::Conditional { .. }
                )
        )
    }
}
