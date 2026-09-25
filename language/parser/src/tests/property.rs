use crate::tests::{TestParser, block_expression_ids};
use tspp_dir::{
    Argument, AssignOperator, AssignPattern, Asynchrony, BinaryOperator, Block, ClassDeclaration,
    CommentKind, Declaration, Expression, FunctionDeclaration, FunctionForm, FunctionRole,
    GenericArgument, GenericParameter, IntegerType, InterfaceDeclaration, Literal, Member,
    MethodAbstraction, Name, NodeType, Parameter, Property, TokenType, TypeExpression, TypeLiteral,
    TypeMember, Visibility,
};

use crate::parse::TypeMemberContainerKind;
use crate::{assert_comment, assert_expression_path, assert_node, assert_path, assert_string};

#[test]
fn test_parse_member_override_field() {
    let test = TestParser::new("override foo: int32");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Field { name: Name::Identifier(name), declared_type: Some(value), is_override, .. } => {
        assert!(*is_override);
        assert_string!(parser, *name, "foo");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
            assert_eq!(
                *value,
                TypeLiteral::Integer(IntegerType::Fixed {
                    width: 32,
                    is_signed: true,
                })
            );
        });
    });
}

#[test]
fn test_parse_member_default_object_arrow_with_this_member_call_argument() {
    let test = TestParser::new(
        r"
port2 = {
  postMessage: () => {
    setTimeout(this.port1.onmessage, 0);
  }
}
",
    );
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    // port2 = { postMessage: () => { setTimeout(this.port1.onmessage, 0) } }
    assert_node!(parser.tree, member_id, Member::Field { name: Name::Identifier(name), declared_type: None, default: Some(default), .. } => {
        assert_string!(parser, *name, "port2");
        assert_node!(parser.tree, *default, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "postMessage");
                assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                        assert_eq!(signature.form, FunctionForm::Lambda);
                        assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                            assert_node!(parser.tree, *block_id, Block { .. } => {
                                let expressions = block_expression_ids(parser.tree.get(*block_id));
                                assert_eq!(expressions.len(), 1);
                                assert_node!(parser.tree, expressions[0], Expression::Call { arguments, .. } => {
                                        assert_eq!(arguments.len(), 2);
                                        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                                            assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                                                assert_string!(parser, *name, "onmessage");
                                                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                                    assert_string!(parser, *name, "port1");
                                                    assert_node!(parser.tree, *left, Expression::This);
                                                });
                                            });
                                        });
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_member_abstract_override_method() {
    let test = TestParser::new("abstract override foo(): void");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, abstraction, is_override, .. } => {
        assert_string!(parser, *name, "foo");
        assert!(signature.is_abstract);
        assert_eq!(*abstraction, MethodAbstraction::Abstract);
        assert!(*is_override);
    });
}

#[test]
fn test_parse_member_virtual_method() {
    let test = TestParser::new("virtual foo(): void {}");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, abstraction, body: Some(body), .. } => {
        assert_string!(parser, *name, "foo");
        assert_eq!(*abstraction, MethodAbstraction::Virtual);
        assert!(!signature.is_abstract);
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

#[test]
fn test_parse_member_async_override_method() {
    let test = TestParser::new("public async override foo(): void");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, visibility, is_override, .. } => {
        assert_eq!(*visibility, Some(Visibility::Public));
        assert_string!(parser, *name, "foo");
        assert_eq!(signature.asynchrony, Asynchrony::Async);
        assert!(*is_override);
    });
}

