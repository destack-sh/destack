use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, DeclarationDescriptor, Generics, Heritage, Keyword, LocalNodeId, NodeType,
    TokenType,
};
use destack_source::NodeSpanType;

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
        start: &ParserMark,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // keyword
        self.eat_keyword(Keyword::Extension)?;

        // name and static parameters
        let (static_parameters, name, name_span) =
            // named extension
            if self.peek_name_is() && !self.is_keyword(Keyword::For) {
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
        let target_start = self.mark_span();
        let target_type =
            self.eat_expression(self.options.nested().in_super_type().in_before_block())?;

        // record the full type span for the target type
        self.tree.set_side_span(
            target_type,
            NodeSpanType::Type,
            self.get_span_from(&target_start),
        );

        // implements types
        let implements_types = self.eat_implements_types_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        let body_cursor = self.normalize_to_scanner_cursor();
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        self.eat_newlines_maybe()?;
        let members = self.eat_members(false)?;
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

        // attach declaration header-to-body boundary annotations before `{`
        self.attach_boundary(
            body_cursor.index,
            body_cursor.skipped_newline_count.saturating_add(1),
            extension_id.id,
            crate::parse::annotation::AnnotationBoundaryKind::Infix,
        );

        Ok(extension_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, Declaration, DeclarationDescriptor, DeclarationKind, Expression, IntType,
        Parameter, TypeLiteral, WhereClause,
    };
    use destack_source::NodeSpanType;

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
            .eat_extension(&start, DeclarationDescriptor::default())
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
    fn test_parse_extension_rejects_comma_separated_members() {
        let mut test = TestParser::new(
            r###"
extension for Foo {
    value: int32,
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let result = parser.eat_extension(&start, DeclarationDescriptor::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_extension_target_type_span() {
        let mut test = TestParser::new("extension for Foo.Bar {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationDescriptor::default())
            .unwrap();

        // target type span
        assert_node!(parser.tree, extension_id, Declaration::Extension { target_type, .. } => {
            let span = parser
                .tree
                .get_side_span(*target_type, NodeSpanType::Type)
                .expect("expected target type span");
            assert_eq!(parser.get_span_str(span), "Foo.Bar");
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
            .eat_extension(&start, DeclarationDescriptor::default())
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
            .eat_extension(&start, DeclarationDescriptor::default())
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
            .eat_extension(&start, DeclarationDescriptor::default())
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
            .eat_extension(&start, DeclarationDescriptor::default())
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
extension for Foo where Guard: Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension { descriptor, generics, target_type, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(!generics.is_empty());

            // where Guard: Limit
            let where_items = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);
            assert_node!(parser.tree, where_items[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });

            // Foo target_type
            assert_node!(parser.tree, *target_type, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Foo");
            });
        });
    }
}
