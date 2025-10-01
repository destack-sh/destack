//! Parse calls, static calls, dynamic calls, etc.

use crate::parse::prelude::*;
use dyst_token::TokenType;

use crate::{
    Call, Cast, Coalesce, Expression, Index, Keyword, NodeId, NodeType, ParseError, ParseResult,
    Parser, Runtime, ScalarLiteral,
};

impl<'a> Parser<'a> {
    /// Eat an explicit index (postfix, excluding the receiver, with `[` and `]`).
    ///
    /// Examples:
    /// ```
    /// []
    /// [1]
    /// [1..3]
    /// ["bar"]
    /// [variable+1]
    /// ```
    pub fn eat_index_postfix_explicit(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Index>> {
        let start = self.mark();

        // open bracket
        self.eat_token(TokenType::OpenBracket)?;

        // bare index
        if self.peek_token(TokenType::CloseBracket).is_ok() {
            self.bump(); // eat close bracket
            let index_id = self.tree.allocate(
                Index::Declarative {
                    receiver: receiver_id,
                },
                self.get_span_from(start),
            );
            return Ok(index_id);
        }

        // expression
        let index = self
            .with_options(self.options.in_nested(), |parser| parser.eat_expression())
            .for_node_type(NodeType::Index)?;

        // close bracket
        self.eat_token(TokenType::CloseBracket)?;

        // index
        let index_id = self.tree.allocate(
            Index::Explicit {
                receiver: receiver_id,
                index,
            },
            self.get_span_from(start),
        );
        Ok(index_id)
    }

    /// Eat an implicit tuple index (postfix, excluding the receiver, with `.`).
    ///
    /// Examples:
    /// ```
    /// .1
    /// ```
    pub fn eat_index_postfix_implicit(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Index>> {
        let start = self.mark();

        // dot
        self.eat_token(TokenType::Dot)?;

        // literal
        let literal_id = self.eat_scalar_literal().for_node_type(NodeType::Index)?;
        let index = match self.tree.get(literal_id) {
            ScalarLiteral::Integer(index) => *index,
            _ => return Err(ParseError::unexpected(self.peek()?.span)),
        };

        // index
        let index_id = self.tree.allocate(
            Index::Member {
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
        runtime: Option<Runtime>,
    ) -> ParseResult<NodeId<Call>> {
        let start = self.mark();

        // dynamic arguments (may be empty)
        let dynamic_arguments = self.eat_dynamic_arguments()?;

        // call
        let call_id = self.tree.allocate(
            Call {
                runtime,
                receiver: receiver_id,
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
        let r#type = self.eat_expression().for_node_type(NodeType::Cast)?;
        let cast_id = self.tree.allocate(
            Cast {
                receiver: receiver_id,
                r#type,
            },
            self.get_span_from(start),
        );
        Ok(cast_id)
    }

    /// Eat a coalesce (postfix, excluding the receiver).
    ///
    /// Examples:
    /// ```
    /// ?? 0
    /// ```
    pub fn eat_coalesce_postfix(
        &mut self,
        receiver_id: NodeId<Expression>,
    ) -> ParseResult<NodeId<Coalesce>> {
        let start = self.mark();
        self.eat_token(TokenType::Coalesce)?;
        let default = self.eat_expression().for_node_type(NodeType::Coalesce)?;
        let coalesce_id = self.tree.allocate(
            Coalesce {
                receiver: receiver_id,
                default,
            },
            self.get_span_from(start),
        );
        Ok(coalesce_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Cast, Expression, Index, IntType, NodeId, Parser, Runtime, ScalarLiteral,
        TypeLiteral, assert_node, assert_string,
    };

    fn make_self_expression(parser: &mut Parser<'_>) -> NodeId<Expression> {
        let self_str = parser.intern_string("self");
        let self_path = parser.intern_path(vec![self_str]);
        parser.tree.allocate(
            Expression::Path {
                path: self_path,
                static_arguments: None,
            },
            parser.peek().unwrap().span,
        )
    }

    #[test]
    fn test_parse_index_postfix_explicit() {
        // [1]
        let mut test = TestParser::new("[1]");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);

        let index_id = parser.eat_index_postfix_explicit(recv).unwrap();
        assert_node!(parser.tree, index_id, Index::Explicit { receiver, index } => {
            assert_eq!(*receiver, recv);
            assert_node!(parser.tree, *index, Expression::ScalarLiteral(lit_id) => {
                assert_node!(parser.tree, *lit_id, ScalarLiteral::Integer(1));
            });
        });
    }

    #[test]
    fn test_parse_index_postfix_implicit() {
        // .1
        let mut test = TestParser::new(".1");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);
        let index_id = parser.eat_index_postfix_implicit(recv).unwrap();
        assert_node!(parser.tree, index_id, Index::Member { receiver, index } => {
            assert_eq!(*receiver, recv);
            assert_eq!(*index, 1);
        });
    }

    #[test]
    fn test_parse_call_postfix() {
        // (1, x: 2)
        let mut test = TestParser::new("(1, x: 2)");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);
        let call_id = parser
            .eat_call_postfix(recv, Some(Runtime::Dynamic))
            .unwrap();

        assert_node!(parser.tree, call_id, crate::Call { receiver, runtime, dynamic_arguments } => {
            assert_eq!(*receiver, recv);
            assert_eq!(*runtime, Some(Runtime::Dynamic));

            // (1, x: 2)
            assert_eq!(dynamic_arguments.len(), 2);

            // 1
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(lit_id) => {
                    assert_node!(parser.tree, *lit_id, ScalarLiteral::Integer(1));
                });
            });

            // x: 2
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Named { name, value } => {
                assert_string!(parser.session, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(lit_id) => {
                    assert_node!(parser.tree, *lit_id, ScalarLiteral::Integer(2));
                });
            });
        });
    }

    #[test]
    fn test_parse_call_postfix_empty() {
        // ()
        let mut test = TestParser::new("()");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);

        let call_id = parser
            .eat_call_postfix(recv, Some(Runtime::Dynamic))
            .unwrap();
        assert_node!(parser.tree, call_id, crate::Call { receiver, runtime, dynamic_arguments } => {
            assert_eq!(*receiver, recv);
            assert_eq!(*runtime, Some(Runtime::Dynamic));
            assert_eq!(dynamic_arguments.len(), 0);
        });
    }

    #[test]
    fn test_parse_as_postfix() {
        // as int32
        let mut test = TestParser::new("as int32");
        let mut parser = test.prepare();
        let recv = make_self_expression(&mut parser);

        let cast_id = parser.eat_as_postfix(recv).unwrap();
        assert_node!(parser.tree, cast_id, Cast { receiver, r#type } => {
            assert_eq!(*receiver, recv);
            assert_node!(parser.tree, *r#type, Expression::TypeLiteral(literal_id) => {
                assert_node!(parser.tree, *literal_id, TypeLiteral::Int(IntType { width: 32, is_signed: true }));
            });
        });
    }
}
