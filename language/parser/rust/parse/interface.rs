use dyst_ast::DefinitionMeta;

use crate::TokenType;

use crate::parse::prelude::*;
use crate::{Definition, Keyword, NodeId, NodeType, Parser, ParserResult};

impl<'a> Parser<'a> {
    /// Eat a Interface.
    ///
    /// Examples:
    /// ```
    /// interface { // anonymous interface
    ///     ...
    /// }
    ///
    /// interface _ {} // explicit anonymous interface (for disambiguation)
    ///
    /// interface Foo: Baz { // Foo extends Baz
    ///     ..Bar
    ///     ..Boz
    ///     
    ///     myField: int32
    ///     myOtherField: boolean | Vector2
    ///     
    ///     const x: int32 // constant
    ///     function foo() => int32
    ///
    ///     function myFunc() { // nested declaration, default implementation
    ///     }
    /// }
    ///
    /// interface Baz<T> {
    ///     ..Bar
    ///
    ///     isThing: true
    ///
    ///     function baz() => T // semicolon optional
    /// }
    /// ```
    pub fn eat_interface(&mut self, mut meta: DefinitionMeta) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword_in(&[Keyword::Interface, Keyword::Trait])
            .for_node_type(NodeType::Definition)?;

        // optional name
        meta.name = self.eat_name_or_wildcard_maybe()?;

        // optional static parameters: < ... >
        let static_parameters = self.eat_static_parameters_maybe()?;

        // optional super types: : ...
        let super_types = self.eat_super_types_maybe()?;

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.eat_token(TokenType::OpenBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let (fields, expressions) = self
            .eat_variant_body_mixed(true)
            .for_node_type(NodeType::Definition)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;

        let interface_id = self.tree.insert(
            Definition::Interface {
                meta,
                static_parameters,
                super_types,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            },
            self.get_span_from(start),
        );
        Ok(interface_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{DeclarationKind, Mutability, Name};

    use crate::parse::tests::TestParser;
    use crate::{
        Definition, DefinitionMeta, Expression, IntType, ScalarLiteral, TypeLiteral, VariantField,
        WhereClause, WithClause, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_interface_anonymous_empty() {
        let mut test = TestParser::new("interface {}");
        let mut parser = test.prepare();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, static_parameters, with_clauses, where_clauses, expressions, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(meta.name.is_none());
            assert!(static_parameters.is_none());
            assert!(with_clauses.is_none());
            assert!(where_clauses.is_none());
            assert!(expressions.is_empty());
        });
    }

    #[test]
    fn test_parse_interface_with_super_types() {
        let mut test = TestParser::new("interface Foo: Bar {}");
        let mut parser = test.prepare();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, super_types, where_clauses, expressions, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_eq!(expressions.len(), 0);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(expressions.is_empty());
            assert!(where_clauses.is_none());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_interface_with_members() {
        let mut test = TestParser::new(
            r###"
interface Foo: Baz {
    readonly value: int32
    count: int32 = 4

    const x: int32 = 4

    function foo() => int32

    woo() => void
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, fields, expressions, super_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert_eq!(fields.len(), 2);
            assert_eq!(expressions.len(), 3);
            assert!(where_clauses.is_none());

            // : Baz
            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });

            // value: int32
            assert_node!(parser.tree, fields[0], VariantField::Named { mutability, visibility, name: Name::Identifier(name), ty, default, .. } => {
                assert!(mutability.is_some());
                assert_eq!(*mutability.as_ref().unwrap(), Mutability::Immutable);
                assert!(visibility.is_none());
                assert_string!(parser, *name, "value");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
            });

            // count: int32 = 4
            assert_node!(parser.tree, fields[1], VariantField::Named { mutability, visibility, name: Name::Identifier(name), ty, default, .. } => {
                assert!(mutability.is_none());
                assert!(visibility.is_none());
                assert_string!(parser, *name, "count");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
                let default_id = default.expect("expected default value");
                assert_node!(parser.tree, default_id, Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
                    assert_eq!(*value, 4);
                });
            });

            // woo() => void
            let expression_id = expressions[2];
            assert_node!(parser.tree, expression_id, Expression::Definition(definition_id) => {
                // woo
                assert_node!(parser.tree, *definition_id, Definition::Function { meta, return_type, .. } => {
                    assert_string!(parser, meta.name.unwrap().string(), "woo");
                    // => void
                    assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
                });
            });
        });
    }

    #[test]
    fn test_parse_interface_with_static_parameters() {
        let mut test = TestParser::new("interface Baz<T> {}");
        let mut parser = test.prepare();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, static_parameters, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Baz");
            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);
        });
    }

    #[test]
    fn test_parse_interface_with_clause() {
        let mut test = TestParser::new(
            r###"
interface Baz<T> with T: Copy where Requirement: Interface {
    function baz() => T
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, static_parameters, with_clauses, where_clauses, expressions, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Baz");

            let params = static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 1);

            // with T: Copy
            let with_items = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_items.len(), 1);
            assert_node!(parser.tree, with_items[0], WithClause { alias, right } => {
                assert_string!(parser, alias.unwrap(), "T");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Copy");
                });
            });

            // where Requirement: Interface
            let where_items = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);
            assert_node!(parser.tree, where_items[0], WhereClause::Assertion { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Interface");
                });
            });

            assert_eq!(expressions.len(), 1);

            // function baz() => T
            let expression_id = expressions[0];
            assert_node!(parser.tree, expression_id, Expression::Definition(definition_id) => {
                assert_node!(parser.tree, *definition_id, Definition::Function { meta, return_type, .. } => {
                    // baz
                    assert_string!(parser, meta.name.unwrap().string(), "baz");
                    // => T
                    let ret = return_type.expect("expected return type");
                    assert_node!(parser.tree, ret, Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            })
        });
    }

    #[test]
    fn test_parse_interface_with_implicit_functions() {
        let mut test = TestParser::new(
            r#"
interface Client {
    onconnect: (this: Client) => void;
    onclose: (this: Client, error: Error) => void;
}
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, fields, expressions, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Client");
            assert_eq!(expressions.len(), 0);
            assert_eq!(fields.len(), 2);
            // onconnect: (this: Client) => void;
            assert_node!(parser.tree, fields[0], VariantField::Named { mutability: None, visibility: None, name: Name::Identifier(name), ty, .. } => {
                assert_string!(parser, *name, "onconnect");
                // (this: Client) => void;
                assert_node!(parser.tree, *ty, Expression::Definition(definition_id) => {
                    assert_node!(parser.tree, *definition_id, Definition::Function { dynamic_parameters, .. } => {
                        assert_eq!(dynamic_parameters.len(), 1);
                    });
                });
            });
            // onclose: (this: Client, error: Error) => void;
            assert_node!(parser.tree, fields[1], VariantField::Named { mutability: None, visibility: None, name: Name::Identifier(name), ty, .. } => {
                assert_string!(parser, *name, "onclose");
                // (this: Client, error: Error) => void;
                assert_node!(parser.tree, *ty, Expression::Definition(definition_id) => {
                    assert_node!(parser.tree, *definition_id, Definition::Function { dynamic_parameters, .. } => {
                        assert_eq!(dynamic_parameters.len(), 2);
                    });
                });
            });
        });
    }
}
