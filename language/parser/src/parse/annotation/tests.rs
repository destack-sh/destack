use destack_ast::{
    Annotation, AnnotationPosition, Argument, BinaryOperator, Blank, Block, BlockFormat, Comment,
    CommentStyle, Declaration, DeclarationAbstraction, DeclarationDescriptor, Declarator,
    Decorator, DependencyItem, Doc, DocStyle, Expression, FunctionKind, FunctionMode, IfCondition,
    IfKind,
    ImportTarget, Key, LocalNodeId, Member, Name, Parameter, Property, TypeBinaryOperator,
    TypeKind, TypeLiteral, TypeUnaryOperator,
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
    parser.finish_annotations();

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

    // expected declaration metadata
    let expected_names = ["Foo", "Bar", "Foo", "Foo", "Foo"];
    let expected_abstractions = [
        DeclarationAbstraction::Concrete,
        DeclarationAbstraction::Concrete,
        DeclarationAbstraction::Concrete,
        DeclarationAbstraction::Abstract,
        DeclarationAbstraction::Abstract,
    ];
    let expected_decorators = [
        vec!["decorator"],
        vec!["first.field", "second", "<<call>>"],
        vec!["before", "after"],
        vec!["before"],
        vec!["before", "after"],
    ];

    // per declaration assertions
    for index in 0..expressions.len() {
        // class declaration
        let expression_id = parser.unwrap_statement_expression(expressions[index]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { descriptor, .. } => {
                assert!(descriptor.export.is_some());
                assert_eq!(descriptor.abstraction, expected_abstractions[index]);
                assert_string!(parser, descriptor.name.unwrap().string(), expected_names[index]);
            });
        });

        // decorator list
        let annotations = parser.tree.get_annotations(expression_id.id);
        assert_eq!(annotations.len(), expected_decorators[index].len());
        for annotation_index in 0..annotations.len() {
            let expected_decorator = expected_decorators[index][annotation_index];
            assert_node!(parser.tree, annotations[annotation_index], Annotation::Decorator { node, .. } => {
                assert_node!(parser.tree, *node, Decorator { expression } => {
                    if expected_decorator == "<<call>>" {
                        assert_node!(parser.tree, *expression, Expression::Call { .. } => {});
                    } else {
                        assert_expression_path!(
                            parser,
                            parser.tree.get(*expression),
                            expected_decorator
                        );
                    }
                });
            });
        }
    }
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

                            // match return type or body for the arrow target
                            if let Some(return_type) = signature.return_type {
                                assert_expression_path!(parser, parser.tree.get(return_type), "T");
                            } else {
                                let body = body.expect("expected body expression");
                                assert_expression_path!(parser, parser.tree.get(body), "T");
                            }
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
            let mut prefix_blanks = Vec::new();
            let mut postfix_blanks = Vec::new();
            for annotation_id in second_annotations {
                if let Annotation::Blank { node, position } = parser.tree.get::<Annotation>(annotation_id) {
                    let Blank { lines } = parser.tree.get::<Blank>(*node);
                    let lines = *lines;
                    match position {
                        AnnotationPosition::BlockPrefix => prefix_blanks.push(lines),
                        AnnotationPosition::BlockPostfix => postfix_blanks.push(lines),
                        _ => {}
                    }
                }
            }

            assert_eq!(prefix_blanks, vec![1]);
            assert_eq!(postfix_blanks, vec![1]);
        });
    });
}

