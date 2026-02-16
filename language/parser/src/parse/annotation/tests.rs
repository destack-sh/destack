use destack_ast::{
    Annotation, AnnotationPosition, Argument, Asynchrony, BinaryOperator, Blank, Block,
    BlockFormat, Comment, CommentStyle, Declaration, DeclarationAbstraction, DeclarationDescriptor,
    Declarator, Decorator, DependencyItem, Doc, DocStyle, Expression, FunctionKind, FunctionMode,
    IfCondition, IfKind, ImportTarget, Key, Member, Name, Parameter, Property, TypeBinaryOperator,
    TypeKind, TypeLiteral, TypeUnaryOperator, WhileKind,
};
use destack_source::LanguageType;

use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

/// Empty file with only a line comment should produce a Stub with the comment attached.
#[test]
fn test_attach_comment_to_stub_in_empty_file() {
    let mut test = TestParser::new("// just a comment");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // should have one Stub expression
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Stub => {});

    // the comment should be attached to the Stub as infix (inside the "empty" file)
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockInfix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "just a comment");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Leading comments before an empty statement should still attach without parse errors.
#[test]
fn test_attach_leading_comment_before_semicolon_statement() {
    let mut test = TestParser::new(
        r#"// lead
;value;"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let annotations = parser.tree.get_nodes::<Annotation>();
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, .. } => {
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "lead");
        });
    });
}

/// Block comments should retain all their newlines (including leading and trailing newlines).
#[test]
fn test_attach_block_comment_retain_newlines() {
    let mut test: TestParser = TestParser::new(
        r#"
/*
 * Comment 1
 */
let x;
/*
 * Comment 2.1
 * Comment 2.2
 * Comment 2.3
 */
let y;
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    parser.attach_trivia();
    assert_eq!(expressions.len(), 2);

    // let x;
    let x_annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(x_annotations.len(), 1);
    assert_node!(parser.tree, x_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "\nComment 1\n");
            assert_eq!(*style, CommentStyle::Star);
        });
    });
    // let y;
    let y_annotations = parser.tree.get_annotations(expressions[1].id);
    assert_eq!(y_annotations.len(), 1);
    assert_node!(parser.tree, y_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "\nComment 2.1\nComment 2.2\nComment 2.3\n");
            assert_eq!(*style, CommentStyle::Star);
        });
    });
}

/// Decorator annotations should be parsed around any block.
#[test]
fn test_attach_decorator_to_function() {
    let mut test = TestParser::new(
        r"@foo
function foo() { }",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function foo() { }
    let decorators = parser.tree.get_nodes::<Decorator>();
    assert_eq!(decorators.len(), 1, "decorators: {decorators:?}");
    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1, "annotations: {annotations:?}",);
    // @foo
    assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "foo");
        });
    });
}

/// Decorators on runtime parameters should be attached to the parameter node.
#[test]
fn test_attach_decorator_to_parameter() {
    let mut test = TestParser::new("function demo(@if(true) value: number): void { }");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function demo
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            // value: number
            assert_eq!(signature.dynamic_parameters.len(), 1);
            let parameter_id = signature.dynamic_parameters[0];
            let annotations = parser.tree.get_annotations(parameter_id.id);
            assert_eq!(annotations.len(), 1);

            // @if(true)
            assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "if");
                        assert_eq!(dynamic_arguments.len(), 1);
                    });
                });
            });
        });
    });
}

/// Decorators on multiline parameters should stay attached to the parameter node.
#[test]
fn test_attach_decorator_to_multiline_parameter() {
    let mut test = TestParser::new(
        r#"function process(
    @nonempty
    input: string,
) { return input }"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
            assert_eq!(signature.dynamic_parameters.len(), 1);
            let parameter_id = signature.dynamic_parameters[0];
            let annotations = parser.tree.get_annotations(parameter_id.id);
            assert_eq!(annotations.len(), 1);

            assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "nonempty");
                });
            });
        });
    });
}

/// Decorators on call arguments should be attached to the argument node.
#[test]
fn test_attach_decorator_to_call_argument() {
    let mut test = TestParser::new(
        r"function call(value: number): void { }
call(@if(true) 1);",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // call(@if(true) 1)
    assert_eq!(expressions.len(), 2);
    let call_expression_id = parser.unwrap_statement_expression(expressions[1]);
    assert_node!(parser.tree, call_expression_id, Expression::Call { dynamic_arguments, .. } => {
        // @if(true) 1
        assert_eq!(dynamic_arguments.len(), 1);
        let argument_id = dynamic_arguments[0];
        let annotations = parser.tree.get_annotations(argument_id.id);
        assert_eq!(annotations.len(), 1);

        // @if(true)
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "if");
                    assert_eq!(dynamic_arguments.len(), 1);
                });
            });
        });
    });
}

/// Decorators on class expressions in argument positions should attach to the class expression.
#[test]
fn test_attach_decorator_to_class_expression_in_call_argument() {
    let mut test =
        TestParser::new_with_options("use(@decorator class {})", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Declaration(_) => {});
            let annotations = parser.tree.get_annotations(dynamic_arguments[0].id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "decorator");
                });
            });
        });
    });
}

/// Decorators on class expressions in extends positions should attach to the class expression.
#[test]
fn test_attach_decorator_to_class_expression_in_extends_position() {
    let mut test = TestParser::new_with_options(
        "class Derived extends (@decorator class Base {}) {}",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(extends_types.len(), 1);
            assert_node!(parser.tree, extends_types[0], Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Declaration(_) => {});
                let annotations = parser.tree.get_annotations(expression.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_expression_path!(parser, parser.tree.get(*expression), "decorator");
                    });
                });
            });
        });
    });
}

/// Decorators after export modifiers should attach to the exported declaration expression.
#[test]
fn test_attach_decorator_after_export_modifier() {
    let mut test = TestParser::new_with_options(
        "export default @after abstract class Foo { }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // export default @after abstract class Foo { }
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Export { items, .. } => {
        assert_eq!(items.len(), 1);

        // default value
        let value_id = parser
            .tree
            .get(items[0])
            .value
            .expect("expected export default value");
        assert_node!(parser.tree, value_id, Expression::Declaration(_) => {});

        // @after
        let annotations = parser.tree.get_annotations(value_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "after");
            });
        });
    });
}

/// Top level export decorators should attach to export or class targets as expected.
#[test]
fn test_parse_javascript_decorator_export_top_level_sequences() {
    let mut test = TestParser::new_with_options(
        r"@decorator
export class Foo { }
@first.field @second @(() => decorator)()
export class Bar {}
@before
export @after class Foo { }
@before
export abstract class Foo { }
@before
export @after abstract class Foo { }",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // @decorator ... export class ...
    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 5);

    // @decorator export class Foo {}
    let first_expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, first_expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
            assert!(descriptor.export.is_some());
            assert_eq!(descriptor.abstraction, DeclarationAbstraction::Concrete);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
        });
    });
    let first_annotations = parser.tree.get_annotations(first_expression_id.id);
    assert_eq!(first_annotations.len(), 1);
    assert_node!(parser.tree, first_annotations[0], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "decorator");
        });
    });

    // @first.field @second @(() => decorator)() export class Bar {}
    let second_expression_id = parser.unwrap_statement_expression(expressions[1]);
    assert_node!(parser.tree, second_expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
            assert!(descriptor.export.is_some());
            assert_eq!(descriptor.abstraction, DeclarationAbstraction::Concrete);
            assert_string!(parser, descriptor.name.unwrap().string(), "Bar");
        });
    });
    let second_annotations = parser.tree.get_annotations(second_expression_id.id);
    assert_eq!(second_annotations.len(), 3);
    assert_node!(parser.tree, second_annotations[0], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "first.field");
        });
    });
    assert_node!(parser.tree, second_annotations[1], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "second");
        });
    });
    assert_node!(parser.tree, second_annotations[2], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_node!(parser.tree, *expression, Expression::Call { .. } => {});
        });
    });

    // @before export @after class Foo {}
    let third_expression_id = parser.unwrap_statement_expression(expressions[2]);
    assert_node!(parser.tree, third_expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
            assert!(descriptor.export.is_some());
            assert_eq!(descriptor.abstraction, DeclarationAbstraction::Concrete);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
        });
    });
    let third_annotations = parser.tree.get_annotations(third_expression_id.id);
    assert_eq!(third_annotations.len(), 2);
    assert_node!(parser.tree, third_annotations[0], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "before");
        });
    });
    assert_node!(parser.tree, third_annotations[1], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "after");
        });
    });

    // @before export abstract class Foo {}
    let fourth_expression_id = parser.unwrap_statement_expression(expressions[3]);
    assert_node!(parser.tree, fourth_expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
            assert!(descriptor.export.is_some());
            assert_eq!(descriptor.abstraction, DeclarationAbstraction::Abstract);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
        });
    });
    let fourth_annotations = parser.tree.get_annotations(fourth_expression_id.id);
    assert_eq!(fourth_annotations.len(), 1);
    assert_node!(parser.tree, fourth_annotations[0], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "before");
        });
    });

    // @before export @after abstract class Foo {}
    let fifth_expression_id = parser.unwrap_statement_expression(expressions[4]);
    assert_node!(parser.tree, fifth_expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
            assert!(descriptor.export.is_some());
            assert_eq!(descriptor.abstraction, DeclarationAbstraction::Abstract);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
        });
    });
    let fifth_annotations = parser.tree.get_annotations(fifth_expression_id.id);
    assert_eq!(fifth_annotations.len(), 2);
    assert_node!(parser.tree, fifth_annotations[0], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "before");
        });
    });
    assert_node!(parser.tree, fifth_annotations[1], Annotation::Decorator { node, .. } => {
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "after");
        });
    });
}

/// Decorator static arguments should preserve generic lambda details.
#[test]
fn test_attach_decorator_with_static_arguments() {
    let mut test = TestParser::new(
        r"@foo<<T>(value: T) => T>()
function foo() { }",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function foo() { }
    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);

    // @foo<<T>(value: T) => T>()
    assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_node!(parser.tree, *expression, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                            assert_eq!(signature.kind, FunctionKind::Lambda);
                            let generics = signature.generics.as_ref().expect("expected generics");
                            let static_parameters = generics
                                .static_parameters
                                .as_ref()
                                .expect("expected static parameters");
                            assert_eq!(static_parameters.len(), 1);
                            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                                assert_string!(parser, *name, "T");
                                assert!(ty.is_none());
                            });
                            assert_eq!(signature.dynamic_parameters.len(), 1);
                            assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                                assert_string!(parser, *name, "value");
                                assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "T");
                            });

                            let return_target = signature.return_type.or(*body);
                            let return_target =
                                return_target.expect("expected return type or body expression");
                            assert_expression_path!(parser, parser.tree.get(return_target), "T");
                        });
                    });
                });
                assert!(dynamic_arguments.is_empty());
            });
        });
    });
}

/// Decorator call chains should be parsed as a call chain.
#[test]
fn test_attach_decorator_with_call_chain() {
    let mut test = TestParser::new(
        r"@joiful.string().guid().required()
function foo() { }",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function foo() { }
    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);

    // @joiful.string().guid().required()
    assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                assert!(dynamic_arguments.is_empty());
                assert_node!(parser.tree, *left, Expression::Member { left: required_left, name, static_arguments } => {
                    assert_string!(parser, *name, "required");
                    assert!(static_arguments.is_none());
                    assert_node!(parser.tree, *required_left, Expression::Call { left, dynamic_arguments, .. } => {
                        assert!(dynamic_arguments.is_empty());
                        assert_node!(parser.tree, *left, Expression::Member { left: guid_left, name, static_arguments } => {
                            assert_string!(parser, *name, "guid");
                            assert!(static_arguments.is_none());
                            assert_node!(parser.tree, *guid_left, Expression::Call { left, dynamic_arguments, .. } => {
                                assert!(dynamic_arguments.is_empty());
                                assert_expression_path!(parser, parser.tree.get(*left), "joiful.string");
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse a decorator with static arguments.
#[test]
fn test_attach_decorator_with_static_arguments_simple() {
    let mut test = TestParser::new_with_options(
        r"@foo<T>()
function foo() { }",
        LanguageType::Destack,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function foo() { }
    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);

    // @foo<T>()
    assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_node!(parser.tree, *expression, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "T");
                });
                assert!(dynamic_arguments.is_empty());
            });
        });
    });
}

/// Parse a decorator with a generic function type argument.
#[test]
fn test_attach_decorator_with_shift_left_static_arguments() {
    let mut test = TestParser::new_with_options(
        r"@f<<T>(v: T) => void>()
class Foo {}",
        LanguageType::Destack,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // @f<<T>(v: T) => void>()
    let decorators = parser.tree.get_nodes::<Decorator>();
    assert_eq!(decorators.len(), 1, "decorators: {decorators:?}");
    let decorator_span = parser.tree.get_span(decorators[0]);
    assert_eq!(
        parser.file.span_str(decorator_span),
        "@f<<T>(v: T) => void>()",
    );

    // class Foo {}
    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1, "annotations: {annotations:?}");

    // <<T>(v: T) => void
    assert_node!(parser.tree, annotations[0], Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_node!(parser.tree, *expression, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "f");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                            assert_eq!(signature.kind, FunctionKind::Lambda);
                            assert!(body.is_none());
                            assert_node!(parser.tree, signature.return_type.expect("expected return type"), Expression::TypeLiteral(TypeLiteral::Void));
                            let generics = signature.generics.as_ref().expect("expected generics");
                            let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                            assert_eq!(static_parameters.len(), 1);
                            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, .. } => {
                                assert_string!(parser, *name, "T");
                            });
                        });
                    });
                });
                assert!(dynamic_arguments.is_empty());
            });
        });
    });
}

/// Decorator annotations on accessors should attach to the member node.
#[test]
fn test_attach_decorator_to_accessor_member() {
    let mut test = TestParser::new(
        r#"
class Box {
@if(true)
get value(): int32 {
    return 1;
}

@if(true)
set value(next: int32) {
    let _ = next;
}
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // class Box
    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
            assert_eq!(members.len(), 2);

            // get value(): int32
            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Getter));
            });
            let getter_decorators = parser
                .tree
                .get_annotations(members[0].id)
                .into_iter()
                .filter(|annotation_id| {
                    matches!(
                        parser.tree.get::<Annotation>(*annotation_id),
                        Annotation::Decorator { .. }
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(getter_decorators.len(), 1);
            assert_node!(parser.tree, getter_decorators[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "if");
                        assert_eq!(dynamic_arguments.len(), 1);
                    });
                });
            });

            // set value(next: int32)
            assert_node!(parser.tree, members[1], Member::Method { signature, .. } => {
                assert_eq!(signature.mode, Some(FunctionMode::Setter));
            });
            let setter_decorators = parser
                .tree
                .get_annotations(members[1].id)
                .into_iter()
                .filter(|annotation_id| {
                    matches!(
                        parser.tree.get::<Annotation>(*annotation_id),
                        Annotation::Decorator { .. }
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(setter_decorators.len(), 1);
            assert_node!(parser.tree, setter_decorators[0], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "if");
                        assert_eq!(dynamic_arguments.len(), 1);
                    });
                });
            });
        });
    });
}

