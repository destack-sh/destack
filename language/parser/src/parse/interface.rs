use crate::parse::expression::common::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, InterfaceDeclaration, Keyword, LocalNodeId, NodeType, TokenType, TypeKind,
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
    pub(crate) fn eat_interface(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        kind: TypeKind,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_INTERFACE);
        // typed interface heads do not admit tree literals
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
            let (name, name_span) = if let Some((name, span)) = self.eat_name_maybe_with_span()? {
                (Some(name), Some(span))
            } else {
                (None, None)
            };

            // optional generic parameters: < ... >
            let generic_parameters = self.eat_generic_parameters_maybe(true)?;

            // optional extends types
            let extends_types = self.eat_extends_types_maybe()?;

            // where
            let where_clauses = self.eat_where_maybe()?;

            // body
            self.eat_newlines_maybe()?;
            self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
                .for_node_type(NodeType::Declaration)?;
            self.eat_newlines_maybe()?;

            // parse interface members in type context
            let member_options = self.options.nested().in_variant().in_type();
            let members = self.with_options(member_options, |parser| parser.eat_type_members())?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

            // interface
            let interface_id = self.insert_node(
                Declaration::Interface(InterfaceDeclaration {
                    name,
                    export: header.export,
                    ambient: header.ambient,
                    is_nominal: kind == TypeKind::Nominal,
                    generic_parameters: generic_parameters.unwrap_or_default(),
                    where_clauses: where_clauses.unwrap_or_default(),
                    extends_types: extends_types.unwrap_or_default(),
                    members,
                }),
                self.get_span_from(start),
            );

            // set main span to the name identifier
            if let Some(span) = name_span {
                self.tree.set_main_span(interface_id, span);
            }

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
        CommentKind, Declaration, Expression, FunctionMode, GenericParameter, IntType,
        InterfaceDeclaration, Key, Name, Parameter, TypeExpression, TypeKind, TypeLiteral,
        TypeMember, VarianceModifier, WhereClause,
    };

    use crate::parse::expression::common::DeclarationHeader;
    use crate::{
        TestParser, assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
    };
    use destack_source::LanguageType;

    #[test]
    fn test_parse_interface_anonymous_empty() {
        let mut test = TestParser::new("interface {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, is_nominal, generic_parameters, members, .. }) => {
            assert!(name.is_none());
            assert!(!*is_nominal);
            assert!(generic_parameters.is_empty());
            assert!(members.is_empty());
        });
    }

    #[test]
    fn test_parse_interface_with_extends_types() {
        let mut test = TestParser::new("interface Foo extends Bar {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, is_nominal, generic_parameters, extends_types, members, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Foo");
            assert!(!*is_nominal);
            assert!(members.is_empty());
            assert!(generic_parameters.is_empty());
            assert_eq!(extends_types.len(), 1);
            assert_node!(parser.tree, extends_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_interface_with_missing_close_brace() {
        let mut test = TestParser::new("interface Foo { bar(): Baz");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, members, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Foo");
            assert_eq!(members.len(), 1);
        });
    }

    /// Parse interface call signatures with generic parameters before tree syntax.
    #[test]
    fn test_parse_interface_generic_call_signature_before_tree() {
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();

        // interface call signature with generic parameters
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, members, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Foo");
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], TypeMember::Method { key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Call));
                let generic_parameters = &signature.generic_parameters;
                assert_eq!(generic_parameters.len(), 1);
                assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                    assert_string!(parser, *name, "T");
                    assert!(constraint.is_none());
                });
                assert_eq!(signature.parameters.len(), 1);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "bar");
                    assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "G");
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
            });
        });
    }

    /// Parse interface call signature overloads separated by a blank line.
    #[test]
    fn test_parse_interface_call_signature_overloads_with_blank_line_separator() {
        let mut test = TestParser::new_with_options(
            r#"
interface Example {
  (a: number): typeof a

  <T>(): void
};
"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let roots = parser.parse();

        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
        assert_eq!(roots.len(), 1);

        let root_id = roots[0];
        assert_node!(parser.tree, root_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
                assert_eq!(members.len(), 2);
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { extends_types, .. }) => {
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { extends_types, .. }) => {
            assert_eq!(extends_types.len(), 2);
            assert_expression_path!(parser, parser.tree.get(extends_types[0]), "Bar");
            assert_expression_path!(parser, parser.tree.get(extends_types[1]), "Baz");
        });
    }

    #[test]
    fn test_parse_interface_extends_comma_separated_with_newline() {
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { extends_types, .. }) => {
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, extends_types, members, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Foo");
            assert!(generic_parameters.is_empty());

            // extends Baz
            assert_eq!(extends_types.len(), 1);
            assert_node!(parser.tree, extends_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Baz");
            });

            // readonly value: int32
            assert_node!(parser.tree, members[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type: ty, is_readonly, .. } => {
                assert!(*is_readonly);
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary {
                        width: Some(32),
                        is_signed: true,
                    }));
                });
            });

            // count: int32
            assert_node!(parser.tree, members[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type: ty, .. } => {
                assert_string!(parser, *name, "count");
                assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary {
                        width: Some(32),
                        is_signed: true,
                    }));
                });
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            assert_eq!(members.len(), 2);
        });
    }

    #[test]
    fn test_parse_interface_with_generic_parameters() {
        let mut test = TestParser::new("interface Baz<T> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Baz");
            assert_eq!(generic_parameters.len(), 1);
        });
    }

    #[test]
    fn test_parse_interface_with_empty_generic_parameters() {
        let mut test = TestParser::new_with_options("interface Box<> {}", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Box");
            assert!(generic_parameters.is_empty());
        });
    }

    #[test]
    fn test_parse_interface_with_variance_parameters() {
        let mut test = TestParser::new("interface Baz<in T, out U> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Baz");
            assert_eq!(generic_parameters.len(), 2);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { variance, name, .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(*variance, Some(VarianceModifier::In));
            });
            assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { variance, name, .. } => {
                assert_string!(parser, *name, "U");
                assert_eq!(*variance, Some(VarianceModifier::Out));
            });
        });
    }

    #[test]
    fn test_parse_interface_with_invariant_parameter() {
        let mut test = TestParser::new("interface Holder<in out T> {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Holder");
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { variance, name, .. } => {
                assert_string!(parser, *name, "T");
                assert_eq!(*variance, Some(VarianceModifier::InOut));
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, where_clauses, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "Baz");
            assert_eq!(generic_parameters.len(), 1);

            // where Requirement: Interface
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, members, .. }) => {
            assert_string!(parser, name.unwrap().string(), "SQL");
            assert_eq!(members.len(), 5);

            // <T = any>(value: T): SQL.Result<T>;
            assert_node!(parser.tree, members[0], TypeMember::Method { key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Call));
                let generic_parameters = &signature.generic_parameters;
                // <T = any>
                assert_eq!(generic_parameters.len(), 1);
                assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
                    assert_string!(parser, *name, "T");
                    assert!(constraint.is_none());
                    assert_node!(parser.tree, (*default).unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Any);
                    });
                });
                // value: T
                assert_eq!(signature.parameters.len(), 1);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "T");
                });
                // SQL.Result<T>
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL.Result");
            });

            // (value: any, ...arguments: any[]): SQL.Result<any>;
            assert_node!(parser.tree, members[1], TypeMember::Method { key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Call));
                // (value: any, ...arguments: any[])
                assert_eq!(signature.parameters.len(), 2);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Any);
                    });
                });
                // ...arguments: any[]
                assert_node!(parser.tree, signature.parameters[1], Parameter::VariadicNamed { name, declared_type, .. } => {
                    assert_string!(parser, *name, "arguments");
                    assert_node!(parser.tree, declared_type.unwrap(), destack_ast::TypeExpression::Array { element } => {
                        assert_node!(parser.tree, *element, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Any);
                        });
                    });
                });
                // SQL.Result<any>;
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL.Result");
            });

            // new(): SQL;
            assert_node!(parser.tree, members[2], TypeMember::Method { key: None, signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::New));
                assert_eq!(signature.parameters.len(), 0);
                // SQL
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL");
            });

            // [Symbol.asyncIterator](): AsyncIterableIterator<string>;
            assert_node!(parser.tree, members[3], TypeMember::Method { key: Some(Key::Expression(key)), signature, .. } => {
                // [Symbol.asyncIterator]
                assert_expression_path!(parser, parser.tree.get(*key), "Symbol.asyncIterator");
                assert_eq!(signature.parameters.len(), 0);
                // AsyncIterableIterator<string>
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "AsyncIterableIterator");
            });

            // [Symbol.toPrimitive]?(): number;
            assert_node!(parser.tree, members[4], TypeMember::Method { key: Some(Key::Expression(key)), signature, .. } => {
                // [Symbol.toPrimitive]
                assert_expression_path!(parser, parser.tree.get(*key), "Symbol.toPrimitive");
                assert_eq!(signature.parameters.len(), 0);
                // number
                assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Number);
                });
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            assert_eq!(members.len(), 3);

            // next(...[value]: [] | [TNext]): IteratorResult<T, TReturn>;
            assert_node!(parser.tree, members[0], TypeMember::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "next");
                assert_eq!(signature.parameters.len(), 1);
                assert_node!(parser.tree, signature.parameters[0], Parameter::VariadicNamed { name, declared_type, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 2);
                    });
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "IteratorResult");
            });

            // return?(value?: TReturn): IteratorResult<T, TReturn>;
            assert_node!(parser.tree, members[1], TypeMember::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "return");
                assert_eq!(signature.parameters.len(), 1);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { is_optional, name, declared_type, .. } => {
                    assert!(*is_optional);
                    assert_string!(parser, *name, "value");
                    assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "TReturn");
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "IteratorResult");
            });

            // throw?(e?: any): IteratorResult<T, TReturn>;
            assert_node!(parser.tree, members[2], TypeMember::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "throw");
                assert_eq!(signature.parameters.len(), 1);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { is_optional, name, declared_type, .. } => {
                    assert!(*is_optional);
                    assert_string!(parser, *name, "e");
                    assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Any);
                    });
                });
                assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "IteratorResult");
            });
        });
    }

    #[test]
    fn test_parse_interface_method_overloads_named_where() {
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            assert_eq!(members.len(), 2);

            // where(where: string, parameters?: ObjectLiteral): this
            assert_node!(parser.tree, members[0], TypeMember::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "where");
                assert_eq!(signature.parameters.len(), 2);
                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { is_optional, name, .. } => {
                    assert!(*is_optional);
                    assert_string!(parser, *name, "parameters");
                });
            });

            // where(where: Brackets, parameters?: ObjectLiteral): this
            assert_node!(parser.tree, members[1], TypeMember::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "where");
                assert_eq!(signature.parameters.len(), 2);
                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { is_optional, name, .. } => {
                    assert!(*is_optional);
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Nominal)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, is_nominal, generic_parameters, members, .. }) => {
            assert!(*is_nominal);
            assert!(name.is_none());
            assert!(generic_parameters.is_empty());
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
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Nominal)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, is_nominal, generic_parameters, members, .. }) => {
            assert!(*is_nominal);
            assert_string!(parser, name.unwrap().string(), "Add");

            // <T, R = Self>
            let params = generic_parameters;
            assert_eq!(params.len(), 2);

            // add(other: T): R
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], TypeMember::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                assert_string!(parser, *name, "add");
                assert_eq!(signature.parameters.len(), 1);
            });
        });
    }

    /// 'is' can be used as a property name in TypeScript declaration files.
    #[test]
    fn test_parse_interface_with_is_property_name() {
        let mut test = TestParser::new_with_options(
            r#"interface Webidl {
    is: WebidlIs
}"#,
            destack_source::LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let interface_id = parser
            .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
            .unwrap();
        assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, members, .. }) => {
            assert_string!(parser, name.unwrap().string(), "Webidl");
            assert_eq!(members.len(), 1);

            // is: WebidlIs
            assert_node!(parser.tree, members[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                assert_string!(parser, *name, "is");
                assert_expression_path!(parser, parser.tree.get(*declared_type), "WebidlIs");
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
            assert_node!(parser.tree, *decl_id, Declaration::Interface(InterfaceDeclaration { name, export, members, .. }) => {
                assert_string!(parser, name.unwrap().string(), "Webidl");
                assert!(export.is_some());
                assert_eq!(members.len(), 5);

                // errors: WebidlErrors
                assert_node!(parser.tree, members[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                    assert_string!(parser, *name, "errors");
                    assert_expression_path!(parser, parser.tree.get(*declared_type), "WebidlErrors");
                });

                // util: WebidlUtil
                assert_node!(parser.tree, members[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                    assert_string!(parser, *name, "util");
                    assert_expression_path!(parser, parser.tree.get(*declared_type), "WebidlUtil");
                });

                // converters: WebidlConverters
                assert_node!(parser.tree, members[2], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                    assert_string!(parser, *name, "converters");
                    assert_expression_path!(parser, parser.tree.get(*declared_type), "WebidlConverters");
                });

                // is: WebidlIs
                assert_node!(parser.tree, members[3], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                    assert_string!(parser, *name, "is");
                    assert_expression_path!(parser, parser.tree.get(*declared_type), "WebidlIs");
                });

                // attributes: WebIDLExtendedAttributes
                assert_node!(parser.tree, members[4], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                    assert_string!(parser, *name, "attributes");
                    assert_expression_path!(parser, parser.tree.get(*declared_type), "WebIDLExtendedAttributes");
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

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { .. }) => {});

            let annotations = parser.tree.get_decorators(declaration_id.id);
            assert!(annotations.is_empty());
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "interface-head");
    }

    #[test]
    fn test_parse_export_newtype_interface_with_default_this_parameter() {
        let mut test = TestParser::new(
            r#"
export newtype interface Add<T, R = this> {
    add(other: T): R;
}
"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { name, export, is_nominal, generic_parameters, members, .. }) => {
                assert!(export.is_some());
                assert!(*is_nominal);
                assert_string!(parser, name.unwrap().string(), "Add");

                // <T, R = this>
                let parameters = generic_parameters;
                assert_eq!(parameters.len(), 2);
                assert_node!(parser.tree, parameters[1], GenericParameter::Type { name, default, .. } => {
                    assert_string!(parser, *name, "R");
                    assert_node!(parser.tree, (*default).expect("expected default"), TypeExpression::This);
                });

                // add(other: T): R
                assert_eq!(members.len(), 1);
                assert_node!(parser.tree, members[0], TypeMember::Method { key: Some(Key::Name(Name::Identifier(name))), signature, .. } => {
                    assert_string!(parser, *name, "add");
                    assert_eq!(signature.parameters.len(), 1);
                });
            });
        });
    }
}