/// Enum body-boundary comments should attach to the enum declaration owner.
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
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Enum { .. } => {});

        let mut matches = Vec::new();
        for (owner_id, annotations) in parser.tree.get_all_annotations() {
            for annotation_id in annotations {
                let Annotation::Comment { node, position } = parser.tree.get::<Annotation>(*annotation_id) else {
                    continue;
                };
                let comment = parser.tree.get::<Comment>(*node);
                let text = parser.strings.get(comment.string);
                if text == "enum-body" {
                    matches.push((*owner_id, *position, comment.style));
                }
            }
        }

        assert_eq!(matches.len(), 1);
        let (owner_id, position, style) = matches[0];
        assert_eq!(owner_id, declaration_id.id);
        assert_eq!(position, AnnotationPosition::BlockInfix);
        assert_eq!(style, CommentStyle::Star);
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
    parser.finish_annotations();

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
    parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

    // Promise.all(...).then(...)
    let mut comment_owner_id: Option<u32> = None;
    let mut comment_annotation_id: Option<LocalNodeId<Annotation>> = None;
    for (node_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            if let Annotation::Comment { node, .. } = parser.tree.get(*annotation_id)
                && parser.strings.get(parser.tree.get::<Comment>(*node).string) == "TO DO -- END"
            {
                comment_owner_id = Some(*node_id);
                comment_annotation_id = Some(*annotation_id);
                break;
            }
        }
        if comment_owner_id.is_some() {
            break;
        }
    }

    // // TO DO -- END
    let comment_owner_id = comment_owner_id.expect("expected comment owner");
    let comment_annotation_id = comment_annotation_id.expect("expected comment annotation");
    assert_node!(
        parser.tree,
        comment_annotation_id,
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "TO DO -- END");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );

    // owner should be the `.then` member expression
    let owner_expression_id = LocalNodeId::<Expression>::new(comment_owner_id);
    assert_node!(parser.tree, owner_expression_id, Expression::Member { name, .. } => {
        assert_string!(parser, *name, "then");
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

    // blank between top level calls
    let mut blank_parent = None;
    for (node_id, annotations) in parser.tree.get_all_annotations() {
        let has_blank = annotations.iter().any(|annotation_id| {
            matches!(
                parser.tree.get::<Annotation>(*annotation_id),
                Annotation::Blank { .. }
            )
        });
        if has_blank {
            blank_parent = Some(*node_id);
            break;
        }
    }

    // second Promise.all(...)
    let blank_parent_id = blank_parent.expect("expected blank annotation");
    assert_eq!(blank_parent_id, expressions[1].id);
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
    parser.finish_annotations();

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
    parser.finish_annotations();

    assert_node!(parser.tree, block_id, Block { expressions, .. } => {
        let mut loop_statement_id = None;
        let mut let_after_loop_id = None;
        for expression_id in expressions {
            let source = parser.file.span_str(parser.tree.get_span(*expression_id));
            if source.starts_with("loop {") {
                loop_statement_id = Some(*expression_id);
            }
            if source.starts_with("let x = z()") {
                let_after_loop_id = Some(*expression_id);
            }
        }

        let loop_statement_id = loop_statement_id.expect("expected loop statement");
        let let_after_loop_id = let_after_loop_id.expect("expected let statement after loop");

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
        assert_node!(
            parser.tree,
            let_blank_prefix[0],
            Annotation::Blank { node, position } => {
                assert_eq!(*position, AnnotationPosition::BlockPrefix);
                assert_node!(parser.tree, *node, Blank { lines } => {
                    assert_eq!(*lines, 1);
                });
            }
        );
    });
}

/// Direct block parsing without finish pass should still attach blank prefixes inline.
#[test]
fn test_attach_blank_in_block_after_loop_before_let_statement_without_finish() {
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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

    let mut hop_a_target = None;
    let mut hop_b_target = None;
    for (target_node_id, target_annotations) in parser.tree.get_all_annotations() {
        for annotation_id in target_annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            if *position != AnnotationPosition::LinePostfix {
                continue;
            }
            let Comment { string, .. } = parser.tree.get(*node);
            let text = parser.strings.get(*string);
            if text == "hop-a" {
                hop_a_target = Some(*target_node_id);
            } else if text == "hop-b" {
                hop_b_target = Some(*target_node_id);
            }
        }
    }

    let hop_a_target = hop_a_target.expect("expected hop-a annotation target");
    let hop_b_target = hop_b_target.expect("expected hop-b annotation target");

    let hop_a_expression = LocalNodeId::<Expression>::new(hop_a_target);
    let hop_b_expression = LocalNodeId::<Expression>::new(hop_b_target);
    assert_expression_path!(parser, parser.tree.get(hop_a_expression), "source");
    assert_node!(parser.tree, hop_b_expression, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "source");
            assert_eq!(parser.strings.get(*name), "first");
        });
    });

    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { name, .. } => {
            assert_eq!(parser.strings.get(*name), "second");
        });
    });
}

/// Inline comments after operators attach to the following operand as line prefixes.
#[test]
fn test_attach_inline_comment_between_binary_operands() {
    let mut test = TestParser::new("a && /* keep */ b");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

    // foo(a)
    assert_node!(parser.tree, expr_id, Expression::Call { dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let argument_id = dynamic_arguments[0];
        let annotations = parser.tree.get_annotations(argument_id.id);

        // /* first */ a
        let annotation_owner_id = if annotations.is_empty() {
            let argument = parser.tree.get(argument_id);
            let value_id = match argument {
                Argument::Named { value, .. }
                | Argument::Labeled { value, .. }
                | Argument::Positional { value, .. }
                | Argument::Spread { value, .. } => *value,
            };
            value_id.id
        } else {
            argument_id.id
        };

        // /* first */
        let annotations = parser.tree.get_annotations(annotation_owner_id);
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
    parser.finish_annotations();

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

/// Parameter separator comments should attach to the parameter as prefixes.
#[test]
fn test_attach_parameter_separator_comment_to_parameter_prefix() {
    let mut test = TestParser::new("value /* parameter-type */: number");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    parser.finish_annotations();
    let annotations = parser.tree.get_annotations(parameter_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "parameter-type");
                assert_eq!(*style, CommentStyle::Star);
            });
        }
    );
}

/// Optional parameter separator comments should attach to the parameter as prefixes.
#[test]
fn test_attach_optional_parameter_separator_comment_to_parameter_prefix() {
    let mut test = TestParser::new("value? /* optional-parameter-type */: number");
    let mut parser = test.prepare();
    let parameter_id = parser.eat_parameter().unwrap();
    parser.finish_annotations();

    let annotations = parser.tree.get_annotations(parameter_id.id);
    assert_eq!(annotations.len(), 1);
    assert_node!(
        parser.tree,
        annotations[0],
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::LinePrefix);
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
    parser.finish_annotations();
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

/// Empty new-call boundary comments should stay attached to the callee expression.
#[test]
fn test_attach_empty_new_boundary_comment_to_callee_expression() {
    let mut test = TestParser::new("new require(/* new-boundary */)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::New { left, dynamic_arguments, .. } => {
            assert!(dynamic_arguments.is_empty());
            let annotations = parser.tree.get_annotations(left.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPostfix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_string!(parser, *string, "new-boundary");
                        assert_eq!(*style, CommentStyle::Star);
                    });
                }
            );
        }
    );
}

/// Empty call boundary comments should stay attached to the callee expression.
#[test]
fn test_attach_empty_call_boundary_comment_to_callee_expression() {
    let mut test = TestParser::new("target(/* call-boundary */)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { left, dynamic_arguments, .. } => {
            assert!(dynamic_arguments.is_empty());
            let annotations = parser.tree.get_annotations(left.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPostfix);
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
    parser.finish_annotations();

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

/// If-head trailing comments should stay attached to the condition in parse mode.
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
        assert_eq!(condition_annotations.len(), 1);
        assert_node!(
            parser.tree,
            condition_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "if-head");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            }
        );

        let then_annotations = parser.tree.get_annotations(then_expression.id);
        assert!(then_annotations.is_empty());

        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                let statement_id = parser.unwrap_statement_expression(expressions[0]);
                assert_node!(parser.tree, statement_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "run");
                });
            });
        });
    });
}