#[test]
fn test_parse_member_method_parameter_type_then_default_value() {
    let test = TestParser::new(
        "usersLimitReached(userCount: number, userLimit = get(this.store).userLimit) {}",
    );
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();

    // parse one method where a typed parameter is followed by a defaulted parameter
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, body: Some(_), .. } => {
        assert_string!(parser, *name, "usersLimitReached");
        assert_eq!(signature.parameters.len(), 2);

        // userCount: number
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), default, .. } => {
            assert_string!(parser, *name, "userCount");
            assert!(default.is_none());
            assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Number);
            });
        });

        // userLimit = get(this.store).userLimit
        assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type, default: Some(default), .. } => {
            assert_string!(parser, *name, "userLimit");
            assert!(declared_type.is_none());
            assert_node!(parser.tree, *default, Expression::Member { name, .. } => {
                assert_string!(parser, *name, "userLimit");
            });
        });
    });

    // this signature parses without recovery diagnostics
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_member_method_generic_with_newline_before_parameters() {
    let test = TestParser::new("private method<T>\n(value: T): T { return value }");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();

    // parse one method with a generic parameter and a newline before dynamic parameters
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, body: Some(body), visibility, .. } => {
        assert_eq!(*visibility, Some(Visibility::Private));
        assert_string!(parser, *name, "method");

        // parse the generic, dynamic parameter, and return type as one coherent signature
        let generic_parameters = &signature.generic_parameters;
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
            assert_string!(parser, *name, "T");
        });
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
            assert_string!(parser, *name, "value");
            assert_expression_path!(parser, parser.tree.get(*ty), "T");
        });
        assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected return type")), "T");

        // keep a method body attached after the multiline signature
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

#[test]
fn test_parse_member_method_with_newline_before_return_type() {
    let test = TestParser::new("method(value: string)\n: string { return value }");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();

    // parse one method with a newline before return type marker
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, body: Some(body), .. } => {
        assert_string!(parser, *name, "method");

        // keep the dynamic parameter and return type attached to the same method signature
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
            assert_string!(parser, *name, "value");
            assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
        assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });

        // keep a method body attached after the multiline return type annotation
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

#[test]
fn test_parse_member_method_object_union_return_type() {
    let test =
        TestParser::new("overlaps(): { overlaps: false } | { overlaps: true; reason: string }");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();

    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, body: None, .. } => {
        assert_string!(parser, *name, "overlaps");

        assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Union { elements } => {
            assert_eq!(elements.len(), 2);
            assert_node!(parser.tree, elements[0], TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
            });
            assert_node!(parser.tree, elements[1], TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 2);
            });
        });
    });
}

#[test]
fn test_parse_member_method_body_boundary_comment_on_return_type() {
    let test = TestParser::new("method(): number // method-body\n{ return 1 }");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    parser.finalize_comments();
    assert_node!(parser.tree, member, Member::Method { signature, body: Some(body), .. } => {
        let return_type = signature.return_type.expect("expected return type");
        let return_type_annotations = parser.tree.get_decorators(return_type.id);
        assert!(return_type_annotations.is_empty());

        assert_node!(parser.tree, *body, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let body_statement_annotations = parser.tree.get_decorators(expressions[0].id);
                assert!(body_statement_annotations.is_empty());
            });
        });
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "method-body");
}

#[test]
fn test_parse_member_async_string_literal_name() {
    let test = TestParser::new(r#"async 'delete'(name: string): Promise<boolean> { return true }"#);
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::String(name)), signature, body, .. } => {
        assert_string!(parser, *name, "delete");
        assert_eq!(signature.asynchrony, Asynchrony::Async);
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
            assert_string!(parser, *name, "name");
            assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });
        assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "Promise");
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Boolean);
                    });
            });
        });
        assert_node!(parser.tree, body.expect("expected method body"), Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
            });
        });
    });
}

#[test]
fn test_parse_member_method_named_public() {
    let test = TestParser::new("public() {}");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), visibility, .. } => {
        assert!(visibility.is_none());
        assert_string!(parser, *name, "public");
    });
}

#[test]
fn test_parse_member_static_method_named_protected() {
    let test = TestParser::new("static protected() {}");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), is_static, .. } => {
        assert!(*is_static);
        assert_string!(parser, *name, "protected");
    });
}

#[test]
fn test_parse_member_field_named_static() {
    let test = TestParser::new("static");
    let mut parser = test.prepare();

    let member = parser.parse_member().unwrap();
    assert_node!(parser.tree, member, Member::Field { name: Name::Identifier(name), declared_type: None, default: None, .. } => {
        assert_string!(parser, *name, "static");
    });
}

#[test]
fn test_parse_member_missing_default_expression() {
    // x =
    let test = TestParser::new("x =");
    let mut parser = test.prepare();
    let member = parser.parse_member().unwrap();

    assert_eq!(parser.errors.len(), 1);

    // x =
    assert_node!(parser.tree, member, Member::Field { name: Name::Identifier(name), declared_type: None, default: Some(default), .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *default, Expression::Missing);
    });
}

