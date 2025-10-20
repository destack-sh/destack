#![allow(clippy::type_complexity)]

use crate::TokenType;
use crate::parse::prelude::*;

use crate::{
    Definition, ExportMode, Keyword, NodeId, NodeType, Parser, ParserResult, VariantStyle,
    Visibility,
};

impl<'a> Parser<'a> {
    /// Eat a struct declaration.
    ///
    /// Examples:
    /// ```
    /// struct {} // empty anonymous struct
    ///
    /// struct _ {} // explicit anonymous struct (for disambiguation)
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
    /// struct Foo<T>: Baz { // Foo has a Baz
    ///     myField: int32
    ///     myOtherField: T
    ///
    ///     let x: int32 = 7 // constant
    ///
    ///     ..Bar // Foo has a Bar
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    /// ```
    pub fn eat_struct(
        &mut self,
        visibility: Option<Visibility>,
        export: Option<ExportMode>,
    ) -> ParserResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword_in(&[Keyword::Struct, Keyword::Class])
            .for_node_type(NodeType::Definition)?;

        // optional representation type: ( ... )
        let representation_type = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            let representation_type = self.eat_expression().for_node_type(NodeType::Definition)?;
            self.eat_token(TokenType::CloseParenthesis)?;
            Some(representation_type)
        } else {
            None
        };

        // optional name
        let name = self.eat_identifier_or_wildcard_maybe()?;

        // style / tuple struct
        let (style, tuple_fields) = if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            let tuple_fields = self
                .eat_variant_body()
                .for_node_type(NodeType::Definition)?;
            self.eat_token(TokenType::CloseParenthesis)
                .for_node_type(NodeType::Definition)?;
            (VariantStyle::Tuple, Some(tuple_fields))
        } else {
            (VariantStyle::Struct, None)
        };

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Definition)?;

        // optional super types: : ...
        let super_types = self
            .eat_super_types_maybe()
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
            .eat_variant_body_mixed(style == VariantStyle::Struct)
            .for_node_type(NodeType::Definition)?;
        if let Some(tuple_fields) = tuple_fields {
            // merge in tuple fields
            fields.extend(tuple_fields);
        }
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;

        let struct_id = self.tree.insert(
            Definition::Struct {
                name,
                visibility,
                export,
                style,
                super_types,
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
    use dyst_ast::{Mutability, ScopedMutability, Visibility};

    use crate::parse::tests::TestParser;
    use crate::{
        BinaryOperator, Definition, Expression, IntType, Parameter, TypeLiteral, VariantField,
        VariantStyle, WhereClause, WithClause, assert_expr_path, assert_node, assert_path,
        assert_string,
    };

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
        let struct_id = parser.eat_struct(None, None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, static_parameters, fields, expressions, where_clauses, .. } => {
            assert_eq!(*name, None);
            assert_eq!(*static_parameters, None);
            assert!(expressions.is_empty());
            assert_eq!(fields.len(), 2);
            assert!(where_clauses.is_none());

            // public x: int32
            assert_node!(parser.tree, fields[0], VariantField { mutability, visibility: Some(Visibility::Public), name, ty, default } => {
                assert!(mutability.is_none());
                assert_string!(parser, name.unwrap(), "x");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
            });

            // y: boolean
            assert_node!(parser.tree, fields[1], VariantField { mutability, visibility: None, name, ty, default } => {
                assert!(mutability.is_some());
                assert_eq!(*mutability.as_ref().unwrap(), ScopedMutability::Unscoped { mutability: Mutability::Immutable });
                assert_string!(parser, name.unwrap(), "y");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_super_types() {
        let mut test = TestParser::new(
            r###"
struct Foo: Bar {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None, None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, super_types, fields, expressions, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "Foo");
            assert!(expressions.is_empty());
            assert!(fields.is_empty());
            assert!(where_clauses.is_none());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_struct_with_tuple_style() {
        let mut test = TestParser::new(
            r###"
struct Foo(int32, public boolean) {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None, None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, style, fields, expressions, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "Foo");
            assert_eq!(*style, VariantStyle::Tuple);
            assert_eq!(fields.len(), 2);
            assert!(expressions.is_empty());
            assert!(where_clauses.is_none());

            // int32
            assert_node!(parser.tree, fields[0], VariantField { mutability: None, visibility: None, name, ty, default } => {
                assert!(name.is_none());
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
            });

            // boolean
            assert_node!(parser.tree, fields[1], VariantField { mutability: None, visibility: Some(Visibility::Public), name, ty, default } => {
                assert!(name.is_none());
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Boolean));
            });
        });
    }

    #[test]
    fn test_parse_struct_with_struct_style() {
        let mut test = TestParser::new(
            r###"
struct Foo<T: Numeric>: Boz {
    ..Bar
    ..Baz
    
    public let x: int32 = 4

    a: T
    b?: T
    private b: int32 = 4

    private function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None, None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, static_parameters, fields, expressions, super_types, where_clauses, .. } => {
            assert_string!(parser, name.unwrap(), "Foo");
            assert!(where_clauses.is_none());

            // T: Numeric
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter::Scalar { name, ty, .. } => {
                // T
                assert_string!(parser, *name, "T");
                // Numeric
                assert!(ty.is_some());
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "Numeric");
                });
            });
            // Boz
            assert!(super_types.is_some());
            let super_types = super_types.as_ref().unwrap();
            assert_eq!(super_types.len(), 1);
            assert_node!(parser.tree, super_types[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Boz");
            });

            assert_eq!(fields.len(), 3);
            // a: T
            assert_node!(parser.tree, fields[0], VariantField { mutability: None, visibility: None, name, ty, default } => {
                assert_string!(parser, name.unwrap(), "a");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // b?: T
            assert_node!(parser.tree, fields[1], VariantField { mutability: None, visibility: None, name, ty, default } => {
                assert_string!(parser, name.unwrap(), "b");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::Maybe(expression_id) => {
                    assert_node!(parser.tree, *expression_id, Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
            });
            // private b: int32 = 4
            assert_node!(parser.tree, fields[2], VariantField { mutability: None, visibility: Some(Visibility::Private), name, ty, default } => {
                assert_string!(parser, name.unwrap(), "b");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
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

        let struct_id = parser.eat_struct(None, None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { with_clauses, where_clauses, fields, expressions, .. } => {
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
}