/// Decorator annotations on enum and its variants should be attached correctly.
#[test]
fn test_attach_decorators_to_enum_and_variants() {
    let mut test = TestParser::new(
        r#"
@description("The status of an event.")
export enum EventStatus {
@default
@description("The event is a draft.")
Draft,

@description("The event is upcoming.")
Upcoming,

@description("The event is cancelled.")
Cancelled,
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // @description("The status of an event.")
    assert_eq!(expressions.len(), 1);
    let enum_annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(enum_annotations.len(), 1);
    assert_node!(parser.tree, enum_annotations[0], Annotation::Decorator { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Decorator { expression } => {
            assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, static_arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "description");
                assert!(static_arguments.is_none());
                assert_eq!(dynamic_arguments.len(), 1);
            });
        });
    });

    // EventStatus
    assert_node!(parser.tree, expressions[0], Expression::Declaration(decl) => {
        assert_node!(parser.tree, *decl, Declaration::Enum { fields, .. } => {
            assert_eq!(fields.len(), 3);

            // Draft: @default, @description
            let draft_annotations = parser.tree.get_annotations(fields[0].id);
            assert_eq!(draft_annotations.len(), 2);
            assert_node!(parser.tree, draft_annotations[0], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "default");
                });
            });
            assert_node!(parser.tree, draft_annotations[1], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "description");
                        assert_eq!(dynamic_arguments.len(), 1);
                    });
                });
            });

            // Upcoming: blank + @description
            let upcoming_annotations = parser.tree.get_annotations(fields[1].id);
            assert_eq!(upcoming_annotations.len(), 2);
            assert_node!(parser.tree, upcoming_annotations[0], Annotation::Blank { .. } => {});
            assert_node!(parser.tree, upcoming_annotations[1], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "description");
                        assert_eq!(dynamic_arguments.len(), 1);
                    });
                });
            });

            // Cancelled: blank + @description
            let cancelled_annotations = parser.tree.get_annotations(fields[2].id);
            assert_eq!(cancelled_annotations.len(), 2);
            assert_node!(parser.tree, cancelled_annotations[0], Annotation::Blank { .. } => {});
            assert_node!(parser.tree, cancelled_annotations[1], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "description");
                        assert_eq!(dynamic_arguments.len(), 1);
                    });
                });
            });
        });
    });
}

/// Enum field comments and decorators should preserve source order and ownership.
#[test]
fn test_attach_enum_field_interleaved_comments_and_decorators() {
    let mut test = TestParser::new(
        r#"enum Value {
// before-first
@first
// between
@second
// before-name
Entry
}"#,
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
        assert_node!(parser.tree, *declaration_id, Declaration::Enum { fields, .. } => {
            assert_eq!(fields.len(), 1);
            let annotations = parser.tree.get_annotations(fields[0].id);
            assert_eq!(annotations.len(), 5);

            assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "before-first");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });
            assert_node!(parser.tree, annotations[1], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "first");
                });
            });
            assert_node!(parser.tree, annotations[2], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "between");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });
            assert_node!(parser.tree, annotations[3], Annotation::Decorator { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "second");
                });
            });
            assert_node!(parser.tree, annotations[4], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "before-name");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });
        });
    });
}

/// Enum field trailing and blank-line seams should attach to the correct owners.
#[test]
fn test_attach_enum_field_trailing_and_blank_seams() {
    let mut test = TestParser::new(
        r#"enum Value {
A // a-tail

B

}"#,
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
        assert_node!(parser.tree, *declaration_id, Declaration::Enum { fields, .. } => {
            assert_eq!(fields.len(), 2);

            let first_annotations = parser.tree.get_annotations(fields[0].id);
            assert_eq!(first_annotations.len(), 1);
            assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "a-tail");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });

            let second_annotations = parser.tree.get_annotations(fields[1].id);
            assert_eq!(second_annotations.len(), 2);
            assert_node!(parser.tree, second_annotations[0], Annotation::Blank { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Blank { lines } => {
                    assert_eq!(*lines, 1);
                });
            });
            assert_node!(parser.tree, second_annotations[1], Annotation::Blank { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPostfix);
                assert_node!(parser.tree, *node, Blank { lines } => {
                    assert_eq!(*lines, 1);
                });
            });
        });
    });
}

/// Enum body-boundary comments should attach to the declaration expression seam owner.
#[test]
fn test_attach_enum_body_boundary_comment_on_declaration_owner() {
    let mut test = TestParser::new("enum Value /* enum-body */ { A }");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(_declaration_id) => {});

    let expression_annotations = parser.tree.get_annotations(expression_id.id);
    assert_eq!(expression_annotations.len(), 1);
    assert_node!(parser.tree, expression_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "enum-body");
            assert_eq!(*style, CommentStyle::Star);
        });
    });
}

/// Line suffix is attached to the previous node on the same line.
#[test]
fn test_attach_line_postfix_to_expression() {
    let mut test = TestParser::new("let A = 1 // line comment");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // let A = 1
    assert_eq!(expressions.len(), 1);
    // line comment, suffix
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "line comment");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Trailing comments on chained paths attach to the path expression.
#[test]
fn test_attach_trailing_comment_to_chain_path() {
    let mut test = TestParser::new(
        r"foo
  .getParameters /* trailing comment */
  ?.();",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    // foo.getParameters?.()
    let mut current = expr_id;
    let mut path_id = None;
    loop {
        match parser.tree.get(current) {
            Expression::Path { .. } => {
                path_id = Some(current);
                break;
            }
            Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. } => current = *left,
            _ => break,
        }
    }

    // foo /* trailing comment */
    let path_id = path_id.expect("expected a chained path expression");
    let annotations = parser.tree.get_annotations(path_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "trailing comment");
                assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Line comments between chain segments attach to the following member.
#[test]
fn test_attach_line_comment_before_chain_member() {
    let mut test = TestParser::new(
        r"Promise.all(writeIconFiles)
  // TO DO -- END
  .then(() => writeRegistry())",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    let then_member_id = assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { name, .. } => {
            assert_string!(parser, *name, "then");
        });
        *left
    });

    let annotations = parser.tree.get_annotations(then_member_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "TO DO -- END");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Blank lines after a chained statement attach to the next statement.
#[test]
fn test_attach_blank_after_chained_statement() {
    let mut test = TestParser::new(
        r"Promise.all(writeIconFiles)
  // TO DO -- END
  .then(() => writeRegistry())

Promise.all(writeIconFiles)",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // Promise.all(...); Promise.all(...)
    assert_eq!(expressions.len(), 2);

    let annotations = parser.tree.get_annotations(expressions[1].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Blank { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Blank { lines } => {
            assert_eq!(*lines, 1);
        });
    });
}

/// One blank line between block statements should attach to the following statement.
#[test]
fn test_attach_blank_between_loop_and_next_statement() {
    let mut test = TestParser::new(
        r"{
loop {
    break;
}

let x = z()
}",
    );
    let mut parser = test.prepare();
    let block_id = parser.eat_block().unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, block_id, Block { expressions, .. } => {
        assert_eq!(expressions.len(), 2);
        let loop_statement_id = expressions[0];
        let next_statement_id = expressions[1];

        let loop_annotations = parser.tree.get_annotations(loop_statement_id.id);
        let loop_blank_postfix = loop_annotations
            .into_iter()
            .filter(|annotation_id| {
                matches!(
                    parser.tree.get::<Annotation>(*annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPostfix,
                        ..
                    }
                )
            })
            .collect::<Vec<_>>();
        assert!(loop_blank_postfix.is_empty());

        let annotations = parser.tree.get_annotations(next_statement_id.id);
        let blank_annotations = annotations
            .into_iter()
            .filter(|annotation_id| {
                matches!(
                    parser.tree.get::<Annotation>(*annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(blank_annotations.len(), 1);
        assert_node!(
            parser.tree,
            blank_annotations[0],
            Annotation::Blank { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Blank { lines } => {
                    assert_eq!(*lines, 1);
                });
            }
        );
    });
}

/// Complex block statement spacing should attach one blank prefix to the next statement.
#[test]
fn test_attach_blank_in_block_after_loop_before_let_statement() {
    let mut test = TestParser::new(
        r#"{
import "foo"
import * as baz from "foo"

let x = 1;
let y = 2
y

if (x) {
    y
} else {
    print("foo")
    z(x)
}

loop {
   break;
}

let x = z()
let x = if (let y = 1) {
    z()
} else {
    w()
};

return 5;
}"#,
    );
    let mut parser = test.prepare();
    let block_id = parser.eat_block().unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, block_id, Block { expressions, .. } => {
        assert_eq!(expressions.len(), 10);
        let loop_statement_id = expressions[6];
        let let_after_loop_id = expressions[7];
        let loop_annotations = parser.tree.get_annotations(loop_statement_id.id);
        assert_eq!(loop_annotations.len(), 1);
        assert_node!(
            parser.tree,
            loop_annotations[0],
            Annotation::Blank { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Blank { lines } => {
                    assert_eq!(*lines, 1);
                });
            }
        );
        let let_annotations = parser.tree.get_annotations(let_after_loop_id.id);
        assert_eq!(let_annotations.len(), 1);
        assert_node!(
            parser.tree,
            let_annotations[0],
            Annotation::Blank { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Blank { lines } => {
                    assert_eq!(*lines, 1);
                });
            }
        );
    });
}

/// Direct block parsing should attach blanks after an explicit trivia pass.
#[test]
fn test_attach_blank_in_block_after_loop_before_let_statement_direct_entrypoint() {
    let mut test = TestParser::new(
        r#"{
loop {
   break;
}

let x = z()
}"#,
    );
    let mut parser = test.prepare();
    let block_id = parser.eat_block().unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, block_id, Block { expressions, .. } => {
        assert_eq!(expressions.len(), 2);
        let let_after_loop_id = expressions[1];
        let let_annotations = parser.tree.get_annotations(let_after_loop_id.id);
        let let_all_blank_annotations = let_annotations
            .iter()
            .copied()
            .filter(|annotation_id| {
                matches!(
                    parser.tree.get::<Annotation>(*annotation_id),
                    Annotation::Blank { .. }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(let_all_blank_annotations.len(), 1);
        let let_blank_prefix = let_annotations
            .into_iter()
            .filter(|annotation_id| {
                matches!(
                    parser.tree.get::<Annotation>(*annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(let_blank_prefix.len(), 1);
    });
}

/// Inline comments between path segments attach as line postfix boundaries.
#[test]
fn test_attach_inline_comment_between_path_segments() {
    let mut test = TestParser::new(
        r"wow /* inline comment */
  .omg!",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    // wow.omg!
    let mut current = expr_id;
    let mut path_id = None;
    loop {
        match parser.tree.get(current) {
            Expression::Path { .. } => {
                path_id = Some(current);
                break;
            }
            Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. } => current = *left,
            _ => break,
        }
    }

    // wow /* inline comment */
    let path_id = path_id.expect("expected a chained path expression");
    let annotations = parser.tree.get_annotations(path_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "inline comment");
                assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Inline comments before a dot path segment attach as line postfix annotations.
#[test]
fn test_attach_inline_comment_before_dot_member() {
    let mut test = TestParser::new("wow /* inline comment */ .omg");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    // wow .omg
    let mut current = expr_id;
    loop {
        match parser.tree.get(current) {
            Expression::Path { .. } => break,
            Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. } => current = *left,
            _ => panic!("expected a path expression"),
        }
    }

    // wow /* inline comment */
    let annotations = parser.tree.get_annotations(current.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "inline comment");
                    assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Inline member-hop comments should stay attached to the hop boundary owners.
#[test]
fn test_attach_inline_comment_between_member_hops_to_boundary_owners() {
    let mut test = TestParser::new("source /* hop-a */ .first() /* hop-b */ .second()");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    let second_member_id =
        assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => { *left });
    let first_call_id = assert_node!(parser.tree, second_member_id, Expression::Member { left, name, .. } => {
        assert_eq!(parser.strings.get(*name), "second");
        *left
    });
    let first_member_id =
        assert_node!(parser.tree, first_call_id, Expression::Call { left, .. } => { *left });
    let source_id = assert_node!(parser.tree, first_member_id, Expression::Member { left, name, .. } => {
        assert_eq!(parser.strings.get(*name), "first");
        assert_expression_path!(parser, parser.tree.get(*left), "source");
        *left
    });

    let source_annotations = parser.tree.get_annotations(source_id.id);
    assert_eq!(source_annotations.len(), 1);
    assert_node!(parser.tree, source_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "hop-a");
            assert_eq!(*style, CommentStyle::Star);
        });
    });

    let first_call_annotations = parser.tree.get_annotations(first_call_id.id);
    assert_eq!(first_call_annotations.len(), 1);
    assert_node!(parser.tree, first_call_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "hop-b");
            assert_eq!(*style, CommentStyle::Star);
        });
    });
}

/// Inline comments after operators attach to the following operand as line prefixes.
#[test]
fn test_attach_inline_comment_between_binary_operands() {
    let mut test = TestParser::new("a && /* keep */ b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    // a && b
    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { right, .. } => {
            let annotations = parser.tree.get_annotations(right.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "keep");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// Own-line comments before parenthesized JSX operands attach to the inner grouped expression.
#[test]
fn test_attach_line_comment_before_parenthesized_jsx_binary_operand() {
    let mut test = TestParser::new(
        r#"xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx" && (
  // test
  <div></div>
)"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    assert_node!(
        parser.tree,
        expressions[0],
        Expression::Statement(expression_id) => {
            assert_node!(
                parser.tree,
                *expression_id,
                Expression::Binary { right, .. } => {
                    let inner_expression_id = match parser.tree.get(*right) {
                        Expression::Parenthesized { expression } => *expression,
                        _ => panic!("expected parenthesized right operand"),
                    };
                    let annotations = parser.tree.get_annotations(inner_expression_id.id);
                    assert_eq!(annotations.len(), 1);
                    assert_node!(
                        parser.tree,
                        annotations[0],
                        Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::BlockPrefix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_string!(parser, *string, "test");
                                assert_eq!(*style, CommentStyle::Slash);
                            });
                        }
                    );
                }
            );
        }
    );
}

/// Own-line comments before parenthesized JSX operands stay on the inner grouped expression in expression mode.
#[test]
fn test_attach_line_comment_before_parenthesized_jsx_binary_operand_expression_mode() {
    let mut test = TestParser::new(
        r#"xxxxxxxxxxxx === "xxxxxxxxxxxxxxxxx" && (
  // test
  <div></div>
)"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expr_id,
        Expression::Binary { right, .. } => {
            let inner_expression_id = match parser.tree.get(*right) {
                Expression::Parenthesized { expression } => *expression,
                _ => panic!("expected parenthesized right operand"),
            };
            let annotations = parser.tree.get_annotations(inner_expression_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "test");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );
        }
    );
}