/// Direct `eat_if` entrypoints should keep if-head trailing comments after finish.
#[test]
fn test_attach_if_head_trailing_comment_on_direct_if_entrypoint() {
    let mut test = TestParser::new_with_options(
        r"if (ready) // if-head
    run()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let if_id = parser.eat_if().unwrap();
    parser.finish_annotations();

    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        let IfCondition::Expression { condition } = condition else {
            panic!("expected expression if condition");
        };

        let if_annotations = parser.tree.get_annotations(if_id.id);
        assert!(if_annotations.is_empty());

        let condition_annotations = parser.tree.get_annotations(condition.id);
        assert_eq!(condition_annotations.len(), 1);
        assert_node!(
            parser.tree,
            condition_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_string!(parser, *string, "if-head");
                    assert_eq!(*style, CommentStyle::Slash);
                });
            }
        );

        let then_annotations = parser.tree.get_annotations(then_expression.id);
        assert!(then_annotations.is_empty());

        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                let statement_id = parser.unwrap_statement_expression(expressions[0]);
                assert_node!(parser.tree, statement_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "run");
                });
            });
        });
    });
}

/// Infix seam comments after `as` should attach to the right type as a prefix.
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

                    let right_annotations = parser.tree.get_annotations(right.id);
                    assert_eq!(right_annotations.len(), 1, "expected one right-type annotation");
                    assert_node!(
                        parser.tree,
                        right_annotations[0],
                        Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::BlockPrefix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(*style, CommentStyle::Slash);
                                assert_string!(parser, *string, "as-tail");
                            });
                        }
                    );

                    let left_annotations = parser.tree.get_annotations(left.id);
                    assert!(
                        left_annotations.iter().all(|annotation_id| {
                            let Annotation::Comment { node, .. } = parser.tree.get(*annotation_id) else {
                                return true;
                            };
                            let comment = parser.tree.get::<Comment>(*node);
                            parser.strings.get(comment.string) != "as-tail"
                        }),
                        "as-tail should not attach to the left expression"
                    );

                    assert_expression_path!(parser, parser.tree.get(*left), "source");
                });
            });
        }
    );
}

/// Infix seam comments after `satisfies` should attach to the right type as a prefix.
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

                    let right_annotations = parser.tree.get_annotations(right.id);
                    assert_eq!(right_annotations.len(), 1, "expected one right-type annotation");
                    assert_node!(
                        parser.tree,
                        right_annotations[0],
                        Annotation::Comment { node, position } => {
                            assert_eq!(*position, AnnotationPosition::BlockPrefix);
                            assert_node!(parser.tree, *node, Comment { string, style } => {
                                assert_eq!(*style, CommentStyle::Slash);
                                assert_string!(parser, *string, "sat-tail");
                            });
                        }
                    );

                    let left_annotations = parser.tree.get_annotations(value_id.id);
                    assert!(
                        left_annotations.iter().all(|annotation_id| {
                            let Annotation::Comment { node, .. } = parser.tree.get(*annotation_id) else {
                                return true;
                            };
                            let comment = parser.tree.get::<Comment>(*node);
                            parser.strings.get(comment.string) != "sat-tail"
                        }),
                        "sat-tail should not attach to the full type-binary owner"
                    );
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
                assert!(
                    left_annotations.into_iter().all(|annotation_id| {
                        let Annotation::Comment { node, .. } = parser.tree.get(annotation_id) else {
                            return true;
                        };
                        let Comment { string, .. } = parser.tree.get(*node);
                        parser.strings.get(*string) != "between"
                    }),
                    "between should not attach to the left value expression"
                );
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
                assert!(
                    left_annotations.into_iter().all(|annotation_id| {
                        let Annotation::Comment { node, .. } = parser.tree.get(annotation_id) else {
                            return true;
                        };
                        let Comment { string, .. } = parser.tree.get(*node);
                        parser.strings.get(*string) != "sat-between"
                    }),
                    "sat-between should not attach to the left value expression"
                );
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

/// Line comments between `as` and `const` should attach as infix on the type unary node.
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
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Slash);
                    assert_string!(parser, *string, "before-const");
                });
            }
        );
    });
}

/// Multiline block comments between `as` and `const` should attach as infix on the type unary node.
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
                assert_eq!(*position, AnnotationPosition::BlockInfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "\nblock-comment\n");
                });
            }
        );
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

/// Ternary seam comments after `?` and `:` should attach to branch prefixes.
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
            assert_node!(parser.tree, value_id, Expression::If { then_expression, else_expression, .. } => {
                let else_expression_id = else_expression.expect("expected ternary else branch");

                let then_annotations = parser.tree.get_annotations(then_expression.id);
                assert_eq!(then_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    then_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Slash);
                            assert_string!(parser, *string, "then-seam");
                        });
                    }
                );

                let else_annotations = parser.tree.get_annotations(else_expression_id.id);
                assert_eq!(else_annotations.len(), 1);
                assert_node!(
                    parser.tree,
                    else_annotations[0],
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
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

/// Inline boundary line comments should attach to the following call argument.
#[test]
fn test_attach_call_argument_inline_boundary_comment_to_next_argument() {
    let mut test = TestParser::new(
        r"target(first, // call-argument-boundary
    second)",
    );
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();
    assert_node!(
        parser.tree,
        expression_id,
        Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
            let second_argument_id = dynamic_arguments[1];
            let annotations = parser.tree.get_annotations(second_argument_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(
                parser.tree,
                annotations[0],
                Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePrefix);
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
    parser.finish_annotations();

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

/// Inline boundary comments before direct calls should attach to the callee expression.
#[test]
fn test_attach_inline_comment_before_direct_call_parenthesis_to_callee() {
    let mut test = TestParser::new("target /* call-boundary */ (arg)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

    assert_node!(parser.tree, expression_id, Expression::Call { left, .. } => {
        let callee_annotations = parser.tree.get_annotations(left.id);
        assert_eq!(callee_annotations.len(), 1);
        assert_node!(
            parser.tree,
            callee_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "call-boundary");
                });
            }
        );
    });
}

