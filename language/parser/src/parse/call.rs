use destack_dir::{
    Expression, GenericArgument, Keyword, LocalNodeId, NodeType, PostfixPosition, TokenType,
};
use destack_source::Span;

use crate::{Parser, ParserResult};

impl Parser {
    /// Eat one value-space explicit index postfix.
    ///
    /// Examples:
    /// ```
    /// []
    /// [1]
    /// ["bar"]
    /// [variable+1]
    /// ```
    pub(crate) fn eat_index(
        &mut self,
        receiver_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        let receiver_span = self.tree.get_span(receiver_id);

        // open bracket
        self.eat_token(TokenType::OpenBracket)?;

        // bare index
        if self.peek_is(TokenType::CloseBracket) {
            self.bump(); // eat close bracket
            let index_span = self.get_span_from(&start);
            let span = Span::new(index_span.file, receiver_span.start, index_span.end);
            let index_id = self.insert_node(
                Expression::Index {
                    position,
                    left: receiver_id,
                    index: None,
                },
                span,
            );
            return Ok(index_id);
        }

        // missing index
        let is_missing_index = Self::is_expression_slot_boundary_token(self.peek_token_type());
        let index = if is_missing_index {
            self.recover_missing_expression_here(NodeType::Expression)
        } else {
            self.eat_expression(self.flags.nested().with_sequence_expression(true))?
        };

        // close bracket
        if !is_missing_index {
            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
        }

        let index_expression = Expression::Index {
            position,
            left: receiver_id,
            index: Some(index),
        };
        let index_span = self.get_span_from(&start);
        let span = Span::new(index_span.file, receiver_span.start, index_span.end);
        let index_id = self.insert_node(index_expression, span);
        Ok(index_id)
    }

    /// Eat a new constructor call.
    ///
    /// Examples:
    /// ```
    /// new Foo
    /// new Foo()
    /// new Foo(1, 2)
    /// new Foo<T>()
    /// new _()
    /// ```
    pub fn eat_new(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::New)?;

        // constructor name
        let ty = if self.current_token_is_on_new_line() {
            self.recover_missing_type_expression_here(NodeType::Expression)
        } else {
            let ty_flags = self.flags.not_in_position().in_type().in_new_receiver();
            self.eat_type_expression_or_recover_missing(ty_flags, NodeType::Expression)?
        };

        // call arguments: untyped value mode accepts `new Foo` without parentheses
        let arguments = self.eat_dynamic_arguments_maybe()?.unwrap_or_default();

        // call
        let call_id = self.insert_node(
            Expression::New { ty, arguments },
            self.get_span_from(&start),
        );
        Ok(call_id)
    }

    /// Eat a call (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// ()
    /// (1, 2, 3)
    /// <int32>(1, 2, 3)
    /// <Validate: false>(1, 2, 3)
    /// (Vector2 {x: 1, y: 2}, (true, 3))
    /// ```
    pub fn eat_call(
        &mut self,
        receiver_id: LocalNodeId<Expression>,
        generic_arguments: Option<Vec<LocalNodeId<GenericArgument>>>,
        position: PostfixPosition,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        let receiver_span = self.tree.get_span(receiver_id);

        // generic arguments from postfix or immediate call form
        let generic_arguments = match generic_arguments {
            Some(generic_arguments) => Some(generic_arguments),
            None => self.eat_generic_arguments_maybe()?,
        };

        // dynamic arguments (may be empty)
        let arguments = self.eat_dynamic_arguments()?;

        // call
        let call_id = self.insert_node(
            Expression::Call {
                position,
                left: receiver_id,
                generic_arguments: generic_arguments.unwrap_or_default(),
                arguments,
            },
            {
                let call_span = self.get_span_from(&start);
                Span::new(call_span.file, receiver_span.start, call_span.end)
            },
        );
        Ok(call_id)
    }
}
