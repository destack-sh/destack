use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

#[test]
fn test_parse_type_template_literal_with_generic_arguments() {
    let mut test = TestParser::new("type T = `foo-${Capitalize<K>}`");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = `foo-${Capitalize<K>}`
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::TemplateLiteral { strings, spans } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(spans.len(), 1);
                assert_string!(parser, strings[0], "foo-");
                assert_string!(parser, strings[1], "");
                assert_node!(parser.tree, spans[0], TypeExpression::Reference { path, generic_arguments } => {
                    assert_path!(parser, *path, "Capitalize");
                    assert_eq!(generic_arguments.len(), 1);
                    assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                                assert!(generic_arguments.is_empty());
                                assert_path!(parser, *path, "K");
                            });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_template_literal_with_union_interpolation_after_outer_union() {
    let mut test = TestParser::new(
        "type Issuer =\n  | \"https://oauth.battlenet.com.cn\"\n  | `https://${\"us\" | \"eu\"}.battle.net/oauth`",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type Issuer = | "https://oauth.battlenet.com.cn" | `https://${"us" | "eu"}.battle.net/oauth`
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[1], TypeExpression::TemplateLiteral { strings, spans } => {
                    assert_eq!(strings.len(), 2);
                    assert_eq!(spans.len(), 1);
                    assert_string!(parser, strings[0], "https://");
                    assert_string!(parser, strings[1], ".battle.net/oauth");

                    assert_node!(parser.tree, spans[0], TypeExpression::Union { elements } => {
                        assert_eq!(elements.len(), 2);
                        assert_node!(parser.tree, elements[0], TypeExpression::ScalarLiteral { value } => {
                            let ScalarLiteral::String(string_id) = value else {
                                panic!("expected string literal");
                            };
                            assert_string!(parser, *string_id, "us");
                        });
                        assert_node!(parser.tree, elements[1], TypeExpression::ScalarLiteral { value } => {
                            let ScalarLiteral::String(string_id) = value else {
                                panic!("expected string literal");
                            };
                            assert_string!(parser, *string_id, "eu");
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression() {
    let mut test = TestParser::new("type T = { readonly [K in keyof T as `foo-${K}`]-?: T[K] }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { readonly [K in keyof T as `foo-${K}`]-?: T[K] }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, readonly, optional, value } => {
                assert_eq!(*readonly, TypeModifier::Present);
                assert_eq!(*optional, TypeModifier::Remove);
                assert_string!(parser, parameter.name, "K");
                assert_node!(parser.tree, parameter.source_type, TypeExpression::KeyOf { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, parameter.key_remap.unwrap(), TypeExpression::TemplateLiteral { strings, spans } => {
                    assert_eq!(strings.len(), 2);
                    assert_eq!(spans.len(), 1);
                    assert_string!(parser, strings[0], "foo-");
                    assert_string!(parser, strings[1], "");
                    assert_expression_path!(parser, parser.tree.get(spans[0]), "K");
                });
                assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                    assert_node!(parser.tree, *left, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "T");
                    });
                    assert_node!(parser.tree, *index, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "K");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_literal_call_signature_with_parameters() {
    let mut test = TestParser::new(
        r#"type T = {
(num: number): number
(str: string): string
}"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { (num: number): number (str: string): string }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 2);
                // (num: number): number
                assert_node!(parser.tree, properties[0], TypeMember::CallSignature { signature } => {
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "num");
                        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                    });
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
                // (str: string): string
                assert_node!(parser.tree, properties[1], TypeMember::CallSignature { signature } => {
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "str");
                        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                    });
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_literal_construct_signature() {
    let mut test = TestParser::new("type T = { new (x: number): Foo }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { new (x: number): Foo }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::ConstructSignature { signature } => {
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type, .. } => {
                        assert_string!(parser, *name, "x");
                        assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                    });
                    assert_expression_path!(parser, parser.tree.get(signature.return_type.unwrap()), "Foo");
                });
            });
        });
    });
}

