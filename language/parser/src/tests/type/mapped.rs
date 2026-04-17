use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

#[test]
fn test_parse_mapped_type() {
    let input = r#"type A = { [test in "a" | "b"] }
 type OptionsFlags<Type> = {
   [Property in keyof Type]: boolean;
 };
 type CreateMutable<Type> = {
 	-readonly [Property in keyof Type]: Type[Property];
 };
 type Concrete<Type> = {
   [Property in keyof Type]-?: Type[Property]
 };
 type Getters<Type> = {
 [Property in keyof Type as `get${Capitalize<string & Property>}`]: () => Type[Property]
 };
"#;
    let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 5);

    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
            assert_string!(parser, name.string(), "A");
            assert!(generic_parameters.is_empty());
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, readonly, optional, value } => {
                assert_string!(parser, parameter.name, "test");
                assert_eq!(*readonly, TypeModifier::None);
                assert_eq!(*optional, TypeModifier::None);
                assert_node!(parser.tree, parameter.source_type, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], TypeExpression::ScalarLiteral { value } => {
                        assert_node!(value, ScalarLiteral::String(value) => {
                            assert_string!(parser, *value, "a");
                        });
                    });
                    assert_node!(parser.tree, elements[1], TypeExpression::ScalarLiteral { value } => {
                        assert_node!(value, ScalarLiteral::String(value) => {
                            assert_string!(parser, *value, "b");
                        });
                    });
                });
                assert_node!(parser.tree, *value, TypeExpression::Missing);
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
            assert_string!(parser, name.string(), "OptionsFlags");
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "Type");
            });
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, readonly, optional, value } => {
                assert_string!(parser, parameter.name, "Property");
                assert_eq!(*readonly, TypeModifier::None);
                assert_eq!(*optional, TypeModifier::None);
                assert!(parameter.key_remap.is_none());
                assert_node!(parser.tree, parameter.source_type, TypeExpression::KeyOf { target_type } => {
                    assert_expression_path!(parser, parser.tree.get(*target_type), "Type");
                });
                assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Boolean);
                });
            });
        });
    });

    assert_node!(parser.tree, expressions[2], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
            assert_string!(parser, name.string(), "CreateMutable");
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "Type");
            });
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, readonly, optional, value } => {
                assert_string!(parser, parameter.name, "Property");
                assert_eq!(*readonly, TypeModifier::Remove);
                assert_eq!(*optional, TypeModifier::None);
                assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "Type");
                    assert_expression_path!(parser, parser.tree.get(*index), "Property");
                });
            });
        });
    });

    assert_node!(parser.tree, expressions[3], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
            assert_string!(parser, name.string(), "Concrete");
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "Type");
            });
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, readonly, optional, value } => {
                assert_string!(parser, parameter.name, "Property");
                assert_eq!(*readonly, TypeModifier::None);
                assert_eq!(*optional, TypeModifier::Remove);
                assert_node!(parser.tree, *value, TypeExpression::Index { left, index } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "Type");
                    assert_expression_path!(parser, parser.tree.get(*index), "Property");
                });
            });
        });
    });

    assert_node!(parser.tree, expressions[4], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { name, generic_parameters, value, .. }) => {
            assert_string!(parser, name.string(), "Getters");
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "Type");
            });
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, readonly, optional, value } => {
                assert_string!(parser, parameter.name, "Property");
                assert_eq!(*readonly, TypeModifier::None);
                assert_eq!(*optional, TypeModifier::None);
                assert_node!(parser.tree, parameter.source_type, TypeExpression::KeyOf { target_type } => {
                    assert_expression_path!(parser, parser.tree.get(*target_type), "Type");
                });
                assert_node!(parser.tree, parameter.key_remap.expect("expected key remap"), TypeExpression::TemplateLiteral { strings, spans } => {
                    assert_eq!(strings.len(), 2);
                    assert_eq!(spans.len(), 1);
                    assert_string!(parser, strings[0], "get");
                    assert_string!(parser, strings[1], "");
                });
                assert_node!(parser.tree, *value, TypeExpression::Declaration { declaration } => {
                    assert_node!(parser.tree, *declaration, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                        assert_eq!(signature.parameters.len(), 0);
                        assert!(signature.return_type.is_some());
                    });
                });
            });
        });
    });
}

