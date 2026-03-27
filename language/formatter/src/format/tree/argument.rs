use crate::format::analysis::{first_non_trivia_token_in_span, last_non_trivia_token_in_span};
use crate::format::chain::{
    chain_nodes, has_comment_between_expressions, member_has_intervening_comment,
    transparent_inner_expression,
};
use crate::format::collection::{
    collection_nodes_have_annotations, collection_value_should_force_break,
};
use crate::format::expression::{
    argument_value, is_complex_expression, is_expression_breakable, is_trivial_expression,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Argument, Declaration, Expression, FunctionKind, IfCondition, IfKind,
    LocalNodeId, NodeTree, Property, ScalarLiteral, TokenType,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, format_with, group, hard_line_break, line_postfix_boundary, soft_block_indent,
    space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

/// Return whether one annotation is one line slash comment.
fn annotation_id_is_line_slash_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
        return false;
    };
    if !matches!(
        position,
        AnnotationPosition::LinePrefix
            | AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
    ) {
        return false;
    }

    let comment = context.tree.get::<destack_ast::Comment>(node);
    comment.style == destack_ast::CommentStyle::Slash
}

/// Return whether one annotation is one comment annotation.
fn annotation_id_is_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    matches!(
        context.annotation(annotation_id),
        Annotation::Comment { .. }
    )
}

/// Return whether one annotation is one prefix comment or doc annotation.
fn annotation_id_is_prefix_comment_or_doc(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = context.annotation(annotation_id);
    if !matches!(
        annotation.position(),
        AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
    ) {
        return false;
    }

    matches!(
        annotation,
        Annotation::Comment { .. } | Annotation::Doc { .. }
    )
}

/// Return whether one argument node has at least one comment annotation.
fn argument_has_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context.any_annotation_id(argument_id, |annotation_id| {
        annotation_id_is_comment(context, annotation_id)
    })
}

/// Return whether one expression node has at least one comment annotation.
fn expression_has_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context.any_annotation_id(expression_id, |annotation_id| {
        annotation_id_is_comment(context, annotation_id)
    })
}

/// Return whether one tree argument or its value has a line slash comment annotation.
fn tree_argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    context.any_annotation_id(argument_id, |annotation_id| {
        annotation_id_is_line_slash_comment(context, annotation_id)
    }) || context.any_annotation_id(value_id, |annotation_id| {
        annotation_id_is_line_slash_comment(context, annotation_id)
    })
}

/// Return whether one ternary value has a line slash comment on any branch.
fn ternary_value_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::If {
        kind: IfKind::Ternary,
        condition,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(value_id)
    else {
        return false;
    };

    let condition_id = match condition {
        IfCondition::Expression { condition } => *condition,
        IfCondition::Let { .. } => return false,
    };

    let condition_has_line_comment = context.any_annotation_id(condition_id, |annotation_id| {
        annotation_id_is_line_slash_comment(context, annotation_id)
    });
    if condition_has_line_comment {
        return true;
    }

    let then_has_line_comment = context.any_annotation_id(*then_expression, |annotation_id| {
        annotation_id_is_line_slash_comment(context, annotation_id)
    });
    if then_has_line_comment {
        return true;
    }

    else_expression.is_some_and(|expression_id| {
        context.any_annotation_id(expression_id, |annotation_id| {
            annotation_id_is_line_slash_comment(context, annotation_id)
        })
    })
}

/// Return whether JSX argument formatting should force multiline mode.
pub(crate) fn has_multiline_jsx_argument(
    tree: &NodeTree,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if arguments.len() != 1 {
        return false;
    }

    let Some(value_id) = argument_value(tree, arguments[0]) else {
        return false;
    };

    // check if it's a JSX element with children
    if let Expression::TreeExpression { elements, .. } = tree.get(value_id) {
        elements.as_ref().is_some_and(|e| !e.is_empty())
    } else {
        false
    }
}

/// Return whether one tree argument source span is wrapped with `{ ... }`.
pub(crate) fn tree_argument_is_wrapped_in_braces(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let argument_span = context.span(argument_id);
    let starts_with_open_brace = first_non_trivia_token_in_span(context, argument_span)
        .is_some_and(|token| token.token.ty == TokenType::OpenBrace);
    let ends_with_close_brace = last_non_trivia_token_in_span(context, argument_span)
        .is_some_and(|token| token.token.ty == TokenType::CloseBrace);

    starts_with_open_brace && ends_with_close_brace
}

