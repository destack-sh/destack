use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

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
        start: &ParserMark,
        mut descriptor: DeclarationDescriptor,
        kind: TypeKind,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_INTERFACE);
        // disable tree literals while parsing typescript interfaces
        let allow_tree_literals = if self.language.is_typescript() {
            let allow_tree_literals = self.allow_tree_literals();
            self.set_allow_tree_literals(false);
            Some(allow_tree_literals)
        } else {
            None
        };

        let result = (|| {
            // keyword
            self.eat_keyword(Keyword::Interface)
                .for_node_type(NodeType::Declaration)?;

            // interface keyword cannot be followed by a newline
            if self.peek_is(TokenType::Newline) {
                let error = ParseError::unexpected(self.peek()?.span);
                self.error(&error);
                self.eat_newlines_maybe()?;
            }

            // optional name / key
            let name_span = if let Some((name, span)) = self.eat_name_maybe_with_span()? {
                descriptor = descriptor.with_name(name);
                Some(span)
            } else {
                None
            };

            // optional static parameters: < ... >
            let static_parameters = self.eat_static_parameters_maybe(true)?;

            // optional extends types
            let extends_types = self.eat_extends_types_maybe()?;

            // where
            let where_clauses = self.eat_where_maybe()?;

            // body
            let body_cursor = self.normalize_to_scanner_cursor();
            self.eat_newlines_maybe()?;
            self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
                .for_node_type(NodeType::Declaration)?;
            self.eat_newlines_maybe()?;

            // parse interface members in type context for typescript
            let members = if self.language.is_typescript() {
                let member_options = self.options.nested().in_variant().in_type();
                self.with_options(member_options, |parser| parser.eat_members(true))?
            } else {
                let member_options = self.options.nested().in_variant();
                self.with_options(member_options, |parser| parser.eat_members(true))?
            };
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

            // set main span to the name identifier
            if let Some(span) = name_span {
                self.tree.set_main_span(interface_id, span);
            }

            // attach declaration header-to-body boundary annotations before `{`
            self.bind_annotation_seam(
                body_cursor.index,
                body_cursor.skipped_newline_count.saturating_add(1),
                interface_id.id,
                super::annotation::AnnotationSeamKind::Infix,
            );

            Ok(interface_id)
        })();

        if let Some(allow_tree_literals) = allow_tree_literals {
            self.set_allow_tree_literals(allow_tree_literals);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Annotation, AnnotationPosition, BinaryOperator, BindingKind, Comment, CommentStyle,
        Declaration, DeclarationDescriptor, DeclarationKind, Expression, FunctionMode, IntType,
        Key, Member, Mutability, Name, Parameter, ScalarLiteral, TypeKind, TypeLiteral,
        VarianceModifier, WhereClause,
    };

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};
    use destack_source::LanguageType;

    #[test]
    fn test_parse_interface_anonymous_empty() {
        let mut test = TestParser::new("interface {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
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

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
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

    /// Parse TSX interface call signatures with generic parameters.
    #[test]
    fn test_parse_tsx_interface_generic_call_signature() {
        let mut test = TestParser::new_with_options(
            r#"
interface Foo<G> {
    <T>(bar: G): T;
}
"#,
            destack_source::LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();

        // interface call signature with generic parameters
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, members, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], Member::Method { key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Call));
                let static_parameters = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    .expect("expected static parameters");
                assert_eq!(static_parameters.len(), 1);
                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, .. } => {
                    assert_string!(parser, *name, "T");
                });
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "bar");
                    assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "G");
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
            });
        });
    }

    #[test]
    fn test_parse_interface_extends_with_newline() {
        let mut test = TestParser::new(
            r###"
interface Foo extends Bar
{
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("expected extends");
            assert_eq!(extends_types.len(), 1);
            assert_expression_path!(parser, parser.tree.get(extends_types[0]), "Bar");
        });
    }

    #[test]
    fn test_parse_interface_extends_with_newline_separated_types() {
        let mut test = TestParser::new(
            r###"
interface Foo extends Bar
Baz {
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("expected extends");
            assert_eq!(extends_types.len(), 2);
            assert_expression_path!(parser, parser.tree.get(extends_types[0]), "Bar");
            assert_expression_path!(parser, parser.tree.get(extends_types[1]), "Baz");
        });
    }

    #[test]
    fn test_parse_typescript_interface_extends_comma_separated_with_newline() {
        let mut test = TestParser::new_with_options(
            r###"
interface Foo extends Bar,
Baz {
}
"###,
            destack_source::LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("expected extends");
            assert_eq!(extends_types.len(), 2);
            assert_expression_path!(parser, parser.tree.get(extends_types[0]), "Bar");
            assert_expression_path!(parser, parser.tree.get(extends_types[1]), "Baz");
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

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
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
    fn test_parse_interface_allows_comma_separated_members() {
        let mut test = TestParser::new(
            r###"
interface Foo {
    value: int32,
    count: int32,
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 2);
        });
    }

    #[test]
    fn test_parse_interface_with_static_parameters() {
        let mut test = TestParser::new("interface Baz<T> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
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
    fn test_parse_interface_with_empty_static_parameters_typescript() {
        let mut test = TestParser::new_with_options("interface Box<> {}", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Box");
            let params = generics
                .static_parameters
                .as_ref()
                .expect("expected static params");
            assert!(params.is_empty());
        });
    }

    #[test]
    fn test_parse_interface_with_variance_parameters() {
        let mut test = TestParser::new("interface Baz<in T, out U> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Baz");
            let params = generics
                .static_parameters
                .as_ref()
                .expect("expected static params");
            assert_eq!(params.len(), 2);
            assert_node!(parser.tree, params[0], Parameter::Named { modifiers: Some(modifiers), name, .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(modifiers.variance, Some(VarianceModifier::In));
            });
            assert_node!(parser.tree, params[1], Parameter::Named { modifiers: Some(modifiers), name, .. } => {
                assert_string!(parser, *name, "U");
                assert_eq!(modifiers.variance, Some(VarianceModifier::Out));
            });
        });
    }

    #[test]
    fn test_parse_interface_with_invariant_parameter() {
        let mut test = TestParser::new("interface Holder<in out T> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, generics, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Holder");
            let params = generics
                .static_parameters
                .as_ref()
                .expect("expected static params");
            assert_eq!(params.len(), 1);
            assert_node!(parser.tree, params[0], Parameter::Named { modifiers: Some(modifiers), name, .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(modifiers.variance, Some(VarianceModifier::InOut));
            });
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

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
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
            assert_node!(parser.tree, where_items[0], WhereClause { left, right } => {
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

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
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
                assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::VariadicNamed { name, ty: Some(ty), .. } => {
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
    fn test_parse_interface_with_iterator_methods() {
        let mut test = TestParser::new(
            r#"
interface Iterator<T, TReturn = any, TNext = any> {
    next(...[value]: [] | [TNext]): IteratorResult<T, TReturn>;
    return?(value?: TReturn): IteratorResult<T, TReturn>;
    throw?(e?: any): IteratorResult<T, TReturn>;
}
"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 3);

            // next(...[value]: [] | [TNext]): IteratorResult<T, TReturn>;
            assert_node!(parser.tree, members[0], Member::Method { modifiers: None, key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "next");
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::VariadicNamed { name, ty: Some(ty), .. } => {
                    assert_string!(parser, *name, "value");
                    assert_node!(parser.tree, *ty, Expression::Binary { operator, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    });
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "IteratorResult");
            });

            // return?(value?: TReturn): IteratorResult<T, TReturn>;
            assert_node!(parser.tree, members[1], Member::Method { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                assert_string!(parser, *name, "return");
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { modifiers: Some(modifiers), name, ty: Some(ty), .. } => {
                    assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(*ty), "TReturn");
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "IteratorResult");
            });

            // throw?(e?: any): IteratorResult<T, TReturn>;
            assert_node!(parser.tree, members[2], Member::Method { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                assert_string!(parser, *name, "throw");
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { modifiers: Some(modifiers), name, ty: Some(ty), .. } => {
                    assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                    assert_string!(parser, *name, "e");
                    assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::Any));
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "IteratorResult");
            });
        });
    }

    #[test]
    fn test_parse_interface_method_overloads_named_where_typescript() {
        let mut test = TestParser::new_with_options(
            r#"interface Query {
where(where: string, parameters?: ObjectLiteral): this
where(where: Brackets, parameters?: ObjectLiteral): this
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 2);

            // where(where: string, parameters?: ObjectLiteral): this
            assert_node!(parser.tree, members[0], Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "where");
                assert_eq!(signature.dynamic_parameters.len(), 2);
                assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Named { modifiers: Some(modifiers), name, .. } => {
                    assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                    assert_string!(parser, *name, "parameters");
                });
            });

            // where(where: Brackets, parameters?: ObjectLiteral): this
            assert_node!(parser.tree, members[1], Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "where");
                assert_eq!(signature.dynamic_parameters.len(), 2);
                assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Named { modifiers: Some(modifiers), name, .. } => {
                    assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                    assert_string!(parser, *name, "parameters");
                });
            });
        });
    }

    #[test]
    fn test_parse_newtype_interface_empty() {
        let mut test = TestParser::new("interface {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationDescriptor::default(), TypeKind::Nominal)
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

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationDescriptor::default(), TypeKind::Nominal)
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

    /// 'is' can be used as a property name in TypeScript declaration files.
    #[test]
    fn test_parse_interface_with_is_property_name_in_typescript() {
        let mut test = TestParser::new_with_options(
            r#"interface Webidl {
    is: WebidlIs
}"#,
            destack_source::LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(
                &start,
                DeclarationDescriptor::default(),
                TypeKind::Structural,
            )
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface { descriptor, members, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "Webidl");
            assert_eq!(members.len(), 1);

            // is: WebidlIs
            assert_node!(parser.tree, members[0], Member::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), .. } => {
                assert_string!(parser, *name, "is");
                assert_expression_path!(parser, parser.tree.get(*ty), "WebidlIs");
            });
        });
    }

    /// 'is' as property name works in multi-member interfaces.
    #[test]
    fn test_parse_interface_with_is_and_other_members() {
        let mut test = TestParser::new_with_options(
            r#"export interface Webidl {
    errors: WebidlErrors
    util: WebidlUtil
    converters: WebidlConverters
    is: WebidlIs
    attributes: WebIDLExtendedAttributes
}"#,
            destack_source::LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Interface { descriptor, members, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "Webidl");
                assert!(descriptor.export.is_some());
                assert_eq!(members.len(), 5);

                // errors: WebidlErrors
                assert_node!(parser.tree, members[0], Member::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), .. } => {
                    assert_string!(parser, *name, "errors");
                    assert_expression_path!(parser, parser.tree.get(*ty), "WebidlErrors");
                });

                // util: WebidlUtil
                assert_node!(parser.tree, members[1], Member::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), .. } => {
                    assert_string!(parser, *name, "util");
                    assert_expression_path!(parser, parser.tree.get(*ty), "WebidlUtil");
                });

                // converters: WebidlConverters
                assert_node!(parser.tree, members[2], Member::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), .. } => {
                    assert_string!(parser, *name, "converters");
                    assert_expression_path!(parser, parser.tree.get(*ty), "WebidlConverters");
                });

                // is: WebidlIs
                assert_node!(parser.tree, members[3], Member::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), .. } => {
                    assert_string!(parser, *name, "is");
                    assert_expression_path!(parser, parser.tree.get(*ty), "WebidlIs");
                });

                // attributes: WebIDLExtendedAttributes
                assert_node!(parser.tree, members[4], Member::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(ty), .. } => {
                    assert_string!(parser, *name, "attributes");
                    assert_expression_path!(parser, parser.tree.get(*ty), "WebIDLExtendedAttributes");
                });
            });
        });
    }

    #[test]
    fn test_parse_interface_head_comment_before_body_on_declaration_owner() {
        let mut test = TestParser::new_with_options(
            "interface Shape // interface-head\n{\n  area: number\n}",
            destack_source::LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Interface { .. } => {});

            let annotations = parser.tree.get_annotations(declaration_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Slash);
                    assert_string!(parser, *string, "interface-head");
                });
            });
        });
    }
}