/// Inline comments before call arguments attach as line prefixes.
#[test]
fn test_attach_inline_comment_before_call_argument() {
    let mut test = TestParser::new("foo(/* first */ a)");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    // foo(a)
    assert_node!(parser.tree, expr_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let argument_id = dynamic_arguments[0];
        let annotations = parser.tree.get_annotations(argument_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "first");
                    assert_eq!(*style, CommentStyle::Star);
                });
            }
        );
    });
}

/// Inline comments before computed object keys attach to the property, not the key expression.
#[test]
fn test_attach_inline_comment_before_computed_object_key_to_property() {
    let mut test = TestParser::new("({ /* key */ [k]: value })");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expr_id,
        Expression::Parenthesized { expression } => {
            assert_node!(
                parser.tree,
                *expression,
                Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    let property_id = properties[0];
                    let property_annotations = parser.tree.get_annotations(property_id.id);
                    assert_eq!(property_annotations.len(), 1);
                    assert_node!(
                        parser.tree,
                        property_annotations[0],
                        Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePrefix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_string!(parser, *string, "key");
                                assert_eq!(*style, CommentStyle::Star);
                            });
                        }
                    );

                    assert_node!(
                        parser.tree,
                        property_id,
                        Property::Field { key: Some(Key::Expression(key_expression_id)), .. } => {
                            let key_annotations = parser.tree.get_annotations(key_expression_id.id);
                            assert_eq!(key_annotations.len(), 0);
                        }
                    );
                }
            );
        }
    );
}

/// Closure cast doc comments in parenthesized calls should attach to the inner callee path.
#[test]
fn test_attach_closure_cast_doc_comment_to_inner_call_expression() {
    let mut test = TestParser::new_with_options(
        "let assignment = (/** @type {string} */ getValue())",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::Parenthesized { expression } => {
                let parenthesized_annotations = parser.tree.get_annotations(value_id.id);
                assert!(parenthesized_annotations.is_empty());

                assert_node!(parser.tree, *expression, Expression::Call { left, .. } => {
                    let inner_call_annotations = parser.tree.get_annotations(expression.id);
                    assert!(inner_call_annotations.is_empty());

                    let callee_annotations = parser.tree.get_annotations(left.id);
                    assert_eq!(callee_annotations.len(), 1);
                    assert_node!(parser.tree, callee_annotations[0], Annotation::Doc { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Doc { string, style } => {
                            assert_eq!(*style, DocStyle::Star);
                            assert_string!(parser, *string, "@type {string}");
                        });
                    });
                });
            });
        });
    });
}

/// Closure cast doc comments on member bases should attach to the base path expression.
#[test]
fn test_attach_closure_cast_doc_comment_to_member_base_expression() {
    let mut test = TestParser::new_with_options(
        "var newArray = (/** @type {array} */ numberOrString).map((x) => x)",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::Call { left, .. } => {
                assert_node!(parser.tree, *left, Expression::Member { left: base_id, .. } => {
                    let member_annotations = parser.tree.get_annotations(left.id);
                    assert!(member_annotations.is_empty());

                    assert_node!(parser.tree, *base_id, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "numberOrString");
                        });

                        let parenthesized_annotations = parser.tree.get_annotations(base_id.id);
                        assert!(parenthesized_annotations.is_empty());

                        let base_annotations = parser.tree.get_annotations(expression.id);
                        assert_eq!(base_annotations.len(), 1);
                        assert_node!(parser.tree, base_annotations[0], Annotation::Doc { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePrefix);
                            assert_node!(parser.tree, *node, Doc { string, style } => {
                                assert_eq!(*style, DocStyle::Star);
                                assert_string!(parser, *string, "@type {array}");
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parameter separator comments should attach to the parameter type seam.
#[test]
fn test_attach_parameter_separator_comment_to_parameter_prefix() {
    let mut test = TestParser::new("value /* parameter-type */: number");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    parser.attach_trivia();
    let annotations = parser.tree.get_annotations(parameter_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockInfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "parameter-type");
                assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Optional parameter separator comments should attach to the parameter type seam.
#[test]
fn test_attach_optional_parameter_separator_comment_to_parameter_prefix() {
    let mut test = TestParser::new("value? /* optional-parameter-type */: number");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    parser.attach_trivia();

    let annotations = parser.tree.get_annotations(parameter_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockInfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "optional-parameter-type");
                assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Prefix parameter comments should attach to the parameter node.
#[test]
fn test_attach_parameter_prefix_comment_to_parameter() {
    let mut test = TestParser::new("/* before-name */ value: number");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    parser.attach_trivia();
    let annotations = parser.tree.get_annotations(parameter_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "before-name");
                assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Lambda arrow separator comments should attach to the lambda declaration seam.
#[test]
fn test_attach_lambda_arrow_separator_comment_to_lambda_infix() {
    let mut test = TestParser::new("() /**/ => 1");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Declaration(declaration_id) => {
            let annotations = parser.tree.get_annotations(declaration_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Doc { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockInfix);
                    assert_node!(parser.tree, *node, Doc { string, style } => {
                        assert_string!(parser, *string, "/**/");
                        assert_eq!(*style, DocStyle::Star);
                    });
                }
            );
        }
    );
}

/// Empty new-call boundary comments should stay attached to the call expression seam.
#[test]
fn test_attach_empty_new_boundary_comment_to_callee_expression() {
    let mut test = TestParser::new("new require(/* new-boundary */)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::New { left, dynamic_arguments, .. } => {
            let _ = left;
            assert!(dynamic_arguments.is_empty());
            let annotations = parser.tree.get_annotations(expression_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockInfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "new-boundary");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// Empty call boundary comments should stay attached to the call expression seam.
#[test]
fn test_attach_empty_call_boundary_comment_to_callee_expression() {
    let mut test = TestParser::new("target(/* call-boundary */)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { left, dynamic_arguments, .. } => {
            let _ = left;
            assert!(dynamic_arguments.is_empty());
            let annotations = parser.tree.get_annotations(expression_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockInfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "call-boundary");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// If-condition boundary comments before `)` should attach to the condition expression.
#[test]
fn test_attach_if_condition_boundary_comment_before_close_parenthesis() {
    let mut test = TestParser::new("if (true /* condition-boundary */ ) {}");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::If { condition, .. } => {
            let IfCondition::Expression { condition } = condition else {
                panic!("expected if expression condition");
            };

            let annotations = parser.tree.get_annotations(condition.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "condition-boundary");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// If-head trailing comments should stay attached to the then branch in parse mode.
#[test]
fn test_attach_if_head_trailing_comment_in_parse_mode() {
    let mut test = TestParser::new_with_options(
        r"if (ready) // if-head
    run()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let if_id = match parser.tree.get(expressions[0]) {
        Expression::Statement(statement_id) => *statement_id,
        Expression::If { .. } => expressions[0],
        _ => panic!("expected if expression"),
    };

    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        let IfCondition::Expression { condition } = condition else {
            panic!("expected expression if condition");
        };

        let if_annotations = parser.tree.get_annotations(if_id.id);
        assert!(if_annotations.is_empty());

        let condition_annotations = parser.tree.get_annotations(condition.id);
        assert!(condition_annotations.is_empty());

        let then_annotations = parser.tree.get_annotations(then_expression.id);
        assert!(then_annotations.is_empty());

        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                let then_statement_annotations = parser.tree.get_annotations(expressions[0].id);
                assert_eq!(then_statement_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    then_statement_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "if-head");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    }
                );
                let statement_id = parser.unwrap_statement_expression(expressions[0]);
                assert_node!(parser.tree, statement_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "run");
                });
            });
        });
    });
}

/// Direct `eat_if` entrypoints should keep if-head trailing comments on the then branch.
#[test]
fn test_attach_if_head_trailing_comment_on_direct_if_entrypoint() {
    let mut test = TestParser::new_with_options(
        r"if (ready) // if-head
    run()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let if_id = parser.eat_if().unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        let IfCondition::Expression { condition } = condition else {
            panic!("expected expression if condition");
        };

        let if_annotations = parser.tree.get_annotations(if_id.id);
        assert!(if_annotations.is_empty());

        let condition_annotations = parser.tree.get_annotations(condition.id);
        assert!(condition_annotations.is_empty());

        let then_annotations = parser.tree.get_annotations(then_expression.id);
        assert!(then_annotations.is_empty());

        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                let then_statement_annotations = parser.tree.get_annotations(expressions[0].id);
                assert_eq!(then_statement_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    then_statement_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "if-head");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    }
                );
                let statement_id = parser.unwrap_statement_expression(expressions[0]);
                assert_node!(parser.tree, statement_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "run");
                });
            });
        });
    });
}

/// Break trailing comments should attach to the break statement owner.
#[test]
fn test_attach_break_trailing_comment_to_break_statement_owner() {
    let mut test = TestParser::new_with_options(
        r"while (running) {
  if (done) break // break-tail
  tick()
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let while_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, while_id, Expression::While { condition, body, .. } => {
        assert_expression_path!(parser, parser.tree.get(*condition), "running");

        assert_node!(parser.tree, *body, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 2);

            let if_id = parser.unwrap_statement_expression(expressions[0]);
            assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
                let IfCondition::Expression { condition } = condition else {
                    panic!("expected if expression condition");
                };
                assert_expression_path!(parser, parser.tree.get(*condition), "done");

                assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                        assert_eq!(expressions.len(), 1);
                        let break_statement_id = expressions[0];
                        let break_id = parser.unwrap_statement_expression(break_statement_id);
                        assert_node!(parser.tree, break_id, Expression::Break { .. } => {});

                        let annotations = parser.tree.get_annotations(if_id.id);
                        assert_eq!(annotations.len(), 1);
                        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_string!(parser, *string, "break-tail");
                                assert_eq!(*style, CommentStyle::Slash);
                            });
                        });
                    });
                });
            });

            let tick_id = parser.unwrap_statement_expression(expressions[1]);
            assert_node!(parser.tree, tick_id, Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "tick");
            });
            let tick_annotations = parser.tree.get_annotations(tick_id.id);
            assert!(tick_annotations.is_empty());
        });
    });
}

/// Continue trailing comments should attach to the continue statement owner.
#[test]
fn test_attach_continue_trailing_comment_to_continue_statement_owner() {
    let mut test = TestParser::new_with_options(
        r"for (const item of items) {
  if (!item) continue // continue-tail
  use(item)
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let for_each_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, for_each_id, Expression::ForEach { iterator, body, .. } => {
        assert_expression_path!(parser, parser.tree.get(*iterator), "items");

        assert_node!(parser.tree, *body, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 2);

            let if_id = parser.unwrap_statement_expression(expressions[0]);
            assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
                let IfCondition::Expression { condition } = condition else {
                    panic!("expected if expression condition");
                };
                assert_node!(parser.tree, *condition, Expression::Unary { .. } => {});

                assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                        assert_eq!(expressions.len(), 1);
                        let continue_statement_id = expressions[0];
                        let continue_id = parser.unwrap_statement_expression(continue_statement_id);
                        assert_node!(parser.tree, continue_id, Expression::Continue { .. } => {});

                        let annotations = parser.tree.get_annotations(if_id.id);
                        assert_eq!(annotations.len(), 1);
                        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_string!(parser, *string, "continue-tail");
                                assert_eq!(*style, CommentStyle::Slash);
                            });
                        });
                    });
                });
            });

            let use_id = parser.unwrap_statement_expression(expressions[1]);
            assert_node!(parser.tree, use_id, Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "use");
            });
            let use_annotations = parser.tree.get_annotations(use_id.id);
            assert!(use_annotations.is_empty());
        });
    });
}

/// Yield trailing comments should attach to the yield statement owner.
#[test]
fn test_attach_yield_trailing_comment_to_yield_statement_owner() {
    let mut test = TestParser::new_with_options(
        r"function* run() {
  yield value // yield-tail
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let function_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, function_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
            let body_id = body.expect("expected function body");
            assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    let yield_statement_id = expressions[0];
                    let yield_id = parser.unwrap_statement_expression(yield_statement_id);
                    assert_node!(parser.tree, yield_id, Expression::Yield { value, .. } => {
                        let value_id = value.expect("expected yield value");
                        assert_expression_path!(parser, parser.tree.get(value_id), "value");
                        let annotations = parser.tree.get_annotations(value_id.id);
                        assert_eq!(annotations.len(), 1);
                        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_string!(parser, *string, "yield-tail");
                                assert_eq!(*style, CommentStyle::Slash);
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Try and catch trailing comments should stay attached to their call statement owners.
#[test]
fn test_attach_try_catch_trailing_comments_to_call_statement_owners() {
    let mut test = TestParser::new_with_options(
        r"try {
  run() // try-tail
} catch (error) {
  recover(error) // catch-tail
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let try_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_pattern, catch_expression, .. } => {
        assert!(catch_pattern.is_some());
        let catch_expression_id = catch_expression.expect("expected catch expression");

        assert_node!(parser.tree, *try_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                let call_statement_id = expressions[0];
                let call_id = parser.unwrap_statement_expression(call_statement_id);
                assert_node!(parser.tree, call_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "run");
                });

                let annotations = parser.tree.get_annotations(call_statement_id.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "try-tail");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                });
            });
        });

        assert_node!(parser.tree, catch_expression_id, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                let call_statement_id = expressions[0];
                let call_id = parser.unwrap_statement_expression(call_statement_id);
                assert_node!(parser.tree, call_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "recover");
                });

                let annotations = parser.tree.get_annotations(call_statement_id.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "catch-tail");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                });
            });
        });
    });
}

/// Label-separator comments after `:` should attach to the labelled owner as prefix.
#[test]
fn test_attach_label_separator_comment_to_labelled_body_prefix() {
    let mut test = TestParser::new_with_options(
        r"start: // label-tail
while (true) {
  break start
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    assert_node!(parser.tree, expressions[0], Expression::Labelled { label, body } => {
        assert_string!(parser, *label, "start");
        assert_node!(parser.tree, *body, Expression::While { .. } => {});

        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "label-tail");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });

        let body_annotations = parser.tree.get_annotations(body.id);
        assert!(body_annotations.is_empty());
    });
}

/// Do-while trailing comments should attach to the loop owner.
#[test]
fn test_attach_do_while_trailing_comment_to_loop_owner() {
    let mut test = TestParser::new_with_options(
        r"do {
  process()
} while (condition) // do-tail",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::While { kind, condition, body } => {
        assert_eq!(*kind, WhileKind::DoWhile);
        assert_expression_path!(parser, parser.tree.get(*condition), "condition");
        assert_node!(parser.tree, *body, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 1);
            let process_id = parser.unwrap_statement_expression(expressions[0]);
            assert_node!(parser.tree, process_id, Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "process");
            });
        });

        let annotations = parser.tree.get_annotations(expression_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "do-tail");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
    });
}