#[test]
fn test_parse_members_recover_error_slot() {
    // +\ny: int32
    let test = TestParser::new("+\ny: int32");
    let mut parser = test.prepare();
    let members = parser.parse_members().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(members.len(), 2);

    // error, y: int32
    assert_node!(parser.tree, members[0], Member::Error);
    assert_node!(parser.tree, members[1], Member::Field { name: Name::Identifier(name), declared_type: Some(value), default: None, .. } => {
        assert_string!(parser, *name, "y");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
            assert_eq!(
                *value,
                TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                })
            );
        });
    });
}

#[test]
fn test_recover_members_embedded_type() {
    let test = TestParser::new("...Transform\nx: int32");
    let mut parser = test.prepare();
    let members = parser.parse_members().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "...");
    assert_eq!(members.len(), 2);

    assert_node!(parser.tree, members[0], Member::Error);
    assert_node!(parser.tree, members[1], Member::Field { name: Name::Identifier(name), declared_type: Some(value), default: None, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
            assert_eq!(
                *value,
                TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                })
            );
        });
    });
}

#[test]
fn test_report_member_method_signature_without_separator() {
    let test = TestParser::new("method() method2()");
    let mut parser = test.prepare();
    let error = parser.parse_member().unwrap_err();

    assert_eq!(parser.range_str(error.range()), "method2");
}

#[test]
fn test_parse_interface_get_set_with_newlines() {
    let test = TestParser::new(
        r#"interface Foo {
  get
  foo(): string;
  set
  bar(v);
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();
    // parse interface members with get and set
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            let mut getter: Option<Name> = None;
            let mut setter: Option<Name> = None;
            for member_id in members {
                if let TypeMember::Method { signature, name, .. } = parser.tree.get(*member_id) {
                    match signature.role {
                        Some(FunctionRole::Getter) => getter = Some(*name),
                        Some(FunctionRole::Setter) => setter = Some(*name),
                        _ => {}
                    }
                }
            }
            let getter = getter.expect("expected getter member");
            let setter = setter.expect("expected setter member");
            assert!(matches!(getter, Name::Identifier(_)));
            assert!(matches!(setter, Name::Identifier(_)));
        });
    });
}

#[test]
fn test_parse_member_get_set_newline_only() {
    let test = TestParser::new(
        r#"get
foo(): string;"#,
    );
    let mut parser = test.prepare();
    let member = parser.parse_member().unwrap();

    assert_node!(parser.tree, member, Member::Method { name: Some(Name::Identifier(name)), signature, body: None, .. } => {
        assert_string!(parser, *name, "foo");
        assert_eq!(signature.role, Some(FunctionRole::Getter));
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Keyword { value: TypeLiteral::String });
    });
    assert_eq!(parser.peek_token_type(), TokenType::Semicolon);
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_property_with_value() {
    let test = TestParser::new("x: int32");
    let mut parser = test.prepare();
    let property = parser.parse_property().unwrap();
    assert_node!(parser.tree, property, Property::Field { name: Name::Identifier(name), value, is_shorthand } => {
        assert_string!(parser, *name, "x");
        assert!(!*is_shorthand);
        assert_node!(parser.tree, *value, Expression::Identifier { name } => {
            assert_string!(parser, *name, "int32");
        });
    });
}

#[test]
fn test_parse_property_with_default_value() {
    let test = TestParser::new("x = 42");
    let mut parser = test.prepare();
    let property = parser.parse_property().unwrap();
    assert_node!(parser.tree, property, Property::Field { name: Name::Identifier(name), value, is_shorthand } => {
        assert_string!(parser, *name, "x");
        assert!(*is_shorthand);
        assert_node!(parser.tree, *value, Expression::Assign { left, operator, right } => {
            assert_eq!(*operator, AssignOperator::Assign);
            assert_expression_path!(parser, parser.tree.get(*left), "x");
            assert_node!(parser.tree, *left, AssignPattern::Place { expression } => {
                let main_span = parser
                    .tree
                    .get_main_span(*expression)
                    .expect("expected shorthand value main span");
                assert_eq!(parser.span_str(main_span), "x");
            });
            assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(42)));
        });
    });
}