/// Format inline stub comments attached to one expression node.
fn format_inline_stub_expression_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Some(annotations) = f.context().annotations(expression_id) else {
        return Ok(false);
    };

    let mut first = true;
    for annotation_id in annotations {
        let Annotation::Comment { node, .. } = f.context().annotation(annotation_id) else {
            continue;
        };

        if !first {
            write!(f, [space()])?;
        }
        first = false;

        write!(f, [node])?;
    }

    Ok(!first)
}

/// Format inline stub comments attached to one argument node.
fn format_inline_stub_argument_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<bool> {
    let Some(annotations) = f.context().annotations(argument_id) else {
        return Ok(false);
    };

    let mut first = true;
    for annotation_id in annotations {
        let Annotation::Comment { node, .. } = f.context().annotation(annotation_id) else {
            continue;
        };

        if !first {
            write!(f, [space()])?;
        }
        first = false;

        write!(f, [node])?;
    }

    Ok(!first)
}

/// Collect comment annotation nodes from one annotation list.
fn collect_stub_comment_nodes(
    context: &DestackFormatContext<'_>,
    annotations: Option<&[LocalNodeId<Annotation>]>,
) -> Vec<LocalNodeId<destack_ast::Comment>> {
    let mut comment_nodes = Vec::new();
    let Some(annotations) = annotations else {
        return comment_nodes;
    };

    for annotation_id in annotations {
        let Annotation::Comment { node, .. } = context.annotation(*annotation_id) else {
            continue;
        };
        comment_nodes.push(node);
    }

    comment_nodes
}

/// Return whether one comment list contains at least one line comment.
fn stub_comment_nodes_have_line_comment(
    context: &DestackFormatContext<'_>,
    comment_nodes: &[LocalNodeId<destack_ast::Comment>],
) -> bool {
    comment_nodes.iter().any(|comment_id| {
        let comment = context.tree.get::<destack_ast::Comment>(*comment_id);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Format comment nodes one per line for multiline stub expression containers.
fn format_multiline_stub_comment_nodes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment_nodes: &[LocalNodeId<destack_ast::Comment>],
) -> FormatResult<()> {
    for (index, comment_id) in comment_nodes.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }
        write!(f, [*comment_id])?;
    }

    Ok(())
}