/// Parse type literal overloads with generic call signatures.
#[test]
fn test_parse_type_literal_generic_call_overloads() {
    let mut test = TestParser::new_with_options(
        r#"type Tmp = {
<N extends number>(num: N): typeof num
<S extends string>(str: S): typeof str
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type Tmp = { <N extends number>(num: N): typeof num <S extends string>(str: S): typeof str }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 2);
                // <N extends number>(num: N): typeof num
                assert_node!(parser.tree, properties[0], TypeMember::CallSignature { signature } => {

                    let generic_parameter_span = parser
                        .tree
                        .get_side_span(properties[0], NodeSpanType::Region(NodeSpanRegion::GenericParameters))
                        .expect("missing generic parameter span");
                    assert_eq!(parser.get_span_str(generic_parameter_span), "<N extends number>");

                    let parameter_span = parser
                        .tree
                        .get_side_span(properties[0], NodeSpanType::Region(NodeSpanRegion::Parameters))
                        .expect("missing parameter span");
                    assert_eq!(parser.get_span_str(parameter_span), "(num: N)");

                    let generic_parameters = &signature.generic_parameters;
                    assert_eq!(generic_parameters.len(), 1);
                    assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), .. } => {
                        assert_string!(parser, *name, "N");
                        assert_node!(parser.tree, *constraint, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Number);
                        });
                    });
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                        assert_string!(parser, *name, "num");
                        assert_node!(parser.tree, *declared_type, TypeExpression::Reference { path, .. } => {
                            assert_path!(parser, *path, "N");
                        });
                    });
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::TypeOfValue { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "num");
                    });
                });
                // <S extends string>(str: S): typeof str
                assert_node!(parser.tree, properties[1], TypeMember::CallSignature { signature } => {

                    let generic_parameter_span = parser
                        .tree
                        .get_side_span(properties[1], NodeSpanType::Region(NodeSpanRegion::GenericParameters))
                        .expect("missing generic parameter span");
                    assert_eq!(parser.get_span_str(generic_parameter_span), "<S extends string>");

                    let parameter_span = parser
                        .tree
                        .get_side_span(properties[1], NodeSpanType::Region(NodeSpanRegion::Parameters))
                        .expect("missing parameter span");
                    assert_eq!(parser.get_span_str(parameter_span), "(str: S)");

                    let generic_parameters = &signature.generic_parameters;
                    assert_eq!(generic_parameters.len(), 1);
                    assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(constraint), .. } => {
                        assert_string!(parser, *name, "S");
                        assert_node!(parser.tree, *constraint, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                    });
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(declared_type), .. } => {
                        assert_string!(parser, *name, "str");
                        assert_node!(parser.tree, *declared_type, TypeExpression::Reference { path, .. } => {
                            assert_path!(parser, *path, "S");
                        });
                    });
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::TypeOfValue { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "str");
                    });
                });
            });
        });
    });
}

/// Parse type literal overloads with generic call signatures returning paths.
#[test]
fn test_parse_type_literal_generic_call_overloads_with_path_returns() {
    let mut test = TestParser::new_with_options(
        r#"type Tmp = {
<N extends number>(num: N): MyType
<S extends string>(str: S): MyType
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type Tmp = { <N extends number>(num: N): MyType <S extends string>(str: S): MyType }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 2);
                assert_node!(parser.tree, properties[0], TypeMember::CallSignature { signature } => {
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Reference { path, .. } => {
                        assert_path!(parser, *path, "MyType");
                    });
                });
                assert_node!(parser.tree, properties[1], TypeMember::CallSignature { signature } => {
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Reference { path, .. } => {
                        assert_path!(parser, *path, "MyType");
                    });
                });
            });
        });
    });
}

