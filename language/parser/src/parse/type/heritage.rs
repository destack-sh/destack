use crate::{Parser, ParserError, ParserResult};

use destack_dir::{
    Expression, InterfaceHeritage, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression,
};
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

    /// Eat one optional interface extends clause.
    ///
    /// Examples:
    /// ```ds
    /// extends Base
    /// extends Base<T>, Other<T>
    /// extends namespace.Base
    /// ```
    #[inline]
    pub fn eat_interface_extends_if_present(
        &mut self,
    ) -> ParserResult<Option<Vec<InterfaceHeritage>>> {
        if !self.is_keyword(Keyword::Extends) {
            return Ok(None);
        }
        self.bump(); // eat extends

        let flags = self
            .flags
            .not_in_position()
            .in_super_type()
            .not_in_new_receiver()
            .not_in_type();
        let extends = self.with_flags(flags, |parser| {
            parser.eat_interface_heritage_list(&[
                Keyword::Implements,
                Keyword::With,
                Keyword::Where,
            ])
        })?;

        Ok(Some(extends))
    }

    /// Eat one optional extends expression clause.
    ///
    /// Examples:
    /// ```ds
    /// extends Base
    /// extends Base<T>, Other
    /// extends mixin(Base)
    /// ```
    #[inline]
    pub fn eat_extends_expressions_if_present(
        &mut self,
    ) -> ParserResult<Option<Vec<LocalNodeId<Expression>>>> {
        if !self.is_keyword(Keyword::Extends) {
            return Ok(None);
        }
        self.bump(); // eat extends

        let flags = self
            .flags
            .not_in_position()
            .in_super_type()
            .not_in_new_receiver()
            .not_in_type();
        let expressions = self.with_flags(flags, |parser| {
            parser.eat_super_expression_list(&[Keyword::Implements, Keyword::With, Keyword::Where])
        })?;

        Ok(Some(expressions))
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

    /// Eat value heritage entries.
    ///
    /// Examples:
    /// ```ds
    /// Base
    /// Base<T>, mixin(Other)
    /// (factory())()
    /// ```
    fn eat_super_expression_list(
        &mut self,
        terminators: &[Keyword],
    ) -> ParserResult<Vec<LocalNodeId<Expression>>> {
        self.eat_super_list(
            terminators,
            |parser| parser.eat_heritage_expression(),
            |parser, ty, super_type_span, item_starts_with_parenthesis| {
                parser.tree.set_side_span(
                    *ty,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    super_type_span,
                );

                if !item_starts_with_parenthesis
                    && parser.super_type_has_invalid_unparenthesized_head(*ty)
                {
                    return Err(ParserError::unexpected(parser.tree.get_span(*ty)));
                }

                Ok(())
            },
        )
    }

    /// Eat interface heritage entries.
    ///
    /// Examples:
    /// ```ds
    /// Base
    /// Base<T>, Other<U>
    /// Namespace.Base
    /// ```
    fn eat_interface_heritage_list(
        &mut self,
        terminators: &[Keyword],
    ) -> ParserResult<Vec<InterfaceHeritage>> {
        self.eat_super_list(
            terminators,
            |parser| {
                let expression = parser.eat_heritage_expression()?;

                Ok(parser.interface_heritage_entry_from_expression(expression))
            },
            |parser, heritage, heritage_span, item_starts_with_parenthesis| {
                parser.tree.set_side_span(
                    heritage.expression,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    heritage_span,
                );

                if !item_starts_with_parenthesis
                    && parser.super_type_has_invalid_unparenthesized_head(heritage.expression)
                {
                    return Err(ParserError::unexpected(
                        parser.tree.get_span(heritage.expression),
                    ));
                }

                Ok(())
            },
        )
    }

    /// Eat one heritage expression head.
    ///
    /// Examples:
    /// ```ds
    /// Base
    /// Base<T>
    /// Namespace.Base
    /// ```
    fn eat_heritage_expression(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let flags = self.flags.in_before_block().not_in_sequence_expression();

        self.eat_expression_at_precedence(flags, u16::MAX)
    }

    /// Build one interface heritage entry from a parsed expression.
    fn interface_heritage_entry_from_expression(
        &self,
        expression: LocalNodeId<Expression>,
    ) -> InterfaceHeritage {
        match self.tree.get(expression) {
            Expression::Instantiation {
                left,
                generic_arguments,
            } => InterfaceHeritage {
                expression: *left,
                generic_arguments: generic_arguments.clone(),
            },
            _ => InterfaceHeritage {
                expression,
                generic_arguments: Vec::new(),
            },
        }
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
        if matches!(
            self.tree.get(expression_id),
            Expression::Unary { .. }
                | Expression::Binary { .. }
                | Expression::If { .. }
                | Expression::Assign { .. }
                | Expression::SequenceExpression { .. }
        ) {
            return true;
        }

        let Expression::Type { value } = self.tree.get(expression_id) else {
            return false;
        };

        matches!(
            self.tree.get(*value),
            TypeExpression::Union { .. }
                | TypeExpression::Intersection { .. }
                | TypeExpression::Conditional { .. }
        )
    }
}