/// Inline boundary comments before member call parentheses should stay on the full member callee.
#[test]
fn test_attach_inline_comment_before_member_call_parenthesis_to_member_callee() {
    let mut test = TestParser::new("items.map /* keep */ ((item) => item)");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

    assert_node!(parser.tree, expression_id, Expression::Call { left: callee_id, .. } => {
        let callee_id = *callee_id;
        assert_node!(parser.tree, callee_id, Expression::Path { path, .. } => {
            assert_path!(parser, *path, "items.map");
        });
        let callee_span = parser.tree.get_span(callee_id);
        let callee_source = parser.file.span_str(callee_span);
        assert_eq!(callee_source, "items.map");

        let callee_annotations = parser.tree.get_annotations(callee_id.id);
        assert_eq!(callee_annotations.len(), 1);
        assert_node!(
            parser.tree,
            callee_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfix);
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
    parser.finish_annotations();

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

/// Inline boundary comments before `new` call arguments should stay on the callee expression.
#[test]
fn test_attach_inline_comment_before_new_call_parenthesis_to_callee() {
    let mut test =
        TestParser::new_with_options("new Factory /* new-call */ (arg)", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

    assert_node!(parser.tree, expression_id, Expression::New { left, dynamic_arguments, .. } => {
        assert_eq!(dynamic_arguments.len(), 1);
        let callee_annotations = parser.tree.get_annotations(left.id);
        assert_eq!(callee_annotations.len(), 1);
        assert_node!(
            parser.tree,
            callee_annotations[0],
            Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePostfix);
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
    parser.finish_annotations();

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
                callee_annotations.len(),
                1,
                "call annotations: {}, maybe annotations: {}, callee annotations: {}",
                annotations.len(),
                maybe_annotations.len(),
                callee_annotations.len()
            );
            assert!(annotations.is_empty());
            assert!(maybe_annotations.is_empty());
            assert_node!(
                parser.tree,
                callee_annotations[0],
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
            assert!(statement_annotations.is_empty());

            assert_node!(parser.tree, *statement_id, Expression::Call { left, dynamic_arguments, .. } => {
                assert!(dynamic_arguments.is_empty());
                let call_annotations = parser.tree.get_annotations(statement_id.id);
                let maybe_annotations = parser.tree.get_annotations(left.id);
                let callee_annotations = assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                    parser.tree.get_annotations(left.id)
                });
                assert_eq!(
                    call_annotations.len() + maybe_annotations.len() + callee_annotations.len(),
                    1
                );

                let annotation = if !call_annotations.is_empty() {
                    call_annotations[0]
                } else if !maybe_annotations.is_empty() {
                    maybe_annotations[0]
                } else {
                    callee_annotations[0]
                };
                assert_node!(
                    parser.tree,
                    annotation,
                    Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "C4");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    }
                );
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
            assert_eq!(statement_annotations.len() + call_annotations.len(), 1);
            let annotation = if !statement_annotations.is_empty() {
                statement_annotations[0]
            } else {
                call_annotations[0]
            };
            assert_node!(
                parser.tree,
                annotation,
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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();
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
    parser.finish_annotations();
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
    parser.finish_annotations();

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
                assert!(
                    value_annotations.iter().all(|annotation_id| {
                        !matches!(
                            parser.tree.get::<Annotation>(*annotation_id),
                            Annotation::Comment { node, .. }
                                if parser.strings.get(parser.tree.get::<Comment>(*node).string)
                                    == "format-ignore-start"
                        )
                    }),
                    "format-ignore-start should not attach to argument value"
                );
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
                assert!(
                    value_annotations.iter().all(|annotation_id| {
                        !matches!(
                            parser.tree.get::<Annotation>(*annotation_id),
                            Annotation::Comment { node, .. }
                                if parser.strings.get(parser.tree.get::<Comment>(*node).string)
                                    == "format-ignore-end"
                        )
                    }),
                    "format-ignore-end should not attach to argument value"
                );
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

    // // @ts-expect-error keep spacing
    let mut expect_error_owners: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();
    // // @ts-ignore
    let mut ignore_owners: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();

    for (node_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get(*annotation_id) else {
                continue;
            };
            let comment_text = parser.strings.get(parser.tree.get::<Comment>(*node).string);
            if comment_text == "@ts-expect-error keep spacing" {
                expect_error_owners.push((*node_id, *annotation_id));
            } else if comment_text == "@ts-ignore" {
                ignore_owners.push((*node_id, *annotation_id));
            }
        }
    }

    assert_eq!(expect_error_owners.len(), 1);
    let (expect_error_owner, expect_error_annotation) = expect_error_owners[0];
    assert_eq!(expect_error_owner, expressions[0].id);
    assert_node!(
        parser.tree,
        expect_error_annotation,
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "@ts-expect-error keep spacing");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );

    assert_eq!(ignore_owners.len(), 1);
    let (ignore_owner, ignore_annotation) = ignore_owners[0];
    assert_eq!(ignore_owner, expressions[1].id);
    assert_node!(
        parser.tree,
        ignore_annotation,
        Annotation::Comment { node, position } => {
            assert_eq!(*position, AnnotationPosition::BlockPrefix);
            assert_node!(parser.tree, *node, Comment { string, style } => {
                assert_string!(parser, *string, "@ts-ignore");
                assert_eq!(*style, CommentStyle::Slash);
            });
        }
    );
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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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

/// TSX ternary alternate line comments should have exactly one owner: the alternate branch.
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
    parser.finish_annotations();

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
                    assert_eq!(then_annotations.len(), 0);

                    let else_expression_id = else_expression.expect("expected else branch");
                    let else_annotations = parser.tree.get_annotations(else_expression_id.id);
                    assert_eq!(else_annotations.len(), 1);
                    assert_node!(parser.tree, else_annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePrefix);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_eq!(*style, CommentStyle::Slash);
                            assert_string!(parser, *string, "alt-line");
                        });
                    });
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
    parser.finish_annotations();

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