/// For-await head comments should attach to the iterator expression boundary.
#[test]
fn test_attach_for_await_head_comment_to_iterator_owner() {
    let mut test = TestParser::new_with_options(
        r"for await (const item of stream) // for-head
{
  consume(item)
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let for_each_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, for_each_id, Expression::ForEach { asynchrony, iterator, body, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Async);
        assert_expression_path!(parser, parser.tree.get(*iterator), "stream");

        let iterator_annotations = parser.tree.get_annotations(iterator.id);
        assert_eq!(iterator_annotations.len(), 1);
        assert_node!(parser.tree, iterator_annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "for-head");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });

        assert_node!(parser.tree, *body, Block { expressions, .. } => {
            assert_eq!(expressions.len(), 1);
            let consume_id = parser.unwrap_statement_expression(expressions[0]);
            assert_node!(parser.tree, consume_id, Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "consume");
            });
        });
    });
}

/// Infix seam comments after `as` should attach to the cast expression boundary.
#[test]
fn test_attach_as_assertion_trailing_line_comment_to_cast_boundary() {
    let mut test = TestParser::new_with_options(
        r"const value = source as // as-tail
number",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
                let value_id = value.expect("expected initializer");
                assert_node!(parser.tree, value_id, Expression::TypeBinary { left, operator, right } => {
                    assert_eq!(*operator, TypeBinaryOperator::Cast);

                    let cast_annotations = parser.tree.get_annotations(value_id.id);
                    assert_eq!(cast_annotations.len(), 1, "expected one cast annotation");
                    assert_node!(
                        parser.tree,
                        cast_annotations[0],
                        Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(*style, CommentStyle::Slash);
                                assert_string!(parser, *string, "as-tail");
                            });
                        }
                    );

                    let left_annotations = parser.tree.get_annotations(left.id);
                    assert!(left_annotations.is_empty());
                    let right_annotations = parser.tree.get_annotations(right.id);
                    assert!(right_annotations.is_empty());

                    assert_expression_path!(parser, parser.tree.get(*left), "source");
                });
            });
        }
    );
}
/// Infix seam comments after `satisfies` should attach to the satisfies expression boundary.
#[test]
fn test_attach_satisfies_trailing_line_comment_to_operator_boundary() {
    let mut test = TestParser::new_with_options(
        r"const config = { retries: 3 } satisfies // sat-tail
Record<string, number>",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
                let value_id = value.expect("expected initializer");
                assert_node!(parser.tree, value_id, Expression::TypeBinary { operator, right, .. } => {
                    assert_eq!(*operator, TypeBinaryOperator::Satisfies);

                    let satisfies_annotations = parser.tree.get_annotations(value_id.id);
                    assert_eq!(
                        satisfies_annotations.len(),
                        1,
                        "expected one satisfies annotation"
                    );
                    assert_node!(
                        parser.tree,
                        satisfies_annotations[0],
                        Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(*style, CommentStyle::Slash);
                                assert_string!(parser, *string, "sat-tail");
                            });
                        }
                    );

                    let right_annotations = parser.tree.get_annotations(right.id);
                    assert!(right_annotations.is_empty());
                });
            });
        }
    );
}

/// Infix seam block comments after `as` and `satisfies` should attach to the right type as prefixes.
#[test]
fn test_attach_type_binary_block_comments_to_right_type_prefix() {
    let mut test = TestParser::new_with_options(
        "const a = value as /* between */ Foo;\nconst b = value satisfies /* sat-between */ Bar;",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 2);

    let first_expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, first_expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::TypeBinary { left, right, .. } => {
                let right_annotations = parser.tree.get_annotations(right.id);
                assert_eq!(right_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    right_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "between");
                        });
                    }
                );

                let left_annotations = parser.tree.get_annotations(left.id);
                assert!(left_annotations.is_empty());
            });
        });
    });

    let second_expression_id = parser.unwrap_statement_expression(expressions[1]);
    assert_node!(parser.tree, second_expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::TypeBinary { left, right, .. } => {
                let right_annotations = parser.tree.get_annotations(right.id);
                assert_eq!(right_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    right_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "sat-between");
                        });
                    }
                );

                let left_annotations = parser.tree.get_annotations(left.id);
                assert!(left_annotations.is_empty());
            });
        });
    });
}

/// Boundary comments before `as const` should stay attached to the left operand.
#[test]
fn test_attach_as_const_boundary_comment_to_left_operand() {
    let mut test = TestParser::new_with_options(
        "const values = [1, 2, 3] /* as-const */ as const",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::TypeUnary { operator, right } => {
                assert_eq!(*operator, TypeUnaryOperator::AsConst);

                let operand_annotations = parser.tree.get_annotations(right.id);
                assert_eq!(operand_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    operand_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "as-const");
                        });
                    }
                );

                let assertion_annotations = parser.tree.get_annotations(value_id.id);
                assert!(assertion_annotations.is_empty());
            });
        });
    });
}

/// Line comments between `as` and `const` should attach to the as-const boundary.
#[test]
fn test_attach_as_const_line_comment_between_operator_tokens() {
    let mut test =
        TestParser::new_with_options("1 as // before-const\nconst;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::TypeUnary { operator, .. } => {
        assert_eq!(*operator, TypeUnaryOperator::AsConst);
        let annotations = parser.tree.get_annotations(expression_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Slash);
                    assert_string!(parser, *string, "before-const");
                });
            }
        );
    });
}

/// Multiline block comments between `as` and `const` should attach to the as-const boundary.
#[test]
fn test_attach_as_const_multiline_block_comment_between_operator_tokens() {
    let mut test = TestParser::new_with_options(
        "1 as\n/*\nblock-comment\n*/\nconst;",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::TypeUnary { operator, .. } => {
        assert_eq!(*operator, TypeUnaryOperator::AsConst);
        let annotations = parser.tree.get_annotations(expression_id.id);
        assert_eq!(annotations.len(), 1);
        assert_node!(
            parser.tree,
            annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "\nblock-comment\n");
                });
            }
        );
    });
}

/// Single-line block comments between `as` and `const` should stay on the left operand.
#[test]
fn test_attach_as_const_inline_block_comment_between_operator_tokens_to_left_operand() {
    let mut test =
        TestParser::new_with_options("1 as /* between */ const;", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::TypeUnary { operator, right } => {
        assert_eq!(*operator, TypeUnaryOperator::AsConst);

        let operand_annotations = parser.tree.get_annotations(right.id);
        assert_eq!(operand_annotations.len(), 1);
        assert_node!(parser.tree, operand_annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(*style, CommentStyle::Star);
                assert_string!(parser, *string, "between");
            });
        });

        let assertion_annotations = parser.tree.get_annotations(expression_id.id);
        assert!(assertion_annotations.is_empty());
    });
}

/// Type-binary block seam comments should also attach in expression-statement form.
#[test]
fn test_attach_type_binary_block_comments_in_expression_statements() {
    let mut test = TestParser::new_with_options(
        "1 as /* between */ Foo;\n1 satisfies /* sat-between */ Bar;",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 2);

    let first_expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, first_expression_id, Expression::TypeBinary { right, .. } => {
        let right_annotations = parser.tree.get_annotations(right.id);
        assert_eq!(
            right_annotations.len(),
            1,
            "expected exactly one right-type annotation in first expression statement"
        );
        assert_node!(
            parser.tree,
            right_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "between");
                });
            }
        );
    });

    let second_expression_id = parser.unwrap_statement_expression(expressions[1]);
    assert_node!(parser.tree, second_expression_id, Expression::TypeBinary { right, .. } => {
        let right_annotations = parser.tree.get_annotations(right.id);
        assert_eq!(
            right_annotations.len(),
            1,
            "expected exactly one right-type annotation in second expression statement"
        );
        assert_node!(
            parser.tree,
            right_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "sat-between");
                });
            }
        );
    });
}

/// Ternary branch inline comments should stay on their corresponding branch expressions.
#[test]
fn test_attach_ternary_inline_branch_comments_to_branch_nodes() {
    let mut test = TestParser::new_with_options(
        "const value = cond ? left /* left-note */ : right /* right-note */",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::If { then_expression, else_expression, .. } => {
                let else_expression_id = else_expression.expect("expected ternary else branch");

                let then_annotations = parser.tree.get_annotations(then_expression.id);
                assert_eq!(then_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    then_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "left-note");
                        });
                    }
                );

                let else_annotations = parser.tree.get_annotations(else_expression_id.id);
                assert_eq!(else_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    else_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "right-note");
                        });
                    }
                );

                let ternary_annotations = parser.tree.get_annotations(value_id.id);
                assert!(ternary_annotations.is_empty(), "ternary node should not own branch comments");
            });
        });
    });
}

/// Ternary line seam comments after `?` and `:` should stay on left boundaries.
#[test]
fn test_attach_ternary_operator_seam_comments_to_branch_prefixes() {
    let mut test = TestParser::new_with_options(
        r"const value = cond ? // then-seam
left : // else-seam
right",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::If { condition, then_expression, else_expression, .. } => {
                let else_expression_id = else_expression.expect("expected ternary else branch");
                let condition_expression_id = match condition {
                    IfCondition::Expression { condition } => *condition,
                    IfCondition::Let { .. } => panic!("unexpected ternary let condition"),
                };

                let condition_annotations = parser.tree.get_annotations(condition_expression_id.id);
                assert_eq!(condition_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    condition_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Slash);
                            assert_string!(parser, *string, "then-seam");
                        });
                    }
                );

                let else_annotations = parser.tree.get_annotations(else_expression_id.id);
                assert!(else_annotations.is_empty());

                let then_annotations = parser.tree.get_annotations(then_expression.id);
                assert_eq!(then_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    then_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Slash);
                            assert_string!(parser, *string, "else-seam");
                        });
                    }
                );
            });
        });
    });
}

/// Ternary seam block comments on their own lines should attach to then-branch prefixes.
#[test]
fn test_attach_ternary_operator_seam_block_comment_to_then_prefix() {
    let mut test = TestParser::new_with_options(
        r"const value = cond
    ?
    /* ternary-boundary */
    on_true
    : on_false",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::If { then_expression, .. } => {
                let then_annotations = parser.tree.get_annotations(then_expression.id);
                assert_eq!(then_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    then_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "ternary-boundary");
                        });
                    }
                );
            });
        });
    });
}

/// Inline boundary line comments should attach to the preceding call argument boundary.
#[test]
fn test_attach_call_argument_inline_boundary_comment_to_previous_argument_boundary() {
    let mut test = TestParser::new(
        r"target(first, // call-argument-boundary
    second)",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
            let first_argument_id = dynamic_arguments[0];
            let second_argument_id = dynamic_arguments[1];
            let first_annotations = parser.tree.get_annotations(first_argument_id.id);
            let second_annotations = parser.tree.get_annotations(second_argument_id.id);
            assert!(second_annotations.is_empty());
            assert_eq!(first_annotations.len(), 1);
            assert_node!(
                parser.tree,
                first_annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "call-argument-boundary");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );
        }
    );
}

/// Inline boundary comments before direct computed index should attach to the left expression.
#[test]
fn test_attach_inline_comment_before_direct_index_to_left_expression() {
    let mut test = TestParser::new("source /* before-index */ [key]");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::Index { left, .. } => {
        let left_annotations = parser.tree.get_annotations(left.id);
        assert_eq!(left_annotations.len(), 1);
        assert_node!(
            parser.tree,
            left_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "before-index");
                });
            }
        );
    });
}

/// Inline boundary comments before direct call parentheses should stay on first arguments.
#[test]
fn test_attach_inline_comment_before_direct_call_parenthesis_to_callee() {
    let mut test = TestParser::new("target /* call-boundary */ (arg)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::Call { left, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let callee_annotations = parser.tree.get_annotations(left.id);
        assert!(callee_annotations.is_empty());
        let first_argument_annotations = parser.tree.get_annotations(dynamic_arguments[0].id);
        assert_eq!(first_argument_annotations.len(), 1);
        assert_node!(
            parser.tree,
            first_argument_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "call-boundary");
                });
            }
        );
    });
}

/// Inline boundary comments before member call parentheses should stay on first arguments.
#[test]
fn test_attach_inline_comment_before_member_call_parenthesis_to_member_callee() {
    let mut test = TestParser::new("items.map /* keep */ ((item) => item)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::Call { left: callee_id, dynamic_arguments, .. } => {
        let callee_id = *callee_id;
        assert_node!(parser.tree, callee_id, Expression::Path { path, .. } => {
            assert_path!(parser, *path, "items.map");
        });
        let callee_span = parser.tree.get_span(callee_id);
        let callee_source = parser.file.span_str(callee_span);
        assert_eq!(callee_source, "items.map");

        let callee_annotations = parser.tree.get_annotations(callee_id.id);
        assert!(callee_annotations.is_empty());
        assert_eq!(dynamic_arguments.len(), 1);
        let first_argument_annotations = parser.tree.get_annotations(dynamic_arguments[0].id);
        assert_eq!(first_argument_annotations.len(), 1);
        assert_node!(
            parser.tree,
            first_argument_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "keep");
                });
            }
        );
    });
}

/// Inline boundary comments before optional chain operators should stay on the preceding segment.
#[test]
fn test_attach_inline_comment_before_optional_chain_operator_to_previous_member() {
    let mut test = TestParser::new_with_options(
        "source\n  .first /* first-boundary */\n  ?.second()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                let owner_annotations = parser.tree.get_annotations(left.id);
                assert_eq!(owner_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    owner_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "first-boundary");
                        });
                    }
                );

                assert_node!(parser.tree, *left, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "source.first");
                });
            });
        });
    });
}

/// Inline boundary comments before optional chains should stay on the same owner in parse mode.
#[test]
fn test_attach_inline_comment_before_optional_chain_operator_to_previous_member_in_parse_mode() {
    let mut test = TestParser::new_with_options(
        "source\n  .first /* first-boundary */\n  ?.second()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_statement_expression(expressions[0]);

    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                let owner_annotations = parser.tree.get_annotations(left.id);
                assert_eq!(owner_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    owner_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Star);
                            assert_string!(parser, *string, "first-boundary");
                        });
                    }
                );

                assert_node!(parser.tree, *left, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "source.first");
                });
            });
        });
    });
}

/// Inline boundary comments before `new` call parentheses should stay on first arguments.
#[test]
fn test_attach_inline_comment_before_new_call_parenthesis_to_callee() {
    let mut test =
        TestParser::new_with_options("new Factory /* new-call */ (arg)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::New { left, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let callee_annotations = parser.tree.get_annotations(left.id);
        assert!(callee_annotations.is_empty());
        let first_argument_annotations = parser.tree.get_annotations(dynamic_arguments[0].id);
        assert_eq!(first_argument_annotations.len(), 1);
        assert_node!(
            parser.tree,
            first_argument_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "new-call");
                });
            }
        );
    });
}

