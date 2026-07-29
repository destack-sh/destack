use destack_dir::{
    Expression, GenericArgument, Keyword, LocalNodeId, NodeType, PostfixPosition, TokenType,
};
use destack_source::{ByteRange, NodeSpanRegion, NodeSpanType};

use crate::parse::context::{ExpressionContext, TypeContext, TypeMode};
use crate::{Parser, ParserResult};

impl Parser {
    /// Parse one value-space explicit index postfix.
    ///
    /// Examples:
    /// ```ds
    /// []
    /// [1]
    /// ["bar"]
    /// [variable+1]
    /// ```
    pub(crate) fn parse_index(
        &mut self,
        receiver_id: LocalNodeId<Expression>,
        position: PostfixPosition,
        context: ExpressionContext,
        is_optional: bool,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let receiver_range = self.tree.get_range(receiver_id);
        let operator_range = self.peek_token().range();

        // open bracket
        self.eat_token(TokenType::OpenBracket)?;

        // bare index
        if self.peek_is(TokenType::CloseBracket) {
            self.bump();
            let index_range = self.range_since(&start);
            let range = ByteRange {
                start: receiver_range.start,
                end: index_range.end,
            };
            let index_id = self.insert_node(
                Expression::Index {
                    position,
                    left: receiver_id,
                    index: None,
                    is_optional,
                },
                range,
            );
            self.tree.set_main_range(index_id, operator_range);

            return Ok(index_id);
        }

        // missing index
        let is_missing_index = Self::is_expression_slot_boundary_token(self.peek_token_type());
        let index = if is_missing_index {
            self.recover_missing_expression_here(NodeType::Expression)
        } else {
            self.parse_expression(context.nested())?
        };

        // close bracket
        if !is_missing_index {
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
        }

        let index_expression = Expression::Index {
            position,
            left: receiver_id,
            index: Some(index),
            is_optional,
        };
        let index_range = self.range_since(&start);
        let range = ByteRange {
            start: receiver_range.start,
            end: index_range.end,
        };
        let index_id = self.insert_node(index_expression, range);
        self.tree.set_main_range(index_id, operator_range);

        Ok(index_id)
    }

    /// Parse a new constructor call.
    ///
    /// Examples:
    /// ```ds
    /// new Foo
    /// new Foo()
    /// new Foo(1, 2)
    /// new Foo<T>()
    /// new _()
    /// ```
    pub(crate) fn parse_new(
        &mut self,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // keyword
        self.eat_keyword(Keyword::New)?;
        let is_maybe = self.eat_token_if(TokenType::Maybe);

        // constructor name
        let ty = if self.peek_is_on_new_line() {
            self.recover_missing_type_expression_here(NodeType::Expression)
        } else {
            self.parse_type_or_recover_missing(
                TypeContext {
                    function: context.function,
                    mode: TypeMode::NewReceiver,
                    ..TypeContext::default()
                },
                NodeType::Expression,
            )?
        };

        // constructor arguments are optional
        let arguments_start = self.mark_parse_start();
        let arguments = self.parse_arguments_if_present(context.nested())?;
        let arguments_range = arguments
            .as_ref()
            .map(|_| self.range_since(&arguments_start));
        let arguments = arguments.unwrap_or_default();

        // call
        let expression = if is_maybe {
            Expression::NewMaybe { ty, arguments }
        } else {
            Expression::New { ty, arguments }
        };
        let call_id = self.insert_node(expression, self.range_since(&start));
        if let Some(arguments_range) = arguments_range {
            let span_type = NodeSpanType::Region(NodeSpanRegion::Arguments);
            self.tree
                .set_side_range(call_id, span_type, arguments_range);
        }

        Ok(call_id)
    }

    /// Parse a call (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```ds
    /// ()
    /// (1, 2, 3)
    /// <int32>(1, 2, 3)
    /// <Validate: false>(1, 2, 3)
    /// (Vector2 {x: 1, y: 2}, (true, 3))
    /// ```
    pub(crate) fn parse_call(
        &mut self,
        receiver_id: LocalNodeId<Expression>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
        position: PostfixPosition,
        context: ExpressionContext,
        is_optional: bool,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let receiver_range = self.tree.get_range(receiver_id);

        // dynamic arguments (may be empty)
        let arguments = self.parse_argument_list(context.nested())?;
        let arguments_range = self.range_since(&start);

        // call
        let call_id = self.insert_node(
            Expression::Call {
                position,
                left: receiver_id,
                generic_arguments,
                arguments,
                is_optional,
            },
            {
                let call_range = self.range_since(&start);
                ByteRange {
                    start: receiver_range.start,
                    end: call_range.end,
                }
            },
        );
        let span_type = NodeSpanType::Region(NodeSpanRegion::Arguments);
        self.tree
            .set_side_range(call_id, span_type, arguments_range);

        Ok(call_id)
    }
}