/// Declaration header boundary comments should stay on declaration owners.
#[test]
fn test_attach_declaration_body_boundary_comment_on_declaration_owner() {
    let mut test = TestParser::new("class Value /* declaration-body */ {}");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.options).unwrap();
    parser.finish_annotations();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { .. } => {});

            // locate the declaration marker comment globally and verify owner and position
            let mut marker_matches = Vec::new();
            for (owner_id, annotations) in parser.tree.get_all_annotations() {
                for annotation_id in annotations {
                    let Annotation::Comment { node, position } = parser.tree.get(*annotation_id) else {
                        continue;
                    };
                    let comment = parser.tree.get::<Comment>(*node);
                    let comment_text = parser.strings.get(comment.string);
                    if comment_text == "declaration-body" {
                        marker_matches.push((*owner_id, *annotation_id, *position, comment.style));
                    }
                }
            }

            assert_eq!(marker_matches.len(), 1);
            let (owner_id, annotation_id, position, style) = marker_matches[0];
            assert_eq!(owner_id, declaration_id.id);
            assert_eq!(position, AnnotationPosition::BlockInfix);
            assert_eq!(style, CommentStyle::Star);
            assert_node!(parser.tree, annotation_id, Annotation::Comment { node, .. } => {
                assert_node!(parser.tree, *node, Comment { string, .. } => {
                    assert_string!(parser, *string, "declaration-body");
                });
            });
        }
    );
}

/// Method signature boundary comments should stay on method return or body owners.
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
    parser.finish_annotations();

    assert_node!(
        parser.tree,
        expression_id,
        Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { members, .. } => {
                assert_eq!(members.len(), 1);
                assert_node!(parser.tree, members[0], Member::Method { signature, body, .. } => {
                    let return_type = signature.return_type.expect("expected return type");
                    assert!(body.is_some(), "expected method body");
                    let annotations = parser.tree.get_annotations(return_type.id);
                    assert_eq!(annotations.len(), 1);
                    assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                        assert_node!(parser.tree, *node, Comment { string, style } => {
                            assert_string!(parser, *string, "method-body-boundary");
                            assert_eq!(*style, CommentStyle::Slash);
                        });
                    });
                });
            });
        }
    );
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
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                let return_type_id = signature.return_type.expect("expected return type");
                assert_expression_path!(parser, parser.tree.get(return_type_id), "Promise");
            });
        });
    });
    let mut comment_matches: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "return-tail" {
                comment_matches.push((*owner_id, *annotation_id));
            }
        }
    }

    assert_eq!(comment_matches.len(), 1, "expected one return-tail comment");
    let (owner_id, annotation_id) = comment_matches[0];
    let owner_expression_id = LocalNodeId::<Expression>::new(owner_id);
    assert_expression_path!(parser, parser.tree.get(owner_expression_id), "Promise");
    assert_node!(parser.tree, annotation_id, Annotation::Comment { node, position } => {
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
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface { members, .. } => {
            assert_eq!(members.len(), 1);
            assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    });

    let mut comment_matches: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "value-tail" {
                comment_matches.push((*owner_id, *annotation_id));
            }
        }
    }

    assert_eq!(comment_matches.len(), 1, "expected one value-tail comment");
    let (owner_id, annotation_id) = comment_matches[0];
    let owner_parameter_id = LocalNodeId::<Parameter>::new(owner_id);
    assert_node!(parser.tree, owner_parameter_id, Parameter::Named { name, .. } => {
        assert_string!(parser, *name, "value");
    });
    assert_node!(parser.tree, annotation_id, Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "value-tail");
        });
    });
}