/// Optional call boundary comments should attach to the full optional call expression.
#[test]
fn test_attach_optional_call_boundary_line_comment_to_call_expression() {
    let mut test = TestParser::new_with_options(
        r"call // C4
?.()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { left, dynamic_arguments, .. } => {
            assert!(dynamic_arguments.is_empty());
            let maybe_left = assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                *left
            });

            let annotations = parser.tree.get_annotations(expression_id.id);
            let maybe_annotations = parser.tree.get_annotations(left.id);
            let callee_annotations = parser.tree.get_annotations(maybe_left.id);
            assert_eq!(
                annotations.len(),
                1,
                "call annotations: {}, maybe annotations: {}, callee annotations: {}",
                annotations.len(),
                maybe_annotations.len(),
                callee_annotations.len()
            );
            assert!(maybe_annotations.is_empty());
            assert!(callee_annotations.is_empty());
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "C4");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );
        }
    );
}

/// Optional call boundary comments should not be duplicated in statement parse mode.
#[test]
fn test_attach_optional_call_boundary_line_comment_once_in_parse_mode() {
    let mut test = TestParser::new_with_options(
        r"call // C4
?.()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    assert_node!(
        parser.tree,
        expressions[0],
        Expression::Statement(statement_id) => {
            let statement_annotations = parser.tree.get_annotations(expressions[0].id);
            assert_eq!(statement_annotations.len(), 1);
            assert_node!(
                parser.tree,
                statement_annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "C4");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );

            assert_node!(parser.tree, *statement_id, Expression::Call { left, dynamic_arguments, .. } => {
                assert!(dynamic_arguments.is_empty());
                let call_annotations = parser.tree.get_annotations(statement_id.id);
                let maybe_annotations = parser.tree.get_annotations(left.id);
                let callee_annotations = assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                    parser.tree.get_annotations(left.id)
                });
                assert!(call_annotations.is_empty());
                assert!(maybe_annotations.is_empty());
                assert!(callee_annotations.is_empty());
            });
        }
    );
}

/// Optional call trailing comments should be attached once in parse mode.
#[test]
fn test_attach_optional_call_trailing_comment_once_in_parse_mode() {
    let mut test = TestParser::new_with_options("call?.(); // C4", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    assert_node!(
        parser.tree,
        expressions[0],
        Expression::Statement(statement_id) => {
            let statement_annotations = parser.tree.get_annotations(expressions[0].id);
            let call_annotations = parser.tree.get_annotations(statement_id.id);
            assert_eq!(statement_annotations.len(), 1);
            assert!(call_annotations.is_empty());
            assert_node!(
                parser.tree,
                statement_annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "C4");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );
        }
    );
}

/// Optional call block comments before `?.` should stay infix on the callee.
#[test]
fn test_attach_optional_call_block_comment_before_chain_operator() {
    let mut test =
        TestParser::new_with_options("getParameters /* marker */\n?.()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                let annotations = parser.tree.get_annotations(left.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "marker");
                            assert_eq!(*style, CommentStyle::Star);
                        });
                    }
                );
            });
        }
    );
}

/// Optional call block comments on the same line should stay on the callee boundary.
#[test]
fn test_attach_optional_call_block_comment_before_chain_operator_same_line() {
    let mut test =
        TestParser::new_with_options("target /* opt-call */ ?.()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                let annotations = parser.tree.get_annotations(left.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "opt-call");
                            assert_eq!(*style, CommentStyle::Star);
                        });
                    }
                );
            });
        }
    );
}

/// Trailing comments on the last call argument should stay on that argument boundary.
#[test]
fn test_attach_trailing_comment_to_last_call_argument_boundary() {
    let mut test = TestParser::new(
        r#"call(
() => {
    // ...
},
"good" // trailing
)"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
            let last_argument = dynamic_arguments[1];
            let annotations = parser.tree.get_annotations(last_argument.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "trailing");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );
        }
    );
}

/// Static argument comments should stay attached to each static argument.
#[test]
fn test_attach_comments_to_static_type_arguments() {
    let mut test = TestParser::new_with_options(
        "makePair</* key */ string, /* value */ number>",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Instantiation { static_arguments, .. } => {
            assert_eq!(static_arguments.len(), 2);

            let key_annotations = parser.tree.get_annotations(static_arguments[0].id);
            assert_eq!(key_annotations.len(), 1);
            assert_node!(
                parser.tree,
                key_annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "key");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );

            let value_annotations = parser.tree.get_annotations(static_arguments[1].id);
            assert_eq!(value_annotations.len(), 1);
            assert_node!(
                parser.tree,
                value_annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "value");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// Multiline block comments before lambda call arguments should attach to the argument.
#[test]
fn test_attach_multiline_call_argument_prefix_comment_before_lambda() {
    let mut test = TestParser::new(
        r"call(/* comment */
() => {
    //
})",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            let argument_id = dynamic_arguments[0];
            let annotations = parser.tree.get_annotations(argument_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// The javascript parser should keep multiline prefix comments on lambda call arguments.
#[test]
fn test_attach_multiline_call_argument_prefix_comment_before_lambda_javascript() {
    let mut test = TestParser::new_with_options(
        r"call(/* comment */
() => {
    //
})",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            let argument_id = dynamic_arguments[0];
            let annotations = parser.tree.get_annotations(argument_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// Format-ignore range comments in call arguments should attach to argument wrappers.
#[test]
fn test_attach_format_ignore_range_comments_to_call_arguments() {
    let mut test = TestParser::new(
        r#"doThing(
    1,
    // format-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // format-ignore-end
    4,
)"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    // doThing(1, foo(...), bar(3), 4)
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 4);

            // // format-ignore-start
            let start_annotations = parser.tree.get_annotations(dynamic_arguments[1].id);
            assert_eq!(start_annotations.len(), 1);
            let start_annotation_span = parser.tree.get_span::<Annotation>(start_annotations[0]);
            let (_, start_annotation_column) = parser
                .file
                .get_position(start_annotation_span.start)
                .expect("expected start comment position");
            assert_eq!(start_annotation_column, 4);
            assert_node!(
                parser.tree,
                start_annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "format-ignore-start");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
                let value_annotations = parser.tree.get_annotations(value.id);
                assert!(value_annotations.is_empty());
            });

            // // format-ignore-end
            let end_annotations = parser.tree.get_annotations(dynamic_arguments[3].id);
            assert_eq!(end_annotations.len(), 1);
            let end_annotation_span = parser.tree.get_span::<Annotation>(end_annotations[0]);
            let (_, end_annotation_column) = parser
                .file
                .get_position(end_annotation_span.start)
                .expect("expected end comment position");
            assert_eq!(end_annotation_column, 4);
            assert_node!(
                parser.tree,
                end_annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "format-ignore-end");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                }
            );
            assert_node!(parser.tree, dynamic_arguments[3], Argument::Positional { value, .. } => {
                let value_annotations = parser.tree.get_annotations(value.id);
                assert!(value_annotations.is_empty());
            });
        }
    );
}

/// Format-ignore range comments in call arguments should stay on wrappers in full parse mode.
#[test]
fn test_attach_format_ignore_range_comments_to_call_arguments_parse_entrypoint() {
    let mut test = TestParser::new(
        r#"doThing(
1,
// format-ignore-start
foo ( 1 ,2 ),
bar(3),
// format-ignore-end
4,
)"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Statement(call_expression) => {
        assert_node!(
            parser.tree,
            *call_expression,
            Expression::Call { dynamic_arguments, .. } => {
                assert_eq!(dynamic_arguments.len(), 4);

                // start marker should be owned by the second argument wrapper
                let start_annotations = parser.tree.get_annotations(dynamic_arguments[1].id);
                assert_eq!(start_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    start_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "format-ignore-start");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    }
                );

                // end marker should be owned by the fourth argument wrapper
                let end_annotations = parser.tree.get_annotations(dynamic_arguments[3].id);
                assert_eq!(end_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    end_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "format-ignore-end");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    }
                );
            }
        );
    });
}

/// Statement trailing line comments should stay as line postfix boundaries.
#[test]
fn test_attach_statement_trailing_line_comment_as_boundary_postfix() {
    let mut test = TestParser::new(
        r"const X = 1; // statement-tail
const Y = 2;",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "statement-tail");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );
}

/// Typescript diagnostic directives should stay as statement prefixes without extra blanks.
#[test]
fn test_attach_typescript_directive_comments_as_statement_prefix() {
    let mut test = TestParser::new_with_options(
        r#"// @ts-expect-error keep spacing
call(   a, b)

// @ts-ignore
value   =   compute(  1,  2)"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // call(...); value = compute(...)
    assert_eq!(expressions.len(), 2);

    let expect_error_annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(expect_error_annotations.len(), 1);
    assert_node!(
        parser.tree,
        expect_error_annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "@ts-expect-error keep spacing");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );

    let ignore_annotations = parser.tree.get_annotations(expressions[1].id);
    assert_eq!(ignore_annotations.len(), 2);
    assert_node!(
        parser.tree,
        ignore_annotations[0],
        Annotation::Blank { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Blank { lines } => {
                assert_eq!(*lines, 1);
            });
        }
    );
    assert_node!(
        parser.tree,
        ignore_annotations[1],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "@ts-ignore");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );
}

/// Assignment seams with `@ts-ignore` should keep one owner for the directive comment.
#[test]
fn test_attach_assignment_rhs_ts_ignore_comment_single_owner() {
    let mut test = TestParser::new_with_options(
        r"longVariableName1 = // @ts-ignore
(variable01 + veryLongVariableNameNumber2).method()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let (assign_id, call_id, member_id, member_left_id) = assert_node!(
        parser.tree,
        expression_id,
        Expression::Assign { right, .. } => {
            assert_node!(parser.tree, *right, Expression::Call { left, .. } => {
                assert_node!(parser.tree, *left, Expression::Member { left: member_left, .. } => {
                    (expression_id, *right, *left, *member_left)
                })
            })
        }
    );

    let assign_annotations = parser.tree.get_annotations(assign_id.id);
    assert!(
        assign_annotations.is_empty(),
        "assignment should not own @ts-ignore in this seam"
    );

    let member_annotations = parser.tree.get_annotations(member_id.id);
    assert!(
        member_annotations.is_empty(),
        "member should not duplicate @ts-ignore ownership"
    );

    let member_left_annotations = parser.tree.get_annotations(member_left_id.id);
    assert!(
        member_left_annotations.is_empty(),
        "parenthesized left should not duplicate @ts-ignore ownership"
    );

    let call_annotations = parser.tree.get_annotations(call_id.id);
    assert_eq!(
        call_annotations.len(),
        1,
        "call should own exactly one @ts-ignore comment"
    );
    assert_node!(
        parser.tree,
        call_annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_eq!(*style, CommentStyle::Slash);
                assert_string!(parser, *string, "@ts-ignore");
            });
        }
    );
}

/// Format-ignore comments should attach as statement prefixes.
#[test]
fn test_attach_format_ignore_comment_as_statement_prefix() {
    let mut test = TestParser::new_with_options(
        r#"// format-ignore
call(   a, b)"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "format-ignore");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );
}

/// Prettier-ignore comments should attach as statement prefixes.
#[test]
fn test_attach_prettier_ignore_comment_as_statement_prefix() {
    let mut test = TestParser::new_with_options(
        r#"// prettier-ignore
call(   a, b)"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "prettier-ignore");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );
}

/// Biome ignore comments should attach as statement prefixes.
#[test]
fn test_attach_biome_ignore_format_comment_as_statement_prefix() {
    let mut test = TestParser::new_with_options(
        r#"// biome-ignore format
call(   a, b)"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "biome-ignore format");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );
}

/// Pure pragma block comments should stay normal prefix comments.
#[test]
fn test_attach_pure_pragma_block_comment_as_statement_prefix_comment() {
    let mut test = TestParser::new_with_options("/*#__PURE__*/ make()", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "#__PURE__");
                assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Block comments around strict directives should stay attached to directive and following statement owners.
#[test]
fn test_attach_block_comments_around_strict_directive_boundaries() {
    let mut test = TestParser::new_with_options(
        r#"/******/ "use strict" /**/
/******/ run()"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);

    // first statement: "use strict"
    let first_annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(first_annotations.len(), 2);
    assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Star);
            assert_string!(parser, *string, "***");
        });
    });
    assert_node!(parser.tree, first_annotations[1], Annotation::Doc { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Doc { string, style } => {
            assert_eq!(*style, DocStyle::Star);
            assert_string!(parser, *string, "/**/");
        });
    });

    // second statement: run()
    let second_annotations = parser.tree.get_annotations(expressions[1].id);
    assert_eq!(second_annotations.len(), 1);
    assert_node!(parser.tree, second_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Star);
            assert_string!(parser, *string, "***");
        });
    });
}

/// JSX expression container comments should be preserved as annotations.
#[test]
fn test_attach_jsx_expression_container_comment() {
    let mut test = TestParser::new_with_options(
        "<div>{/* jsx-comment */}</div>",
        LanguageType::JavaScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::TreeExpression { elements, .. } => {
            let elements = elements.as_ref().expect("expected tree elements");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Stub => {});
                let annotations = parser.tree.get_annotations(value.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockInfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "jsx-comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                });
            });
        }
    );
}

/// JSX multiline expression container dangling comments should stay on the value expression.
#[test]
fn test_attach_jsx_expression_container_dangling_line_comment_on_multiline_value() {
    let mut test = TestParser::new_with_options(
        r#"<>
{
    value
    // this comment should stay here
}
</>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    // <> {value} </>
    assert_node!(
        parser.tree,
        expression_id,
        Expression::TreeExpression { elements, .. } => {
            let elements = elements.as_ref().expect("expected tree elements");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                let annotations = parser.tree.get_annotations(value.id);
                assert_eq!(annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "this comment should stay here");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    }
                );
            });
        }
    );
}

/// JSX expression container trailing line comments should stay on the nearest completed branch expression.
#[test]
fn test_attach_jsx_expression_container_trailing_line_comment() {
    let mut test = TestParser::new_with_options(
        r#"<div>{isVideo ? <Video /> : <Image /> // eslint-disable-line
}</div>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::TreeExpression { elements, .. } => {
            let elements = elements.as_ref().expect("expected tree elements");
            assert_eq!(elements.len(), 1);

            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::If { else_expression, .. } => {
                    let else_expression_id = else_expression.expect("expected ternary else branch");
                    let annotations = parser.tree.get_annotations(else_expression_id.id);
                    assert_eq!(annotations.len(), 1);
                    assert_node!(
                        parser.tree,
                        annotations[0],
                        Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_string!(parser, *string, "eslint-disable-line");
                                assert_eq!(*style, CommentStyle::Slash);
                            });
                        }
                    );
                });
            });
        }
    );
}

