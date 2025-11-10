#![allow(clippy::type_complexity)]

use crate::parse::prelude::*;
use crate::{Parser, ParserResult};

use dyst_ast::{
    Definition, DefinitionMeta, Keyword, NodeId, NodeType, StructKind, TokenType, VariantFormat,
};

impl<'a> Parser<'a> {
    /// Eat a struct declaration.
    ///
    /// Examples:
    /// ```
    /// struct {} // empty anonymous struct
    ///
    /// struct A() // unit struct (no fields)
    ///
    /// struct Number(int32) // tuple struct (1 field)
    ///
    /// struct Number(int32, isAwesome: boolean) { // tuple struct (2 fields)
    ///     ...
    /// }
    ///
    /// struct { a: int32, b: boolean }
    ///
    /// struct { // anonymous struct (for use as a value)
    ///     myField: int32 // colon optional
    ///     myOtherField: boolean
    /// }
    ///
    /// struct(uint64) Bar { // 64-bit representation
    ///     myField: int32
    ///     myOtherField: boolean
    /// }
    ///
    /// struct Foo<T> extends Baz { // Foo has a Baz
    ///     myField: int32
    ///     myOtherField: T
    ///
    ///     const x: int32 = 7 // constant
    ///
    ///     ..Bar // Foo has a Bar
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    pub fn eat_struct(&mut self, mut meta: DefinitionMeta) -> ParserResult<NodeId<Definition>> {
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

        // optional representation type: ( ... )
        let representation_type = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            let representation_type = self.eat_expression().for_node_type(NodeType::Definition)?;
            self.eat_token(TokenType::CloseParenthesis)?;
            Some(representation_type)
        } else {
            None
        };

        // optional name / key
        meta = meta.with_name_or_key_maybe(self.eat_name_or_key_maybe()?);

