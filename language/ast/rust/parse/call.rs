//! Parse calls, static calls, dynamic calls, etc.

use destack_language_token::TokenType;

use crate::{Call, Cast, Expression, FunctionRuntime, Index, Keyword, NodeId, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Eat an index (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// [1]
    /// [1..3]
    /// ["bar"]
    /// [variable+1]
    /// ```
    pub fn eat_index_postfix(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Index>> {
        let start = self.mark();
        self.eat_token(TokenType::OpenBracket)?;
        let index = self.eat_expression()?;
        self.eat_token(TokenType::CloseBracket)?;
        let index_id = self.tree.allocate(
            Index {
                receiver: receiver_id,
                index,
            },
            self.get_span_from(start),
        );
        Ok(index_id)
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
    pub fn eat_call_postfix(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Call>> {
        let start = self.mark();
        // static arguments
        let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
            self.eat_token(TokenType::LessThan)?;
            let static_arguments = self.eat_arguments_body()?;
            self.eat_token(TokenType::GreaterThan)?;
            Some(static_arguments)
        } else {
            None
        };
        // dynamic arguments
        self.eat_token(TokenType::OpenParenthesis)?;
        let dynamic_arguments = self.eat_arguments_body()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        // call
        let call_id = self.tree.allocate(
            Call {
                // todo!: determine / pass function runtime? (lookbehind?)
                runtime: FunctionRuntime::Dynamic,
                receiver: receiver_id,
                static_arguments,
                dynamic_arguments,
            },
            self.get_span_from(start),
        );
        Ok(call_id)
    }

    /// Eat an as cast (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// as int32
    /// as Vector2
    /// as some_module.MyType
    /// ```
    pub fn eat_as_postfix(&mut self, receiver_id: NodeId<Expression>) -> ParseResult<NodeId<Cast>> {
        let start = self.mark();
        self.eat_keyword(Keyword::As)?;
        let r#type = self.eat_type()?;
        let cast_id = self.tree.allocate(
            Cast {
                receiver: receiver_id,
                r#type,
            },
            self.get_span_from(start),
        );
        Ok(cast_id)
    }
}