/// Ternary branch boundary comments should attach to the branch expression prefixes.
#[test]
fn test_attach_value_ternary_branch_prefix_comments() {
    let mut test = TestParser::new_with_options(
        r"const value = cond ? /* then-note */ left : /* else-note */ right",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected ternary value");
            assert_node!(parser.tree, value_id, Expression::If { kind, then_expression, else_expression, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
                let else_expression_id = else_expression.expect("expected else branch");

                let then_annotations = parser.tree.get_annotations(then_expression.id);
                assert_eq!(then_annotations.len(), 1);
                assert_node!(parser.tree, then_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Star);
                        assert_string!(parser, *string, "then-note");
                    });
                });

                let else_annotations = parser.tree.get_annotations(else_expression_id.id);
                assert_eq!(else_annotations.len(), 1);
                assert_node!(parser.tree, else_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Star);
                        assert_string!(parser, *string, "else-note");
                    });
                });
            });
        });
    });
}

/// TSX ternary branch comments should attach to TSX branch expression prefixes.
#[test]
fn test_attach_tsx_ternary_branch_prefix_comments() {
    let mut test = TestParser::new_with_options(
        r#"<div>{cond ? /* then-note */ <Video /> : /* else-note */ <Image />}</div>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected tree elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::If { kind, then_expression, else_expression, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
                let else_expression_id = else_expression.expect("expected else branch");

                let then_annotations = parser.tree.get_annotations(then_expression.id);
                assert_eq!(then_annotations.len(), 1);
                assert_node!(parser.tree, then_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Star);
                        assert_string!(parser, *string, "then-note");
                    });
                });

                let else_annotations = parser.tree.get_annotations(else_expression_id.id);
                assert_eq!(else_annotations.len(), 1);
                assert_node!(parser.tree, else_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Star);
                        assert_string!(parser, *string, "else-note");
                    });
                });
            });
        });
    });
}

/// TSX ternary alternate line comments should have exactly one owner: the then branch.
#[test]
fn test_attach_tsx_ternary_alternate_line_comment_once() {
    let mut test = TestParser::new_with_options(
        r#"<>
    {x ? <A /> : // alt-line
    <B />}
</>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::TreeExpression { elements, .. } => {
            let elements = elements.as_ref().expect("expected tree elements");
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                let ternary_annotations = parser.tree.get_annotations(value.id);
                assert_eq!(ternary_annotations.len(), 0);
                assert_node!(parser.tree, *value, Expression::If { then_expression, else_expression, .. } => {
                    let then_annotations = parser.tree.get_annotations(then_expression.id);
                    assert_eq!(then_annotations.len(), 1);
                    assert_node!(parser.tree, then_annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Slash);
                            assert_string!(parser, *string, "alt-line");
                        });
                    });

                    let else_expression_id = else_expression.expect("expected else branch");
                    let else_annotations = parser.tree.get_annotations(else_expression_id.id);
                    assert_eq!(else_annotations.len(), 0);
                });
            });
        }
    );
}

/// TSX ternary alternate block comments should stay attached to the alternate branch expression.
#[test]
fn test_attach_tsx_ternary_alternate_block_comment_to_else_postfix() {
    let mut test = TestParser::new_with_options(
        r#"const Component = () => (
  <div>
    {"error" ? (
      <Error />
    ) : (
      <Success />
      /* keep-inside-branch */
    )}
  </div>
)"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let function_id = value.expect("expected function initializer");
            assert_node!(parser.tree, function_id, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
                    let body_id = body.expect("expected function body");
                    assert_node!(parser.tree, body_id, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { elements, .. } => {
                            let elements = elements.as_ref().expect("expected tree elements");
                            assert_eq!(elements.len(), 1);
                            assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
                                assert_node!(parser.tree, *value, Expression::If { else_expression, .. } => {
                                    let else_expression_id = else_expression.expect("expected else branch");
                                    assert_node!(parser.tree, else_expression_id, Expression::Parenthesized { expression } => {
                                        let outer_annotations = parser.tree.get_annotations(else_expression_id.id);
                                        assert_eq!(outer_annotations.len(), 0);

                                        let inner_annotations = parser.tree.get_annotations(expression.id);
                                        assert_eq!(inner_annotations.len(), 1);
                                        assert_node!(parser.tree, inner_annotations[0], Annotation::Comment { node, position } => {
                                            assert_eq!(*position, AnnotationPosition::BlockPostfix);
                                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                                assert_eq!(*style, CommentStyle::Star);
                                                assert_string!(parser, *string, "keep-inside-branch");
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

/// TSX logical trailing line comments should stay attached to the logical expression.
#[test]
fn test_attach_tsx_logical_expression_trailing_line_comment_to_expression_postfix() {
    let mut test = TestParser::new_with_options(
        r#"<div>{ready && <Body /> // logical-tail
}</div>"#,
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expression_id, Expression::TreeExpression { elements, .. } => {
        let elements = elements.as_ref().expect("expected tree elements");
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], Argument::Positional { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                let value_annotations = parser.tree.get_annotations(value.id);
                assert_eq!(value_annotations.len(), 0);

                let right_annotations = parser.tree.get_annotations(right.id);
                assert_eq!(right_annotations.len(), 1);
                assert_node!(parser.tree, right_annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Slash);
                    assert_string!(parser, *string, "logical-tail");
                });
            });
            });
        });
    });
}

/// Declaration header boundary comments should stay on declaration seam owners.
#[test]
fn test_attach_declaration_body_boundary_comment_on_declaration_owner() {
    let mut test = TestParser::new("class Value /* declaration-body */ {}");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { .. } => {});
            let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
            assert!(declaration_annotations.is_empty());

            let expression_annotations = parser.tree.get_annotations(expression_id.id);
            assert_eq!(expression_annotations.len(), 1);
            assert_node!(parser.tree, expression_annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "declaration-body");
                    assert_eq!(*style, CommentStyle::Star);
                });
            });
        }
    );
}

/// Method signature boundary comments should stay on method return type boundaries.
#[test]
fn test_attach_method_body_boundary_comment_on_method_owner() {
    let mut test = TestParser::new(
        r"class Value {
method(): number // method-body-boundary
{
    return 1;
}
}",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
                assert_eq!(members.len(), 1);
                assert_node!(parser.tree, members[0], Member::Method { signature, body, .. } => {
                    let return_type = signature.return_type.expect("expected return type");
                    let body_id = body.expect("expected method body");
                    let body_annotations = parser.tree.get_annotations(body_id.id);
                    assert!(body_annotations.is_empty());

                    let annotations = parser.tree.get_annotations(return_type.id);
                    assert_eq!(annotations.len(), 1);
                    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "method-body-boundary");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                    let member_annotations = parser.tree.get_annotations(members[0].id);
                    assert!(member_annotations.is_empty());
                });
            });
        }
    );
}

/// Method bodies should keep expression statements wrapped as statement nodes.
#[test]
fn test_attach_method_body_keeps_statement_wrappers() {
    let mut test = TestParser::new_with_options(
        r"class Box {
  run() {
    call()
  }
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let statement_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, statement_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], Member::Method { body, .. } => {
                let body_id = body.expect("expected method body");
                assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                        assert_eq!(expressions.len(), 1);
                        assert_node!(parser.tree, expressions[0], Expression::Statement(inner) => {
                            assert_node!(parser.tree, *inner, Expression::Call { left, .. } => {
                                assert_expression_path!(parser, parser.tree.get(*left), "call");
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Interface method return-type separator comments should stay attached to the return type.
#[test]
fn test_attach_interface_method_return_separator_comment_to_return_type_prefix() {
    let mut test = TestParser::new_with_options(
        r"interface Worker {
  run(): // return-tail
  Promise<void>
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let member_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 1);
            members[0]
        })
    });
    let return_type_id = assert_node!(parser.tree, member_id, Member::Method { signature, .. } => {
        let method_return_type_id = signature.return_type.expect("expected return type");
        assert_expression_path!(parser, parser.tree.get(method_return_type_id), "Promise");
        method_return_type_id
    });
    let annotations = parser.tree.get_annotations(return_type_id.id);
    assert_eq!(annotations.len(), 1, "expected one return-tail comment");
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "return-tail");
        });
    });
}

/// Interface method parameter trailing separator comments should stay on the same parameter.
#[test]
fn test_attach_interface_method_parameter_trailing_comment_to_parameter_tail() {
    let mut test = TestParser::new_with_options(
        r"interface Worker {
  run(
    value: string, // value-tail
  ): number
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let member_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 1);
            members[0]
        })
    });
    let parameter_id = assert_node!(parser.tree, member_id, Member::Method { signature, .. } => {
        assert_eq!(signature.dynamic_parameters.len(), 1);
        assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
            assert_string!(parser, *name, "value");
        });
        signature.dynamic_parameters[0]
    });
    let annotations = parser.tree.get_annotations(parameter_id.id);
    assert_eq!(annotations.len(), 1, "expected one value-tail comment");
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "value-tail");
        });
    });
}

/// Mapped-type value separator comments should stay attached to the mapped value boundary.
#[test]
fn test_attach_mapped_type_value_separator_comment_to_value_prefix() {
    let mut test = TestParser::new_with_options(
        r"type Flags<T> = {
  [K in keyof T]: // mapped-line
  boolean
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let mapped_value_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeMapped { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Boolean) => {});
                *value
            })
        })
    });
    let annotations = parser.tree.get_annotations(mapped_value_id.id);
    assert_eq!(annotations.len(), 1, "expected one mapped-line comment");
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "mapped-line");
        });
    });
}

/// Type-conditional branch comments should stay attached to then and else branch type owners.
#[test]
fn test_attach_type_conditional_branch_comments_to_branch_type_prefixes() {
    let mut test = TestParser::new_with_options(
        r"type Value<T> = T extends /* extends-note */ string
  ? /* true-note */ number
  : /* false-note */ boolean",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let (then_type_id, else_type_id) = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeConditional { then_type, else_type, .. } => {
                assert_node!(parser.tree, *then_type, Expression::TypeLiteral(TypeLiteral::Number) => {});
                assert_node!(parser.tree, *else_type, Expression::TypeLiteral(TypeLiteral::Boolean) => {});
                (*then_type, *else_type)
            })
        })
    });
    let then_annotations = parser.tree.get_annotations(then_type_id.id);
    assert_eq!(then_annotations.len(), 1);
    assert_node!(parser.tree, then_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Star);
            assert_string!(parser, *string, "true-note");
        });
    });

    let else_annotations = parser.tree.get_annotations(else_type_id.id);
    assert_eq!(else_annotations.len(), 1);
    assert_node!(parser.tree, else_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Star);
            assert_string!(parser, *string, "false-note");
        });
    });
}

/// Export-head comments before newline declaration heads should stay attached and parse as declarations.
#[test]
fn test_attach_export_head_comment_before_interface_declaration() {
    let mut test = TestParser::new_with_options(
        r"export // export-head
interface Shape {
  value: string // value-tail
}",
        LanguageType::TypeScript,
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
    let declaration_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface { descriptor, .. } => {
            assert_eq!(descriptor.export, Some(destack_ast::DependencyMode::Item));
            assert_string!(parser, descriptor.name.unwrap().string(), "Shape");
        });
        *declaration_id
    });

    let annotations = parser.tree.get_annotations(declaration_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockInfix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "export-head");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Class heritage boundary comments should stay on extends and implements owners.
#[test]
fn test_attach_class_heritage_boundary_comments_to_super_types() {
    let mut test = TestParser::new_with_options(
        r"class Derived extends Base // base-tail
implements
// impl-head
A,
B // impl-tail
{}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let (base_type_id, first_implements_type_id, second_implements_type_id, declaration_id) = assert_node!(
        parser.tree,
        expression_id,
        Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
                let extends_types = heritage.extends_types.as_ref().expect("expected extends");
                let implements_types = heritage
                    .implements_types
                    .as_ref()
                    .expect("expected implements");
                assert_eq!(extends_types.len(), 1);
                assert_eq!(implements_types.len(), 2);
                (extends_types[0], implements_types[0], implements_types[1], *declaration_id)
            })
        }
    );

    let base_annotations = parser.tree.get_annotations(base_type_id.id);
    assert_eq!(base_annotations.len(), 1);
    assert_node!(parser.tree, base_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "base-tail");
        });
    });

    let first_impl_annotations = parser.tree.get_annotations(first_implements_type_id.id);
    assert_eq!(first_impl_annotations.len(), 1);
    assert_node!(parser.tree, first_impl_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-head");
        });
    });

    let second_impl_annotations = parser.tree.get_annotations(second_implements_type_id.id);
    assert_eq!(second_impl_annotations.len(), 1);
    assert_node!(parser.tree, second_impl_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-tail");
        });
    });

    let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
    assert!(declaration_annotations.is_empty());
}

/// Class extends-tail comments should stay attached to the superclass boundary.
#[test]
fn test_attach_class_superclass_boundary_comment_to_super_type() {
    let mut test = TestParser::new_with_options(
        r"class Child extends Base // extends-tail
{
  value = 1
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let (superclass_type_id, first_member_id, declaration_id) = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, members, .. } => {
            let extends_types = heritage.extends_types.as_ref().expect("expected extends");
            assert_eq!(extends_types.len(), 1);
            assert_eq!(members.len(), 1);
            (extends_types[0], members[0], *declaration_id)
        })
    });

    let superclass_annotations = parser.tree.get_annotations(superclass_type_id.id);
    assert_eq!(
        superclass_annotations.len(),
        1,
        "expected one extends-tail comment"
    );
    assert_node!(parser.tree, superclass_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "extends-tail");
        });
    });

    let member_annotations = parser.tree.get_annotations(first_member_id.id);
    assert!(member_annotations.is_empty());

    let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
    assert!(declaration_annotations.is_empty());
}

/// Implement-list comments should keep separator and tail ownership inside the list.
#[test]
fn test_attach_class_implement_list_comments_to_interface_types() {
    let mut test = TestParser::new_with_options(
        r"class Child implements First, // impl-first
Second // impl-second
{
  value = 1
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let (first_implements_type_id, second_implements_type_id, first_member_id, declaration_id) = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, members, .. } => {
            let implements_types = heritage
                .implements_types
                .as_ref()
                .expect("expected implements");
            assert_eq!(implements_types.len(), 2);
            assert_eq!(members.len(), 1);
            (implements_types[0], implements_types[1], members[0], *declaration_id)
        })
    });

    let second_annotations = parser.tree.get_annotations(second_implements_type_id.id);
    assert_eq!(second_annotations.len(), 1);
    assert_node!(parser.tree, second_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-second");
        });
    });

    // separator line comments stay with the previous list element
    let first_annotations = parser.tree.get_annotations(first_implements_type_id.id);
    assert_eq!(first_annotations.len(), 1);
    assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-first");
        });
    });

    let member_annotations = parser.tree.get_annotations(first_member_id.id);
    assert!(member_annotations.is_empty());

    let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
    assert!(declaration_annotations.is_empty());
}