/// Resolve the token syntax style for one tree named attribute value.
fn tree_named_attribute_syntax_style(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> (bool, bool) {
    let argument_span = context.span(argument_id);
    let tokens = context.non_trivia_tokens_in_span(argument_span);
    let Some(assign_index) = tokens
        .iter()
        .position(|token| token.token.ty == TokenType::Assign)
    else {
        return (false, false);
    };

    if tokens
        .get(assign_index + 1)
        .is_some_and(|token| token.token.ty == TokenType::OpenBrace)
    {
        return (true, false);
    }

    (false, true)
}

/// Write one tree expression argument.
pub(crate) fn write_tree_expression_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    let argument = f.context().tree.get(argument_id);
    let argument_is_spread = matches!(argument, Argument::Spread { .. });
    let stub_value_id = match argument {
        Argument::Positional { value, .. }
            if matches!(f.context().tree.get(*value), Expression::Stub) =>
        {
            Some(*value)
        }
        _ => None,
    };

    // stub argument prefix comments and docs must stay inside `{ ... }`
    if stub_value_id.is_none() && !argument_is_spread {
        write!(f, [f.context().any_prefix_annotations(argument_id)])?;
    }

    let mut stub_argument_annotations_rendered_inline = false;
    match argument {
        Argument::Named { name, value, .. } => {
            // destack tree literals normalize boolean true and string literal attribute values
            if f.context().options.language_type.is_destack() {
                let value_expr = f.context().tree.get(*value);
                if let Expression::ScalarLiteral(ScalarLiteral::Boolean(true)) = value_expr {
                    write!(f, [name])?;
                } else {
                    write!(f, [name])?;
                    let is_string_literal = matches!(
                        value_expr,
                        Expression::ScalarLiteral(ScalarLiteral::String(_))
                            | Expression::ScalarLiteral(ScalarLiteral::Character(_))
                    );
                    if is_string_literal {
                        write!(f, [token("="), value])?;
                    } else {
                        format_tree_attribute_value(f, *value)?;
                    }
                }
            } else {
                // tsx and jsx preserve named attribute token syntax
                write!(f, [name])?;
                let (is_equals_braced, is_equals_unbraced) =
                    tree_named_attribute_syntax_style(f.context(), argument_id);
                if is_equals_braced {
                    format_tree_attribute_value(f, *value)?;
                } else if is_equals_unbraced {
                    write!(f, [token("="), value])?;
                }
            }
        }
        Argument::Labeled { label, value, .. } => {
            // label
            write!(f, [label])?;
            // value
            write!(f, [token(":"), space(), value])?;
        }
        Argument::Positional { value, .. } => {
            // in tree expressions, expression children need braces too
            let value_expr = f.context().tree.get(*value);
            let argument_is_braced = tree_argument_is_wrapped_in_braces(f.context(), argument_id);
            let force_multiline_braced_expression =
                tree_argument_has_line_comment_annotation(f.context(), argument_id, *value);
            let needs_braces = argument_is_braced
                || !matches!(
                    value_expr,
                    Expression::ScalarLiteral(ScalarLiteral::String(_))
                        | Expression::TreeExpression { .. }
                );
            if needs_braces {
                if matches!(value_expr, Expression::Stub) {
                    let keep_stub_prefix_inside_braces = stub_value_id.is_some_and(|value_id| {
                        f.context().any_annotation_id(argument_id, |annotation_id| {
                            annotation_id_is_prefix_comment_or_doc(f.context(), annotation_id)
                        }) || f.context().any_annotation_id(value_id, |annotation_id| {
                            annotation_id_is_prefix_comment_or_doc(f.context(), annotation_id)
                        })
                    });
                    if keep_stub_prefix_inside_braces {
                        write!(f, [token("{")])?;
                        write!(f, [f.context().any_prefix_annotations(argument_id)])?;
                        write!(f, [f.context().any_prefix_annotations(*value)])?;
                        write!(f, [token("}")])?;
                        stub_argument_annotations_rendered_inline = true;
                    } else {
                        let expression_comment_nodes = collect_stub_comment_nodes(
                            f.context(),
                            f.context().annotations(*value).as_deref(),
                        );
                        let argument_comment_nodes = collect_stub_comment_nodes(
                            f.context(),
                            f.context().annotations(argument_id).as_deref(),
                        );

                        let mut comment_nodes = expression_comment_nodes;
                        if comment_nodes.is_empty() {
                            comment_nodes = argument_comment_nodes;
                            if !comment_nodes.is_empty() {
                                stub_argument_annotations_rendered_inline = true;
                            }
                        }

                        if stub_comment_nodes_have_line_comment(f.context(), &comment_nodes) {
                            write!(
                                f,
                                [group(&format_args![
                                    token("{"),
                                    block_indent(&format_with(|f| {
                                        format_multiline_stub_comment_nodes(f, &comment_nodes)
                                    })),
                                    hard_line_break(),
                                    token("}")
                                ])]
                            )?;
                        } else {
                            write!(f, [token("{")])?;
                            let mut wrote_stub_comment =
                                format_inline_stub_expression_comments(f, *value)?;
                            if !wrote_stub_comment {
                                wrote_stub_comment =
                                    format_inline_stub_argument_comments(f, argument_id)?;
                                if wrote_stub_comment {
                                    stub_argument_annotations_rendered_inline = true;
                                }
                            }
                            write!(f, [token("}")])?;
                        }
                    }
                } else if force_multiline_braced_expression {
                    write!(
                        f,
                        [group(&format_args![
                            token("{"),
                            block_indent(&group(value).should_expand(true)),
                            hard_line_break(),
                            token("}")
                        ])]
                    )?;
                } else {
                    // keep jsx expression containers inline for common expression forms
                    if tree_child_should_inline_braced_expression(f.context(), argument_id) {
                        write!(f, [token("{"), value, token("}")])?;
                    } else if expression_has_chain_seam_comment(f.context(), *value)
                        || ternary_value_has_line_comment_annotation(f.context(), *value)
                    {
                        write!(
                            f,
                            [group(&format_args![
                                token("{"),
                                group(value).should_expand(true),
                                token("}")
                            ])]
                        )?;
                    } else {
                        write!(
                            f,
                            [group(&format_args![
                                token("{"),
                                soft_block_indent(&value),
                                token("}")
                            ])]
                        )?;
                    }
                }
            } else {
                write!(f, [value])?;
            }
        }
        Argument::Spread { value, .. } => {
            // keep spread-head annotations inside `{ ... }` like prettier and oxc
            let has_spread_comment_annotation =
                argument_has_comment_annotation(f.context(), argument_id)
                    || expression_has_comment_annotation(f.context(), *value);
            let spread_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [f.context().any_prefix_annotations(argument_id)])?;
                write!(f, [token("..."), value])
            });

            if has_spread_comment_annotation {
                write!(
                    f,
                    [group(&format_args![
                        token("{"),
                        soft_block_indent(&spread_inner),
                        line_postfix_boundary(),
                        token("}")
                    ])]
                )?;
            } else {
                write!(
                    f,
                    [
                        token("{"),
                        spread_inner,
                        line_postfix_boundary(),
                        token("}")
                    ]
                )?;
            }
        }
        Argument::Error => {
            write!(f, [token("{"), token("/* ERROR */"), token("}")])?;
        }
    }

    if !stub_argument_annotations_rendered_inline {
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(argument_id)]
        )?;
    }

    Ok(())
}