/// Parse mapped types in generic type arguments with readonly removal.
#[test]
fn test_parse_type_mapped_expression_in_generic_arguments() {
    let mut test = TestParser::new("type T = Promise<{ -readonly [P in keyof T]: T[P] }>");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Promise");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Mapped { readonly, optional, .. } => {
                            assert_eq!(*readonly, TypeModifier::Remove);
                            assert_eq!(*optional, TypeModifier::None);
                        });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_with_newline_between_plus_and_readonly() {
    let mut test = TestParser::new("type T = { +\nreadonly [K in keyof T]: T[K] }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { +\nreadonly [K in keyof T]: T[K] }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { readonly, optional, .. } => {
                assert_eq!(*readonly, TypeModifier::Add);
                assert_eq!(*optional, TypeModifier::None);
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_distinguishes_plain_and_explicit_add_modifiers() {
    let mut test = TestParser::new(
        "type A = { readonly [K in keyof T]?: T[K] }; type B = { +readonly [K in keyof T]+?: T[K] };",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { readonly, optional, .. } => {
                assert_eq!(*readonly, TypeModifier::Present);
                assert_eq!(*optional, TypeModifier::Present);
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { readonly, optional, .. } => {
                assert_eq!(*readonly, TypeModifier::Add);
                assert_eq!(*optional, TypeModifier::Add);
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_with_semicolon() {
    let mut test = TestParser::new("type T = { [K in T]: T[K]; }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { [K in T]: T[K]; }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { .. });
        });
    });
}

/// Parse mapped types without explicit value types.
#[test]
fn test_parse_type_mapped_expression_without_value_type() {
    let mut test = TestParser::new("type Keys = 'a' | 'b'; type A = { [K in Keys] };");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { readonly, optional, value, .. } => {
                assert_eq!(*readonly, TypeModifier::None);
                assert_eq!(*optional, TypeModifier::None);
                assert_node!(parser.tree, *value, TypeExpression::Missing);
            });
        });
    });
}

