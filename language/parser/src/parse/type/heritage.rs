use crate::{Parser, ParserError, ParserResult};

use destack_dir::{Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

impl Parser {
    /// Return whether the next non-newline token terminates one heritage clause.
    fn newline_before_super_clause_terminator(&mut self, terminators: &[Keyword]) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.peek_is(TokenType::CloseParenthesis)
            || terminators
                .iter()
                .any(|terminator| self.is_keyword(*terminator))
    }

    /// Eat one heritage list with shared separator and recovery rules.
    ///
    /// Examples:
    /// ```ds
    /// Base
    /// Base, Other
    /// Base<T> implements Contract
    /// ```
    fn eat_super_list<Item>(
        &mut self,
        terminators: &[Keyword],
        mut eat_item: impl FnMut(&mut Parser) -> ParserResult<Item>,
        mut finish_item: impl FnMut(&mut Parser, &Item, Span, bool) -> ParserResult<()>,
    ) -> ParserResult<Vec<Item>> {
        let mut items = Vec::new();
        let mut expects_item = true;

        // newline alone only separates heritage items in block-value mode
        let allow_newline_separator =
            !(self.language.is_javascript() || self.language.is_typescript());

        while self.has_more_tokens() {
            // clause boundary
            if self.is_super_clause_terminator(terminators) {
                if expects_item && !items.is_empty() {
                    return Err(ParserError::unexpected(self.peek()?.span));
                }
                break;
            }

            // newline separator or newline before the next clause
            if self.current_token_is_on_new_line() {
                if self.newline_before_super_clause_terminator(terminators) {
                    if expects_item && !items.is_empty() {
                        return Err(ParserError::unexpected(self.peek()?.span));
                    }
                    break;
                }

                if !allow_newline_separator && !expects_item {
                    return Err(ParserError::unexpected(self.peek()?.span));
                }
                if !expects_item {
                    expects_item = true;
                }
            }

            // explicit comma separator
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop()?;
                expects_item = true;
                continue;
            }

            // generic item separator
            if self.is_item_stop() {
                if self.peek_is(TokenType::End) {
                    break;
                }

                self.eat_item_stop()?;
                expects_item = true;
                continue;
            }

            // next heritage item
            if !expects_item {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            let item_start = self.span_start();
            let item_starts_with_parenthesis = self.peek_is(TokenType::OpenParenthesis);
            let item = eat_item(self)?;
            let item_span = self.get_span_from(&item_start);
            finish_item(self, &item, item_span, item_starts_with_parenthesis)?;

            items.push(item);
            expects_item = false;
        }

        Ok(items)
    }

    /// Eat one optional extends type clause.
    ///
    /// Examples:
    /// ```ds
    /// extends Base
    /// extends Base<T>, Other
    /// extends (abstract new () => Instance)
    /// ```
    pub fn eat_extends_types_if_present(
        &mut self,
    ) -> ParserResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        if !self.is_keyword(Keyword::Extends) {
            return Ok(None);
        }
        self.bump(); // eat extends

        let flags = self
            .flags
            .not_in_position()
            .in_super_type()
            .not_in_new_receiver()
            .in_type();
        let types = self.with_flags(flags, |parser| {
            parser.eat_super_type_list(&[Keyword::Implements, Keyword::With, Keyword::Where])
        })?;

        Ok(Some(types))
    }

    /// Eat one optional implements type clause.
    ///
    /// Examples:
    /// ```ds
    /// implements Contract
    /// implements First, Second
    /// implements Namespace.Contract<T>
    /// ```
    #[inline]
    pub fn eat_implements_types_if_present(
        &mut self,
    ) -> ParserResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        if !self.is_keyword(Keyword::Implements) {
            return Ok(None);
        }
        self.bump(); // eat implements

        let flags = self
            .flags
            .not_in_position()
            .in_super_type()
            .not_in_new_receiver()
            .in_type();
        let types = self.with_flags(flags, |parser| {
            parser.eat_super_type_list(&[Keyword::With, Keyword::Where])
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
                TypeExpression::Function(_) | TypeExpression::Constructor(_)
            ) {
                break;
            }

            expression_id = inner_expression_id;
        }

        expression_id
    }

    /// Eat type heritage entries.
    ///
    /// Examples:
    /// ```ds
    /// Base
    /// Base<T>, Other
    /// (abstract new () => Instance)
    /// ```
    fn eat_super_type_list(
        &mut self,
        terminators: &[Keyword],
    ) -> ParserResult<Vec<LocalNodeId<TypeExpression>>> {
        self.eat_super_list(
            terminators,
            |parser| {
                let ty = parser.eat_type_expression_or_recover_missing(
                    parser.flags.in_before_block().in_type(),
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
}