#[test]
fn test_parse_property_missing_value_expression() {
    // x:
    let test = TestParser::new("x:");
    let mut parser = test.prepare();
    let property = parser.parse_property().unwrap();

    assert_eq!(parser.errors.len(), 1);

    // x:
    assert_node!(parser.tree, property, Property::Field { name: Name::Identifier(name), value, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *value, Expression::Missing);
    });
}

#[test]
fn test_parse_property_with_typed_arrow_value() {
    let test = TestParser::new("reproFunc: (_: unknown): unknown => { }");
    let mut parser = test.prepare();
    let property = parser.parse_property().unwrap();
    assert_node!(parser.tree, property, Property::Field { name: Name::Identifier(name), value, .. } => {
        assert_string!(parser, *name, "reproFunc");
        assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(_), .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_eq!(signature.parameters.len(), 1);
            });
        });
    });
}

#[test]
fn test_parse_member_type_keyword_as_field_key() {
    let test = TestParser::new("type: string");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    assert_node!(parser.tree, member_id, Member::Field { name: Name::Identifier(name), declared_type: Some(value), .. } => {
        assert_string!(parser, *name, "type");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::String });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_type_member_keyword_keys_as_fields() {
    let test = TestParser::new(
        r#"type: string;
const: number"#,
    );
    let mut parser = test.prepare();
    let members = parser
        .parse_type_members(TypeMemberContainerKind::TypeLiteral)
        .unwrap();

    assert_eq!(members.len(), 2);
    assert_node!(parser.tree, members[0], TypeMember::Field { name: Name::Identifier(name), declared_type: Some(value), .. } => {
        assert_string!(parser, *name, "type");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::String });
    });
    assert_node!(parser.tree, members[1], TypeMember::Field { name: Name::Identifier(name), declared_type: Some(value), .. } => {
        assert_string!(parser, *name, "const");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Number });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_type_members_recover_before_associated_and_readonly_members() {
    let test = TestParser::new(
        r#"interface Boundary {
broken: ;
type Item = string
readonly value: string
}"#,
    );
    let mut parser = test.prepare();
    let roots = parser.parse_in_place();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, roots[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            assert_eq!(members.len(), 3);

            assert_node!(parser.tree, members[0], TypeMember::Field { name: Name::Identifier(name), declared_type: Some(value), .. } => {
                assert_string!(parser, *name, "broken");
                assert_node!(parser.tree, *value, TypeExpression::Missing);
            });
            assert_node!(parser.tree, members[1], TypeMember::AssociatedType { name, value: Some(value), .. } => {
                assert_string!(parser, *name, "Item");
                assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::String });
            });
            assert_node!(parser.tree, members[2], TypeMember::Field { is_readonly, name: Name::Identifier(name), declared_type: Some(value), .. } => {
                assert!(*is_readonly);
                assert_string!(parser, *name, "value");
                assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::String });
            });
        });
    });
}

#[test]
fn test_parse_type_members_with_associated_abstraction_modifiers() {
    let test = TestParser::new(
        r#"interface Boundary {
abstract type Item
override const Rows: number = 4
}"#,
    );
    let mut parser = test.prepare();
    let roots = parser.parse_in_place();

    assert_node!(parser.tree, roots[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface(InterfaceDeclaration { members, .. }) => {
            assert_eq!(members.len(), 2);

            assert_node!(parser.tree, members[0], TypeMember::AssociatedType { name, is_abstract, is_override, .. } => {
                assert_string!(parser, *name, "Item");
                assert!(*is_abstract);
                assert!(!*is_override);
            });
            assert_node!(parser.tree, members[1], TypeMember::AssociatedConst { name, is_abstract, is_override, .. } => {
                assert_string!(parser, *name, "Rows");
                assert!(!*is_abstract);
                assert!(*is_override);
            });
        });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_property_with_value_and_default_value() {
    let test = TestParser::new("x: int32 = 42");
    let mut parser = test.prepare();
    let property = parser.parse_property().unwrap();
    assert_node!(parser.tree, property, Property::Field { name: Name::Identifier(name), value, .. } => {
        assert_string!(parser, *name, "x");
        assert_node!(parser.tree, *value, Expression::Assign { .. });
    });
}

#[test]
fn test_parse_properties_recover_error_slot() {
    // +\ny: int32
    let test = TestParser::new("+\ny: int32");
    let mut parser = test.prepare();
    let properties = parser.parse_object_properties().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(properties.len(), 2);

    // error, y: int32
    assert_node!(parser.tree, properties[0], Property::Error);
    assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
        assert_string!(parser, *name, "y");
        assert_node!(parser.tree, *value, Expression::Identifier { name } => {
            assert_string!(parser, *name, "int32");
        });
    });
}

