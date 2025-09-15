use crate::parse::ParserOptions;
use crate::{BlockFormat, Implement, Keyword, NodeId, ParseResult, Parser};
use dyst_language_token::TokenType;

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
        self.eat_keyword(Keyword::Implement)?;

        // static arguments
        let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
            self.eat_token(TokenType::LessThan)?;
            let static_arguments = self.with_options(
                ParserOptions {
                    in_static_type: true,
                    ..self.options
                },
                |parser| parser.eat_arguments_body(),
            )?;
            self.eat_token(TokenType::GreaterThan)?;
            Some(static_arguments)
        } else {
            None
        };

        // target
        let receiver = self.eat_type()?;

        // for
        let for_trait = if self.peek_keyword(Keyword::For).is_ok() {
            self.eat_keyword(Keyword::For)?;
            Some(self.eat_type()?)
        } else {
            None
        };

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
        self.eat_newlines_maybe()?;
        let statements = self.eat_block_body(BlockFormat::Explicit)?;
        self.eat_token(TokenType::CloseBrace)?;

        // implement
        let implement_id = self.tree.allocate(
            Implement {
                static_arguments,
                receiver,
                for_trait,
                statements,
            },
            self.get_span_from(start),
        );
        Ok(implement_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Expression, Implement, Type, assert_node, assert_path,
    };

    #[test]
    fn test_parse_implement_simple() {
        let mut test = TestParser::new(
            r###"
implement Foo {
}
"###,
        );
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, statements } => {
            assert!(static_arguments.is_none());
            assert!(for_trait.is_none());
            assert!(statements.is_empty());

            // Foo
            assert_node!(parser.tree, *receiver, Type::Path { path, .. } => {
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
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, statements } => {
            assert!(static_arguments.is_none());
            assert!(for_trait.is_none());
            assert!(statements.is_empty());

            // Foo<int32>
            assert_node!(parser.tree, *receiver, Type::Path { path, static_arguments } => {
                assert_path!(parser.session, *path, "Foo");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Path(path_id) => {
                        assert_path!(parser.session, *path_id, "int32");
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
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, statements } => {
            assert!(static_arguments.is_none());
            assert!(statements.is_empty());

            // Bar<int32>
            assert_node!(parser.tree, *receiver, Type::Path { path, static_arguments } => {
                assert_path!(parser.session, *path, "Bar");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Path(path_id) => {
                        assert_path!(parser.session, *path_id, "int32");
                    });
                });
            });

            // for Baz
            assert_node!(parser.tree, for_trait.unwrap(), Type::Path { path, .. } => {
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
        let mut parser = test.parser();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement().unwrap();
        assert_node!(parser.tree, implement_id, Implement { static_arguments, receiver, for_trait, statements } => {
            assert!(statements.is_empty());

            // <T>
            let static_args = static_arguments.as_ref().expect("expected static arguments");
            assert_eq!(static_args.len(), 1);
            assert_node!(parser.tree, static_args[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::Path(path_id) => {
                    assert_path!(parser.session, *path_id, "T");
                });
            });

            // Bar<T>
            assert_node!(parser.tree, *receiver, Type::Path { path, static_arguments } => {
                // Bar
                assert_path!(parser.session, *path, "Bar");
                // <T>
                let receiver_static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(receiver_static_args.len(), 1);
                assert_node!(parser.tree, receiver_static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Path(path_id) => {
                        assert_path!(parser.session, *path_id, "T");
                    });
                });
            });

            // for Baz<T>
            assert_node!(parser.tree, for_trait.unwrap(), Type::Path { path, .. } => {
                // Baz
                assert_path!(parser.session, *path, "Baz");
                // <T>
                let receiver_static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(receiver_static_args.len(), 1);
                assert_node!(parser.tree, receiver_static_args[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Path(path_id) => {
                        assert_path!(parser.session, *path_id, "T");
                    });
                });
            });
        });
    }
}