/// Parse generic call signatures with a const type parameter and conditional mapped bound.
#[test]
fn test_parse_type_literal_call_signature_with_const_parameter_conditional_bound() {
    let mut test = TestParser::new_with_options(
        r#"type T = {
  <
Self extends Field<any> | Field.ValueAny,
const Mapping extends (Self extends Field<infer S> ? { readonly [K in keyof S]?: (variant: S[K]) => Field.ValueAny } : { readonly [K in Variants[number]]?: (variant: Self) => Field.ValueAny })
  >(f: Mapping): Self
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::CallSignature { signature } => {

                    let generic_parameters = &signature.generic_parameters;
                    assert_eq!(generic_parameters.len(), 2);

                    // Self extends Field<any> | Field.ValueAny
                    assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint: Some(ty), .. } => {
                        assert_string!(parser, *name, "Self");
                        assert_node!(parser.tree, *ty, TypeExpression::Union { elements } => {
                            assert_eq!(elements.len(), 2);
                        });
                    });

                    // const Mapping extends (...)
                    assert_node!(parser.tree, generic_parameters[1], GenericParameter::Value { name, declared_type: Some(ty), is_comptime, .. } => {
                        assert_string!(parser, *name, "Mapping");
                        assert!(*is_comptime);
                        assert_node!(parser.tree, *ty, TypeExpression::Parenthesized { expression } => {
                            assert_node!(parser.tree, *expression, TypeExpression::Conditional { .. });
                        });
                    });

                    // (f: Mapping): Self
                    assert_eq!(signature.parameters.len(), 1);
                    assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), .. } => {
                        assert_string!(parser, *name, "f");
                        assert_expression_path!(parser, parser.tree.get(*ty), "Mapping");
                    });
                    assert_expression_path!(parser, parser.tree.get(signature.return_type.expect("expected return type")), "Self");
                });
            });
        });
    });
}

/// Parse const type parameters when `extends` starts on the next line.
#[test]
fn test_parse_type_literal_call_signature_const_parameter_newline_extends() {
    let mut test = TestParser::new_with_options(
        r#"type T = {
  <
const Mapping
  extends string
  >(value: Mapping): Mapping
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::CallSignature { signature } => {
                    let generic_parameters = &signature.generic_parameters;
                    assert_eq!(generic_parameters.len(), 1);
                    assert_node!(parser.tree, generic_parameters[0], GenericParameter::Value { name, declared_type: Some(ty), is_comptime, .. } => {
                        assert_string!(parser, *name, "Mapping");
                        assert!(*is_comptime);
                        assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                    });
                });
            });
        });
    });
}

/// Parse typeof queries that target readonly named values.
#[test]
fn test_parse_typeof_query_with_readonly_identifier() {
    let mut test =
        TestParser::new_with_options("type T = typeof readonly", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // type T = typeof readonly
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::TypeOfValue { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "readonly");
            });
        });
    });
}

/// Parse typeof queries that target type named values.
#[test]
fn test_parse_typeof_query_with_type_identifier() {
    let mut test = TestParser::new_with_options("type T = typeof type", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // type T = typeof type
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::TypeOfValue { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "type");
            });
        });
    });
}

#[test]
fn test_parse_typeof_query_missing_operand() {
    // type T = typeof
    let mut test = TestParser::new("type T = typeof");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.get_span_str(parser.errors[0].leaf_span()), "");

    // type T = typeof
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::TypeOfValue { value } => {
                assert_node!(parser.tree, *value, Expression::Missing);
            });
        });
    });
}

#[test]
fn test_parse_keyof_query_missing_operand() {
    // type T = keyof
    let mut test = TestParser::new("type T = keyof");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.get_span_str(parser.errors[0].leaf_span()), "");

    // type T = keyof
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::KeyOf { target_type } => {
                assert_node!(parser.tree, *target_type, TypeExpression::Missing);
            });
        });
    });
}

#[test]
fn test_parse_type_literal_abstract_construct_signature() {
    let mut test = TestParser::new("type T = { abstract new (x: number): Foo }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { abstract new (x: number): Foo }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::ConstructSignature { signature } => {
                    assert!(signature.is_abstract);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_literal_index_signature() {
    let mut test = TestParser::new("type T = { readonly [k: string]?: Foo }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { readonly [k: string]?: Foo }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::IndexSignature { is_optional, is_readonly, name, key_type, value_type } => {
                    assert!(*is_optional);
                    assert!(*is_readonly);
                    assert_string!(parser, *name, "k");
                    assert_node!(parser.tree, *key_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                    assert_expression_path!(parser, parser.tree.get(*value_type), "Foo");
                });
            });
        });
    });
}

