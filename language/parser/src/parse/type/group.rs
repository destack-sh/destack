use crate::parse::context::{ExpressionContext, TypeContext, TypeMode, TypeStops};
use crate::{ParseStart, Parser, ParserResult};
use destack_dir::{Expression, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use destack_source::{ByteRange, NodeSpanBoundary, NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse one parenthesized type, tuple, or function head.
    pub(super) fn parse_parenthesized_type(
        &mut self,
        start: &ParseStart,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        // empty tuple
        if self.peek_is(TokenType::CloseParenthesis) {
            return Ok(self.parse_empty_tuple_type(start));
        }

        // tuple head
        if self.peek_type_tuple() {
            return self.parse_parenthesized_tuple_type(start, context);
        }

        // first type
        let ty = self.parse_type_or_recover_missing(context.nested(), NodeType::TypeExpression)?;

        // tuple tail
        if self.peek_is(TokenType::Comma) || self.peek_tuple_element_optional() {
            return self.parse_parenthesized_tuple_tail(start, ty, context);
        }

        // grouped type
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(self.retain_type_parentheses(start, ty))
    }

    /// Parse one empty tuple type after its opening parenthesis.
    fn parse_empty_tuple_type(&mut self, start: &ParseStart) -> LocalNodeId<TypeExpression> {
        self.bump();

        self.insert_node(
            TypeExpression::Tuple {
                elements: Vec::new(),
            },
            self.range_since(start),
        )
    }

    /// Parse one parenthesized tuple with an explicit tuple head.
    fn parse_parenthesized_tuple_type(
        &mut self,
        start: &ParseStart,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let elements = self.parse_type_tuple_elements_body(context)?;
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(self.insert_node(TypeExpression::Tuple { elements }, self.range_since(start)))
    }

    /// Parse one parenthesized tuple after its first type.
    fn parse_parenthesized_tuple_tail(
        &mut self,
        start: &ParseStart,
        first: LocalNodeId<TypeExpression>,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let elements = self.parse_type_tuple_tail(start, first, context)?;
        self.eat_close_token_or_recover_missing(
            TokenType::CloseParenthesis,
            NodeType::TypeExpression,
        )?;

        Ok(self.insert_node(TypeExpression::Tuple { elements }, self.range_since(start)))
    }

    /// Retain type parentheses as a node or source region.
    fn retain_type_parentheses(
        &mut self,
        start: &ParseStart,
        ty: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        if self.retains_parentheses() {
            return self.insert_node(
                TypeExpression::Parenthesized { expression: ty },
                self.range_since(start),
            );
        }

        let type_range = self.tree.get_range(ty);
        let leading_range = ByteRange {
            start: start.token_end(),
            end: type_range.start,
        };
        if leading_range.start < leading_range.end {
            self.tree.set_side_range(
                ty,
                NodeSpanType::Boundary(NodeSpanBoundary::Leading),
                leading_range,
            );
        }
        self.extend_node_region_range(ty, NodeSpanRegion::Parentheses, self.range_since(start));

        ty
    }

    /// Return whether the current token starts a constructor type expression.
    pub(super) fn peek_construct_type(&self) -> bool {
        if self.peek_keyword() == Some(Keyword::New) {
            return matches!(
                self.peek_next_token_type(),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        }

        if self.peek_keyword_at(0) == Some(Keyword::Abstract)
            && self.peek_keyword_at(1) == Some(Keyword::New)
        {
            return matches!(
                self.peek_token_type_at(2),
                TokenType::LessThan | TokenType::OpenParenthesis
            );
        }

        false
    }

    /// Return whether the current parenthesis group is a function type head.
    pub(super) fn peek_parenthesized_function_type(&self, context: TypeContext) -> bool {
        if self.peek_token_type() != TokenType::OpenParenthesis {
            return false;
        }

        let Some(follow) =
            self.peek_token_after_group(0, TokenType::OpenParenthesis, TokenType::CloseParenthesis)
        else {
            return false;
        };

        if follow.is(TokenType::Colon) {
            return !context.stops.contains(TypeStops::CONDITIONAL_COLON);
        }
        if !follow.is(TokenType::ArrowWide) {
            return false;
        }

        context.mode != TypeMode::ArrowReturn || self.peek_parenthesized_parameter_list()
    }

    /// Parse one slice or fixed-array type.
    pub(super) fn parse_bracket_type(
        &mut self,
        start: &ParseStart,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.eat_token(TokenType::OpenBracket)?;

        // element type
        let element =
            self.parse_type_or_recover_missing(context.nested(), NodeType::TypeExpression)?;

        // fixed array
        if self.peek_is(TokenType::Semicolon) {
            return self.parse_fixed_array_type(start, element, context);
        }

        // slice close
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(TypeExpression::Slice { element }, self.range_since(start)))
    }

    /// Parse one fixed-array type after its element.
    fn parse_fixed_array_type(
        &mut self,
        start: &ParseStart,
        element: LocalNodeId<TypeExpression>,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.bump();
        let length = if self.peek_identifier_is("_") {
            let length_start = self.mark_parse_start();
            let ty = self.parse_type_infer_hole(&length_start);

            self.insert_node(
                Expression::Type { value: ty },
                self.range_since(&length_start),
            )
        } else {
            self.parse_expression_or_recover_missing(
                ExpressionContext {
                    function: context.function,
                    ..ExpressionContext::default()
                },
                NodeType::Expression,
            )?
        };
        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        Ok(self.insert_node(
            TypeExpression::FixedArray { element, length },
            self.range_since(start),
        ))
    }
}
