use crate::parse::prelude::*;
use crate::{Parser, ParserResult};

use dyst_ast::{Definition, DefinitionMeta, Keyword, NodeId, NodeType, TokenType};

impl<'a> Parser<'a> {
    /// Eat a Interface.
    ///
    /// Examples:
    /// ```
    /// interface { // anonymous interface
    ///     ...
    /// }
    ///
    /// interface Foo extends Baz { // Foo extends Baz
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
        self.eat_keyword(Keyword::Interface)
            .for_node_type(NodeType::Definition)?;

        // optional name / key
        meta = meta.with_name_maybe(self.eat_name_maybe()?);

        // optional static parameters: < ... >
        let static_parameters = self.eat_static_parameters_maybe()?;

        // optional extends types
        let extends_types = self.eat_extends_types_maybe()?;

        // with
        let with_clauses = self.eat_with_header_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let properties = self.eat_properties().for_node_type(NodeType::Definition)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;

        let interface_id = self.tree.insert(
            Definition::Interface {
                meta,
                static_parameters,
                extends_types,
                with_clauses,
                where_clauses,
                properties,
            },
            self.get_span_from(start),
        );
        Ok(interface_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        DeclarationKind, Definition, DefinitionMeta, Expression, FunctionMode, IntType, Key,
        Mutability, Name, Property, ScalarLiteral, TypeLiteral, WhereClause, WithClause,
    };

    use crate::parse::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_interface_anonymous_empty() {
        let mut test = TestParser::new("interface {}");
        let mut parser = test.prepare();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, static_parameters, with_clauses, where_clauses, properties, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(meta.name.is_none());
            assert!(static_parameters.is_none());
            assert!(with_clauses.is_none());
            assert!(where_clauses.is_none());
            assert!(properties.is_empty());
        });
    }

    #[test]
    fn test_parse_interface_with_extends_types() {
        let mut test = TestParser::new("interface Foo extends Bar {}");
        let mut parser = test.prepare();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, extends_types, where_clauses, properties, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(properties.is_empty());
            assert!(where_clauses.is_none());

            let supers = extends_types.as_ref().expect("expected extends types");
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
interface Foo extends Baz {
    readonly value: int32
    count: int32 = 4

    static x: int32 = 4

    woo() => void
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, properties, extends_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert_eq!(properties.len(), 4);
            assert!(where_clauses.is_none());

            // extends Baz
            let supers = extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });

            // readonly value: int32
            assert_node!(parser.tree, properties[0], Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), ty: Some(ty), value, .. } => {
                assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
                assert_string!(parser, *name, "value");
                assert!(value.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });

            // count: int32 = 4
            assert_node!(parser.tree, properties[1], Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), ty: Some(ty), value: Some(value), .. } => {
                assert_string!(parser, *name, "count");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });

            // woo() => void
            assert_node!(parser.tree, properties[2], Property::Method { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), definition, .. } => {
                assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
                assert_string!(parser, *name, "woo");
                assert_node!(parser.tree, *definition, Definition::Function { meta, return_type, .. } => {
                    assert_string!(parser, meta.name.unwrap().string(), "woo");
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
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, static_parameters, with_clauses, where_clauses, properties, .. } => {
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

            assert_eq!(properties.len(), 1);

            // function baz() => T
            assert_node!(parser.tree, properties[0], Property::Method { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), definition, .. } => {
                assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
                assert_string!(parser, *name, "baz");
                assert_node!(parser.tree, *definition, Definition::Function { meta, return_type, .. } => {
                    assert_string!(parser, meta.name.unwrap().string(), "baz");
                    assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
                });
            });
        });
    }

    #[test]
    fn test_parse_interface_with_nameless_shorthand_functions() {
        let mut test = TestParser::new(
            r#"
interface SQL {
    <T = any>(value: T): SQL.Result<T>;
    
    (value: any, ...arguments: any[]): SQL.Result<any>[];

    new(): SQL;

    [Symbol.asyncIterator](): AsyncIterableIterator<string>;

    [Symbol.toPrimitive]?(): number;
}
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser.eat_interface(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, interface_id, Definition::Interface { meta, properties, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "SQL");
            assert_eq!(properties.len(), 5);

            // <T = any>(value: T): SQL.Result<T>;
            assert_node!(parser.tree, properties[0], Property::Method { modifiers: Some(_), key: None, definition, .. } => {
                assert_node!(parser.tree, *definition, Definition::Function { meta, mode, static_parameters: Some(static_parameters), dynamic_parameters, return_type, .. } => {
                    assert!(meta.name.is_none());
                    assert_eq!(*mode, Some(FunctionMode::Call));
                    assert_eq!(static_parameters.len(), 1);
                    assert_eq!(dynamic_parameters.len(), 1);
                    assert!(return_type.is_some());
                });
            });

            // (value: any, ...arguments: any[]): SQL.Result<any>[];
            assert_node!(parser.tree, properties[1], Property::Method { modifiers: Some(_), key: None, definition, .. } => {
                assert_node!(parser.tree, *definition, Definition::Function { meta, mode, dynamic_parameters, return_type, .. } => {
                    assert!(meta.name.is_none());
                    assert_eq!(*mode, Some(FunctionMode::Call));
                    assert_eq!(dynamic_parameters.len(), 2);
                    assert!(return_type.is_some());
                });
            });

            // new(): SQL;
            assert_node!(parser.tree, properties[2], Property::Method { modifiers: Some(_), key: None, definition, .. } => {
                assert_node!(parser.tree, *definition, Definition::Function { meta, mode, dynamic_parameters, return_type, .. } => {
                    assert!(meta.name.is_none());
                    assert_eq!(*mode, Some(FunctionMode::New));
                    assert_eq!(dynamic_parameters.len(), 0);
                    assert!(return_type.is_some());
                });
            });

            // [Symbol.asyncIterator](): AsyncIterableIterator<string>;
            assert_node!(parser.tree, properties[3], Property::Method { modifiers: Some(_), key: Some(Key::Expression(expression)), definition, .. } => {
                assert_node!(parser.tree, *definition, Definition::Function { meta, mode, dynamic_parameters, return_type, .. } => {
                    assert!(meta.name.is_none());
                    assert_expr_path!(parser, parser.tree.get(*expression), "Symbol.asyncIterator");
                    assert_eq!(*mode, Some(FunctionMode::Call));
                    assert_eq!(dynamic_parameters.len(), 0);
                    assert!(return_type.is_some());
                });
            });

            // [Symbol.toPrimitive]?(): number;
            assert_node!(parser.tree, properties[4], Property::Method { modifiers: Some(_), key: Some(Key::Expression(expression)), definition, .. } => {
                assert_node!(parser.tree, *definition, Definition::Function { meta, mode, dynamic_parameters, return_type, .. } => {
                    assert!(meta.name.is_none());
                    assert_expr_path!(parser, parser.tree.get(*expression), "Symbol.toPrimitive");
                    assert_eq!(*mode, Some(FunctionMode::Call));
                    assert_eq!(dynamic_parameters.len(), 0);
                    assert!(return_type.is_some());
                });
            });
        });
    }
}
