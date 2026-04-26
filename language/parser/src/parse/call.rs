use destack_ast::{
    Expression, GenericArgument, Keyword, LocalNodeId, NodeType, PostfixPosition, TokenType,
    TypeExpression,
};
use destack_source::Span;

use crate::{ParseResult, Parser};

impl Parser {
    /// Eat one type-space bracket postfix.
    pub(crate) fn eat_type_index(
        &mut self,
        receiver_id: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();
        let receiver_span = self.tree.get_span(receiver_id);

        // open bracket
        self.eat_token(TokenType::OpenBracket)?;

        // bare brackets in type positions mean array type form: `T[]`
        if self.peek_is(TokenType::CloseBracket) {
            self.bump(); // eat close bracket
            let element = receiver_id;

            let array_expression = TypeExpression::Array { element };
            let index_span = self.get_span_from(&start);
            let span = Span::new(index_span.file, receiver_span.start, index_span.end);
            let array_id = self.insert_node(array_expression, span);

            return Ok(array_id);
        }

        // missing index
        let is_missing_index = Self::is_expression_slot_boundary_token(self.peek_token_type());
        let index = if is_missing_index {
            self.recover_missing_type_expression_here(NodeType::TypeExpression)
        } else {
            let index_options = self.options.nested().in_type();

            self.eat_type_expression_node_or_recover_missing(
                index_options,
                NodeType::TypeExpression,
            )?
        };

        // close bracket
        if !is_missing_index {
            self.eat_close_token_or_recover_missing(
                TokenType::CloseBracket,
                NodeType::TypeExpression,
            )?;
        }

        let left = receiver_id;

        let index_expression = TypeExpression::Index { left, index };
        let index_span = self.get_span_from(&start);
        let span = Span::new(index_span.file, receiver_span.start, index_span.end);
        let index_id = self.insert_node(index_expression, span);

        Ok(index_id)
    }

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
    ) -> ParseResult<LocalNodeId<Expression>> {
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
            self.eat_expression(self.options.nested())?
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

    /// Eat a new constructor call (including the receiver).
    ///
    /// Examples:
    /// ```
    /// new Foo
    /// new Foo()
    /// new Foo(1, 2)
    /// new Foo<T>()
    /// ```
    pub fn eat_new(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::New)?;

        // receiver
        let left = if self.current_token_is_on_new_line() {
            self.recover_missing_expression_here(NodeType::Expression)
        } else {
            let receiver_options = self.options.not_in_position().in_new_receiver();
            self.with_options(receiver_options, |parser| {
                parser.eat_expression_or_recover_missing(parser.options, NodeType::Expression)
            })?
        };

        // hoist generic arguments parsed on the receiver
        let mut generic_arguments = None;
        if let Expression::QualifiedReference {
            generic_arguments: path_arguments,
            ..
        } = self.tree.get_mut(left)
        {
            generic_arguments = Some(std::mem::take(path_arguments));
        }

        // generic arguments: may be empty
        if generic_arguments.is_none() {
            generic_arguments = self.try_eat_generic_arguments(false, true);
        }

        // dynamic arguments: untyped value mode accepts `new Foo` without parentheses
        let arguments = self.eat_dynamic_arguments_maybe()?.unwrap_or_default();

        // call
        let call_id = self.insert_node(
            Expression::New {
                left,
                generic_arguments: generic_arguments.unwrap_or_default(),
                arguments,
            },
            self.get_span_from(&start),
        );
        Ok(call_id)
    }

    /// Eat a delete expression.
    ///
    /// Examples:
    /// ```
    /// delete
    /// delete foo
    /// delete foo.bar
    /// delete foo['result']
    /// ```
    pub fn eat_delete(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Delete)?;

        // value
        let value_options = self.options.not_in_position();
        let value = self.with_options(value_options, |parser| {
            parser.eat_expression(parser.options)
        })?;

        // delete
        let delete_id = self
            .tree
            .insert(Expression::Delete { value }, self.get_span_from(&start));
        Ok(delete_id)
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
    ) -> ParseResult<LocalNodeId<Expression>> {
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