/// Mapped-type value separator comments should stay attached to the mapped value prefix.
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
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeMapped { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Boolean) => {});
            });
        });
    });
    let mut comment_matches: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "mapped-line" {
                comment_matches.push((*owner_id, *annotation_id));
            }
        }
    }

    assert_eq!(comment_matches.len(), 1, "expected one mapped-line comment");
    let (owner_id, annotation_id) = comment_matches[0];
    let owner_expression_id = LocalNodeId::<Expression>::new(owner_id);
    assert_node!(parser.tree, owner_expression_id, Expression::TypeLiteral(TypeLiteral::Boolean) => {});
    assert_node!(parser.tree, annotation_id, Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
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
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type { value, .. } => {
            assert_node!(parser.tree, *value, Expression::TypeConditional { then_type, else_type, .. } => {
                assert_node!(parser.tree, *then_type, Expression::TypeLiteral(TypeLiteral::Number) => {});
                assert_node!(parser.tree, *else_type, Expression::TypeLiteral(TypeLiteral::Boolean) => {});
            });
        });
    });

    let mut true_note_match: Option<(u32, LocalNodeId<Annotation>)> = None;
    let mut false_note_match: Option<(u32, LocalNodeId<Annotation>)> = None;
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "true-note" {
                true_note_match = Some((*owner_id, *annotation_id));
            } else if comment_text == "false-note" {
                false_note_match = Some((*owner_id, *annotation_id));
            }
        }
    }

    let (true_owner_id, true_annotation_id) = true_note_match.expect("expected true-note comment");
    let true_owner_expression_id = LocalNodeId::<Expression>::new(true_owner_id);
    assert_node!(parser.tree, true_owner_expression_id, Expression::TypeLiteral(TypeLiteral::Number) => {});
    assert_node!(parser.tree, true_annotation_id, Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::LinePrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Star);
            assert_string!(parser, *string, "true-note");
        });
    });

    let (false_owner_id, false_annotation_id) =
        false_note_match.expect("expected false-note comment");
    let false_owner_expression_id = LocalNodeId::<Expression>::new(false_owner_id);
    assert_node!(parser.tree, false_owner_expression_id, Expression::TypeLiteral(TypeLiteral::Boolean) => {});
    assert_node!(parser.tree, false_annotation_id, Annotation::Comment { node, position } => {
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
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Interface { descriptor, .. } => {
            assert_eq!(descriptor.export, Some(destack_ast::DependencyMode::Item));
            assert_string!(parser, descriptor.name.unwrap().string(), "Shape");
        });
    });

    // locate the export marker comment and ensure it stays on the declaration boundary
    let mut export_head_matches = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "export-head" {
                export_head_matches.push((*owner_id, *position));
            }
        }
    }
    assert_eq!(export_head_matches.len(), 1);
    let (owner_id, position) = export_head_matches[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_eq!(owner_id, declaration_id.id);
    });
    assert_eq!(position, AnnotationPosition::LinePostfixBoundary);
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
    let (base_type_id, first_implements_type_id, second_implements_type_id, declaration_id) =
        match parser.tree.get(expression_id) {
            Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
                Declaration::Class { heritage, .. } => {
                    let extends_types = heritage.extends_types.as_ref().expect("expected extends");
                    let implements_types = heritage
                        .implements_types
                        .as_ref()
                        .expect("expected implements");
                    assert_eq!(extends_types.len(), 1);
                    assert_eq!(implements_types.len(), 2);
                    (
                        extends_types[0],
                        implements_types[0],
                        implements_types[1],
                        *declaration_id,
                    )
                }
                _ => panic!("expected class declaration"),
            },
            _ => panic!("expected declaration expression"),
        };

    let mut base_tail_matches: Vec<(u32, LocalNodeId<Annotation>, AnnotationPosition)> = Vec::new();
    let mut impl_head_matches: Vec<(u32, LocalNodeId<Annotation>, AnnotationPosition)> = Vec::new();
    let mut impl_tail_matches: Vec<(u32, LocalNodeId<Annotation>, AnnotationPosition)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "base-tail" {
                base_tail_matches.push((*owner_id, *annotation_id, *position));
            } else if comment_text == "impl-head" {
                impl_head_matches.push((*owner_id, *annotation_id, *position));
            } else if comment_text == "impl-tail" {
                impl_tail_matches.push((*owner_id, *annotation_id, *position));
            }
        }
    }

    assert_eq!(base_tail_matches.len(), 1, "expected one base-tail comment");
    assert_eq!(impl_head_matches.len(), 1, "expected one impl-head comment");
    assert_eq!(impl_tail_matches.len(), 1, "expected one impl-tail comment");

    let (base_owner_id, base_annotation_id, base_position) = base_tail_matches[0];
    assert_eq!(base_owner_id, base_type_id.id);
    assert_eq!(base_position, AnnotationPosition::LinePostfixBoundary);
    assert_node!(parser.tree, base_annotation_id, Annotation::Comment { node, .. } => {
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "base-tail");
        });
    });

    let (impl_head_owner_id, impl_head_annotation_id, impl_head_position) = impl_head_matches[0];
    assert_eq!(impl_head_owner_id, first_implements_type_id.id);
    assert_eq!(impl_head_position, AnnotationPosition::BlockPrefix);
    assert_node!(parser.tree, impl_head_annotation_id, Annotation::Comment { node, .. } => {
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-head");
        });
    });

    let (impl_tail_owner_id, impl_tail_annotation_id, impl_tail_position) = impl_tail_matches[0];
    assert_eq!(impl_tail_owner_id, second_implements_type_id.id);
    assert_eq!(impl_tail_position, AnnotationPosition::LinePostfixBoundary);
    assert_node!(parser.tree, impl_tail_annotation_id, Annotation::Comment { node, .. } => {
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-tail");
        });
    });

    // no declaration-level ownership for heritage seam comments
    let declaration_annotations = parser.tree.get_annotations(declaration_id.id);
    for annotation_id in declaration_annotations {
        assert_node!(parser.tree, annotation_id, Annotation::Comment { node, .. } => {
            let comment = parser.tree.get::<Comment>(*node);
            let text = parser.strings.get(comment.string);
            assert_ne!(text, "base-tail");
            assert_ne!(text, "impl-head");
            assert_ne!(text, "impl-tail");
        });
    }
}

