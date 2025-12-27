use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, DeclarationDescriptor, Generics, Heritage, Keyword, LocalNodeId, NodeType,
    TokenType,
};

impl Parser {
    /// Eat an extension (incl. `extension` keyword).
    ///
    /// Examples:
    /// ```
    /// extension for Foo {
    ///     ...
    /// }
    ///
    /// extension MyExt for Foo<int32> {
    ///     ...
    /// }
    ///
    /// extension for Bar<int32> implements Baz {
    ///     ...
    /// }
    ///
    /// extension MyExt<T> for Bar<T> implements Baz {
    ///     ...
    /// }
    ///
    /// extension<T> for Bar<T> implements Baz {
    ///     ...
    /// }
    /// ```
    pub fn eat_extension(
        &mut self,
        start: ParserMark,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // keyword
        self.eat_keyword(Keyword::Extension)?;

        // name and static parameters
        let (static_parameters, name, name_span) =
            // named extension
            if self.peek_name().is_ok() && self.peek_keyword(Keyword::For).is_err() {
                let (name, span) = self.eat_name_with_span()?;
                let static_parameters = self.eat_static_parameters_maybe()?;
                (static_parameters, Some(name), Some(span))
            }
            // anonymous extension
            else {
                let static_parameters = self.eat_static_parameters_maybe()?;
                (static_parameters, None, None)
            };

        descriptor.name = name;

        // `for` keyword (required)
        self.eat_keyword(Keyword::For)?;

        // target type
        let target_type = self.with_options(
            self.options.nested().in_super_type().in_before_block(),
            |parser| parser.eat_expression(),
        )?;

        // implements types
        let implements_types = self.eat_implements_types_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        self.eat_newlines_maybe()?;
        let members = self.eat_members()?;
        self.eat_token(TokenType::CloseBrace)?;

        // extension
        let generics = Generics::new(static_parameters, where_clauses);
        let heritage = Heritage::new(None, implements_types);
        let extension_id = self.tree.insert(
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                members,
            },
            self.get_span_from(start),
        );

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(extension_id, span);
        }

        Ok(extension_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, BinaryOperator, Declaration, DeclarationDescriptor, DeclarationKind, Expression,
        IntType, Parameter, TypeLiteral, WhereClause,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_extension_simple() {
        let mut test = TestParser::new(
            r###"
extension for Foo {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension { descriptor, generics, heritage, target_type, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(generics.is_empty());
            assert!(heritage.is_empty());

            // Foo
            assert_node!(parser.tree, *target_type, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Foo");
            });
        });
    }

    #[test]
    fn test_parse_extension_with_static_arguments_and_alias() {
        let mut test = TestParser::new(
            r###"
extension MyExt for Foo<int32> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension { descriptor, generics, heritage, target_type, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "MyExt");
            assert!(generics.is_empty());
            assert!(heritage.is_empty());

            // Foo<int32>
            assert_node!(parser.tree, *target_type, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Foo");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                        assert_eq!(*width, Some(32));
                        assert!(*is_signed);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_extension_with_implements_type() {
        let mut test = TestParser::new(
            r###"
extension for Bar<int32> implements Baz {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension { descriptor, generics, heritage, target_type, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(generics.is_empty());
            assert!(!heritage.is_empty());

            // Bar<int32>
            assert_node!(parser.tree, *target_type, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Bar");

                let static_args = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_args.len(), 1);
                // int32
                assert_node!(parser.tree, static_args[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width, is_signed })) => {
                        assert_eq!(*width, Some(32));
                        assert!(*is_signed);
                    });
                });
            });

            let implements = heritage
                .implements_types
                .as_ref()
                .expect("expected implements types");
            assert_eq!(implements.len(), 1);
            assert_node!(parser.tree, implements[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });
        });
    }

    #[test]
    fn test_parse_extension_with_static_parameters() {
        let mut test = TestParser::new(
            r###"
extension<U> for Bar<T> implements Baz<T> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension { descriptor, generics, heritage, target_type, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(descriptor.name.is_none()); // anonymous
            assert!(!generics.is_empty());

            // extension<U>
            let static_parameters = generics
                .static_parameters
                .as_ref()
                .expect("expected static parameters");
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
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });

            // : Baz<T>
            assert!(!heritage.is_empty());
            let implements = heritage
                .implements_types
                .as_ref()
                .expect("expected implements types");
            assert_eq!(implements.len(), 1);
            assert_node!(parser.tree, implements[0], Expression::Path { path, static_arguments } => {
                // Baz
                assert_path!(parser, *path, "Baz");
                // <T>
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_extension_named_with_static_parameters() {
        let mut test = TestParser::new(
            r###"
extension MyExt<U> for Bar<T> implements Baz<T> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension { descriptor, generics, heritage, target_type, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "MyExt"); // named
            assert!(!generics.is_empty());

            // MyExt<U>
            let static_parameters = generics
                .static_parameters
                .as_ref()
                .expect("expected static parameters");
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
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });

            // implements Baz<T>
            assert!(!heritage.is_empty());
            let implements = heritage
                .implements_types
                .as_ref()
                .expect("expected implements types");
            assert_eq!(implements.len(), 1);
            assert_node!(parser.tree, implements[0], Expression::Path { path, static_arguments } => {
                // Baz
                assert_path!(parser, *path, "Baz");
                // <T>
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_extension_with_path_name_and_where() {
        let mut test = TestParser::new(
            r###"
extension for Foo where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension { descriptor, generics, target_type, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(!generics.is_empty());

            // where Guard > Limit
            let where_items = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);
            assert_node!(parser.tree, where_items[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expression_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expression_path!(parser, parser.tree.get(*right), "Limit");
                });
            });

            // Foo target_type
            assert_node!(parser.tree, *target_type, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Foo");
            });
        });
    }
}