/// Get the value expression for any tree attribute argument variant.
pub(crate) fn tree_attribute_value_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => Some(*value),
        Argument::Error => None,
    }
}

/// Return an argument value expression with transparent wrappers removed.
pub(crate) fn argument_transparent_value_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    tree_attribute_value_id(context.tree, argument_id)
        .map(|value_id| transparent_inner_expression(context, value_id))
}

/// Return a function declaration id from an expression when present.
pub(crate) fn expression_function_declaration_id(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Declaration>> {
    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return None;
    };

    if matches!(tree.get(*declaration_id), Declaration::Function { .. }) {
        Some(*declaration_id)
    } else {
        None
    }
}

/// Return whether a function declaration is a lambda.
pub(crate) fn declaration_is_lambda(
    tree: &NodeTree,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    matches!(
        tree.get(declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

/// Return a lambda body expression id with transparent wrappers removed.
pub(crate) fn lambda_body_expression_id(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> Option<LocalNodeId<Expression>> {
    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = context.tree.get(declaration_id)
    else {
        return None;
    };
    if signature.kind != FunctionKind::Lambda {
        return None;
    }

    Some(transparent_inner_expression(context, *body_id))
}

/// Return a lambda declaration id from an argument when present.
pub(crate) fn argument_lambda_declaration_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Declaration>> {
    let value_id = argument_transparent_value_id(context, argument_id)?;
    let declaration_id = expression_function_declaration_id(context.tree, value_id)?;
    declaration_is_lambda(context.tree, declaration_id).then_some(declaration_id)
}

/// Return the receiver of a postfix-like expression node.
pub(crate) fn expression_postfix_receiver_id(
    expression: &Expression,
) -> Option<LocalNodeId<Expression>> {
    match expression {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::Parenthesized { expression: left }
        | Expression::Statement(left) => Some(*left),
        _ => None,
    }
}

pub(crate) fn property_has_complex_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    // annotations on the property force complexity
    if context.has_annotation(property_id) {
        return true;
    }

    let property = tree.get(property_id);

    // field values and defaults can be complex
    if let Property::Field { value, default, .. } = property {
        // inspect the field value
        let value_is_complex = value.is_some_and(|value_id| {
            let value_expr = tree.get(value_id);
            is_complex_expression(tree, value_expr) || context.has_annotation(value_id)
        });

        // inspect the field default
        let default_is_complex = default.is_some_and(|default_id| {
            let default_expr = tree.get(default_id);
            is_complex_expression(tree, default_expr) || context.has_annotation(default_id)
        });

        return value_is_complex || default_is_complex;
    }

    // methods with bodies are always complex in object literals
    if let Property::Method { body, .. } = property {
        return body.is_some();
    }

    // spread properties inherit complexity from their value
    if let Property::Spread { value, .. } = property {
        let value_expr = tree.get(*value);
        return is_complex_expression(tree, value_expr) || context.has_annotation(*value);
    }

    false
}

/// Check whether a property contains a complex type value.
pub(crate) fn property_has_complex_type_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    if context.has_annotation(property_id) {
        return true;
    }

    let is_complex_type_expression = |expression_id: LocalNodeId<Expression>| {
        let expression = tree.get(expression_id);
        context.has_annotation(expression_id)
            || is_expression_breakable(tree, expression)
            || !is_trivial_expression(tree, expression)
    };

    match tree.get(property_id) {
        Property::Field { value, default, .. } => {
            value.is_some_and(is_complex_type_expression)
                || default.is_some_and(is_complex_type_expression)
        }
        Property::Method { body, .. } => body.is_some(),
        Property::Spread { value, .. } => is_complex_type_expression(*value),
        Property::Error => true,
    }
}

/// Decide whether tree attributes should force the element to break.
pub(crate) fn should_force_break_tree_attributes(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    let tree = context.tree;

    // comments on attributes force a break
    if collection_nodes_have_annotations(context, arguments) {
        return true;
    }

    for argument_id in arguments {
        let Some(value_id) = tree_attribute_value_id(tree, *argument_id) else {
            continue;
        };

        // comments on attribute values force a break
        if context.has_annotation(value_id) {
            return true;
        }

        // collect value signals for complexity checks
        let value_expr = tree.get(value_id);

        // callback-rich expression values should break the opening tag
        if expression_has_complex_callback(context, value_id) {
            return true;
        }

        // complex object and array values should break the element
        match value_expr {
            Expression::ObjectExpression { properties, .. } => {
                // collect object signals
                let has_many_properties = properties.len() > 1;
                let has_complex_property = properties
                    .iter()
                    .copied()
                    .any(|property_id| property_has_complex_value(context, property_id));

                // break when the object is clearly complex
                let should_break_object =
                    collection_value_should_force_break(has_many_properties, has_complex_property);

                if should_break_object {
                    return true;
                }
            }
            Expression::ArrayExpression { elements } => {
                // collect array signals
                let has_many_elements = elements.len() > 1;
                let has_complex_element = elements.iter().any(|element_id| {
                    // annotations on the element force complexity
                    let element_has_annotation = context.has_annotation(*element_id);

                    // inspect the element value when present
                    let element_value_is_complex = tree_attribute_value_id(tree, *element_id)
                        .is_some_and(|element_value_id| {
                            let element_expr = tree.get(element_value_id);
                            is_complex_expression(tree, element_expr)
                                || context.has_annotation(element_value_id)
                        });

                    element_has_annotation || element_value_is_complex
                });

                // break when the array is clearly complex
                let should_break_array =
                    collection_value_should_force_break(has_many_elements, has_complex_element);

                if should_break_array {
                    return true;
                }
            }
            Expression::TreeExpression { elements, .. } => {
                // nested trees with children force a break
                let has_children = elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty());

                if has_children {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

/// Format a tree or JSX attribute value.
pub(crate) fn format_tree_attribute_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [token("="), token("{"), value_id, token("}")])
}

/// Check whether a tree text child is whitespace-only.
pub(crate) fn tree_text_is_whitespace_only(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let has_annotation = context.has_annotation(argument_id) || context.has_annotation(*value);
    let wrapped_in_braces = tree_argument_is_wrapped_in_braces(context, argument_id);
    if wrapped_in_braces && has_annotation {
        return Some((false, false));
    }

    match tree.get(*value) {
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            let content = strings.get(*string_id);
            let has_non_whitespace = content
                .chars()
                .any(|character| !is_jsx_whitespace_char(character));
            if has_non_whitespace {
                return Some((false, false));
            }

            let has_newline = content.contains(['\n', '\r']);
            Some((true, has_newline))
        }
        Expression::ScalarLiteral(ScalarLiteral::Character(value)) => {
            if !is_jsx_whitespace_char(*value) {
                return Some((false, false));
            }

            let has_newline = matches!(value, '\n' | '\r');
            Some((true, has_newline))
        }
        _ => None,
    }
}

/// Return whether one character is JSX whitespace.
#[inline]
pub(crate) fn is_jsx_whitespace_char(character: char) -> bool {
    matches!(character, ' ' | '\n' | '\r' | '\t')
}

/// Return whether source preserves an empty line between two tree child arguments.
pub(crate) fn tree_children_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    previous_argument_id: LocalNodeId<Argument>,
    next_argument_id: LocalNodeId<Argument>,
) -> bool {
    let previous_span = context.span(previous_argument_id);
    let next_span = context.span(next_argument_id);
    let Some(between_span) = previous_span.gap_to(next_span) else {
        return false;
    };
    context.has_blank_line(between_span)
}

/// Check whether a tree child expression should stay inline inside `{ ... }`.
pub(crate) fn tree_child_should_inline_braced_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expr = context.tree.get(value_id);
    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);

    if argument_span.file == value_span.file {
        if argument_span.start < value_span.start
            && context.has_comment(Span::new(
                argument_span.file,
                argument_span.start,
                value_span.start,
            ))
        {
            return false;
        }

        if value_span.end < argument_span.end
            && context.has_comment(Span::new(
                argument_span.file,
                value_span.end,
                argument_span.end,
            ))
        {
            return false;
        }
    }

    if argument_has_line_comment_annotation(context, argument_id) {
        return false;
    }

    match value_expr {
        Expression::ScalarLiteral(ScalarLiteral::String(_))
        | Expression::ScalarLiteral(ScalarLiteral::Character(_)) => true,
        Expression::ArrayExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::Call { .. }
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Binary { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => {
            !expression_has_line_comment_annotation(context, value_id)
                && !expression_has_chain_seam_comment(context, value_id)
        }
        Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            let has_parenthesized_branch = matches!(
                context.tree.get(*then_expression),
                Expression::Parenthesized { .. }
            ) || else_expression.is_some_and(|else_id| {
                matches!(context.tree.get(else_id), Expression::Parenthesized { .. })
            });
            let has_branch_prefix_star_comment =
                expression_chain_has_prefix_star_comment_annotation(context, *then_expression)
                    || else_expression.is_some_and(|else_id| {
                        expression_chain_has_prefix_star_comment_annotation(context, else_id)
                    });
            if has_parenthesized_branch && has_branch_prefix_star_comment {
                return false;
            }

            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => return false,
            };
            if expression_has_line_comment_annotation(context, value_id)
                || expression_has_line_comment_annotation(context, condition_id)
                || expression_has_line_comment_annotation(context, *then_expression)
                || else_expression
                    .is_some_and(|else_id| expression_has_line_comment_annotation(context, else_id))
            {
                return false;
            }
            true
        }
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        ),
        _ => false,
    }
}