/// Class extends-tail comments should stay attached to the superclass expression.
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
    let superclass_type_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Class { heritage, .. } => {
                let extends_types = heritage.extends_types.as_ref().expect("expected extends");
                assert_eq!(extends_types.len(), 1);
                extends_types[0]
            }
            _ => panic!("expected class declaration"),
        },
        _ => panic!("expected declaration expression"),
    };

    let mut extends_tail_matches: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "extends-tail" {
                assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                extends_tail_matches.push((*owner_id, *annotation_id));
            }
        }
    }

    assert_eq!(
        extends_tail_matches.len(),
        1,
        "expected one extends-tail comment"
    );
    let (owner_id, annotation_id) = extends_tail_matches[0];
    assert_eq!(owner_id, superclass_type_id.id);
    assert_node!(parser.tree, annotation_id, Annotation::Comment { node, .. } => {
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "extends-tail");
        });
    });
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
    let (first_implements_type_id, second_implements_type_id) = match parser.tree.get(expression_id)
    {
        Expression::Declaration(declaration_id) => match parser.tree.get(*declaration_id) {
            Declaration::Class { heritage, .. } => {
                let implements_types = heritage
                    .implements_types
                    .as_ref()
                    .expect("expected implements");
                assert_eq!(implements_types.len(), 2);
                (implements_types[0], implements_types[1])
            }
            _ => panic!("expected class declaration"),
        },
        _ => panic!("expected declaration expression"),
    };

    let mut impl_first_matches: Vec<(u32, LocalNodeId<Annotation>, AnnotationPosition)> =
        Vec::new();
    let mut impl_second_matches: Vec<(u32, LocalNodeId<Annotation>, AnnotationPosition)> =
        Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "impl-first" {
                impl_first_matches.push((*owner_id, *annotation_id, *position));
            } else if comment_text == "impl-second" {
                impl_second_matches.push((*owner_id, *annotation_id, *position));
            }
        }
    }

    assert_eq!(
        impl_first_matches.len(),
        1,
        "expected one impl-first comment"
    );
    assert_eq!(
        impl_second_matches.len(),
        1,
        "expected one impl-second comment"
    );

    let (impl_first_owner_id, impl_first_annotation_id, impl_first_position) =
        impl_first_matches[0];
    assert_eq!(impl_first_owner_id, second_implements_type_id.id);
    assert_eq!(impl_first_position, AnnotationPosition::BlockPrefix);
    assert_node!(parser.tree, impl_first_annotation_id, Annotation::Comment { node, .. } => {
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-first");
        });
    });

    let (impl_second_owner_id, impl_second_annotation_id, impl_second_position) =
        impl_second_matches[0];
    assert_eq!(impl_second_owner_id, second_implements_type_id.id);
    assert_eq!(
        impl_second_position,
        AnnotationPosition::LinePostfixBoundary
    );
    assert_node!(parser.tree, impl_second_annotation_id, Annotation::Comment { node, .. } => {
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "impl-second");
        });
    });

    // ensure the head type does not receive list-separator comments
    let first_annotations = parser.tree.get_annotations(first_implements_type_id.id);
    for annotation_id in first_annotations {
        assert_node!(parser.tree, annotation_id, Annotation::Comment { node, .. } => {
            let comment = parser.tree.get::<Comment>(*node);
            let text = parser.strings.get(comment.string);
            assert_ne!(text, "impl-first");
            assert_ne!(text, "impl-second");
        });
    }
}

