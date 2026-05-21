use destack_dir::{
    CommentKind, Declaration, Expression, GenericArgument, GenericParameter, IntegerType,
    InterfaceDeclaration, Key, Name, Parameter, Pattern, PatternField, TypeExpression, TypeKind,
    TypeLiteral, TypeMember, VarianceModifier, WhereClause,
};

use crate::parse::DeclarationHeader;
use crate::{
    TestParser, assert_comment, assert_expression_path, assert_name, assert_node, assert_path,
    assert_string,
};
use destack_source::{LanguageType, NodeSpanBoundary, NodeSpanRegion, NodeSpanType};

#[test]
fn test_parse_interface_anonymous_empty() {
    let mut test = TestParser::new("interface {}");
    let mut parser = test.prepare();

    let start = parser.span_start();
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
fn test_parse_interface_with_extends() {
    let mut test = TestParser::new("interface Foo extends Bar {}");
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, is_nominal, generic_parameters, extends, members, .. }) => {
        assert_string!(parser, name.expect("expected name").string(), "Foo");
        assert!(!*is_nominal);
        assert!(members.is_empty());
        assert!(generic_parameters.is_empty());
        assert_eq!(extends.len(), 1);
        assert_expression_path!(parser, parser.tree.get(extends[0].expression), "Bar");
        assert!(extends[0].generic_arguments.is_empty());
    });
}

#[test]
fn test_parse_interface_extends_with_generic_arguments() {
    let mut test = TestParser::new_with_language(
        r#"
interface Foo extends Bar<Baz>, Namespace.Qux<string> {}
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();

    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { extends, .. }) => {
        assert_eq!(extends.len(), 2);

        assert_expression_path!(parser, parser.tree.get(extends[0].expression), "Bar");
        assert_eq!(extends[0].generic_arguments.len(), 1);
        assert_node!(parser.tree, extends[0].generic_arguments[0], GenericArgument::Type { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "Baz");
        });

        assert_expression_path!(parser, parser.tree.get(extends[1].expression), "Namespace.Qux");
        assert_eq!(extends[1].generic_arguments.len(), 1);
        assert_node!(parser.tree, extends[1].generic_arguments[0], GenericArgument::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
    });
}

#[test]
fn test_parse_interface_with_missing_close_brace() {
    let mut test = TestParser::new("interface Foo { bar(): Baz");
    let mut parser = test.prepare();

    let start = parser.span_start();
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
    let mut test = TestParser::new_with_language(
        r#"
interface Foo<G> {
    <T>(bar: G): T;
}
"#,
        destack_source::LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();

    // interface call signature with generic parameters
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, members, .. }) => {
        assert_string!(parser, name.expect("expected name").string(), "Foo");
        assert_eq!(members.len(), 1);
        assert_node!(parser.tree, members[0], TypeMember::CallSignature { signature } => {
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
    let mut test = TestParser::new_with_language(
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

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { extends, .. }) => {
        assert_eq!(extends.len(), 1);
        assert_expression_path!(parser, parser.tree.get(extends[0].expression), "Bar");
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

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { extends, .. }) => {
        assert_eq!(extends.len(), 2);
        assert_expression_path!(parser, parser.tree.get(extends[0].expression), "Bar");
        assert_expression_path!(parser, parser.tree.get(extends[1].expression), "Baz");
    });
}

#[test]
fn test_parse_interface_extends_comma_separated_with_newline() {
    let mut test = TestParser::new_with_language(
        r###"
interface Foo extends Bar,
Baz {
}
"###,
        destack_source::LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { extends, .. }) => {
        assert_eq!(extends.len(), 2);
        assert_expression_path!(parser, parser.tree.get(extends[0].expression), "Bar");
        assert_expression_path!(parser, parser.tree.get(extends[1].expression), "Baz");
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

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, extends, members, .. }) => {
        assert_string!(parser, name.expect("expected name").string(), "Foo");
        assert!(generic_parameters.is_empty());

        // extends Baz
        assert_eq!(extends.len(), 1);
        assert_expression_path!(parser, parser.tree.get(extends[0].expression), "Baz");
        assert!(extends[0].generic_arguments.is_empty());

        // readonly value: int32
        assert_node!(parser.tree, members[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type: ty, is_readonly, .. } => {
            assert!(*is_readonly);
            assert_string!(parser, *name, "value");
            assert_node!(parser.tree, ty.expect("expected declared type"), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                }));
            });
        });

        // count: int32
        assert_node!(parser.tree, members[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type: ty, .. } => {
            assert_string!(parser, *name, "count");
            assert_node!(parser.tree, ty.expect("expected declared type"), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
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

    let start = parser.span_start();
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

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, .. }) => {
        assert_string!(parser, name.expect("expected name").string(), "Baz");
        assert_eq!(generic_parameters.len(), 1);
    });

    let generic_parameter_span = parser
        .tree
        .get_side_span(
            interface_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .expect("missing interface generic parameter span");
    assert_eq!(parser.get_span_str(generic_parameter_span), "<T>");
}