#[test]
fn test_parse_properties_recover_unkeyed_value_field() {
    let test = TestParser::new(": 1,\ny: 2");
    let mut parser = test.prepare();
    let properties = parser.parse_object_properties().unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Property),
            None,
            Some(TokenType::Identifier),
            ":",
        )],
    );
    assert_eq!(properties.len(), 2);

    // error, y: 2
    assert_node!(parser.tree, properties[0], Property::Error);
    assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
        assert_string!(parser, *name, "y");
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
    });
}

#[test]
fn test_parse_properties_recover_unkeyed_default_field() {
    let test = TestParser::new("= 1,\ny: 2");
    let mut parser = test.prepare();
    let properties = parser.parse_object_properties().unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Property),
            None,
            Some(TokenType::Identifier),
            "=",
        )],
    );
    assert_eq!(properties.len(), 2);

    // error, y: 2
    assert_node!(parser.tree, properties[0], Property::Error);
    assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
        assert_string!(parser, *name, "y");
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
    });
}

#[test]
fn test_parse_property_method_call() {
    let test = TestParser::new("<T = unknown>(x: T): T");
    let mut parser = test.prepare();
    let property_id = parser.parse_property().unwrap();
    // <T = unknown>(x: T): T
    assert_node!(parser.tree, property_id, Property::Method { signature, .. } => {
        assert_eq!(signature.role, Some(FunctionRole::Call));
        let generic_parameters = &signature.generic_parameters;
        // <T = unknown>
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
            assert_string!(parser, *name, "T");
            assert!(constraint.is_none());
            assert_node!(parser.tree, default.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Unknown);
            });
        });
        // x: T
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "x");
            assert_expression_path!(parser, parser.tree.get(declared_type.unwrap()), "T");
        });
        // T
        assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "T");
    });
}

#[test]
fn test_parse_property_method_object_return_type() {
    let test = TestParser::new("method(): { value: string; count: number }");
    let mut parser = test.prepare();
    let property_id = parser.parse_property().unwrap();

    assert_node!(parser.tree, property_id, Property::Method { name: Some(Name::Identifier(name)), signature, .. } => {
        assert_string!(parser, *name, "method");
        assert_node!(parser.tree, signature.return_type.expect("expected return type"), TypeExpression::Object { members: properties } => {
            assert_eq!(properties.len(), 2);
        });
    });
}

#[test]
fn test_parse_object_property_constructor_method_as_key() {
    let test = TestParser::new("constructor(x: int32);");
    let mut parser = test.prepare();

    let property_id = parser.parse_property().unwrap();
    assert_node!(parser.tree, property_id, Property::Method { name: Some(Name::Identifier(name)), signature, .. } => {
        // constructor
        assert_string!(parser, *name, "constructor");
        assert!(signature.role.is_none());
        assert!(signature.generic_parameters.is_empty());
        // x: int32
        assert_eq!(signature.parameters.len(), 1);
        assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true }));
            });
        });
    });
}