/// Declare-class head comments before generics should stay on the declaration owner.
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
    let declaration_id = match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class { .. } => {});
            *declaration_id
        }
        _ => panic!("expected declaration expression"),
    };

    let mut box_head_matches: Vec<(u32, LocalNodeId<Annotation>, AnnotationPosition)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "box-head" {
                box_head_matches.push((*owner_id, *annotation_id, *position));
            }
        }
    }

    assert_eq!(box_head_matches.len(), 1, "expected one box-head comment");
    let (owner_id, annotation_id, position) = box_head_matches[0];
    assert_eq!(owner_id, declaration_id.id);
    assert_eq!(position, AnnotationPosition::BlockInfix);
    assert_node!(parser.tree, annotation_id, Annotation::Comment { node, .. } => {
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
fn test_attach_type_union_line_comment_after_separator_to_right_operand() {
    let mut test = TestParser::new_with_options(
        r"type Value = First | // union-line
Second | Third",
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

    let mut union_line_comment_owner_id: Option<u32> = None;
    let mut union_line_comment_annotation_id: Option<LocalNodeId<Annotation>> = None;
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "union-line" {
                union_line_comment_owner_id = Some(*owner_id);
                union_line_comment_annotation_id = Some(*annotation_id);
                break;
            }
        }
    }

    let union_line_comment_owner_id =
        union_line_comment_owner_id.expect("expected union line comment owner");
    let union_line_comment_annotation_id =
        union_line_comment_annotation_id.expect("expected union line comment annotation");
    assert_node!(parser.tree, union_line_comment_annotation_id, Annotation::Comment { node, position } => {
        assert_eq!(*position, AnnotationPosition::BlockPrefix);
        assert_node!(parser.tree, *node, Comment { string, style } => {
            assert_eq!(*style, CommentStyle::Slash);
            assert_string!(parser, *string, "union-line");
        });
    });

    let union_line_owner_expression_id =
        LocalNodeId::<Expression>::new(union_line_comment_owner_id);
    assert_expression_path!(
        parser,
        parser.tree.get(union_line_owner_expression_id),
        "Second"
    );
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
fn test_attach_type_intersection_line_comment_between_arms_to_right_operand_prefix() {
    let mut test = TestParser::new_with_options(
        r"type Value = First & // intersection-line
Second",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let mut match_result = None;
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let text = parser.strings.get(comment.string);
            if text == "intersection-line" {
                match_result = Some((*owner_id, *position));
            }
        }
    }

    let (owner_id, position) = match_result.expect("expected intersection-line annotation");
    let owner_expression_id = LocalNodeId::<Expression>::new(owner_id);
    assert_expression_path!(parser, parser.tree.get(owner_expression_id), "Second");
    assert_eq!(position, AnnotationPosition::BlockPrefix);
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
    parser.finish_annotations();

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
    let first_item_id = match parser.tree.get(expression_id) {
        Expression::Import { items, .. } => {
            assert_eq!(items.len(), 2);
            items[0]
        }
        _ => panic!("expected import expression"),
    };

    let mut comment_matches: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "first-spec" {
                comment_matches.push((*owner_id, *annotation_id));
            }
        }
    }

    assert_eq!(comment_matches.len(), 1);
    let (owner_id, annotation_id) = comment_matches[0];
    assert_eq!(owner_id, first_item_id.id);
    let owner_item_id = LocalNodeId::<DependencyItem>::new(owner_id);
    assert_node!(parser.tree, owner_item_id, DependencyItem { .. } => {});
    assert_node!(parser.tree, annotation_id, Annotation::Comment { node, position } => {
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
    let first_item_id = match parser.tree.get(expression_id) {
        Expression::Export { items, .. } => {
            assert_eq!(items.len(), 2);
            items[0]
        }
        _ => panic!("expected export expression"),
    };

    let mut comment_matches: Vec<(u32, LocalNodeId<Annotation>)> = Vec::new();
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, .. } = parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let comment_text = parser.strings.get(comment.string);
            if comment_text == "first-export" {
                comment_matches.push((*owner_id, *annotation_id));
            }
        }
    }

    assert_eq!(comment_matches.len(), 1);
    let (owner_id, annotation_id) = comment_matches[0];
    assert_eq!(owner_id, first_item_id.id);
    let owner_item_id = LocalNodeId::<DependencyItem>::new(owner_id);
    assert_node!(parser.tree, owner_item_id, DependencyItem { .. } => {});
    assert_node!(parser.tree, annotation_id, Annotation::Comment { node, position } => {
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
    let mut target_expression_id = None;
    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::Import { target, .. } => {
                if let ImportTarget::Expression { target } = target {
                    target_expression_id = Some(*target);
                }
            });
        });
    });
    let target_expression_id =
        target_expression_id.expect("expected dynamic import expression target");

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

    // let X = A && B
    assert_eq!(expressions.len(), 1);

    // A && B
    assert_node!(parser.tree, expressions[0], Expression::Statement(statement_id) => {
        assert_node!(parser.tree, *statement_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, right, operator } => {
                assert_eq!(*operator, BinaryOperator::And);

                // comment lookup
                let mut pre_a_comment_matches = Vec::new();
                let mut a_comment_matches = Vec::new();
                let mut b_comment_matches = Vec::new();
                for (owner_id, annotations) in parser.tree.get_all_annotations() {
                    for annotation_id in annotations {
                        let Annotation::Comment { node, position } = parser.tree.get(*annotation_id) else {
                            continue;
                        };
                        let comment = parser.tree.get::<Comment>(*node);
                        let comment_text = parser.strings.get(comment.string);
                        if comment_text == "Pre-A comment" {
                            pre_a_comment_matches.push((*owner_id, *position, comment.style));
                        } else if comment_text == "A comment" {
                            a_comment_matches.push((*owner_id, *position, comment.style));
                        } else if comment_text == "B comment" {
                            b_comment_matches.push((*owner_id, *position, comment.style));
                        }
                    }
                }

                assert_eq!(pre_a_comment_matches.len(), 1);
                assert_eq!(a_comment_matches.len(), 1);
                assert_eq!(b_comment_matches.len(), 1);

                let (pre_a_owner_id, pre_a_position, pre_a_style) = pre_a_comment_matches[0];
                let (a_owner_id, a_position, a_style) = a_comment_matches[0];
                let (b_owner_id, b_position, b_style) = b_comment_matches[0];

                assert_eq!(pre_a_owner_id, left.id);
                assert_eq!(pre_a_position, AnnotationPosition::LinePrefix);
                assert_eq!(pre_a_style, CommentStyle::Star);

                assert_eq!(a_owner_id, left.id);
                assert_eq!(a_position, AnnotationPosition::LinePostfix);
                assert_eq!(a_style, CommentStyle::Star);

                assert_eq!(b_owner_id, right.id);
                assert_eq!(b_position, AnnotationPosition::LinePostfixBoundary);
                assert_eq!(b_style, CommentStyle::Star);

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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
    parser.finish_annotations();

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

    let mut first_tail = None;
    let mut second_head = None;
    let mut third_tail = None;
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let text = parser.strings.get(comment.string);
            match text {
                "first-tail" => first_tail = Some((*owner_id, *position)),
                "second-head" => second_head = Some((*owner_id, *position)),
                "third-tail" => third_tail = Some((*owner_id, *position)),
                _ => {}
            }
        }
    }

    assert_eq!(
        first_tail,
        Some((first_element_id.id, AnnotationPosition::LinePostfixBoundary))
    );
    assert_eq!(
        second_head,
        Some((second_element_id.id, AnnotationPosition::LinePrefix))
    );
    assert_eq!(
        third_tail,
        Some((third_element_id.id, AnnotationPosition::LinePostfixBoundary))
    );
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
    let (first_property_id, second_property_id) = assert_node!(parser.tree, statement_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        let value = parser
            .tree
            .get(declarators[0])
            .value
            .expect("expected declarator value");
        assert_node!(parser.tree, value, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 2);
            (properties[0], properties[1])
        })
    });

    let mut first_tail = None;
    let mut second_tail = None;
    for (owner_id, annotations) in parser.tree.get_all_annotations() {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } =
                parser.tree.get::<Annotation>(*annotation_id)
            else {
                continue;
            };
            let comment = parser.tree.get::<Comment>(*node);
            let text = parser.strings.get(comment.string);
            match text {
                "first-tail" => first_tail = Some((*owner_id, *position)),
                "second-tail" => second_tail = Some((*owner_id, *position)),
                _ => {}
            }
        }
    }

    assert_eq!(
        first_tail,
        Some((
            first_property_id.id,
            AnnotationPosition::LinePostfixBoundary
        ))
    );
    assert_eq!(
        second_tail,
        Some((
            second_property_id.id,
            AnnotationPosition::LinePostfixBoundary
        ))
    );
}