#[test]
fn test_parse_interface_with_empty_generic_parameters() {
    let mut test = TestParser::new_with_language("interface Box<> {}", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let start = parser.span_start();
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

    let start = parser.span_start();
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

    let start = parser.span_start();
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

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, generic_parameters, where_clauses, .. }) => {
        assert_string!(parser, name.expect("expected name").string(), "Baz");
        assert_eq!(generic_parameters.len(), 1);

        // where Requirement: Interface
        assert_eq!(where_clauses.len(), 1);
        assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Requirement");
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

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, members, .. }) => {
        assert_string!(parser, name.unwrap().string(), "SQL");
        assert_eq!(members.len(), 5);

        // <T = any>(value: T): SQL.Result<T>;
        assert_node!(parser.tree, members[0], TypeMember::CallSignature { signature } => {

            let generic_parameter_span = parser
                .tree
                .get_side_span(members[0], NodeSpanType::Region(NodeSpanRegion::GenericParameters))
                .expect("missing generic parameter span");
            assert_eq!(parser.get_span_str(generic_parameter_span), "<T = any>");

            let parameter_span = parser
                .tree
                .get_side_span(members[0], NodeSpanType::Region(NodeSpanRegion::Parameters))
                .expect("missing parameter span");
            assert_eq!(parser.get_span_str(parameter_span), "(value: T)");

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
        assert_node!(parser.tree, members[1], TypeMember::CallSignature { signature } => {
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
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Array { element } => {
                    assert_node!(parser.tree, *element, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Any);
                    });
                });
            });
            // SQL.Result<any>;
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL.Result");
        });

        // new(): SQL;
        assert_node!(parser.tree, members[2], TypeMember::ConstructSignature { signature } => {
            assert_eq!(signature.parameters.len(), 0);
            // SQL
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "SQL");
        });

        // [Symbol.asyncIterator](): AsyncIterableIterator<string>;
        assert_node!(parser.tree, members[3], TypeMember::Method { key: Key::Expression(key), signature, .. } => {
            // [Symbol.asyncIterator]
            assert_expression_path!(parser, parser.tree.get(*key), "Symbol.asyncIterator");
            assert_eq!(signature.parameters.len(), 0);
            // AsyncIterableIterator<string>
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "AsyncIterableIterator");
        });

        // [Symbol.toPrimitive]?(): number;
        assert_node!(parser.tree, members[4], TypeMember::Method { key: Key::Expression(key), signature, .. } => {
            // [Symbol.toPrimitive]
            assert_expression_path!(parser, parser.tree.get(*key), "Symbol.toPrimitive");

            let optional_span = parser
                .tree
                .get_side_span(members[4], NodeSpanType::Boundary(NodeSpanBoundary::Trailing))
                .expect("missing optional marker span");
            assert_eq!(parser.get_span_str(optional_span), "?");

            let parameter_span = parser
                .tree
                .get_side_span(members[4], NodeSpanType::Region(NodeSpanRegion::Parameters))
                .expect("missing parameter span");
            assert_eq!(parser.get_span_str(parameter_span), "()");

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

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
        assert_eq!(members.len(), 3);

        // next(...[value]: [] | [TNext]): IteratorResult<T, TReturn>;
        assert_node!(parser.tree, members[0], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, .. } => {
            assert_string!(parser, *name, "next");
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.parameters[0], Parameter::VariadicPattern { pattern, declared_type, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
                    assert_eq!(fields.len(), 1);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, .. } => {
                        assert_name!(parser, *name, "value");
                    });
                });

                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                });
            });
            assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "IteratorResult");
        });

        // return?(value?: TReturn): IteratorResult<T, TReturn>;
        assert_node!(parser.tree, members[1], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, .. } => {
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
        assert_node!(parser.tree, members[2], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, .. } => {
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

/// Parse TypeScript interface members that use semicolon separators.
#[test]
fn test_parse_interface_semicolon_member_separators() {
    let mut test = TestParser::new_with_language(
        r#"
interface MacroContext {
    readonly trigger: MacroTrigger;
    readonly symbol?: Symbol;
    resolve(name: string): Symbol | undefined;
}
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);

    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
        assert_eq!(members.len(), 3);

        // readonly trigger: MacroTrigger;
        assert_node!(parser.tree, members[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, is_readonly, is_optional, .. } => {
            assert_string!(parser, *name, "trigger");
            assert!(*is_readonly);
            assert!(!*is_optional);
            assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "MacroTrigger");
        });

        // readonly symbol?: Symbol;
        assert_node!(parser.tree, members[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, is_readonly, is_optional, .. } => {
            assert_string!(parser, *name, "symbol");
            assert!(*is_readonly);
            assert!(*is_optional);
            assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "Symbol");
        });

        // resolve(name: string): Symbol | undefined;
        assert_node!(parser.tree, members[2], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, body, .. } => {
            assert_string!(parser, *name, "resolve");
            assert!(body.is_none());
            assert_eq!(signature.parameters.len(), 1);
            assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
            });
        });
    });
}

