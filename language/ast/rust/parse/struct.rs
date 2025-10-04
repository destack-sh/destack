//! Parse structs.
#![allow(clippy::type_complexity)]

use crate::parse::prelude::*;
use dyst_token::TokenType;

use crate::{
    Definition, Expression, Keyword, NodeId, NodeType, AstError, AstResult, Parser,
    StructField, StructStyle, Visibility,
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
    ) -> AstResult<NodeId<Definition>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Struct)?;

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
                .eat_struct_tuple_body()
                .for_node_type(NodeType::Definition)?;
            self.eat_token(TokenType::CloseParenthesis)
                .for_node_type(NodeType::Definition)?;
            (StructStyle::Tuple, Some(tuple_fields))
        } else {
            (StructStyle::Struct, None)
        };

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe()
            .for_node_type(NodeType::Definition)?;

        // optional super types: : ...
        let super_types = self
            .eat_super_types_maybe()
            .for_node_type(NodeType::Definition)?;

        // optional with declaration
        let with = self.eat_with_maybe().for_node_type(NodeType::Definition)?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;
        self.eat_newlines_maybe()?;
        let (mut fields, expressions) = self
            .eat_struct_body(style)
            .for_node_type(NodeType::Definition)?;
        if let Some(tuple_fields) = tuple_fields {
            // merge in tuple fields
            fields.extend(tuple_fields);
        }
        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Definition)?;

        let struct_id = self.tree.allocate(
            Definition::Struct {
                name,
                visibility,
                style,
                super_types,
                static_parameters,
                representation_type,
                with,
                fields,
                expressions,
            },
            self.get_span_from(start),
        );

        Ok(struct_id)
    }

    /// Eat a struct tuple body (without the parenthesis).
    /// Because it's just a tuple body, it doesn't have any expressions.
    pub fn eat_struct_tuple_body(&mut self) -> AstResult<Vec<NodeId<StructField>>> {
        let mut tuple_fields: Vec<NodeId<StructField>> = Vec::new();
        loop {
            // stop at closing parenthesis
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // keep eating tuple fields
            else {
                let field = self.eat_struct_field()?;
                tuple_fields.push(field);
            }
        }
        Ok(tuple_fields)
    }

    /// Eat a struct body (without the header or `{` and `}`).
    /// Struct fields are only parsed if it's a struct-style struct.
    pub fn eat_struct_body(
        &mut self,
        style: StructStyle,
    ) -> AstResult<(Vec<NodeId<StructField>>, Vec<NodeId<Expression>>)> {
        // eat everything
        let mut fields: Vec<NodeId<StructField>> = Vec::new();
        let mut expressions: Vec<NodeId<Expression>> = Vec::new();
        loop {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // struct field
            else if style == StructStyle::Struct && self.peek_struct_field().is_ok() {
                let field = self
                    .eat_struct_field()
                    .for_node_type(NodeType::StructField)?;
                fields.push(field);
            }
            // eat expressions
            else {
                let expression_id = self
                    .try_eat_expression_as_statement()
                    .for_node_type(NodeType::Expression)?;
                expressions.push(expression_id);
            }
        }

        Ok((fields, expressions))
    }

    /// Peek a struct field: `name: Type` with optional default `= <expr>`.
    fn peek_struct_field(&self) -> AstResult<()> {
        if self.peek_identifier().is_ok()
            && (self.peek_next_token(TokenType::Colon).is_ok()
                || self.peek_next_token(TokenType::Assign).is_ok()
                || self.peek_next_token(TokenType::Newline).is_ok()
                || self.peek_next_token(TokenType::Comma).is_ok()
                || self.peek_next_token(TokenType::Semicolon).is_ok()
                || self.peek_next_token(TokenType::CloseBrace).is_ok())
        {
            Ok(())
        } else {
            Err(AstError::expected(
                self.peek()?.span,
                TokenType::Identifier,
            ))
        }
    }

    /// Eat a single struct field: `T`,`name: T`, or `name: T = <expr>`.
    fn eat_struct_field(&mut self) -> AstResult<NodeId<StructField>> {
        let start = self.mark();

        // name:
        let name =
            if self.peek_identifier().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                let name = self.eat_identifier()?;
                self.eat_colon()?;
                Some(name)
            } else {
                None
            };

        // type
        let ty = self
            .with_options(self.options.in_type(), |parser| parser.eat_expression())
            .for_node_type(NodeType::Definition)?;

        // optional default value: `= <expr>`
        let default = if self.peek_token(TokenType::Assign).is_ok() {
            self.eat_token(TokenType::Assign)?;
            Some(self.eat_expression().for_node_type(NodeType::Definition)?)
        } else {
            None
        };

        let field_id = self.tree.allocate(
            StructField {
                name,
                ty,
                default,
            },
            self.get_span_from(start),
        );
        Ok(field_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Definition, Expression, IntType, Parameter, StructField, StructStyle, TypeLiteral,
        assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_struct_anonymous() {
        let mut test = TestParser::new(
            r###"
struct { x: int32, y: boolean
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // struct { x: int32, y: boolean }
        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, static_parameters, fields, expressions, .. } => {
            assert_eq!(*name, None);
            assert_eq!(*static_parameters, None);
            assert!(expressions.is_empty());
            assert_eq!(fields.len(), 2);

            // x: int32
            assert_node!(parser.tree, fields[0], StructField { name, ty, default } => {
                assert_string!(parser.session, name.unwrap(), "x");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
            });

            // y: boolean
            assert_node!(parser.tree, fields[1], StructField { name, ty, default } => {
                assert_string!(parser.session, name.unwrap(), "y");
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

        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, super_types, fields, expressions, .. } => {
            assert_string!(parser.session, name.unwrap(), "Foo");
            assert!(expressions.is_empty());
            assert!(fields.is_empty());

            let supers = super_types.as_ref().expect("expected super types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_struct_with_tuple_style() {
        let mut test = TestParser::new(
            r###"
struct Foo(int32, boolean) {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, style, fields, expressions, .. } => {
            assert_string!(parser.session, name.unwrap(), "Foo");
            assert_eq!(*style, StructStyle::Tuple);
            assert_eq!(fields.len(), 2);
            assert!(expressions.is_empty());

            // int32
            assert_node!(parser.tree, fields[0], StructField { name, ty, default } => {
                assert!(name.is_none());
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
            });

            // boolean
            assert_node!(parser.tree, fields[1], StructField { name, ty, default } => {
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
    
    let x: int32 = 4

    a: T
    b: int32 = 4

    function myFunc() { // nested declaration
    }
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let struct_id = parser.eat_struct(None).unwrap();
        assert_node!(parser.tree, struct_id, Definition::Struct { name, static_parameters, fields, expressions, super_types, .. } => {
            assert_string!(parser.session, name.unwrap(), "Foo");

            // T: Numeric
            assert!(static_parameters.is_some());
            let static_parameters = static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 1);
            assert_node!(parser.tree, static_parameters[0], Parameter { name, ty, .. } => {
                // T
                assert_string!(parser.session, *name, "T");
                // Numeric
                assert!(ty.is_some());
                assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "Numeric");
                });
            });
            // Boz
            assert!(super_types.is_some());
            let super_types = super_types.as_ref().unwrap();
            assert_eq!(super_types.len(), 1);
            assert_node!(parser.tree, super_types[0], Expression::Path { path, .. } => {
                assert_path!(parser.session, *path, "Boz");
            });

            assert_eq!(fields.len(), 2);
            // a: T
            assert_node!(parser.tree, fields[0], StructField { name, ty, default } => {
                assert_string!(parser.session, name.unwrap(), "a");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, Expression::Path { path, .. } => {
                    assert_path!(parser.session, *path, "T");
                });
            });
            // b: int32 = 4
            assert_node!(parser.tree, fields[1], StructField { name, ty, default } => {
                assert_string!(parser.session, name.unwrap(), "b");
                assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
                assert!(default.is_some());
            });

            assert_eq!(expressions.len(), 4);
        });
    }
}