#[test]
fn test_parse_type_literal_index_signature_union_key_on_union_rhs() {
    let mut test = TestParser::new("type T = string | { [x: string | number | symbol]: unknown }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = string | { [x: string | number | symbol]: unknown }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
                assert_node!(parser.tree, elements[1], TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], TypeMember::IndexSignature { name, key_type, value_type, .. } => {
                        assert_string!(parser, *name, "x");
                        assert_node!(parser.tree, *key_type, TypeExpression::Union { elements } => {
                            assert_eq!(elements.len(), 3);
                            assert_node!(parser.tree, elements[0], TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::String);
                            });
                            assert_node!(parser.tree, elements[1], TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::Number);
                            });
                            assert_node!(parser.tree, elements[2], TypeExpression::Literal { value } => {
                                assert_eq!(*value, TypeLiteral::Symbol);
                            });
                        });
                        assert_node!(parser.tree, *value_type, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::Unknown);
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_literal_index_signature_with_multiline_brackets() {
    let mut test = TestParser::new_with_options(
        r#"type T = {
  [
topic: string
  ]: number;
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // type T = { [topic: string]: number }
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::IndexSignature { name, key_type, value_type, .. } => {
                    assert_string!(parser, *name, "topic");
                    assert_node!(parser.tree, *key_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                    assert_node!(parser.tree, *value_type, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_literal_readonly_property_name() {
    let mut test = TestParser::new("type T = { readonly?: boolean }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { readonly?: boolean }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::Field { is_optional, is_readonly, key, declared_type } => {
                    assert!(*is_optional);
                    assert!(!*is_readonly);
                    match key {
                        Key::Name(Name::Identifier(name)) => {
                            assert_string!(parser, *name, "readonly");
                        }
                        _ => panic!("expected Key::Name, got {key:?}"),
                    }
                    assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Boolean);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_literal_computed_key() {
    let mut test = TestParser::new("type T = { [mismatch]: string }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { [mismatch]: string }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::Field { key, declared_type, .. } => {
                    match key {
                        Key::Expression(key) => {
                            assert_expression_path!(parser, parser.tree.get(*key), "mismatch");
                        }
                        _ => panic!("expected Key::Expression, got {key:?}"),
                    }
                    assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_intrinsic_type_alias() {
    let mut test = TestParser::new("type Uppercase<S extends string> = intrinsic");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    // type Uppercase<S extends string> = intrinsic
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Intrinsic);
        });
    });
}

#[test]
fn test_parse_intrinsic_type_alias_keeps_non_bare_intrinsic_as_reference() {
    let mut test = TestParser::new("type Uppercase<S extends string> = intrinsic<string>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);

    // type Uppercase<S extends string> = intrinsic<string>
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "intrinsic");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                });
            });
        });
    });
}

/// Generic arrow function types work in declaration files.
#[test]
fn test_parse_generic_arrow_function_type() {
    let mut test = TestParser::new_with_options(
        "type ClassDecorator = <TFunction extends Function>(target: TFunction) => TFunction | void",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type ClassDecorator = <TFunction extends Function>(target: TFunction) => TFunction | void
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, .. }) => {
            assert_string!(parser, name.string(), "ClassDecorator");
        });
    });
}

/// Arrow function type with conditional return.
#[test]
fn test_parse_type_arrow_with_conditional_return() {
    let mut test = TestParser::new_with_options(
        "type T = <X>() => X extends A | B ? true : false",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = <X>() => X extends A | B ? true : false
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FunctionTypeDeclaration(function) => {
                assert_node!(parser.tree, function.return_type.unwrap(), TypeExpression::Conditional { .. });
            });
        });
    });
}

