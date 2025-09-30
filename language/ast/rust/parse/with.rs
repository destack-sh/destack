//! Parse use and with declarations.
use dyst_token::TokenType;

use crate::parse::prelude::*;
use crate::{Keyword, NodeId, NodeType, ParseResult, Parser, With, WithClause};

impl<'a> Parser<'a> {
    /// Eat a with declaration.
    ///
    /// With can declare the use of an item in a scope and refine type bounds.
    ///
    /// Examples:
    /// ```
    /// with T: int32
    /// with Foo
    /// with Foo as Bar
    /// with Foo, Bar
    /// with Foo.Bar
    /// with !Bar
    /// with (
    ///    !Bar,
    ///    Time<float32> // optional comma
    ///    F: Numeric
    /// )
    /// ```
    pub fn eat_with(&mut self) -> ParseResult<NodeId<With>> {
        let start = self.mark();
        self.eat_keyword(Keyword::With)?;
        let with = self.eat_with_body()?;
        self.tree.set_span(with, self.get_span_from(start));
        Ok(with)
    }

    /// Eat the clauses of a `with` declaration (without the `with` keyword).
    pub fn eat_with_body(&mut self) -> ParseResult<NodeId<With>> {
        let start = self.mark();
        let mut clauses: Vec<NodeId<WithClause>> = Vec::new();

        // parenthesized list with newlines
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.eat_token(TokenType::OpenParenthesis)
                .for_node_type(NodeType::With)?;
            self.eat_newlines_maybe()?;
            loop {
                self.eat_newlines_maybe()?;
                if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    break;
                }
                let next_clause = self.eat_with_clause().for_node_type(NodeType::With)?;
                clauses.push(next_clause);
                // optional comma with newlines
                if self.peek_token(TokenType::Comma).is_ok() {
                    self.eat_token(TokenType::Comma)
                        .for_node_type(NodeType::With)?;
                }
            }
            self.eat_token(TokenType::CloseParenthesis)
                .for_node_type(NodeType::With)?;
        }
        // plain list separated by commas
        else {
            loop {
                let clause = self.eat_with_clause().for_node_type(NodeType::With)?;
                clauses.push(clause);
                // required comma
                if self.peek_token(TokenType::Comma).is_ok() {
                    self.eat_token(TokenType::Comma)
                        .for_node_type(NodeType::With)?;
                } else {
                    break;
                }
            }
        }

        let with = self
            .tree
            .allocate(With { clauses }, self.get_span_from(start));
        Ok(with)
    }

    /// Eat a single with clause.
    ///
    /// A clause can be a declaration (`Foo`, `Foo as Bar`, `Foo.Bar as Baz`)
    /// or an assertion (`T: int32`, `Self: geom.Mesh<T>`, `T.Item: Copy`).
    pub fn eat_with_clause(&mut self) -> ParseResult<NodeId<WithClause>> {
        let start = self.mark();

        // first parse the left-hand side type target
        let left = self
            .eat_expression(ExpressionParserOptions::is_before_block())
            .for_node_type(NodeType::WithClause)?;

        // assertion: `T: SomeType`
        if self.peek_colon().is_ok() {
            self.bump(); // eat colon
            let right = self
                .with_options(
                    ParserOptions {
                        in_static_type: true,
                        ..self.options
                    },
                    |parser| parser.eat_expression(ExpressionParserOptions::is_before_block()),
                )
                .for_node_type(NodeType::WithClause)?;
            let clause = self.tree.allocate(
                WithClause::Assertion {
                    target: left,
                    assertion: right,
                },
                self.get_span_from(start),
            );
            Ok(clause)
        }
        // declaration
        else {
            let clause = self.tree.allocate(
                WithClause::Declaration { target: left },
                self.get_span_from(start),
            );
            Ok(clause)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Expression, TypeLiteral, UnaryOperator, With, WithClause, assert_node, assert_path,
    };

    #[test]
    fn test_parse_with_type_assertion() {
        let mut test = TestParser::new("with T: int32");
        let mut parser = test.prepare();
        let with_id = parser.eat_with().unwrap();
        // with T: int32
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "T");
                });
                assert_node!(parser.tree, *assertion, Expression::TypeLiteral(literal_id) => {
                    assert_node!(parser.tree, *literal_id, TypeLiteral::Int(int_ty) => {
                        assert_eq!(int_ty.width, 32);
                        assert!(int_ty.is_signed);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_with_simple_declaration() {
        let mut test = TestParser::new("with Foo");
        let mut parser = test.prepare();
        let with_id = parser.eat_with().unwrap();
        // with Foo
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Foo");
                });
            });
        });
    }

    #[test]
    fn test_parse_with_path_declaration() {
        let mut test = TestParser::new("with Foo.Bar");
        let mut parser = test.prepare();
        let with_id = parser.eat_with().unwrap();
        // with Foo.Bar
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Foo.Bar");
                });
            });
        });
    }

    #[test]
    fn test_parse_with_negated_declaration() {
        let mut test = TestParser::new("with !Bar");
        let mut parser = test.prepare();
        let with_id = parser.eat_with().unwrap();
        // with !Bar
        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::Not);
                    assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                        assert_path!(parser.session, *path, "Bar");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_with_multiple_clauses() {
        let input = "with !Bar, Time, F: Numeric";
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let with_id = parser.eat_with().unwrap();

        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 3);

            // !Bar
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::Not);
                    assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                        assert_path!(parser.session, *path, "Bar");
                    });
                });
            });

            // Time
            assert_node!(parser.tree, clauses[1], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Path { path, static_arguments } => {
                    assert_path!(parser.session, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, clauses[2], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "F");
                });
                assert_node!(parser.tree, *assertion, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Numeric");
                });
            });
        });
    }

    #[test]
    fn test_parse_with_parenthesized_multiline() {
        let input = r##"with (
  !Bar
  Time,
  F: Numeric
)"##;
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let with_id = parser.eat_with().unwrap();

        assert_node!(parser.tree, with_id, With { clauses } => {
            assert_eq!(clauses.len(), 3);

            // !Bar
            assert_node!(parser.tree, clauses[0], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Unary { operator, right } => {
                    assert_eq!(*operator, UnaryOperator::Not);
                    assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                        assert_path!(parser.session, *path, "Bar");
                    });
                });
            });

            // Time
            assert_node!(parser.tree, clauses[1], WithClause::Declaration { target, .. } => {
                assert_node!(parser.tree, *target, Expression::Path { path, static_arguments } => {
                    assert_path!(parser.session, *path, "Time");
                    assert!(static_arguments.is_none());
                });
            });

            // F: Numeric
            assert_node!(parser.tree, clauses[2], WithClause::Assertion { target, assertion } => {
                assert_node!(parser.tree, *target, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "F");
                });
                assert_node!(parser.tree, *assertion, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Numeric");
                });
            });
        });
    }
}