/// Parse TypeScript index signatures on structural interfaces.
#[test]
fn test_parse_interface_index_signature_members() {
    let mut test = TestParser::new_with_language(
        r#"
interface ImportMetaEnv {
    readonly [key: string]: string | undefined;
    length: number;
}
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);

    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
        assert_eq!(members.len(), 2);

        // readonly [key: string]: string | undefined;
        assert_node!(parser.tree, members[0], TypeMember::IndexSignature { is_readonly, is_optional, name, key_type, value_type } => {
            assert!(*is_readonly);
            assert!(!*is_optional);
            assert_string!(parser, *name, "key");
            assert_node!(parser.tree, *key_type, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
            assert_node!(parser.tree, *value_type, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
            });
        });

        // length: number;
        assert_node!(parser.tree, members[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
            assert_string!(parser, *name, "length");
            assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
        });
    });
}

#[test]
fn test_parse_interface_method_overloads_named_where() {
    let mut test = TestParser::new_with_language(
        r#"interface Query {
where(where: string, parameters?: ObjectLiteral): this
where(where: Brackets, parameters?: ObjectLiteral): this
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
        assert_eq!(members.len(), 2);

        // where(where: string, parameters?: ObjectLiteral): this
        assert_node!(parser.tree, members[0], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, .. } => {
            assert_string!(parser, *name, "where");
            assert_eq!(signature.parameters.len(), 2);
            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { is_optional, name, .. } => {
                assert!(*is_optional);
                assert_string!(parser, *name, "parameters");
            });
        });

        // where(where: Brackets, parameters?: ObjectLiteral): this
        assert_node!(parser.tree, members[1], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, .. } => {
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

    let start = parser.span_start();
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

    let start = parser.span_start();
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
        assert_node!(parser.tree, members[0], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, .. } => {
            assert_string!(parser, *name, "add");
            assert_eq!(signature.parameters.len(), 1);
        });
    });
}

/// 'is' can be used as a property name in TypeScript declaration files.
#[test]
fn test_parse_interface_with_is_property_name() {
    let mut test = TestParser::new_with_language(
        r#"interface Webidl {
    is: WebidlIs
}"#,
        destack_source::LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();

    let start = parser.span_start();
    let interface_id = parser
        .eat_interface(&start, DeclarationHeader::default(), TypeKind::Structural)
        .unwrap();
    assert_node!(parser.tree, interface_id, Declaration::Interface(InterfaceDeclaration { name, members, .. }) => {
        assert_string!(parser, name.unwrap().string(), "Webidl");
        assert_eq!(members.len(), 1);

        // is: WebidlIs
        assert_node!(parser.tree, members[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
            assert_string!(parser, *name, "is");
            assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "WebidlIs");
        });
    });
}

/// 'is' as property name works in multi-member interfaces.
#[test]
fn test_parse_interface_with_is_and_other_members() {
    let mut test = TestParser::new_with_language(
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

    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Interface(InterfaceDeclaration { name, export, members, .. }) => {
            assert_string!(parser, name.unwrap().string(), "Webidl");
            assert!(export.is_some());
            assert_eq!(members.len(), 5);

            // errors: WebidlErrors
            assert_node!(parser.tree, members[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                assert_string!(parser, *name, "errors");
                assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "WebidlErrors");
            });

            // util: WebidlUtil
            assert_node!(parser.tree, members[1], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                assert_string!(parser, *name, "util");
                assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "WebidlUtil");
            });

            // converters: WebidlConverters
            assert_node!(parser.tree, members[2], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                assert_string!(parser, *name, "converters");
                assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "WebidlConverters");
            });

            // is: WebidlIs
            assert_node!(parser.tree, members[3], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                assert_string!(parser, *name, "is");
                assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "WebidlIs");
            });

            // attributes: WebIDLExtendedAttributes
            assert_node!(parser.tree, members[4], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                assert_string!(parser, *name, "attributes");
                assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "WebIDLExtendedAttributes");
            });
        });
    });
}

#[test]
fn test_parse_interface_head_comment_before_body_on_declaration_owner() {
    let mut test = TestParser::new_with_language(
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

    let expression_id = parser.unwrap_label_expression(expressions[0]);
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

    let expression_id = parser.unwrap_label_expression(expressions[0]);
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
            assert_node!(parser.tree, members[0], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, .. } => {
                assert_string!(parser, *name, "add");
                assert_eq!(signature.parameters.len(), 1);
            });
        });
    });
}

/// Parse Destack default method bodies on nominal interfaces.
#[test]
fn test_parse_newtype_interface_default_method_body() {
    let mut test = TestParser::new(
        r#"
newtype interface Error {
    source(): Any<Error> | undefined {
        undefined
    }
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { is_nominal, members, .. }) => {
            assert!(*is_nominal);
            assert_eq!(members.len(), 1);

            // source(): Any<Error> | undefined { undefined }
            assert_node!(parser.tree, members[0], TypeMember::Method { key: Key::Name(Name::Identifier(name)), signature, body: Some(body), .. } => {
                assert_string!(parser, *name, "source");
                assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                });
                assert_node!(parser.tree, *body, Expression::Block(_));
            });
        });
    });
}
