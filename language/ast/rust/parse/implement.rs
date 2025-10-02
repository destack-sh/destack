use crate::parse::prelude::*;
use crate::{BlockFormat, Implement, Keyword, NodeId, NodeType, ParseResult, Parser};
use dyst_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat an implement (incl. `implement` keyword).
    ///
    /// Examples:
    /// ```
    /// implement Foo {
    ///     ...
    /// }
    ///
    /// implement Foo<int32> {
    ///     ...
    /// }
    ///
    /// implement Bar<int32> for Baz {
    ///     ...
    /// }
    ///
    /// implement<T> Bar<T> for Baz {
    ///     ...
    /// }
    /// ```
    pub fn eat_implement(&mut self) -> ParseResult<NodeId<Implement>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Implement)?;

        // static arguments
        let static_arguments = self.eat_static_arguments_maybe()?;

        // receiver
        let receiver = self
            .with_options(self.options.in_before_block(), |parser| {
                parser.eat_expression()
            })
            .for_node_type(NodeType::Implement)?;

        // for
        let for_trait = if self.peek_keyword(Keyword::For).is_ok() {
            self.bump(); // eat for
            let for_trait = self
                .with_options(self.options.in_before_block(), |parser| {
                    parser.eat_expression()
                })
                .for_node_type(NodeType::Implement)?;
            Some(for_trait)
        } else {
            None
        };

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
        self.eat_newlines_maybe()?;
        let expressions = self
            .eat_block_body(BlockFormat::Explicit)
            .for_node_type(NodeType::Implement)?;
        self.eat_token(TokenType::CloseBrace)?;

        // implement
        let implement_id = self.tree.allocate(
            Implement {
                static_arguments,
                receiver,
                for_trait,
                expressions,
            },
            self.get_span_from(start),
        );
        Ok(implement_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{Argument, Expression, Implement, IntType, TypeLiteral, assert_node, assert_path};

    #[test]
    fn test_parse_implement_simple() {
        let mut test = TestParser::new(
            r###"
implement Foo {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, expressions } => {
            assert!(static_arguments.is_none());
            assert!(for_trait.is_none());
            assert!(expressions.is_empty());

            // Foo
            assert_node!(parser.tree, *receiver, Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Foo");
            });
        });
    }

    #[test]
    fn test_parse_implement_with_static_arguments() {
        let mut test = TestParser::new(
            r###"
implement Foo<int32> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, expressions } => {
            assert!(static_arguments.is_none());
            assert!(for_trait.is_none());
            assert!(expressions.is_empty());

            // Foo<int32>
            assert_node!(parser.tree, *receiver, Expression::Path { path, static_arguments } => {
                assert_path!(parser.session, *path, "Foo");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(literal_id) => {
                        assert_node!(parser.tree, *literal_id, TypeLiteral::Int(IntType { width, is_signed }) => {
                            assert_eq!(*width, Some(32));
                            assert!(*is_signed);
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_implement_for_trait() {
        let mut test = TestParser::new(
            r###"
implement Bar<int32> for Baz {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, expressions } => {
            assert!(static_arguments.is_none());
            assert!(expressions.is_empty());

            // Bar<int32>
            assert_node!(parser.tree, *receiver, Expression::Path { path, static_arguments } => {
                assert_path!(parser.session, *path, "Bar");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(literal_id) => {
                        assert_node!(parser.tree, *literal_id, TypeLiteral::Int(IntType { width, is_signed }) => {
                            assert_eq!(*width, Some(32));
                            assert!(*is_signed);
                        });
                    });
                });
            });

            // for Baz
            assert_node!(parser.tree, for_trait.unwrap(), Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Baz");
            });
        });
    }

    #[test]
    fn test_parse_implement_with_generic_parameters() {
        let mut test = TestParser::new(
            r###"
implement<T> Bar<T> for Baz<T> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, expressions } => {
            assert!(expressions.is_empty());

            // <T>
            let static_args = static_arguments.as_ref().expect("expected static arguments");
            assert_eq!(static_args.len(), 1);
            assert_node!(parser.tree, static_args[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                    assert_path!(parser.session, *path, "T");
                });
            });

            // Bar<T>
            assert_node!(parser.tree, *receiver, Expression::Path { path, static_arguments } => {
                // Bar
                assert_path!(parser.session, *path, "Bar");
                // <T>
                let receiver_static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(receiver_static_args.len(), 1);
                assert_node!(parser.tree, receiver_static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser.session, *path, "T");
                    });
                });
            });

            // for Baz<T>
            assert_node!(parser.tree, for_trait.unwrap(), Expression::Path { path, .. } => {
                // Baz
                assert_path!(parser.session, *path, "Baz");
                // <T>
                let receiver_static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(receiver_static_args.len(), 1);
                assert_node!(parser.tree, receiver_static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser.session, *path, "T");
                    });
                });
            });
        });
    }
}