        // format
        let (format, tuple_fields) = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            let tuple_fields = self
                .eat_variant_body_fields()
                .for_node_type(NodeType::Definition)?;
            self.eat_token(TokenType::CloseParenthesis)
                .for_node_type(NodeType::Definition)?;
            (VariantFormat::Tuple, Some(tuple_fields))
        } else {
            (VariantFormat::Struct, None)
        };

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
        let (mut fields, expressions) = self
            .eat_variant_body_mixed(format == VariantFormat::Struct)
            .for_node_type(NodeType::Definition)?;
        if let Some(tuple_fields) = tuple_fields {
            // merge in tuple fields
            fields.extend(tuple_fields);
        }
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;

        let struct_id = self.tree.insert(
            Definition::Struct {
                meta,
                kind,
                format,
                extends_types,
                implements_types,
                static_parameters,
                representation_type,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            },
            self.get_span_from(start),
        );

        Ok(struct_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        BinaryOperator, BindingKind, DeclarationKind, Definition, DefinitionMeta, Expression,
        Field, IntType, Mutability, Name, Parameter, StructKind, TypeLiteral, VariantFormat,
        Visibility, WhereClause, WithClause,
    };

    use crate::parse::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

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
        let struct_id = parser.eat_struct(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { meta, static_parameters, fields, expressions, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(meta.name.is_none());
            assert_eq!(*static_parameters, None);
            assert!(expressions.is_empty());
            assert_eq!(fields.len(), 2);
            assert!(where_clauses.is_none());

            // public x: int32
            assert_node!(parser.tree, fields[0], Field::Named { modifiers: Some(modifiers), name: Name::Identifier(name), ty, default, .. } => {
                assert!(modifiers.mutability.is_none());
                assert_eq!(*modifiers.visibility.as_ref().unwrap(), Visibility::Public);
                assert_string!(parser, *name, "x");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });

            // readonly y: boolean
            assert_node!(parser.tree, fields[1], Field::Named { modifiers: Some(modifiers), name: Name::Identifier(name), ty, default, .. } => {
                assert!(*modifiers.mutability.as_ref().unwrap() == Mutability::Immutable);
                assert!(modifiers.visibility.is_none());
                assert_string!(parser, *name, "y");
                assert!(default.is_none());
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

        let struct_id = parser.eat_struct(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { meta, extends_types, implements_types, fields, expressions, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(expressions.is_empty());
            assert!(fields.is_empty());
            assert!(where_clauses.is_none());

            assert!(implements_types.is_none());

            let supers = extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_struct_with_tuple_format() {
        let mut test = TestParser::new(
            r###"
struct Foo(int32, public boolean) {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { meta, kind, format, fields, expressions, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert_eq!(*kind, StructKind::Struct);
            assert_eq!(*format, VariantFormat::Tuple);
            assert_eq!(fields.len(), 2);
            assert!(expressions.is_empty());
            assert!(where_clauses.is_none());

            // int32
            assert_node!(parser.tree, fields[0], Field::Positional { modifiers: None, ty, default, .. } => {
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });

            // public boolean
            assert_node!(parser.tree, fields[1], Field::Positional { modifiers: Some(modifiers), ty, default, .. } => {
                assert_eq!(*modifiers.visibility.as_ref().unwrap(), Visibility::Public);
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_struct_kind() {
        let mut test = TestParser::new(
            r###"
struct Foo<T: Numeric> extends Boz implements Quux {
    ..Bar
    ..Baz
    
    public static const x: int32 = 4

    a: T
    b?: T
    c: T?
    private d: int32 = 4

    private static function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { meta, static_parameters, fields, expressions, extends_types, implements_types, where_clauses, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_string!(parser, meta.name.unwrap().string(), "Foo");
            assert!(where_clauses.is_none());

            // T: Numeric
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
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

            // Boz
            assert!(extends_types.is_some());
            let extends_types = extends_types.as_ref().unwrap();
            assert_eq!(extends_types.len(), 1);
            assert_node!(parser.tree, extends_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Boz");
            });

            assert!(implements_types.is_some());
            let implements_types = implements_types.as_ref().unwrap();
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Quux");
            });

            assert_eq!(fields.len(), 4);
            // a: T
            assert_node!(parser.tree, fields[0], Field::Named { modifiers: None, name: Name::Identifier(name), ty, default, .. } => {
                assert_string!(parser, *name, "a");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // b?: T
            assert_node!(parser.tree, fields[1], Field::Named { modifiers: Some(modifiers), name: Name::Identifier(name), ty, default, .. } => {
                assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                assert_string!(parser, *name, "b");
                assert!(default.is_none());
                assert_expr_path!(parser, parser.tree.get(*ty), "T");
            });
            // c: T?
            assert_node!(parser.tree, fields[2], Field::Named { modifiers: None, name: Name::Identifier(name), ty, default, .. } => {
                assert_string!(parser, *name, "c");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::Maybe { left, position: _ } => {
                    assert_node!(parser.tree, *left, Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
            // private d: int32 = 4
            assert_node!(parser.tree, fields[3], Field::Named { modifiers: Some(modifiers), name: Name::Identifier(name), ty, default, .. } => {
                assert_eq!(*modifiers.visibility.as_ref().unwrap(), Visibility::Private);
                assert_string!(parser, *name, "d");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                assert!(default.is_some());
            });

            assert_eq!(expressions.len(), 4);
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

        let struct_id = parser.eat_struct(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { meta, with_clauses, where_clauses, fields, expressions, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert!(fields.is_empty());
            assert!(expressions.is_empty());

            // with Context
            let with_clauses = with_clauses.as_ref().expect("expected with clauses");
            assert_eq!(with_clauses.len(), 1);
            assert_node!(parser.tree, with_clauses[0], WithClause { alias: _, right } => {
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "Context");
                    assert!(static_arguments.is_none());
                });
            });

            // where Guard > Limit
            let where_clauses = where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause::Guard { guard } => {
                assert_node!(parser.tree, *guard, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expr_path!(parser, parser.tree.get(*left), "Guard");
                    assert_expr_path!(parser, parser.tree.get(*right), "Limit");
                });
            });
        });
    }

    #[test]
    fn test_parse_struct_with_private_function_shorthand() {
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

        let struct_id = parser.eat_struct(DefinitionMeta::default()).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { meta, fields, expressions, .. } => {
            assert_eq!(meta.kind, DeclarationKind::Definition);
            assert_eq!(fields.len(), 0);
            assert_eq!(expressions.len(), 1);
        });
    }
}