/// Declare-class head comments before generics should stay on generic parameter prefixes.
#[test]
fn test_attach_declare_class_head_comment_before_generics_to_declaration_owner() {
    let mut test = TestParser::new_with_options(
        r"declare class Box // box-head
<T> implements Item<T>, Other {
  value: T
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let first_static_parameter_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { generics, .. } => {
            let static_parameters = generics
                .static_parameters
                .as_ref()
                .expect("expected class static parameters");
            assert_eq!(static_parameters.len(), 1);
            static_parameters[0]
        })
    });

    let annotations = parser.tree.get_annotations(first_static_parameter_id.id);
    assert_eq!(annotations.len(), 1, "expected one box-head comment");
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "box-head");
        });
    });
}

#[test]
fn test_attach_type_union_line_comment_before_separator_to_left_operand_postfix() {
    let mut test = TestParser::new_with_options(
        r"type Value = First
// union-line
| Second
| Third",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
            });
        });
    });

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_expression_path!(parser, parser.tree.get(*right), "Third");
                assert_node!(parser.tree, *left, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_expression_path!(parser, parser.tree.get(*left), "First");
                    assert_expression_path!(parser, parser.tree.get(*right), "Second");

                    let first_annotations = parser.tree.get_annotations(left.id);
                    assert_eq!(first_annotations.len(), 1);
                    assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Slash);
                            assert_string!(parser, *string, "union-line");
                        });
                    });

                    let second_annotations = parser.tree.get_annotations(right.id);
                    assert!(second_annotations.is_empty());
                });
            });
        });
    });
}

#[test]
fn test_attach_type_union_line_comment_after_separator_to_left_operand_postfix_boundary() {
    let mut test = TestParser::new_with_options(
        r"type Value = First | // union-line
Second | Third",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let first_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_expression_path!(parser, parser.tree.get(*right), "Third");
                assert_node!(parser.tree, *left, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_expression_path!(parser, parser.tree.get(*left), "First");
                    assert_expression_path!(parser, parser.tree.get(*right), "Second");
                    *left
                })
            })
        })
    });

    let annotations = parser.tree.get_annotations(first_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "union-line");
        });
    });
}

/// Leading-pipe own-line comments should attach once to the left union arm postfix.
#[test]
fn test_attach_type_union_leading_pipe_comment_to_left_operand_postfix() {
    let mut test = TestParser::new_with_options(
        r"type Value = First
    // union-line
    | Second
    | Third;",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
            });
        });
    });

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_expression_path!(parser, parser.tree.get(*right), "Third");
                assert_node!(parser.tree, *left, Expression::Binary { operator, left, right } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_expression_path!(parser, parser.tree.get(*left), "First");
                    assert_expression_path!(parser, parser.tree.get(*right), "Second");

                    let first_annotations = parser.tree.get_annotations(left.id);
                    assert_eq!(first_annotations.len(), 1);
                    assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Slash);
                            assert_string!(parser, *string, "union-line");
                        });
                    });

                    let second_annotations = parser.tree.get_annotations(right.id);
                    assert!(second_annotations.is_empty());
                });
            });
        });
    });
}

#[test]
fn test_attach_type_union_block_comment_between_arms_to_left_operand_postfix() {
    let mut test = TestParser::new_with_options(
        r"type Value = First /* union-block */ | Second",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_expression_path!(parser, parser.tree.get(*left), "First");
                assert_expression_path!(parser, parser.tree.get(*right), "Second");

                let first_annotations = parser.tree.get_annotations(left.id);
                assert_eq!(first_annotations.len(), 1);
                assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Star);
                        assert_string!(parser, *string, "union-block");
                    });
                });

                let second_annotations = parser.tree.get_annotations(right.id);
                assert!(second_annotations.is_empty());
            });
        });
    });
}

#[test]
fn test_attach_type_intersection_line_comment_between_arms_to_left_operand_postfix_boundary() {
    let mut test = TestParser::new_with_options(
        r"type Value = First & // intersection-line
Second",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let left_id = assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::Binary { operator, left, right } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseAnd);
                assert_expression_path!(parser, parser.tree.get(*left), "First");
                assert_expression_path!(parser, parser.tree.get(*right), "Second");
                *left
            })
        })
    });

    let annotations = parser.tree.get_annotations(left_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "intersection-line");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Decorator-adjacent comments should preserve stable ownership order on the statement node.
#[test]
fn test_attach_decorator_adjacent_comments_on_struct_statement() {
    let mut test = TestParser::new(
        r#"{
// comment before entity
@entity
// comment after entity
// comment before foo
@foo(1, 2, 3)
// comment after foo
struct Entity {}
}"#,
    );
    let mut parser = test.prepare();
    let block_id = parser.eat_block().unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, block_id, Block { expressions, .. } => {
        assert_eq!(expressions.len(), 1);
        let statement_id = expressions[0];
        let annotations = parser.tree.get_annotations(statement_id.id);
        assert_eq!(annotations.len(), 5);

        assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "comment before entity");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
        assert_node!(parser.tree, annotations[1], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_expression_path!(parser, parser.tree.get(*expression), "entity");
            });
        });
        assert_node!(parser.tree, annotations[2], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "comment after entity\ncomment before foo");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
        assert_node!(parser.tree, annotations[3], Annotation::Decorator { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Decorator { expression } => {
                assert_node!(parser.tree, *expression, Expression::Call { left, dynamic_arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    assert_eq!(dynamic_arguments.len(), 3);
                });
            });
        });
        assert_node!(parser.tree, annotations[4], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "comment after foo");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
    });
}

/// Import specifier trailing comments should stay on the same specifier item.
#[test]
fn test_attach_import_specifier_trailing_comment_to_first_item() {
    let mut test = TestParser::new_with_options(
        r#"import {
  first, // first-spec
  second,
} from "mod";"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let first_item_id = assert_node!(parser.tree, expression_id, Expression::Import { items, .. } => {
        assert_eq!(items.len(), 2);
        items[0]
    });
    assert_node!(parser.tree, first_item_id, DependencyItem { .. } => {});

    let annotations = parser.tree.get_annotations(first_item_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "first-spec");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Export specifier trailing comments should stay on the same specifier item.
#[test]
fn test_attach_export_specifier_trailing_comment_to_first_item() {
    let mut test = TestParser::new_with_options(
        r#"export {
  first, // first-export
  second,
};"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let first_item_id = assert_node!(parser.tree, expression_id, Expression::Export { items, .. } => {
        assert_eq!(items.len(), 2);
        items[0]
    });
    assert_node!(parser.tree, first_item_id, DependencyItem { .. } => {});

    let annotations = parser.tree.get_annotations(first_item_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "first-export");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Dynamic import source comments should attach to the import target expression.
#[test]
fn test_attach_dynamic_import_source_comment_to_target_expression() {
    let mut test = TestParser::new_with_options(
        r#"const mod = import(
  // dynamic-source
  "module",
)"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = parser.unwrap_statement_expression(expressions[0]);
    let target_expression_id = assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(
                parser.tree,
                value_id,
                Expression::Import {
                    target: ImportTarget::Expression { target },
                    ..
                } => {
                    *target
                }
            )
        })
    });

    let annotations = parser.tree.get_annotations(target_expression_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "dynamic-source");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Multi-line suffix is attached to the previous node on the same line as a block postfix.
#[test]
fn test_attach_multi_line_postfix_to_expression() {
    let mut test = TestParser::new(
        r"let A = 1 /* line comment
over multiple lines with trailing space    */",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // let A = 1
    assert_eq!(expressions.len(), 1);
    // block comment, postfix
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 1);
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPostfix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "line comment\nover multiple lines with trailing space");
            assert_eq!(*style, CommentStyle::Star);
        });
    });
}

/// Multiline block doc comments are cleaned up properly.
#[test]
fn test_clean_multiline_block_doc() {
    let mut test = TestParser::new(
        r"{
/** some multiline
 * block comment
 * over multiple lines */
let X = 1
}",
    );
    let mut parser = test.prepare();
    let block = parser.eat_block().unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, block, Block { expressions, .. } => {
        assert_eq!(expressions.len(), 1);
        let annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_string!(parser, *string, "some multiline\nblock comment\nover multiple lines");
                assert_eq!(*style, DocStyle::Star);
            });
        });
    });
}

/// Block doc comments in declaration blocks should attach to the following function.
#[test]
fn test_attach_doc_block_prefix_to_next_function() {
    let mut test = TestParser::new(
        r"interface X {
/** Doc A */
a(): A
/** Doc B */
b(): B
}",
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
    parser.attach_trivia();

    // interface X
    assert_node!(parser.tree, interface_id, Declaration::Interface { members, .. } => {
        assert_eq!(members.len(), 2);

        // a(): A
        let annotations = parser.tree.get_annotations(members[0].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_eq!(*style, DocStyle::Star);
                assert_string!(parser, *string, "Doc A");
            });
        });

        // b(): B
        let annotations = parser.tree.get_annotations(members[1].id);
        assert_eq!(annotations.len(), 1);
        assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Doc { string, style } => {
                assert_eq!(*style, DocStyle::Star);
                assert_string!(parser, *string, "Doc B");
            });
        });
    });
}

/// Block doc comments should attach to the following function in a nested function.
#[test]
fn test_attach_doc_block_prefix_to_nested_function() {
    let mut test = TestParser::new(
        r"function foo() {
/** Doc A */
function a(): A
/** Doc B */
function b(): B
/** Doc C */
function c(): C {
    remove(hey.so)
}
}",
    );
    let mut parser = test.prepare();
    let start = parser.mark();
    let function = parser
        .eat_function(&start, DeclarationDescriptor::default(), false, false)
        .unwrap();
    parser.attach_trivia();

    // function foo()
    assert_node!(parser.tree, function, Declaration::Function { body: body_id, .. } => {
        assert_node!(parser.tree, body_id.unwrap(), Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 3);

                // function a(): A
                assert_node!(parser.tree, expressions[0], Expression::Declaration(node) => {
                    assert_node!(parser.tree, *node, Declaration::Function { descriptor, .. } => {
                        assert_string!(parser, descriptor.name.unwrap().string(), "a");
                    });
                });
                let annotations = parser.tree.get_annotations(expressions[0].id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Doc { string, style } => {
                        assert_eq!(*style, DocStyle::Star);
                        assert_string!(parser, *string, "Doc A");
                    });
                });

                // function b(): B
                assert_node!(parser.tree, expressions[1], Expression::Declaration(node) => {
                    assert_node!(parser.tree, *node, Declaration::Function { descriptor, .. } => {
                        assert_string!(parser, descriptor.name.unwrap().string(), "b");
                    });
                });
                let annotations = parser.tree.get_annotations(expressions[1].id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Doc { string, style } => {
                        assert_eq!(*style, DocStyle::Star);
                        assert_string!(parser, *string, "Doc B");
                    });
                });

                // function c() C { .. }
                assert_node!(parser.tree, expressions[2], Expression::Declaration(node) => {
                    assert_node!(parser.tree, *node, Declaration::Function { descriptor, .. } => {
                        assert_string!(parser, descriptor.name.unwrap().string(), "c");
                    });
                });
                let annotations = parser.tree.get_annotations(expressions[2].id);
                assert_eq!(annotations.len(), 1);
                assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Doc { string, style } => {
                        assert_eq!(*style, DocStyle::Star);
                        assert_string!(parser, *string, "Doc C");
                    });
                });
            });
        });
    });
}

/// Inline prefix, infix and suffix comments should be attached to closest inner node on the same line.
#[test]
fn test_attach_line_prefix_infix_postfix_to_expression() {
    let mut test =
        TestParser::new("let X = /* Pre-A comment */ A /* A comment */ && B /* B comment */");
    let mut parser = test.prepare();
    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    parser.attach_trivia();

    // let X = A && B
    assert_eq!(expressions.len(), 1);

    // A && B
    assert_node!(parser.tree, expressions[0], Expression::Statement(statement_id) => {
        assert_node!(parser.tree, *statement_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, right, operator } => {
                assert_eq!(*operator, BinaryOperator::And);
                let left_annotations = parser.tree.get_annotations(left.id);
                assert_eq!(left_annotations.len(), 2);
                assert_node!(parser.tree, left_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "Pre-A comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                });
                assert_node!(parser.tree, left_annotations[1], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "A comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                });

                let right_annotations = parser.tree.get_annotations(right.id);
                assert_eq!(right_annotations.len(), 1);
                assert_node!(parser.tree, right_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "B comment");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                });

                // A
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments: _ } => {
                    assert_path!(parser, *path, "A");
                });

                // B
                assert_node!(parser.tree, *right, Expression::Path { path, static_arguments: _ } => {
                    assert_path!(parser, *path, "B");
                });
            });
            });
        });
    });
}

/// Line suffix is attached separatelyfrom other surrounding comments.
#[test]
fn test_attach_line_postfix_to_expression_with_surrounding_comments() {
    let mut test = TestParser::new(
        r"
// block prefix comment
let A = 1 // line suffix comment
// block postfix comment
",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    parser.attach_trivia();

    // let A = 1
    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 3);

    // block prefix comment
    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "block prefix comment");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
    // line postfix boundary comment
    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "line suffix comment");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
    // block postfix comment
    assert_node!(parser.tree, annotations[2], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPostfix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "block postfix comment");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Blanks (>2 successive newlines) are just annotations and should be attached to the next node.
/// Like any other annotation, if no next or containing node is found, attach to previous node as suffix.
#[test]
fn test_attach_blanks_to_expressions() {
    let mut test = TestParser::new(
        r"

let A = 1

let B = 2

",
    );
    let mut parser = test.prepare();
    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    parser.attach_trivia();

    assert_eq!(expressions.len(), 2);

    // A has one prefix block blank
    let a_annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(a_annotations.len(), 1);
    assert_node!(parser.tree, a_annotations[0], Annotation::Blank { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Blank { lines } => {
            assert_eq!(*lines, 1);
        });
    });

    // B has one prefix block blank and one postfix block blank
    // (the postfix blank after B because there is nothing else to attach to)
    let b_annotations = parser.tree.get_annotations(expressions[1].id);
    assert_eq!(b_annotations.len(), 2);
    assert_node!(parser.tree, b_annotations[0], Annotation::Blank { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Blank { lines } => {
            assert_eq!(*lines, 1);
        });
    });
    assert_node!(parser.tree, b_annotations[1], Annotation::Blank { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPostfix);
        assert_node!(parser.tree, *node, Blank { lines } => {
            assert_eq!(*lines, 1);
        });
    });
}