/// Return whether one expression chain has comments on member or operator seams.
pub(crate) fn expression_has_chain_seam_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let chain = chain_nodes(context.tree, expression_id);
    if chain.len() <= 1 {
        return false;
    }

    if chain
        .windows(2)
        .any(|adjacent| has_comment_between_expressions(context, adjacent[0], adjacent[1]))
    {
        return true;
    }

    chain
        .iter()
        .copied()
        .any(|chain_node_id| member_has_intervening_comment(context, chain_node_id))
}

/// Return whether one expression has a line-oriented slash comment annotation.
fn expression_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context.any_annotation_id(expression_id, |annotation_id| {
        annotation_id_is_line_slash_comment(context, annotation_id)
    })
}

/// Return whether one expression has one prefix block-star comment annotation.
fn expression_has_prefix_star_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context.any_annotation_id(expression_id, |annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            return false;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Star
    })
}

/// Return whether one expression or its parenthesized inner chain has one prefix block-star comment.
fn expression_chain_has_prefix_star_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_expression_id = expression_id;
    loop {
        if expression_has_prefix_star_comment_annotation(context, current_expression_id) {
            return true;
        }

        let Expression::Parenthesized { expression } = context.tree.get(current_expression_id)
        else {
            return false;
        };
        current_expression_id = *expression;
    }
}