#[test]
fn test_parse_member_decorator_argument_this_member_expression() {
    let test = TestParser::new(
        r#"class Segment {
  @if(this.Width == 4)
  narrow: string
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);

            let decorators = parser.tree.get_decorators(members[0].id);
            assert_eq!(decorators.len(), 1);
            assert_node!(parser.tree, decorators[0], tspp_dir::Decorator { expression, .. } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, arguments, .. } => {
                    assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                        assert_string!(parser, *name, "if");
                    });
                    assert_eq!(arguments.len(), 1);
                    assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::Binary { left, operator, .. } => {
                            assert_eq!(*operator, BinaryOperator::Equal);
                            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                assert_string!(parser, *name.as_ref().expect("expected member name"), "Width");
                                assert_node!(parser.tree, *left, Expression::This);
                            });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_member_decorator_argument_import_meta_expression() {
    let test = TestParser::new(
        r#"class Segment {
  @if(import.meta.roles.includes("server"))
  narrow: string
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);

            let decorators = parser.tree.get_decorators(members[0].id);
            assert_eq!(decorators.len(), 1);
            assert_node!(parser.tree, decorators[0], tspp_dir::Decorator { expression, .. } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, arguments, .. } => {
                    assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                        assert_string!(parser, *name, "if");
                    });
                    assert_eq!(arguments.len(), 1);
                    assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::Call { left, arguments, .. } => {
                            assert_eq!(arguments.len(), 1);
                            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                assert_string!(parser, *name.as_ref().expect("expected member name"), "includes");
                                assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                                    assert_string!(parser, *name.as_ref().expect("expected member name"), "roles");
                                    assert_node!(parser.tree, *left, Expression::ImportMeta);
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_member_type_with_value() {
    let test = TestParser::new("type Item = string");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedType { name, generic_parameters, where_clauses, constraint: None, value: Some(value), visibility, is_ambient, .. } => {
        assert_string!(parser, *name, "Item");
        assert!(generic_parameters.is_empty());
        assert!(where_clauses.is_empty());
        assert!(visibility.is_none());
        assert!(!*is_ambient);
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
}

#[test]
fn test_parse_member_type_with_bound() {
    let test = TestParser::new("type Item: Hashable");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedType { name, generic_parameters, where_clauses, constraint: Some(ty), value: None, .. } => {
        assert_string!(parser, *name, "Item");
        assert!(generic_parameters.is_empty());
        assert!(where_clauses.is_empty());
        assert_expression_path!(parser, parser.tree.get(*ty), "Hashable");
    });
}

#[test]
fn test_parse_member_type_with_multiline_bound() {
    let test = TestParser::new(
        r#"type Item:
    | Foo
    | Bar"#,
    );
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedType { name, constraint: Some(ty), value: None, .. } => {
        assert_string!(parser, *name, "Item");
        assert_node!(parser.tree, *ty, TypeExpression::Union { .. });
    });
}

#[test]
fn test_parse_member_type_with_bound_and_value() {
    let test = TestParser::new("type Item: Hashable = string");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedType { name, constraint: Some(ty), value: Some(value), .. } => {
        assert_string!(parser, *name, "Item");
        assert_expression_path!(parser, parser.tree.get(*ty), "Hashable");
        assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::String);
        });
    });
}

#[test]
fn test_parse_member_type_with_visibility() {
    let test = TestParser::new("public type Item = string");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedType { name, value: Some(_), visibility, .. } => {
        assert_eq!(*visibility, Some(Visibility::Public));
        assert_string!(parser, *name, "Item");
    });
}

#[test]
fn test_parse_member_type_with_abstraction_modifiers() {
    let test = TestParser::new("abstract override type Item = string");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    assert_node!(parser.tree, member_id, Member::AssociatedType { name, is_abstract, is_override, .. } => {
        assert_string!(parser, *name, "Item");
        assert!(*is_abstract);
        assert!(*is_override);
    });
}

#[test]
fn test_parse_member_type_with_generic_parameters() {
    let test = TestParser::new("type View<U> = (Item, U)");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedType { name, generic_parameters, where_clauses, constraint: None, value: Some(_), .. } => {
        assert_string!(parser, *name, "View");
        assert!(where_clauses.is_empty());
        assert_eq!(generic_parameters.len(), 1);
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
            assert_string!(parser, *name, "U");
        });
    });
}

#[test]
fn test_parse_member_static_new_method_as_key() {
    let test = TestParser::new("static new<T>(): Set<T> { undefined! }");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    assert_node!(parser.tree, member_id, Member::Method { name: Some(name), signature, is_static, .. } => {
        assert_string!(parser, name.string(), "new");
        assert!(*is_static);
        assert!(signature.role.is_none());
        assert_eq!(signature.generic_parameters.len(), 1);
        assert!(signature.return_type.is_some());
    });
}

