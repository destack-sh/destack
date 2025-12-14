use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use destack_ast::{
    Declaration, DeclarationDescriptor, Generics, Heritage, Keyword, LocalNodeId, NodeType,
    TokenType, TypeKind,
};

impl Parser {
    /// Eat an Interface.
    ///
    /// Interfaces can be structural (default) or nominal (`newtype interface`).
    /// The `kind` parameter determines which.
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
    ///     static x: int32 // constant
    ///     foo() => int32
    ///
    ///     myFunc() { // nested declaration, default implementation
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
    ///
    /// // Nominal interface - requires explicit `implements`
    /// newtype interface Add<T, R = Self> {
    ///     add(other: T): R
    /// }
    ///
    /// // Marker trait - nominal, no methods
    /// newtype interface Send {}
    /// ```
    pub fn eat_interface(
        &mut self,
        mut descriptor: DeclarationDescriptor,
        kind: TypeKind,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Interface)
            .for_node_type(NodeType::Declaration)?;

        // optional name / key
        descriptor = descriptor.with_name_maybe(self.eat_name_maybe()?);

        // optional static parameters: < ... >
        let static_parameters = self.eat_static_parameters_maybe()?;

        // optional extends types
        let extends_types = self.eat_extends_types_maybe()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        self.eat_newlines_maybe()?;
        let members = self
            .with_options(self.options.nested().in_variant(), |parser| {
                parser.eat_members()
            })
            .for_node_type(NodeType::Declaration)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;

        // interface
        let generics = Generics::new(static_parameters, where_clauses);
        let heritage = Heritage::new(extends_types, None);
        let interface_id = self.tree.insert(
            Declaration::Interface {
                descriptor,
                kind,
                generics,
                heritage,
                members,
            },
            self.get_span_from(start),
        );
        Ok(interface_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        BindingKind, Declaration, DeclarationDescriptor, DeclarationKind, Expression, FunctionMode,
        IntType, Key, Member, Mutability, Name, Parameter, ScalarLiteral, TypeKind, TypeLiteral,
        WhereClause,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_interface_anonymous_empty() {
        let mut test = TestParser::new("interface {}");
        let mut parser = test.prepare();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, kind, generics, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_eq!(*kind, TypeKind::Structural);
            assert!(descriptor.name.is_none());
            assert!(generics.is_empty());
            assert!(members.is_empty());
        });
    }

    #[test]
    fn test_parse_interface_with_extends_types() {
        let mut test = TestParser::new("interface Foo extends Bar {}");
        let mut parser = test.prepare();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, kind, generics, heritage, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_eq!(*kind, TypeKind::Structural);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(members.is_empty());
            assert!(generics.is_empty());
            assert!(!heritage.is_empty());

            let supers = heritage.extends_types.as_ref().expect("expected extends types");
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
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, generics, heritage, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(generics.is_empty());

            // extends Baz
            assert!(!heritage.is_empty());
            let supers = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });

            // readonly value: int32
            assert_node!(parser.tree, members[0], Member::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default, .. } => {
                assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                assert!(default.is_none());
            });

            // count: int32 = 4
            assert_node!(parser.tree, members[1], Member::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: Some(value), .. } => {
                assert_string!(parser, *name, "count");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });
    }

    #[test]
    fn test_parse_interface_with_static_parameters() {
        let mut test = TestParser::new("interface Baz<T> {}");
        let mut parser = test.prepare();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Baz");
            let params = generics
                .static_parameters
                .as_ref()
                .expect("expected static params");
            assert_eq!(params.len(), 1);
        });
    }

    #[test]
    fn test_parse_interface_with_where_clause() {
        let mut test = TestParser::new(
            r###"
interface Baz<T> where Requirement: Interface {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Baz");
            assert!(!generics.is_empty());
            let params = generics
                .static_parameters
                .as_ref()
                .expect("expected static params");
            assert_eq!(params.len(), 1);

            // where Requirement: Interface
            let where_items = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_items.len(), 1);
            assert_node!(parser.tree, where_items[0], WhereClause::Assertion { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Interface");
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

    (value: any, ...arguments: any[]): SQL.Result<any>;

    new(): SQL;

    [Symbol.asyncIterator](): AsyncIterableIterator<string>;

    [Symbol.toPrimitive]?(): number;
}"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "SQL");
            assert_eq!(members.len(), 5);

            // <T = any>(value: T): SQL.Result<T>;
            assert_node!(parser.tree, members[0], Member::Method { key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Call));
                let static_parameters = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    .expect("expected static parameters");
                // <T = any>
                assert_eq!(static_parameters.len(), 1);
                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, default, .. } => {
                    assert_string!(parser, *name, "T");
                    assert_node!(parser.tree, default.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
                });
                // value: T
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "T");
                });
                // SQL.Result<T>
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL.Result");
            });

            // (value: any, ...arguments: any[]): SQL.Result<any>;
            assert_node!(parser.tree, members[1], Member::Method { modifiers: None, key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Call));
                // (value: any, ...arguments: any[])
                assert_eq!(signature.dynamic_parameters.len(), 2);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
                });
                // ...arguments: any[]
                assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Variadic { name, ty: Some(ty), .. } => {
                    assert_string!(parser, *name, "arguments");
                    assert_node!(parser.tree, *ty, Expression::Index { .. });
                });
                // SQL.Result<any>;
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL.Result");
            });

            // new(): SQL;
            assert_node!(parser.tree, members[2], Member::Method { modifiers: None, key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::New));
                assert_eq!(signature.dynamic_parameters.len(), 0);
                // SQL
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL");
            });

            // [Symbol.asyncIterator](): AsyncIterableIterator<string>;
            assert_node!(parser.tree, members[3], Member::Method { modifiers: None, key: Some(Key::Expression(key)), signature, .. } => {
                // [Symbol.asyncIterator]
                assert_expression_path!(parser, parser.tree.get(*key), "Symbol.asyncIterator");
                assert_eq!(signature.dynamic_parameters.len(), 0);
                // AsyncIterableIterator<string>
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "AsyncIterableIterator");
            });

            // [Symbol.toPrimitive]?(): number;
            assert_node!(parser.tree, members[4], Member::Method { modifiers: Some(modifiers), key: Some(Key::Expression(key)), signature, .. } => {
                // [Symbol.toPrimitive]
                assert_expression_path!(parser, parser.tree.get(*key), "Symbol.toPrimitive");
                // ?
                assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                assert_eq!(signature.dynamic_parameters.len(), 0);
                // number
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Number));
            });
        });
    }

    #[test]
    fn test_parse_newtype_interface_empty() {
        let mut test = TestParser::new("interface {}");
        let mut parser = test.prepare();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Nominal)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, kind, generics, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_eq!(*kind, TypeKind::Nominal);
            assert!(descriptor.name.is_none());
            assert!(generics.is_empty());
            assert!(members.is_empty());
        });
    }

    #[test]
    fn test_parse_newtype_interface_with_method() {
        let mut test = TestParser::new(
            r#"
interface Add<T, R = Self> {
    add(other: T): R
}
"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let interface_id = parser
            .eat_interface(DeclarationDescriptor::default(), TypeKind::Nominal)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, kind, generics, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_eq!(*kind, TypeKind::Nominal);
            assert_string!(parser, descriptor.name.unwrap().string(), "Add");

            // <T, R = Self>
            let params = generics.static_parameters.as_ref().expect("expected static params");
            assert_eq!(params.len(), 2);

            // add(other: T): R
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "add");
                assert_eq!(signature.dynamic_parameters.len(), 1);
            });
        });
    }
}