/// Return whether one tree argument has a line-oriented slash comment annotation.
fn argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context.any_annotation_id(argument_id, |annotation_id| {
        annotation_id_is_line_slash_comment(context, annotation_id)
    })
}

/// Return whether one ternary expression has any line-comment annotations on its branches.
fn ternary_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::If {
        kind: IfKind::Ternary,
        condition,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(expression_id)
    else {
        return false;
    };

    let condition_id = match condition {
        IfCondition::Expression { condition } => *condition,
        IfCondition::Let { .. } => return true,
    };

    expression_has_line_comment_annotation(context, expression_id)
        || expression_has_line_comment_annotation(context, condition_id)
        || expression_has_line_comment_annotation(context, *then_expression)
        || else_expression
            .is_some_and(|else_id| expression_has_line_comment_annotation(context, else_id))
}

/// Return whether a ternary expression has parenthesized then or else branches.
fn ternary_has_parenthesized_branch(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::If {
        kind: IfKind::Ternary,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(expression_id)
    else {
        return false;
    };

    matches!(
        context.tree.get(*then_expression),
        Expression::Parenthesized { .. }
    ) || else_expression.is_some_and(|else_id| {
        matches!(context.tree.get(else_id), Expression::Parenthesized { .. })
    })
}

/// Check whether a tree child forces the element to break.
pub(crate) fn tree_child_breaks_element(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expr = context.tree.get(value_id);
    let is_text_node = matches!(
        value_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    );
    let has_line_comment_annotation = argument_has_line_comment_annotation(context, argument_id)
        || expression_has_line_comment_annotation(context, value_id);
    if has_line_comment_annotation {
        return true;
    }

    if (context.has_annotation(argument_id) || context.has_annotation(value_id))
        && !is_text_node
        && !matches!(value_expr, Expression::Stub)
    {
        if matches!(
            value_expr,
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        ) {
            return ternary_has_line_comment_annotation(context, value_id)
                || ternary_has_parenthesized_branch(context, value_id);
        }

        return true;
    }

    match value_expr {
        Expression::Stub => context.options.language_type.is_destack(),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => {
            ternary_has_line_comment_annotation(context, value_id)
                || ternary_has_parenthesized_branch(context, value_id)
        }
        Expression::Block(_) | Expression::Match { .. } => true,
        Expression::Declaration(declaration_id) => {
            lambda_body_is_complex_for_tree(context, *declaration_id)
        }
        Expression::TreeExpression { .. } => false,
        _ => expression_has_complex_callback(context, value_id),
    }
}

/// Check whether a lambda body is complex enough to force tree breaking.
pub(crate) fn lambda_body_is_complex_for_tree(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Some(body_id) = lambda_body_expression_id(context, declaration_id) else {
        return false;
    };

    matches!(
        context.tree.get(body_id),
        Expression::Block(_) | Expression::TreeExpression { .. }
    )
}

/// Check whether an argument is a lambda with a complex body for tree literals.
pub(crate) fn argument_is_complex_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(declaration_id) = argument_lambda_declaration_id(context, argument_id) else {
        return false;
    };

    lambda_body_is_complex_for_tree(context, declaration_id)
}

