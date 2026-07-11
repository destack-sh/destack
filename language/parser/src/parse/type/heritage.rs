use crate::parse::context::{FunctionContext, TypeContext, TypeStops};
use crate::{Parser, ParserError, ParserResult};

use destack_dir::{Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse one heritage type list.
    ///
    /// Examples:
    /// ```ds
    /// Base
    /// Base, Other
    /// Base<T> implements Contract
    /// ```
    fn parse_heritage_types(
        &mut self,
        terminators: &[Keyword],
        context: TypeContext,
    ) -> ParserResult<Vec<LocalNodeId<TypeExpression>>> {
        let mut items = Vec::new();
        let mut expects_item = true;

        while self.has_more_tokens() {
            // clause boundary
            if self.peek_heritage_clause_end(terminators) {
                if expects_item && !items.is_empty() {
                    return Err(ParserError::unexpected(self.peek_token_span()));
                }
                break;
            }

            // newline separator or newline before the next clause
            if self.peek_is_on_new_line() {
                if self.peek_heritage_clause_end(terminators) {
                    if expects_item && !items.is_empty() {
                        return Err(ParserError::unexpected(self.peek_token_span()));
                    }
                    break;
                }

                if !expects_item {
                    expects_item = true;
                }
            }

            // explicit comma separator
            if self.peek_is(TokenType::Comma) {
                self.eat_token(TokenType::Comma)?;
                expects_item = true;
                continue;
            }

            // generic item separator
            if self.peek_item_stop() {
                if self.peek_is(TokenType::End) {
                    break;
                }

                self.eat_token(TokenType::Comma)?;
                expects_item = true;
                continue;
            }

            // next heritage item
            if !expects_item {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let item_start = self.mark_parse_start();
            let item = self.parse_type_or_recover_missing(context, NodeType::Declaration)?;
            let item = self.strip_heritage_parentheses(item);
            let item_range = self.range_since(&item_start);
            self.tree
                .set_side_range(item, NodeSpanType::Region(NodeSpanRegion::Type), item_range);

            items.push(item);
            expects_item = false;
        }

        Ok(items)
    }

    /// Parse one optional extends type clause.
    ///
    /// Examples:
    /// ```ds
    /// extends Base
    /// extends Base<T>, Other
    /// extends (abstract new () => Instance)
    /// ```
    pub(crate) fn parse_extends_types_if_present(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        if !self.peek_is_keyword(Keyword::Extends) {
            return Ok(None);
        }
        self.bump();

        let types = self.parse_heritage_types(
            &[Keyword::Implements, Keyword::With, Keyword::Where],
            TypeContext {
                function,
                stops: TypeStops::IMPLEMENTS,
                ..TypeContext::default()
            },
        )?;

        Ok(Some(types))
    }

    /// Parse one optional implements type clause.
    ///
    /// Examples:
    /// ```ds
    /// implements Contract
    /// implements First, Second
    /// implements Namespace.Contract<T>
    /// ```
    #[inline]
    pub(crate) fn parse_implements_types_if_present(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<Option<Vec<LocalNodeId<TypeExpression>>>> {
        if !self.peek_is_keyword(Keyword::Implements) {
            return Ok(None);
        }
        self.bump();

        let types = self.parse_heritage_types(
            &[Keyword::With, Keyword::Where],
            TypeContext {
                function,
                ..TypeContext::default()
            },
        )?;

        Ok(Some(types))
    }

    /// Return true when the current token terminates one heritage clause.
    #[inline]
    fn peek_heritage_clause_end(&self, terminators: &[Keyword]) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.peek_is(TokenType::CloseParenthesis)
            || terminators
                .iter()
                .any(|terminator| self.peek_is_keyword(*terminator))
    }

    /// Strip redundant parentheses from one type heritage entry.
    fn strip_heritage_parentheses(
        &mut self,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        let mut expression_id = expression_id;

        // parenthesized declaration heads are not distinct in heritage lists
        while let TypeExpression::Parenthesized { expression } = self.tree.get(expression_id) {
            let nested_expression = *expression;

            if !matches!(
                self.tree.get(nested_expression),
                TypeExpression::Function(_) | TypeExpression::Constructor(_)
            ) {
                break;
            }

            expression_id = nested_expression;
        }

        expression_id
    }
}