/// Annotations inside an empty node should be treated as infix within the innermost containing node.
#[test]
fn test_attach_comments_infix_in_block() {
    let mut test = TestParser::new(
        r"
function main() {
// block comment, infix
}",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();

    let start = parser.mark();
    let function = parser
        .eat_function(&start, DeclarationDescriptor::default(), false, false)
        .unwrap();
    parser.attach_trivia();

    // (annotation should be infix to innermost node, i.e. the block)
    assert_node!(parser.tree, function, Declaration::Function { body: body_id, .. } => {
        assert_node!(parser.tree, body_id.unwrap(), Expression::Block(block_id) => {
            // block comment, infix
            let annotations = parser.tree.get_annotations(block_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "block comment, infix");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });
        });
    });
}

/// Mixed annotations should also be attached to the node they are attached to.
/// Successive annotations of the same type should be merged as relevant.
/// Within a containing node (like the struct), the comment at the end should be treated as infix
///  since we don't have a following node to attach to (but do have a containing node).
#[test]
fn test_attach_mixed_annotations_to_struct() {
    let mut test = TestParser::new(
        r"
/// doc, floating

/// doc, struct
/// doc, struct continued
struct Floof {
/// doc, struct field
/// doc, struct field continued
a: int32 // doc, struct field infix

// random comment
}",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    parser.attach_trivia();

    // struct Floof
    assert_eq!(expressions.len(), 1);
    let annotations = parser.tree.get_annotations(expressions[0].id);
    assert_eq!(annotations.len(), 3);
    // doc block prefix, floating
    assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Doc { string, style } => {
            assert_string!(parser, *string, "doc, floating");
            assert_eq!(*style, DocStyle::Slash);
        });
    });
    // blank block prefix
    assert_node!(parser.tree, annotations[1], Annotation::Blank { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Blank { lines } => {
            assert_eq!(*lines, 1);
        });
    });
    // doc block prefix
    // struct\ndoc, struct continued
    assert_node!(parser.tree, annotations[2], Annotation::Doc { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Doc { string, style } => {
            assert_string!(parser, *string, "doc, struct\ndoc, struct continued");
            assert_eq!(*style, DocStyle::Slash);
        });
    });

    // struct Floof
    assert_node!(parser.tree, expressions[0], Expression::Declaration(node) => {
        assert_node!(parser.tree, *node, Declaration::Struct { members, .. } => {
            // a: int32
            assert_eq!(members.len(), 1);
                assert_node!(parser.tree, members[0], Member::Field { key: Some(Key::Name(Name::Identifier(name))), .. } => {
                    assert_string!(parser, *name, "a");
                    let annotations = parser.tree.get_annotations(members[0].id);
                    assert_eq!(annotations.len(), 4);

                // doc block prefix
                // struct field\ndoc, struct field continued
                assert_node!(parser.tree, annotations[0], Annotation::Doc { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Doc { string, style } => {
                        assert_string!(parser, *string, "doc, struct field\ndoc, struct field continued");
                        assert_eq!(*style, DocStyle::Slash);
                    });
                });

                // doc line postfix boundary
                // doc, struct field infix
                assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "doc, struct field infix");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                });

                // blank block postfix
                assert_node!(parser.tree, annotations[2], Annotation::Blank { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPostfix);
                    assert_node!(parser.tree, *node, Blank { lines } => {
                        assert_eq!(*lines, 1);
                    });
                });

                // doc block postfix
                assert_node!(parser.tree, annotations[3], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPostfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "random comment");
                        assert_eq!(*style, CommentStyle::Slash);
                    });
                });
            });
            });
    })
}

/// Nested namespace declarations should keep prefix comments on each level.
#[test]
fn test_attach_annotations_in_mixed_nested_declaration() {
    let mut test = TestParser::new(
        r"
// Outer comment
export namespace Outer {
// Middle comment
export namespace Middle {
    // Inner comment
    export type Inner = { }
}
}",
    );
    let mut parser = test.prepare();
    parser.eat_newline().unwrap();
    let expressions = parser.eat_block_body(BlockFormat::Implicit).unwrap();
    parser.attach_trivia();

    // Outer namespace
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(node) => {
        // Outer comment
        let outer_annotations = parser.tree.get_annotations(expressions[0].id);
        assert_eq!(outer_annotations.len(), 1);
        assert_node!(parser.tree, outer_annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "Outer comment");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });

        // Outer
        assert_node!(parser.tree, *node, Declaration::Namespace { descriptor, expressions: outer_expressions, .. } => {
            assert_string!(parser, descriptor.name.unwrap().string(), "Outer");
            assert_eq!(outer_expressions.len(), 1);

            // Middle comment
            let middle_expression_id = outer_expressions[0];
            let middle_annotations = parser.tree.get_annotations(middle_expression_id.id);
            assert_eq!(middle_annotations.len(), 1);
            assert_node!(parser.tree, middle_annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "Middle comment");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            });

            // Middle
            assert_node!(parser.tree, middle_expression_id, Expression::Declaration(node) => {
                assert_node!(parser.tree, *node, Declaration::Namespace { descriptor, expressions: middle_expressions, .. } => {
                    assert_string!(parser, descriptor.name.unwrap().string(), "Middle");
                    assert_eq!(middle_expressions.len(), 1);

                    // Inner comment
                    let inner_expression_id = middle_expressions[0];
                    let inner_annotations = parser.tree.get_annotations(inner_expression_id.id);
                    assert_eq!(inner_annotations.len(), 1);
                    assert_node!(parser.tree, inner_annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "Inner comment");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });
            });
        });
    });
}

/// Successive labelled blocks should keep surrounding comments attached to the right nodes.
#[test]
fn test_attach_multiple_comments_around_expression_in_successive_blocks() {
    let mut test = TestParser::new(
        r"{
// comment part 0
a: {
    // comment part 1
    // comment part 2
    const A = 1
    // comment part 3
    // comment part 4
}
// comment part 5
// comment part 6
b: {
    // comment part 7
    // comment part 8
    const B = 2
    // comment part 9
    // comment part 10
}
// comment part 11
}",
    );
    let mut parser = test.prepare();
    let block = parser.eat_block().unwrap();
    parser.attach_trivia();

    // { a: { .. } b: { .. } }
    assert_node!(parser.tree, block, Block { expressions, .. } => {
        assert_eq!(expressions.len(), 2);

        // a (labelled block)
        let a = expressions[0];
        let a_annotations = parser.tree.get_annotations(a.id);
        assert_eq!(a_annotations.len(), 1); // (0 as block prefix)

        // comment part 0
        assert_node!(parser.tree, a_annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "comment part 0");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });

        // a: { .. } - now Expression::Labelled
        assert_node!(parser.tree, a, Expression::Labelled { label: _, body } => {
            assert_node!(parser.tree, *body, Expression::Block (block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    let annotations = parser.tree.get_annotations(expressions[0].id);
                    assert_eq!(annotations.len(), 2);
                    // comment part 1\ncomment part 2
                    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 1\ncomment part 2");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                    // comment part 3\ncomment part 4
                    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 3\ncomment part 4");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });
            });
        });

        // b (labelled block)
        let b = expressions[1];
        let b_annotations = parser.tree.get_annotations(b.id);
        assert_eq!(b_annotations.len(), 2); // (5+6 as block prefix, 11 as block postfix)

        // comment part 5\ncomment part 6
        assert_node!(parser.tree, b_annotations[0], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "comment part 5\ncomment part 6");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });

        // b: { .. } - now Expression::Labelled
        assert_node!(parser.tree, b, Expression::Labelled { label: _, body } => {
            assert_node!(parser.tree, *body, Expression::Block (block_id) => {
                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                    assert_eq!(expressions.len(), 1);
                    let annotations = parser.tree.get_annotations(expressions[0].id);
                    assert_eq!(annotations.len(), 2);
                    // comment part 7\ncomment part 8
                    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 7\ncomment part 8");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                    // comment part 9\ncomment part 10
                    assert_node!(parser.tree, annotations[1], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::BlockPostfix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "comment part 9\ncomment part 10");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });
            });
        });

        // comment part 11
        assert_node!(parser.tree, b_annotations[1], Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPostfix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "comment part 11");
                assert_eq!(*style, CommentStyle::Slash);
            });
        });
    });
}

/// Blank lines between array elements should be attached as block prefix annotations.
#[test]
fn test_attach_blanks_in_array_elements() {
    let mut test = TestParser::new(
        r"[
1,

2,
]",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.attach_trivia();

    assert_node!(parser.tree, expr_id, Expression::ArrayExpression { elements } => {
        assert_eq!(elements.len(), 2);

        // first element has no annotations
        let first_annotations = parser.tree.get_annotations(elements[0].id);
        assert!(first_annotations.is_empty(), "first element should have no annotations");

        // second element has blank prefix annotation
        let second_annotations = parser.tree.get_annotations(elements[1].id);
        assert_eq!(second_annotations.len(), 1, "second element should have one blank annotation");
        assert_node!(parser.tree, second_annotations[0], Annotation::Blank { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Blank { lines } => {
                assert_eq!(*lines, 1);
            });
        });
    });
}

/// Array boundary comments should attach to stable element owners with stable positions.
#[test]
fn test_attach_array_boundary_comment_owners_and_positions() {
    let mut test = TestParser::new(
        r"const list = [
  first, // first-tail
  /* second-head */ second,
  third // third-tail
]",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let statement_id = parser.unwrap_statement_expression(expressions[0]);
    let (first_element_id, second_element_id, third_element_id) = assert_node!(parser.tree, statement_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let value = parser
            .tree
            .get(declarators[0])
            .value
            .expect("expected declarator value");
        assert_node!(parser.tree, value, Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 3);
            (elements[0], elements[1], elements[2])
        })
    });

    let first_annotations = parser.tree.get_annotations(first_element_id.id);
    assert_eq!(first_annotations.len(), 1);
    assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "first-tail");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });

    let second_annotations = parser.tree.get_annotations(second_element_id.id);
    assert_eq!(second_annotations.len(), 1);
    assert_node!(parser.tree, second_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "second-head");
            assert_eq!(*style, CommentStyle::Star);
        });
    });

    let third_annotations = parser.tree.get_annotations(third_element_id.id);
    assert_eq!(third_annotations.len(), 1);
    assert_node!(parser.tree, third_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "third-tail");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Object property trailing comments should stay attached to the property boundary owner.
#[test]
fn test_attach_object_property_trailing_comment_owners_and_positions() {
    let mut test = TestParser::new(
        r"const config = {
  first: 1, // first-tail
  second: 2 /* second-tail */
}",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let statement_id = parser.unwrap_statement_expression(expressions[0]);
    let ((first_property_id, second_property_id), (first_value_id, second_value_id)) = assert_node!(parser.tree, statement_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let value = parser
            .tree
            .get(declarators[0])
            .value
            .expect("expected declarator value");
        assert_node!(parser.tree, value, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 2);
            let first_value_id = assert_node!(parser.tree, properties[0], Property::Field { value, .. } => {
                value.expect("expected first property value")
            });
            let second_value_id = assert_node!(parser.tree, properties[1], Property::Field { value, .. } => {
                value.expect("expected second property value")
            });
            ((properties[0], properties[1]), (first_value_id, second_value_id))
        })
    });

    let first_annotations = parser.tree.get_annotations(first_property_id.id);
    assert_eq!(first_annotations.len(), 1);
    assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "first-tail");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });

    let second_annotations = parser.tree.get_annotations(second_property_id.id);
    assert_eq!(second_annotations.len(), 1);
    assert_node!(parser.tree, second_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "second-tail");
            assert_eq!(*style, CommentStyle::Star);
        });
    });

    let first_value_annotations = parser.tree.get_annotations(first_value_id.id);
    assert!(
        first_value_annotations.is_empty(),
        "first property value should not own trailing comments"
    );

    let second_value_annotations = parser.tree.get_annotations(second_value_id.id);
    assert!(
        second_value_annotations.is_empty(),
        "second property value should not own trailing comments"
    );
}

/// Trailing line comments after a trailing comma should stay on the previous argument boundary.
#[test]
fn test_attach_call_trailing_comma_line_comment_to_previous_argument_boundary() {
    let mut test = TestParser::new_with_options(
        r#"cb(
  overflowing ? "absolute top-0" : "relative", // keep-conditional
)"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let statement_id = parser.unwrap_statement_expression(expressions[0]);
    let argument_id = assert_node!(parser.tree, statement_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        dynamic_arguments[0]
    });

    let argument_annotations = parser.tree.get_annotations(argument_id.id);
    assert_eq!(argument_annotations.len(), 1);
    assert_node!(parser.tree, argument_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "keep-conditional");
            assert_eq!(*style, CommentStyle::Slash);
        });
    });
}

/// Class field trailing block comments should stay on the field member boundary.
#[test]
fn test_attach_class_field_trailing_block_comment_to_member_boundary() {
    let mut test = TestParser::new_with_options(
        r"class Box {
  value = 1; /* keep-field */
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let statement_id = parser.unwrap_statement_expression(expressions[0]);
    let member_id = assert_node!(parser.tree, statement_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
            assert_eq!(members.len(), 1);
            members[0]
        })
    });

    let member_annotations = parser.tree.get_annotations(member_id.id);
    assert_eq!(member_annotations.len(), 1);
    assert_node!(parser.tree, member_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "keep-field");
            assert_eq!(*style, CommentStyle::Star);
        });
    });

    assert_node!(parser.tree, member_id, Member::Field { .. } => {});
}

/// Class field trailing block comments without explicit semicolons should stay on the member boundary.
#[test]
fn test_attach_class_field_trailing_block_comment_without_semicolon_to_member_boundary() {
    let mut test = TestParser::new_with_options(
        r"class Box {
  value = 1 /* keep-field */
  next = 2
}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let statement_id = parser.unwrap_statement_expression(expressions[0]);
    let (first_member_id, initializer_id) = assert_node!(parser.tree, statement_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
            assert_eq!(members.len(), 2);
            let initializer_id = assert_node!(parser.tree, members[0], Member::Field { value, default, .. } => {
                value.or(*default).expect("expected field initializer")
            });
            (members[0], initializer_id)
        })
    });

    let first_member_annotations = parser.tree.get_annotations(first_member_id.id);
    assert_eq!(first_member_annotations.len(), 1);
    assert_node!(parser.tree, first_member_annotations[0], Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_string!(parser, *string, "keep-field");
            assert_eq!(*style, CommentStyle::Star);
        });
    });

    let initializer_annotations = parser.tree.get_annotations(initializer_id.id);
    assert!(
        initializer_annotations.is_empty(),
        "field initializer should not own trailing comments"
    );
}

/// Marker comments after declarations should stay on the declaration statement boundary.
#[test]
fn test_attach_variable_trailing_marker_comment_to_declaration_statement_boundary() {
    let mut test = TestParser::new_with_options(
        r"declare const PAGE_PATH: string
  //<- keep-marker
;(()=>{})()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 2);

    let declaration_statement_id = expressions[0];
    let declaration_statement_annotations =
        parser.tree.get_annotations(declaration_statement_id.id);
    assert_eq!(declaration_statement_annotations.len(), 1);
    assert_node!(
        parser.tree,
        declaration_statement_annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "<- keep-marker");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );
}
