use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use dyst_ast::{Definition, DefinitionMeta, Keyword, NodeId, NodeType, TokenType};

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
    /// implement Bar<int32> implements Baz {
    ///     ...
    /// }
    ///
    /// implement<T> Bar<T> implements Baz {
    ///     ...
    /// }
    /// ```
    pub fn eat_implement(&mut self, meta: DefinitionMeta) -> ParseResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Implement)?;

        // static parameters
        let static_parameters = self.eat_static_parameters_maybe()?;

        // target type
        let target_type = self
            .with_options(self.options.nested_super_type_in_before_block(), |parser| {
                parser.eat_expression()
            })?;

        // implements types
        let implements_types = self.eat_implements_types_maybe()?;

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let properties = self.eat_properties()?;
        self.eat_token(TokenType::CloseBrace)?;

        // implement
        let implement_id = self.tree.insert(
            Definition::Implement {
                meta,
                static_parameters,
                target_type,
                implements_types,
                with_clauses,
                where_clauses,
                properties,
            },
            self.get_span_from(start),
        );
        Ok(implement_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Argument, BinaryOperator, DeclarationKind, Definition, DefinitionMeta, Expression, IntType,
        Parameter, TypeLiteral, WhereClause, WithClause,
    };

    use crate::parse::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

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

        let implement_id = parser.eat_implement(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, implement_id, Definition::Implement { meta, static_parameters, target_type, implements_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(static_parameters.is_none());
            assert!(implements_types.is_none());
            assert!(where_clauses.is_none());

            // Foo
            assert_node!(parser.tree, *target_type, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Foo");
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

        let implement_id = parser.eat_implement(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, implement_id, Definition::Implement { meta, static_parameters, target_type, implements_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(static_parameters.is_none());
            assert!(implements_types.is_none());
            assert!(where_clauses.is_none());

            // Foo<int32>
            assert_node!(parser.tree, *target_type, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Foo");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { modifiers: None, value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                        assert_eq!(*width, Some(32));
                        assert!(*is_signed);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_implement_with_implements_type() {
        let mut test = TestParser::new(
            r###"
implement Bar<int32> implements Baz {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, implement_id, Definition::Implement { meta, static_parameters, target_type, implements_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(static_parameters.is_none());
            assert!(where_clauses.is_none());

            // Bar<int32>
            assert_node!(parser.tree, *target_type, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Bar");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { modifiers: None, value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                        assert_eq!(*width, Some(32));
                        assert!(*is_signed);
                    });
                });
            });

            let implements = implements_types.as_ref().expect("expected implements types");
            assert_eq!(implements.len(), 1);
            assert_node!(parser.tree, implements[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });
        });
    }

    #[test]
    fn test_parse_implement_with_static_parameters() {
        let mut test = TestParser::new(
            r###"
implement<U> Bar<T> implements Baz<T> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, implement_id, Definition::Implement { meta, static_parameters, target_type, implements_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(where_clauses.is_none());

            // implement<U>
            let static_parameters = static_parameters.as_ref().expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { modifiers: None, name, ty, default } => {
                assert_string!(parser, *name, "U");
                assert!(ty.is_none());
                assert!(default.is_none());
            });

            // Bar<T>
            assert_node!(parser.tree, *target_type, Expression::Path { path, static_arguments } => {
                // Bar
                assert_path!(parser, *path, "Bar");
                // <T>
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: None, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });

            // : Baz<T>
            let implements = implements_types.as_ref().expect("expected implements types");
            assert_eq!(implements.len(), 1);
            assert_node!(parser.tree, implements[0], Expression::Path { path, static_arguments } => {
                // Baz
                assert_path!(parser, *path, "Baz");
                // <T>
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: None, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_implement_with_with_and_where() {
        let mut test = TestParser::new(
            r###"
implement Foo with Context where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let implement_id = parser.eat_implement(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, implement_id, Definition::Implement { meta, with_clauses, where_clauses, target_type, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);

            // with Context
            let with_items = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_items.len(), 1);
            assert_node!(parser.tree, with_items[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });

            // where Guard > Limit
            let where_items = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);
            assert_node!(parser.tree, where_items[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expr_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expr_path!(parser, parser.tree.get(*right), "Limit");
                });
            });

            // Foo target_type
            assert_node!(parser.tree, *target_type, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Foo");
            });
        });
    }
}
