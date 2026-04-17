use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

/// Parse an if extends condition without consuming the block.
#[test]
fn test_parse_if_extends_type_reference() {
    let mut test = TestParser::new(
        r#"if x extends Foo {
    body
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
        assert!(else_expression.is_none());
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        // x extends Foo
        assert_node!(parser.tree, condition_id, Expression::Type { value } => {
            assert_node!(parser.tree, *value, TypeExpression::Conditional { left, extends_type, then_type, else_type } => {
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                assert_expression_path!(parser, parser.tree.get(*extends_type), "Foo");
                assert_node!(parser.tree, *then_type, TypeExpression::Missing);
                assert_node!(parser.tree, *else_type, TypeExpression::Missing);
            });
        });
        // { body }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let body_statement_id = parser.unwrap_labelled_expression(expressions[0]);
                assert_expression_path!(parser, parser.tree.get(body_statement_id), "body");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse an if instanceof condition inside parentheses.
#[test]
fn test_parse_if_instanceof_type_reference() {
    let mut test = TestParser::new(
        r#"if (T instanceof Foo) {
    value
}"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
        assert!(else_expression.is_none());
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };
        // T instanceof Foo
        assert_node!(parser.tree, condition_id, Expression::InstanceOf { value, target } => {
            assert_expression_path!(parser, parser.tree.get(*value), "T");
            assert_expression_path!(parser, parser.tree.get(*target), "Foo");
        });
        // { value }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let value_statement_id = parser.unwrap_labelled_expression(expressions[0]);
                assert_expression_path!(parser, parser.tree.get(value_statement_id), "value");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse `value is string` as one runtime guard expression.
#[test]
fn test_parse_if_is_type_guard() {
    let mut test = TestParser::new(
        r"
if value is string {
  value
}
",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expr_id, Expression::If { condition, then_expression, .. } => {
        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => panic!("expected expression condition"),
        };

        // value is string
        assert_node!(parser.tree, condition_id, Expression::Is { value, target_type } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
            assert_node!(parser.tree, *target_type, TypeExpression::Literal { value } => {
                assert_eq!(*value, TypeLiteral::String);
            });
        });

        // { value }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);

                let value_statement_id = parser.unwrap_labelled_expression(expressions[0]);
                assert_expression_path!(parser, parser.tree.get(value_statement_id), "value");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse `export { bar, baz } from foo`.
#[test]
fn test_parse_export_expression_with_items_block() {
    let mut test = TestParser::new("export { bar, baz } from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // export { bar, baz } from foo
    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export { bar, baz }`.
#[test]
fn test_parse_export_expression_items_without_target() {
    let mut test = TestParser::new("export { bar, baz }");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: None, items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export type { Foo, Bar } from "module"`.
#[test]
fn test_parse_export_expression_type_items_with_target() {
    let mut test = TestParser::new("export type { Foo, Bar } from \"module\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
        assert_eq!(*kind, DependencyKind::Type);
        assert_string!(parser, *target, "module");
        assert_eq!(items.len(), 2);
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "Foo");
            assert!(alias.is_none());
        });
        assert_node!(parser.tree, items[1], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "Bar");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export type { Foo }`.
#[test]
fn test_parse_export_expression_type_items_without_target() {
    let mut test = TestParser::new("export type { Foo }");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: None, items, .. } => {
        assert_eq!(*kind, DependencyKind::Type);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "Foo");
            assert!(alias.is_none());
        });
    });
}

/// Parse `export * as baz from "foo"`.
#[test]
fn test_parse_export_expression_namespace_alias() {
    let mut test = TestParser::new("export * as baz from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // export * as baz from foo
    assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 1);
        // * as baz
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
            assert_eq!(*mode, DependencyMode::Namespace);
            assert_string!(parser, *alias, "baz");
        });
    });
}

/// Parse `export = foo`.
#[test]
fn test_parse_export_expression_module_export() {
    let mut test = TestParser::new("export = foo");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Export { kind, target, items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        // = foo
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: None, value: Some(value), .. } => {
            assert_eq!(*mode, DependencyMode::Namespace);
            assert_expression_path!(parser, parser.tree.get(*value), "foo");
        });
    });
}

/// Parse an export declaration of a type declaration.
#[test]
fn test_parse_export_expression_type_declaration() {
    let mut test = TestParser::new("export type NonNullValue = Something");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, export, .. }) => {
            assert_string!(parser, name.string(), "NonNullValue");
            assert!(export.is_some());
        });
    });
}

/// Parse export default abstract class with decorator prefixes.
#[test]
fn test_parse_export_default_abstract_class_with_decorator_prefixes() {
    let mut test = TestParser::new_with_options(
        "@before\nexport default @after abstract class Foo { }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Export { kind, items, .. } => {
        assert_eq!(*kind, DependencyKind::Value);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, value: Some(value), .. } => {
            assert_eq!(*mode, DependencyMode::Default);
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, .. }) => {
                    assert_string!(parser, name.expect("expected class name").string(), "Foo");
                });
            });
        });
    });
}

/// Disambiguate using `type` as a variable.
#[test]
fn test_parse_type_as_variable() {
    let mut test = TestParser::new(
        r"
let type = 1
type = type * 2
",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();

    // let type = 1
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value: Some(value), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "type");
            });
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
    parser.eat_newline().unwrap();

    // type = type * 2
    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "type");
        assert_eq!(*operator, AssignOperator::Assign);

        // type * 2
        assert_node!(parser.tree, *right, Expression::Binary { left, operator, right, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "type");
            assert_eq!(*operator, BinaryOperator::Multiply);
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    });
    parser.eat_newline().unwrap();
}

/// Parse `namespace[this.dest] = values`.
#[test]
fn test_parse_namespace_as_identifier_in_index_assignment() {
    let mut test =
        TestParser::new_with_options("namespace[this.dest] = values", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
        assert_eq!(*operator, AssignOperator::Assign);

        assert_node!(parser.tree, *left, Expression::Index { left, index, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "namespace");

            assert_node!(parser.tree, index.expect("expected index"), Expression::Member { left, name, .. } => {
                assert_node!(parser.tree, *left, Expression::This);
                assert_string!(parser, *name, "dest");
            });
        });

        assert_expression_path!(parser, parser.tree.get(*right), "values");
    });
}

#[test]
fn test_parse_override_as_identifier_call() {
    for language in [LanguageType::TypeScript, LanguageType::Destack] {
        let mut test = TestParser::new_with_options("override(value)", language);
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression(parser.options).unwrap();
        assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "override");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "value");
            });
        });
    }
}

#[test]
fn test_parse_abstract_as_identifier_call() {
    let mut test = TestParser::new("abstract(value)");
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "abstract");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

#[test]
fn test_parse_type_as_identifier_call() {
    let mut test = TestParser::new_with_options("type(123)", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.options).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "type");
        assert_eq!(arguments.len(), 1);
        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(123)));
        });
    });
}

/// Parse `abstract\nclass B {}` as `abstract; class B {}`.
#[test]
fn test_parse_abstract_newline_as_identifier_then_class() {
    let mut test = TestParser::new_with_options("abstract\nclass B {}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);
    assert_expression_path!(parser, parser.tree.get(expressions[0]), "abstract");
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, .. }) => {
            assert_string!(parser, name.expect("expected class name").string(), "B");
        });
    });
}

/// Reject `declare enum\nE\n{}` and preserve the expression sequence.
#[test]
fn test_reject_declare_enum_newline_and_preserve_expression_sequence() {
    let mut test = TestParser::new_with_options("declare enum\nE\n{}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_error_leaves(&parser, &[(None, None, "enum")]);

    assert_eq!(expressions.len(), 4);
    assert_expression_path!(parser, parser.tree.get(expressions[0]), "declare");
    assert_expression_path!(parser, parser.tree.get(expressions[1]), "enum");
    assert_expression_path!(parser, parser.tree.get(expressions[2]), "E");
    assert_node!(parser.tree, expressions[3], Expression::Block(..));
}

/// Parse `type\nFoo = string;` as `type; Foo = string`.
#[test]
fn test_parse_type_newline_as_identifier_then_assignment() {
    let mut test = TestParser::new_with_options("type\nFoo = string;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);
    assert_expression_path!(parser, parser.tree.get(expressions[0]), "type");
    assert_node!(parser.tree, expressions[1], Expression::Assign { left, operator, right, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "Foo");
        assert_eq!(*operator, AssignOperator::Assign);
        assert_expression_path!(parser, parser.tree.get(*right), "string");
    });
}

/// Parse `type` as a callback parameter and statement identifier.
#[test]
fn test_parse_callback_parameter_named_type() {
    let mut test = TestParser::new_with_options(
        "avplay.setListener({
    onsubtitlechange: (duration, subtitles, type, attributes) => {
        duration // $ExpectType string
        subtitles // $ExpectType string
        type // $ExpectType string
        attributes // $ExpectType AVPlaySubtitleAttribute[]
    }
})",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, expression_id, Expression::Call { left, arguments, .. } => {
        assert_expression_path!(parser, parser.tree.get(*left), "avplay.setListener");
        assert_eq!(arguments.len(), 1);

        assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::ObjectExpression { properties, .. } => {
                assert_eq!(properties.len(), 1);

                assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value } => {
                    assert_string!(parser, *name, "onsubtitlechange");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(body), .. }) => {
                            assert_eq!(signature.parameters.len(), 4);

                            // duration, subtitles, type, attributes
                            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "duration");
                            });
                            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "subtitles");
                            });
                            assert_node!(parser.tree, signature.parameters[2], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "type");
                            });
                            assert_node!(parser.tree, signature.parameters[3], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "attributes");
                            });

                            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                                let expressions = block_expression_ids(parser.tree.get(*block_id));
                                assert_eq!(expressions.len(), 4);
                                assert_expression_path!(parser, parser.tree.get(expressions[0]), "duration");
                                assert_expression_path!(parser, parser.tree.get(expressions[1]), "subtitles");
                                assert_expression_path!(parser, parser.tree.get(expressions[2]), "type");
                                assert_expression_path!(parser, parser.tree.get(expressions[3]), "attributes");
                            });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_reject_export_type_without_binding_or_declaration() {
    let mut test = TestParser::new("export type");
    let mut parser = test.prepare();
    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

#[test]
fn test_reject_export_path_expression() {
    for language in [LanguageType::JavaScript, LanguageType::Destack] {
        let mut test = TestParser::new_with_options("export foo", language);
        let mut parser = test.prepare();
        let error = parser.eat_expression(parser.options).unwrap_err();

        assert_eq!(parser.get_span_str(error.leaf_span()), "foo");
    }
}

#[test]
fn test_reject_export_default_enum() {
    let mut test = TestParser::new("export default enum A { X, Y, Z }");
    let mut parser = test.prepare();
    let result = parser.eat_expression(parser.options);
    assert!(result.is_err());
}

/// Parse `export import foo = bar.baz`.
#[test]
fn test_parse_export_import_equals() {
    let mut test = TestParser::new("export import atob = globalThis.atob");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, export, kind, target, .. }) => {
            assert!(export.is_some());
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, name.string(), "atob");
            match target {
                ImportAliasTarget::Path { path } => {
                    assert_path!(parser, *path, "globalThis.atob");
                }
                ImportAliasTarget::Require { .. } => {
                    panic!("expected import alias path");
                }
            }
        });
    });
}

/// Parse `export import type React = require("react")`.
#[test]
fn test_parse_export_import_type_equals_require() {
    let mut test = TestParser::new(r#"export import type React = require("react")"#);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, export, kind, target, .. }) => {
            assert!(export.is_some());
            assert_eq!(*kind, DependencyKind::Type);
            assert_string!(parser, name.string(), "React");
            match target {
                ImportAliasTarget::Require { target } => {
                    assert_string!(parser, *target, "react");
                }
                ImportAliasTarget::Path { .. } => {
                    panic!("expected import alias require");
                }
            }
        });
    });
}

#[test]
fn test_parse_export_import_type_equals_require_with_newlines() {
    let mut test = TestParser::new_with_options(
        r#"
export
import
type
React = require("react")
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, export, kind, target, .. }) => {
            assert!(export.is_some());
            assert_eq!(*kind, DependencyKind::Type);
            assert_string!(parser, name.string(), "React");
            match target {
                ImportAliasTarget::Require { target } => {
                    assert_string!(parser, *target, "react");
                }
                ImportAliasTarget::Path { .. } => {
                    panic!("expected import alias require");
                }
            }
        });
    });
}

/// Parse `import { bar, baz } from foo`.
#[test]
fn test_parse_import_expression_with_items_block() {
    let mut test = TestParser::new("import { bar, baz } from \"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // import { bar, baz } from foo
    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_import_target_string(&parser, target, "foo");
            let items = items.as_deref().expect("expected import specifier shell");
            assert_eq!(items.len(), 2);
        // bar
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "bar");
            assert!(alias.is_none());
        });
        // baz
        assert_node!(parser.tree, items[1], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
            assert_eq!(*mode, DependencyMode::Item);
            assert_string!(parser, name.string(), "baz");
            assert!(alias.is_none());
        });
    });
}

/// Parse `import * as baz from "foo" with { bar: true }`.
#[test]
fn test_parse_import_expression_namespace_alias_with_arguments() {
    let mut test = TestParser::new("import * as baz from \"foo\" with { bar: true }");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    // import * as baz from foo with { bar: true }
    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, attributes, arguments: None, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_import_target_string(&parser, target, "foo");
            let items = items.as_deref().expect("expected import specifier shell");
            assert_eq!(items.len(), 1);
        // * as baz
        assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
            assert_eq!(*mode, DependencyMode::Namespace);
            assert_string!(parser, *alias, "baz");
        });

        // with { bar: true }
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        let attributes = &attributes.attributes;
        assert_eq!(attributes.len(), 1);
    });
}

/// Reject `import { foo }` without a target.
#[test]
fn test_parse_import_expression_items_without_target_error() {
    let mut test = TestParser::new("import { foo }");
    let mut parser = test.prepare();
    assert!(parser.eat_expression(parser.options).is_err());
}

/// Parse `import("foo")`.
#[test]
fn test_parse_import_call_expression() {
    let mut test = TestParser::new("import(\"foo\")");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
        assert_eq!(*source, ImportSource::ImportCall);
        assert_eq!(*kind, DependencyKind::Value);
        assert_import_target_string(&parser, target, "foo");
        assert!(items.is_none());
    });
}

/// Parse `import("foo", { assert: { type: "json" } })`.
#[test]
fn test_parse_import_call_with_assertions() {
    let mut test = TestParser::new("import(\"foo\", { assert: { type: \"json\" } })");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: Some(arguments), .. } => {
        assert_eq!(*source, ImportSource::ImportCall);
        assert_eq!(*kind, DependencyKind::Value);
        assert_import_target_string(&parser, target, "foo");
        assert!(items.is_none());
        assert_eq!(arguments.len(), 1);
    });
}

/// Parse dynamic import calls with non-literal targets.
#[test]
fn test_parse_import_call_with_expression_target() {
    let mut test = TestParser::new(r#"import(join("file://", process.argv[2]))"#);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
        assert_eq!(*source, ImportSource::ImportCall);
        assert_eq!(*kind, DependencyKind::Value);
        assert!(items.is_none());
        assert_node!(target, ImportTarget::Expression { target } => {
            assert_node!(parser.tree, *target, Expression::Call { .. });
        });
    });
}

/// Recover a missing dynamic import target in place.
#[test]
fn test_parse_import_call_with_missing_target() {
    let mut test = TestParser::new("import()");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, ")")]);

    assert_node!(parser.tree, expression_id, Expression::Import { target, items, arguments: None, .. } => {
        assert!(items.is_none());
        assert_node!(target, ImportTarget::Expression { target } => {
            assert_node!(parser.tree, *target, Expression::Missing);
        });
    });
}

/// Recover a missing dynamic import close parenthesis at one boundary.
#[test]
fn test_parse_import_call_with_missing_close_parenthesis() {
    let mut test = TestParser::new("import(\"foo\"");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    assert_node!(parser.tree, expression_id, Expression::Import { target, items, arguments: None, .. } => {
        assert!(items.is_none());
        assert_import_target_string(&parser, target, "foo");
    });
}
