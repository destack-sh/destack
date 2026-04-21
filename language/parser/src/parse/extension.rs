use crate::parse::expression::common::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{Declaration, ExtensionDeclaration, Keyword, LocalNodeId, NodeType, TokenType};
use destack_source::NodeSpanType;

impl Parser {
    /// Eat an extension (incl. `extension` keyword).
    ///
    /// Examples:
    /// ```
    /// extension of Foo {
    ///     ...
    /// }
    ///
    /// extension MyExt of Foo<int32> {
    ///     ...
    /// }
    ///
    /// extension of Bar<int32> implements Baz {
    ///     ...
    /// }
    ///
    /// extension MyExt<T> of Bar<T> implements Baz {
    ///     ...
    /// }
    ///
    /// extension<T> of Bar<T> implements Baz {
    ///     ...
    /// }
    /// ```
    pub(crate) fn eat_extension(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // keyword
        self.eat_keyword(Keyword::Extension)?;

        // name and generic parameters
        let (generic_parameters, name, name_span) =
            // named extension
            if self.peek_name_is() && !self.is_keyword(Keyword::Of) {
                let (name, span) = self.eat_name_with_span()?;
                let generic_parameters = self.eat_generic_parameters_maybe(false)?;
                (generic_parameters, Some(name), Some(span))
            }
            // anonymous extension
            else {
                let generic_parameters = self.eat_generic_parameters_maybe(false)?;
                (generic_parameters, None, None)
            };

        // `of` keyword
        self.eat_keyword(Keyword::Of)?;

        // target type
        let target_start = self.mark_span();
        let target_type = self.eat_type_expression_node_or_recover_missing(
            self.options
                .nested()
                .in_super_type()
                .in_before_block()
                .in_type(),
            NodeType::Declaration,
        )?;

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
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        self.eat_newlines_maybe()?;
        let members = self.eat_members(false)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // extension
        let extension_id = self.insert_node(
            Declaration::Extension(ExtensionDeclaration {
                name,
                export: header.export,
                ambient: header.ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                target_type,
                implements_types: implements_types.unwrap_or_default(),
                members,
            }),
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
        Declaration, ExtensionDeclaration, GenericArgument, GenericParameter, IntType, Member,
        Parameter, TypeExpression, TypeLiteral, WhereClause,
    };
    use destack_source::NodeSpanType;

    use crate::parse::expression::common::DeclarationHeader;
    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_extension_simple() {
        let mut test = TestParser::new(
            r###"
extension of Foo {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { generic_parameters, implements_types, target_type, .. }) => {
            assert!(generic_parameters.is_empty());
            assert!(implements_types.is_empty());

            // Foo
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Foo");
            });
        });
    }

    #[test]
    fn test_parse_extension_rejects_comma_separated_members() {
        let mut test = TestParser::new(
            r###"
extension of Foo {
    value: int32,
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let result = parser.eat_extension(&start, DeclarationHeader::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_extension_target_type_span() {
        let mut test = TestParser::new("extension of Foo.Bar {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();

        // target type span
        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { target_type, .. }) => {
            let span = parser
                .tree
                .get_side_span(*target_type, NodeSpanType::Type)
                .expect("expected target type span");
            assert_eq!(parser.get_span_str(span), "Foo.Bar");
        });
    }

    #[test]
    fn test_parse_extension_with_generic_arguments_and_alias() {
        let mut test = TestParser::new(
            r###"
extension MyExt of Foo<int32> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { name, generic_parameters, implements_types, target_type, .. }) => {
            assert_string!(parser, name.unwrap().string(), "MyExt");
            assert!(generic_parameters.is_empty());
            assert!(implements_types.is_empty());

            // Foo<int32>
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Foo");

                assert_eq!(generic_arguments.len(), 1);
                // int32
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Int(IntType::Arbitrary { width, is_signed }) } => {
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
extension of Bar<int32> implements Baz {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { generic_parameters, implements_types, target_type, .. }) => {
            assert!(generic_parameters.is_empty());
            assert!(!implements_types.is_empty());

            // Bar<int32>
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Bar");

                assert_eq!(generic_arguments.len(), 1);
                // int32
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value: TypeLiteral::Int(IntType::Arbitrary { width, is_signed }) } => {
                            assert_eq!(*width, Some(32));
                            assert!(*is_signed);
                        });
                });
            });

            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });
        });
    }

    #[test]
    fn test_parse_extension_with_generic_parameters() {
        let mut test = TestParser::new(
            r###"
extension<U> of Bar<T> implements Baz<T> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { name, generic_parameters, implements_types, target_type, .. }) => {
            assert!(name.is_none()); // anonymous
            assert_eq!(generic_parameters.len(), 1);

            // extension<U>
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
                assert_string!(parser, *name, "U");
                assert!(constraint.is_none());
                assert!(default.is_none());
            });

            // Bar<T>
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                // Bar
                assert_path!(parser, *path, "Bar");
                // <T>
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
                            assert_path!(parser, *path, "T");
                        });
                });
            });

            // : Baz<T>
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, generic_arguments } => {
                // Baz
                assert_path!(parser, *path, "Baz");
                // <T>
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
                            assert_path!(parser, *path, "T");
                        });
                });
            });
        });
    }

    #[test]
    fn test_parse_extension_named_with_generic_parameters() {
        let mut test = TestParser::new(
            r###"
extension MyExt<U> of Bar<T> implements Baz<T> {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { name, generic_parameters, implements_types, target_type, .. }) => {
            assert_string!(parser, name.unwrap().string(), "MyExt"); // named
            assert_eq!(generic_parameters.len(), 1);

            // MyExt<U>
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
                assert_string!(parser, *name, "U");
                assert!(constraint.is_none());
                assert!(default.is_none());
            });

            // Bar<T>
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                // Bar
                assert_path!(parser, *path, "Bar");
                // <T>
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
                            assert_path!(parser, *path, "T");
                        });
                });
            });

            // implements Baz<T>
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, generic_arguments } => {
                // Baz
                assert_path!(parser, *path, "Baz");
                // <T>
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
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
extension of Foo where Guard: Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { where_clauses, target_type, .. }) => {
            assert_eq!(where_clauses.len(), 1);

            // where Guard: Limit
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });

            // Foo target_type
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Foo");
            });
        });
    }

    #[test]
    fn test_parse_extension_method_with_explicit_this_parameter() {
        let mut test = TestParser::new(
            r###"
extension<T> of Slice<T> {
    indexSet(this: &Slice<T>, i: number, value: T): void {
        undefined!;
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let extension_id = parser
            .eat_extension(&start, DeclarationHeader::default())
            .unwrap();

        assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                assert!(signature.this_parameter.is_some());
                assert_eq!(signature.parameters.len(), 2);

                let this_parameter_id = signature.this_parameter.expect("expected explicit this parameter");
                assert_node!(parser.tree, this_parameter_id, Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "this");
                    assert!(declared_type.is_some());
                });

                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                    assert_string!(parser, *name, "i");
                });
                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }
}