/// Check whether an argument is a lambda with a block body.
pub(crate) fn argument_is_block_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(declaration_id) = argument_lambda_declaration_id(context, argument_id) else {
        return false;
    };

    lambda_body_expression_id(context, declaration_id)
        .is_some_and(|body_id| matches!(context.tree.get(body_id), Expression::Block(_)))
}

/// Check whether an argument is an object literal expression.
pub(crate) fn argument_is_object_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::ObjectExpression { .. }
        )
    })
}

/// Check whether an argument is an array literal expression.
pub(crate) fn argument_is_array_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::ArrayExpression { .. }
        )
    })
}

/// Check whether an argument is a template literal expression.
pub(crate) fn argument_is_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::TemplateExpression { .. }
        )
    })
}

/// Check whether an expression contains a call with a complex callback.
pub(crate) fn expression_has_complex_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    // unwrap transparent wrappers
    let expression_id = transparent_inner_expression(context, expression_id);

    // walk the expression shape looking for callback lambdas
    match tree.get(expression_id) {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => {
            // check the call arguments
            let has_complex_argument = dynamic_arguments
                .iter()
                .any(|arg_id| argument_is_complex_callback(context, *arg_id));

            if has_complex_argument {
                return true;
            }

            // check chained receivers
            expression_has_complex_callback(context, *left)
        }
        expression if let Some(left) = expression_postfix_receiver_id(expression) => {
            expression_has_complex_callback(context, left)
        }
        Expression::Declaration(declaration_id) => {
            lambda_body_is_complex_for_tree(context, *declaration_id)
        }
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);

            // scan block expressions for complex callbacks
            block
                .expressions
                .iter()
                .any(|expr_id| expression_has_complex_callback(context, *expr_id))
        }
        _ => false,
    }
}
