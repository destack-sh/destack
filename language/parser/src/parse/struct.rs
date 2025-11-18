#![allow(clippy::type_complexity)]

use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use dyst_ast::{
    DeclarationDescriptor, Definition, Generics, Heritage, Keyword, NodeId, NodeType, StructKind,
    TokenType,
};

impl<'a> Parser<'a> {
    /// Eat a struct declaration.
    ///
    /// Examples:
    /// ```
    /// struct {} // empty anonymous struct
    ///
    /// struct { a: int32, b: boolean }
    ///
    /// struct { // anonymous struct (for use as a value)
    ///     myField: int32 // colon optional
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Bar {
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Foo<T> extends Baz { // Foo has a Baz
    ///     myField: int32
    ///     myOtherField: T
    ///
    ///     static x: int32 = 7 // constant
    ///
    ///     ..Bar // Foo has a Bar
    ///
    ///     myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    pub fn eat_struct(
        &mut self,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        let keyword = self
            .eat_keyword_in(&[Keyword::Struct, Keyword::Class])
            .for_node_type(NodeType::Definition)?;
        let kind = match keyword {
            Keyword::Struct => StructKind::Struct,
            Keyword::Class => StructKind::Class,
            _ => unreachable!(),
        };

        // optional name / key
        descriptor = descriptor.with_name_maybe(self.eat_name_maybe()?);

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Definition)?;

        // optional extends types
        let extends_types = self
            .eat_extends_types_maybe()
            .for_node_type(NodeType::Definition)?;

        // optional implements types
        let implements_types = self
            .eat_implements_types_maybe()
            .for_node_type(NodeType::Definition)?;

        // with
        let with_clauses = self
            .eat_with_header_maybe()
            .for_node_type(NodeType::Definition)?;
        // where
        let where_clauses = self.eat_where_maybe().for_node_type(NodeType::Definition)?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let properties = self
            .with_options(self.options.nested().in_variant(), |parser| {
                parser.eat_properties()
            })
            .for_node_type(NodeType::Definition)?;
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;

        // struct
        let generics = Generics::new(static_parameters, with_clauses, where_clauses);
        let heritage = Heritage::new(extends_types, implements_types);
        let struct_id = self.tree.insert(
            Definition::Struct {
                descriptor,
                kind,
                generics,
                heritage,
                properties,
            },
            self.get_span_from(start),
        );

        Ok(struct_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        BinaryOperator, BindingKind, DeclarationDescriptor, DeclarationKind, Definition,
        Expression, IntType, Key, Mutability, Name, Parameter, Property, ScalarLiteral,
        TypeLiteral, Visibility, WhereClause, WithClause,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_struct_anonymous() {
        let mut test = TestParser::new(
            r###"
struct { public x: int32, readonly y: boolean
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // struct { x: int32, y: boolean }
        let struct_id = parser.eat_struct(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { descriptor, generics, properties, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(descriptor.name.is_none());
            assert!(generics.is_empty());
            assert_eq!(properties.len(), 2);

            // public x: int32
            assert_node!(parser.tree, properties[0], Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert!(modifiers.mutability.is_none());
                assert_eq!(*modifiers.visibility.as_ref().unwrap(), Visibility::Public);
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });

            // readonly y: boolean
            assert_node!(parser.tree, properties[1], Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert_eq!(modifiers.mutability.unwrap(), Mutability::Immutable);
                assert!(modifiers.visibility.is_none());
                assert_string!(parser, *name, "y");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_extends_types() {
        let mut test = TestParser::new(
            r###"
struct Foo extends Bar {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { descriptor, heritage, properties, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(properties.is_empty());
            assert!(!heritage.is_empty());
            assert!(heritage.implements_types.is_none());

            let supers = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_struct_with_spread() {
        let mut test = TestParser::new(
            r###"
struct Foo<T: Numeric> extends Boz implements Quux {
    ...Bar
    ...Baz
    
    a: T
    b?: T
    c: T?
    private d: int32 = 4
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { descriptor, generics, heritage, properties, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(!generics.is_empty());

            // T: Numeric
            let static_parameters = generics
                .static_parameters
                .as_ref()
                .expect("expected static parameters");
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                // T
                assert_string!(parser, *name, "T");
                // Numeric
                assert!(ty.is_some());
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Numeric");
                });
            });
            assert!(!heritage.is_empty());

            // Boz
            let extends_types = heritage.extends_types.as_ref().unwrap();
            assert_eq!(extends_types.len(), 1);
            assert_node!(parser.tree, extends_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Boz");
            });

            let implements_types = heritage
                .implements_types
                .as_ref()
                .expect("expected implements types");
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Quux");
            });

            assert_eq!(properties.len(), 6);

            // ..Bar
            assert_node!(parser.tree, properties[0], Property::Spread { modifiers: None, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Bar");
            });
            // ..Baz
            assert_node!(parser.tree, properties[1], Property::Spread { modifiers: None, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Baz");
            });
            // a: T
            assert_node!(parser.tree, properties[2], Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // b?: T
            assert_node!(parser.tree, properties[3], Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert_eq!(modifiers.kind.unwrap(), BindingKind::Maybe);
                assert_string!(parser, *name, "b");
                assert_expression_path!(parser, parser.tree.get(*ty), "T");
            });
            // c: T?
            assert_node!(parser.tree, properties[4], Property::Field { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *ty, Expression::Maybe { left, position: _ } => {
                    assert_node!(parser.tree, *left, Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
            // private d: int32 = 4
            assert_node!(parser.tree, properties[5], Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), default: Some(value), .. } => {
                assert_eq!(modifiers.visibility.unwrap(), Visibility::Private);
                assert_string!(parser, *name, "d");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_with_and_where() {
        let mut test = TestParser::new(
            r###"
struct Foo with Context where Guard > Limit {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { descriptor, generics, properties, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(properties.is_empty());
            assert!(!generics.is_empty());

            // with Context
            let with_clauses = generics.with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_clauses.len(), 1);
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });
            // where Guard > Limit
            let where_clauses = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expression_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expression_path!(parser, parser.tree.get(*right), "Limit");
                });
            });
        });
    }

    #[test]
    fn test_parse_struct_with_private_member_function() {
        let mut test = TestParser::new(
            r###"
struct Foo {
    private enqueue<M extends F<"mutation">>() {
        throw new Error("Not implemented");
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(DeclarationDescriptor::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { descriptor, properties, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_eq!(properties.len(), 1);
        });
    }
}