/// Nested conditional types with arrow functions (expect-type pattern).
#[test]
fn test_parse_type_nested_conditional_with_arrows() {
    let input = r#"type StrictEqual<L, R> =
  (<T>() => T extends (L & T) | T ? true : false) extends <T>() => T extends (R & T) | T ? true : false
? IsNever<L> extends IsNever<R>
  ? true
  : false
: false"#;
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptDeclaration);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type StrictEqual<L, R> = ... ? IsNever<L> extends IsNever<R> ? true : false : false
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { then_type, else_type, .. } => {
                assert_node!(parser.tree, *then_type, TypeExpression::Conditional { .. });
                assert_node!(parser.tree, *else_type, TypeExpression::ScalarLiteral { value } => {
                    assert_eq!(*value, ScalarLiteral::Boolean(false));
                });
            });
        });
    });
}

/// Generic arrow function with complex constraint as property type.
#[test]
fn test_parse_type_member_generic_arrow_complex_constraint() {
    let input = r#"type T = {
  method: <Expected extends IsUnion<Expected> extends true ? "error" : SomeType>(arg: Expected) => true;
}"#;
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptDeclaration);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { method: <...>(...) => true }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::Field { declared_type, .. } => {
                    assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::FunctionTypeDeclaration(function) => {
                        assert!(!function.generic_parameters.is_empty());
                    });
                });
            });
        });
    });
}

/// Keep type literal fields named `where` after function types.
#[test]
fn test_parse_type_literal_where_field_after_function_type() {
    let input = r#"type T = {
  setSelectedFields: (fields: FieldOption[]) => void
  where?: Where
}"#;
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // type T = { setSelectedFields: (fields: FieldOption[]) => void; where?: Where }
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 2);

                // setSelectedFields: (fields: FieldOption[]) => void
                assert_node!(parser.tree, properties[0], TypeMember::Field { key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                    assert_string!(parser, *name, "setSelectedFields");
                    assert_node!(parser.tree, declared_type.expect("expected declared type"), TypeExpression::FunctionTypeDeclaration(function) => {
                        assert_eq!(function.parameters.len(), 1);
                    });
                });

                // where?: Where
                assert_node!(parser.tree, properties[1], TypeMember::Field { is_optional, key: Key::Name(Name::Identifier(name)), declared_type, .. } => {
                    assert!(*is_optional);
                    assert_string!(parser, *name, "where");
                    assert_expression_path!(parser, parser.tree.get(declared_type.expect("expected declared type")), "Where");
                });
            });
        });
    });
}

/// Generic arrow functions in generic arguments.
#[test]
fn test_parse_type_generic_arrow_in_generic_arguments() {
    let input = "type T = Extends<<T>() => T extends X ? true : false, <T>() => T extends Y ? true : false>";
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScriptDeclaration);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = Extends<arrow1, arrow2>
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { generic_arguments, .. } => {
                let args = generic_arguments;
                assert_eq!(args.len(), 2);
            });
        });
    });
}

#[test]
fn test_parse_type_path_empty_generic_arguments_recovers_error_slot() {
    // type T = Container<>
    let mut test = TestParser::new("type T = Container<>");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_eq!(parser.errors.len(), 1);

    // type T = Container<>
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Container");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Error);
            });
        });
    });
}

#[test]
fn test_parse_type_literal_call_signature() {
    let mut test = TestParser::new("type T = { (): string }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { (): string }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                assert_eq!(properties.len(), 1);
                assert_node!(parser.tree, properties[0], TypeMember::CallSignature { signature } => {
                    assert_node!(parser.tree, signature.return_type.unwrap(), TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_template_literal() {
    let mut test = TestParser::new("type T = `foo-${Bar}`");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = `foo-${Bar}`
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::TemplateLiteral { strings, spans } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(spans.len(), 1);
                assert_string!(parser, strings[0], "foo-");
                assert_string!(parser, strings[1], "");
                assert_expression_path!(parser, parser.tree.get(spans[0]), "Bar");
            });
        });
    });
}