/// Parse mapped types with readonly and optional modifiers without explicit value types.
#[test]
fn test_parse_type_mapped_expression_without_value_type_with_modifiers() {
    let mut test =
        TestParser::new("type A = { +readonly [T in number]; }; type B = { [K in number]+? };");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.errors.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { readonly, optional, value, .. } => {
                assert_eq!(*readonly, TypeModifier::Add);
                assert_eq!(*optional, TypeModifier::None);
                assert_node!(parser.tree, *value, TypeExpression::Missing);
            });
        });
    });

    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { readonly, optional, value, .. } => {
                assert_eq!(*readonly, TypeModifier::None);
                assert_eq!(*optional, TypeModifier::Add);
                assert_node!(parser.tree, *value, TypeExpression::Missing);
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_with_intersection() {
    let mut test = TestParser::new(
        "type T = { [P in keyof T]: T[P]; } & { [x: string]: PropertyDescriptor; }",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { [P in keyof T]: T[P]; } & { [x: string]: PropertyDescriptor; }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TypeExpression::Mapped { .. });
                assert_node!(parser.tree, elements[1], TypeExpression::Object { members: properties } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], TypeMember::IndexSignature { name, key_type, value_type, .. } => {
                        assert_string!(parser, *name, "x");
                        assert_node!(parser.tree, *key_type, TypeExpression::Literal { value } => {
                            assert_eq!(*value, TypeLiteral::String);
                        });
                        assert_node!(parser.tree, *value_type, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "PropertyDescriptor");
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_with_key_remap_conditional() {
    let mut test =
        TestParser::new("type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, .. } => {
                assert_string!(parser, parameter.name, "K");
                let key_remap = parameter.key_remap.expect("expected key remap");
                assert_node!(parser.tree, key_remap, TypeExpression::Conditional { .. });
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_in_declaration_file() {
    let mut test = TestParser::new_with_options(
        "type T = { [K in keyof T]: T[K] }",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = { [K in keyof T]: T[K] }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { .. });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_with_remap_in_declaration_file() {
    let mut test = TestParser::new_with_options(
        "type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }",
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T<O> = { [K in keyof O as O[K] extends {} ? K : never]: O[K] }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, .. } => {
                assert_string!(parser, parameter.name, "K");
                assert!(parameter.key_remap.is_some());
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_missing_value_type() {
    // type T = { [K in keyof T]: }
    let mut test = TestParser::new("type T = { [K in keyof T]: }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_eq!(parser.errors.len(), 1);

    // type T = { [K in keyof T]: }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, value, .. } => {
                assert_string!(parser, parameter.name, "K");
                assert_node!(parser.tree, parameter.source_type, TypeExpression::KeyOf { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, *value, TypeExpression::Missing);
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_missing_close_bracket_before_colon() {
    // type T = { [K in keyof T: T[K] }
    let mut test = TestParser::new("type T = { [K in keyof T: T[K] }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_eq!(parser.errors.len(), 1);

    // type T = { [K in keyof T: T[K] }
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, value, .. } => {
                assert_string!(parser, parameter.name, "K");
                assert_node!(parser.tree, parameter.source_type, TypeExpression::KeyOf { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "T");
                    });
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
fn test_parse_type_mapped_expression_with_leading_union_constraint() {
    let mut test = TestParser::new_with_options(
        r#"type T = {
  /* head */
  [K in
| Foo
| Bar]: string
}"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, value, .. } => {
                assert_string!(parser, parameter.name, "K");

                assert_node!(parser.tree, parameter.source_type, TypeExpression::Union { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_expression_path!(parser, parser.tree.get(elements[0]), "Foo");
                    assert_expression_path!(parser, parser.tree.get(elements[1]), "Bar");
                });

                assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
        });
    });
}

#[test]
fn test_parse_type_mapped_expression_with_newline_before_remap_in_declaration_file() {
    let mut test = TestParser::new_with_options(
        r#"type T<O> = {
  [K in keyof O
  as O[K] extends {} ? K : never]: O[K]
}"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Mapped { parameter, .. } => {
                assert_string!(parser, parameter.name, "K");
                assert_node!(
                    parser.tree,
                    parameter.key_remap.expect("expected key remap"),
                    TypeExpression::Conditional { .. }
                );
            });
        });
    });
}

#[test]
fn test_parse_leading_intersection_with_mapped_types_in_declaration_file() {
    let mut test = TestParser::new_with_options(
        r#"type T = (
  & { [K in keyof T]: T[K] }
  & { [K in keyof T as K extends string ? K : never]: T[K] }
)"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = (& { [K in keyof T]: T[K] } & { [K in keyof T as K extends string ? K : never]: T[K] })
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], TypeExpression::Mapped { .. });
                    assert_node!(parser.tree, elements[1], TypeExpression::Mapped { .. });
                });
            });
        });
    });
}

#[test]
fn test_parse_leading_intersection_in_declaration_file() {
    let mut test = TestParser::new_with_options(
        r#"type T = (
  & A
  & B
)"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = (& A & B)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_expression_path!(parser, parser.tree.get(elements[0]), "A");
                    assert_expression_path!(parser, parser.tree.get(elements[1]), "B");
                });
            });
        });
    });
}

#[test]
fn test_parse_leading_intersection_with_mapped_type_and_path_in_declaration_file() {
    let mut test = TestParser::new_with_options(
        r#"type T = (
  & { [K in keyof T]: T[K] }
  & A
)"#,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = (& { [K in keyof T]: T[K] } & A)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, TypeExpression::Intersection { elements } => {
                    assert_eq!(elements.len(), 2);
                    assert_node!(parser.tree, elements[0], TypeExpression::Mapped { .. });
                    assert_expression_path!(parser, parser.tree.get(elements[1]), "A");
                });
            });
        });
    });
}

#[test]
fn test_parse_leading_union_in_type_alias() {
    let mut test = TestParser::new_with_options(
        r#"type IframeChannelIncomingEvent
  = | IframeViewportEvent
| ChannelDoneEvent"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Union { elements } => {
                assert_eq!(elements.len(), 2);
                assert_expression_path!(parser, parser.tree.get(elements[0]), "IframeViewportEvent");
                assert_expression_path!(parser, parser.tree.get(elements[1]), "ChannelDoneEvent");
            });
        });
    });
}