#[test]
fn test_parse_member_static_constructor_method_as_key() {
    let test = TestParser::new("static constructor<T>(): Set<T> { undefined! }");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    assert_node!(parser.tree, member_id, Member::Method { name: Some(name), signature, is_static, .. } => {
        assert_string!(parser, name.string(), "constructor");
        assert!(*is_static);
        assert!(signature.role.is_none());
        assert_eq!(signature.generic_parameters.len(), 1);
        assert!(signature.return_type.is_some());
    });
}

#[test]
fn test_parse_member_associated_const() {
    let test = TestParser::new("const Rows: number = 128");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedConst { name, declared_type: Some(ty), value: Some(value), .. } => {
        assert_string!(parser, *name, "Rows");
        assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Number);
        });
        assert_node!(parser.tree, *value, Expression::Literal(value) => {
            assert_eq!(*value, Literal::Integer(128));
        });
    });
}

#[test]
fn test_parse_member_associated_const_with_abstraction_modifiers() {
    let test = TestParser::new("abstract override const Rows: number");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    assert_node!(parser.tree, member_id, Member::AssociatedConst { name, is_abstract, is_override, .. } => {
        assert_string!(parser, *name, "Rows");
        assert!(*is_abstract);
        assert!(*is_override);
    });
}

#[test]
fn test_parse_member_associated_const_binary_default() {
    let test = TestParser::new("const LaneWidth: number = WidthHint * 2");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::AssociatedConst { value: Some(value), .. } => {
        assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::Multiply);
        });
    });
}

#[test]
fn test_parse_member_associated_const_type_relation_default() {
    let test = TestParser::new("const Width: uint = Row extends string ? 4 : 2");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    assert_node!(parser.tree, member_id, Member::AssociatedConst { value: Some(value), .. } => {
        assert_node!(parser.tree, *value, Expression::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "Row");
                assert_node!(parser.tree, *extends_type, TypeExpression::Keyword { value: TypeLiteral::String });
                assert_node!(parser.tree, *then_type, TypeExpression::Literal { value: Literal::Integer(4) });
                assert_node!(parser.tree, *else_type, TypeExpression::Literal { value: Literal::Integer(2) });
            });
        });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_report_member_static_associated_type() {
    let test = TestParser::new("static type Item = string");
    let mut parser = test.prepare();
    let error = parser.parse_member().unwrap_err();

    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_report_member_static_associated_const() {
    let test = TestParser::new("static const Rows: number = 128");
    let mut parser = test.prepare();
    let error = parser.parse_member().unwrap_err();

    assert_eq!(parser.range_str(error.range()), "static const Rows");
}

#[test]
fn test_parse_member_const_block() {
    let test = TestParser::new("const { assert(true) }");
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::ConstBlock { body } => {
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

#[test]
fn test_parse_member_const_block_after_line_break() {
    let test = TestParser::new(
        r#"const
{ assert(true) }"#,
    );
    let mut parser = test.prepare();
    let member_id = parser.parse_member().unwrap();

    assert_node!(parser.tree, member_id, Member::ConstBlock { body } => {
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

#[test]
fn test_parse_member_method_with_multiline_return_type() {
    let test = TestParser::declaration(
        r#"Type(object: unknown):
    | "Undefined"
    | "Boolean"
    | "String""#,
    );
    let mut parser = test.prepare();

    let member_id = parser.parse_member().unwrap();
    assert_node!(parser.tree, member_id, Member::Method { name: Some(name), signature, .. } => {
        assert_string!(parser, name.string(), "Type");
        assert_eq!(signature.parameters.len(), 1);
        assert!(signature.return_type.is_some());
        assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Union { .. });
    });
}

#[test]
fn test_parse_class_member_trailing_comments_stay_on_member_owner() {
    let test = TestParser::new(
        r#"class Box {
  first = 1 // first-tail
  second = 2 // second-tail
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_eq!(members.len(), 2);

            let first_annotations = parser.tree.get_decorators(members[0].id);
            assert!(first_annotations.is_empty());

            let second_annotations = parser.tree.get_decorators(members[1].id);
            assert!(second_annotations.is_empty());
        });
    });
    assert_eq!(parser.comments().len(), 2);
    assert_comment!(parser, 0, CommentKind::Line, "first-tail");
    assert_comment!(parser, 1, CommentKind::Line, "second-tail");
}
