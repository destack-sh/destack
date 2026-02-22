use crate::format::analysis::scan::{
    first_non_trivia_token_in_span, last_non_trivia_token_in_span,
};
use crate::format::collection::{
    collection_nodes_have_annotations, collection_value_should_force_break,
};
use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, Declaration, DestackFormatContext, DestackFormatter,
    Expression, FormatResult, FunctionKind, HugOptions, IfCondition, IfKind, LocalNodeId, NodeTree,
    TokenType, argument_value, block_indent, format_with, group, hard_line_break, if_group_breaks,
    is_complex_expression, is_expression_breakable, is_trivial_expression,
    lambda_expression_should_break, soft_block_indent, soft_line_break_or_space, space, token,
    transparent_inner_expression,
};
use crate::format::tree::children::{
    expression_has_chain_seam_comment, tree_child_should_inline_braced_expression,
};
use destack_ast::{Property, ScalarLiteral};
use destack_fir::format::{Buffer, Format, GroupId};
use destack_fir::prelude::expand_parent;
use destack_fir::{format_args, write};

/// Return whether one annotation set contains a line slash comment.
fn annotation_ids_have_line_slash_comment(
    context: &DestackFormatContext<'_>,
    annotation_ids: &[LocalNodeId<Annotation>],
) -> bool {
    annotation_ids.iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(*annotation_id) else {
            return false;
        };
        let is_line_position = matches!(
            position,
            AnnotationPosition::LinePrefix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        );
        if !is_line_position {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return whether one tree argument or its value has a line slash comment annotation.
fn tree_argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    context
        .annotations(argument_id)
        .is_some_and(|annotation_ids| {
            annotation_ids_have_line_slash_comment(context, &annotation_ids)
        })
        || context.annotations(value_id).is_some_and(|annotation_ids| {
            annotation_ids_have_line_slash_comment(context, &annotation_ids)
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

    let condition_has_line_comment =
        context
            .annotations(condition_id)
            .is_some_and(|annotation_ids| {
                annotation_ids_have_line_slash_comment(context, &annotation_ids)
            });
    if condition_has_line_comment {
        return true;
    }

    let then_has_line_comment =
        context
            .annotations(*then_expression)
            .is_some_and(|annotation_ids| {
                annotation_ids_have_line_slash_comment(context, &annotation_ids)
            });
    if then_has_line_comment {
        return true;
    }

    else_expression.is_some_and(|expression_id| {
        context
            .annotations(expression_id)
            .is_some_and(|annotation_ids| {
                annotation_ids_have_line_slash_comment(context, &annotation_ids)
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

/// Tree expression argument, using `=` for named arguments.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TreeExpressionArgument {
    pub(crate) argument_id: LocalNodeId<Argument>,
}

/// The token syntax style for one tree named attribute value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreeNamedAttributeSyntaxStyle {
    /// The attribute is shorthand: `<X disabled />`.
    Shorthand,
    /// The attribute uses equals with a non-braced value: `<X title="ok" />`.
    EqualsUnbraced,
    /// The attribute uses equals with a braced expression: `<X title={"ok"} />`.
    EqualsBraced,
}

/// Return whether one token kind is ignorable when recovering tree attribute syntax.
#[inline]
fn is_ignored_tree_attribute_syntax_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Whitespace
            | TokenType::Newline
            | TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
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

/// Resolve the token syntax style for one tree named attribute value.
fn tree_named_attribute_syntax_style(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> TreeNamedAttributeSyntaxStyle {
    let argument_span = context.span(argument_id);
    let tokens = context.tokens;
    let mut token_index = tokens.partition_point(|token| token.span.end <= argument_span.start);
    let mut saw_equals = false;

    while let Some(token) = tokens.get(token_index).copied() {
        if token.span.start >= argument_span.end {
            break;
        }
        token_index += 1;
        if is_ignored_tree_attribute_syntax_token(token.token.ty) {
            continue;
        }

        if token.token.ty != TokenType::Assign {
            continue;
        }

        saw_equals = true;
        break;
    }

    if !saw_equals {
        return TreeNamedAttributeSyntaxStyle::Shorthand;
    }

    while let Some(token) = tokens.get(token_index).copied() {
        if token.span.start >= argument_span.end {
            break;
        }
        token_index += 1;
        if is_ignored_tree_attribute_syntax_token(token.token.ty) {
            continue;
        }

        if token.token.ty == TokenType::OpenBrace {
            return TreeNamedAttributeSyntaxStyle::EqualsBraced;
        }
        return TreeNamedAttributeSyntaxStyle::EqualsUnbraced;
    }

    TreeNamedAttributeSyntaxStyle::EqualsUnbraced
}

impl<'ast> Format<DestackFormatContext<'ast>> for TreeExpressionArgument {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(self.argument_id)])?;

        let argument = f.context().tree.get(self.argument_id);
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
                    let value_style =
                        tree_named_attribute_syntax_style(f.context(), self.argument_id);
                    match value_style {
                        TreeNamedAttributeSyntaxStyle::Shorthand => {}
                        TreeNamedAttributeSyntaxStyle::EqualsUnbraced => {
                            write!(f, [token("="), value])?;
                        }
                        TreeNamedAttributeSyntaxStyle::EqualsBraced => {
                            format_tree_attribute_value(f, *value)?;
                        }
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
                let argument_is_braced =
                    tree_argument_is_wrapped_in_braces(f.context(), self.argument_id);
                let force_multiline_braced_expression = tree_argument_has_line_comment_annotation(
                    f.context(),
                    self.argument_id,
                    *value,
                );
                let needs_braces = argument_is_braced
                    || !matches!(
                        value_expr,
                        Expression::ScalarLiteral(ScalarLiteral::String(_))
                            | Expression::TreeExpression { .. }
                    );
                if needs_braces {
                    if matches!(value_expr, Expression::Stub) {
                        write!(f, [token("{")])?;
                        let mut wrote_stub_comment =
                            format_inline_stub_expression_comments(f, *value)?;
                        if !wrote_stub_comment {
                            wrote_stub_comment =
                                format_inline_stub_argument_comments(f, self.argument_id)?;
                            if wrote_stub_comment {
                                stub_argument_annotations_rendered_inline = true;
                            }
                        }
                        write!(f, [token("}")])?;
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
                        if tree_child_should_inline_braced_expression(f.context(), self.argument_id)
                        {
                            write!(f, [token("{"), value, token("}")])?;
                        } else if expression_has_chain_seam_comment(f.context(), *value) {
                            write!(
                                f,
                                [group(&format_args![
                                    token("{"),
                                    group(value).should_expand(true),
                                    token("}")
                                ])]
                            )?;
                        } else if ternary_value_has_line_comment_annotation(f.context(), *value) {
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
                // spread in jsx needs braces: {...props}
                write!(f, [token("{"), token("..."), value, token("}")])?;
            }
        }

        if !stub_argument_annotations_rendered_inline {
            write!(
                f,
                [f.context()
                    .any_infix_or_postfix_annotations(self.argument_id)]
            )?;
        }

        Ok(())
    }
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

        // preserve explicit multiline attribute values
        let value_span = context.span(value_id);
        if context.has_newline(value_span) {
            return true;
        }

        // collect value signals for complexity checks
        let value_expr = tree.get(value_id);

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

/// Check if an expression is huggable with the given configuration.
/// Return whether an expression is huggable in JSX position.
#[inline]
fn format_tree_attribute_inline_or_hugged<'ast, InlineDoc, HuggedDoc>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    inline_format: InlineDoc,
    hugged_format: HuggedDoc,
) -> FormatResult<()>
where
    InlineDoc: Format<DestackFormatContext<'ast>>,
    HuggedDoc: Format<DestackFormatContext<'ast>>,
{
    // context-based default selection
    if f.context().has_annotation(value_id) {
        f.context()
            .increment_counter("profile.jsx.attribute.by_context.hug", 1);
        hugged_format.format(f)?;
    } else {
        f.context()
            .increment_counter("profile.jsx.attribute.by_context.inline", 1);
        inline_format.format(f)?;
    }

    Ok(())
}

/// Format a tree attribute object value using inline-or-hugged selection.
fn format_tree_attribute_object_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    ty: Option<LocalNodeId<Expression>>,
    properties: Vec<LocalNodeId<Property>>,
) -> FormatResult<()> {
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("="), token("{"), value_id, token("}")])
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [
                token("="),
                token("{"),
                format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if let Some(ty) = ty {
                        write!(f, [ty, space()])?;
                    }
                    write!(
                        f,
                        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(
                                f,
                                [
                                    token("{"),
                                    block_indent(&format_with(
                                        |f: &mut DestackFormatter<'ast, '_>| {
                                            f.join_with(&format_args![
                                                token(","),
                                                soft_line_break_or_space()
                                            ])
                                            .entries(&properties)
                                            .finish()?;
                                            write!(f, [if_group_breaks(&token(","))])
                                        }
                                    )),
                                    token("}")
                                ]
                            )
                        }))
                        .should_expand(true)]
                    )
                }),
                token("}")
            ]
        )
    });

    format_tree_attribute_inline_or_hugged(f, value_id, inline_format, hugged_format)
}

/// Format a tree attribute array value using inline-or-hugged selection.
fn format_tree_attribute_array_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    elements: Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("="), token("{"), value_id, token("}")])
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [
                token("="),
                token("{"),
                group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(
                        f,
                        [
                            token("["),
                            block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                f.join_with(&format_args![token(","), soft_line_break_or_space()])
                                    .entries(&elements)
                                    .finish()?;
                                write!(f, [if_group_breaks(&token(","))])
                            })),
                            token("]")
                        ]
                    )
                }))
                .should_expand(true),
                token("}")
            ]
        )
    });

    format_tree_attribute_inline_or_hugged(f, value_id, inline_format, hugged_format)
}

/// Format a tree/JSX attribute value with hugging for objects and arrays.
pub(crate) fn format_tree_attribute_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // tree attribute layout
    let tree = f.context().tree;

    match tree.get(value_id) {
        // object attribute value
        Expression::ObjectExpression { ty, properties } => {
            format_tree_attribute_object_value(f, value_id, *ty, properties.clone())?;
        }

        // array attribute value
        Expression::ArrayExpression { elements } => {
            format_tree_attribute_array_value(f, value_id, elements.clone())?;
        }

        // non-huggable values use regular braced formatting
        _ => {
            write!(f, [token("="), token("{"), value_id, token("}")])?;
        }
    }

    Ok(())
}

pub(crate) fn is_huggable_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    config: &HugOptions,
) -> bool {
    match tree.get(expression_id) {
        Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. } => true,
        Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
            // arrow functions stay hugged when used as the only call argument
            if let Declaration::Function {
                signature,
                body: Some(_),
                ..
            } = tree.get(*declaration_id)
            {
                signature.kind == FunctionKind::Lambda
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Store arrow-specific layout facts for hugged argument formatting.
#[derive(Clone, Copy, Default)]
struct HuggedArrowLayoutSignals {
    is_arrow_function: bool,
    arrow_force_expand: bool,
    arrow_trailing_line_break_if_breaks: bool,
    arrow_trailing_comma_if_breaks: bool,
}

/// Collect arrow-specific layout signals for one hugged argument value.
fn collect_hugged_arrow_layout_signals(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
    config: &HugOptions,
) -> HuggedArrowLayoutSignals {
    if !config.allow_arrow_functions {
        return HuggedArrowLayoutSignals::default();
    }

    let tree = context.tree;
    let arrow_declaration_id = match tree.get(value_id) {
        Expression::Declaration(declaration_id) => match tree.get(*declaration_id) {
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda => {
                Some(*declaration_id)
            }
            _ => None,
        },
        _ => None,
    };

    let Some(declaration_id) = arrow_declaration_id else {
        return HuggedArrowLayoutSignals::default();
    };

    let (arrow_body_is_block, arrow_body_is_tree, arrow_body_is_lambda) =
        if let Declaration::Function {
            body: Some(body_id),
            ..
        } = tree.get(declaration_id)
        {
            let body_id = transparent_inner_expression(context, *body_id);
            let body_expr = tree.get(body_id);
            let is_block = matches!(body_expr, Expression::Block(_));
            let is_tree = matches!(body_expr, Expression::TreeExpression { .. });
            let is_lambda = matches!(
                body_expr,
                Expression::Declaration(nested_declaration_id)
                    if matches!(
                        tree.get(*nested_declaration_id),
                        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
                    )
            );
            (is_block, is_tree, is_lambda)
        } else {
            (false, false, false)
        };

    let arrow_force_expand = lambda_expression_should_break(context, declaration_id);
    let arrow_trailing_line_break_if_breaks =
        !arrow_body_is_block && !arrow_body_is_tree && !arrow_body_is_lambda;
    let arrow_trailing_comma_if_breaks =
        arrow_trailing_line_break_if_breaks && !arrow_body_is_lambda;

    HuggedArrowLayoutSignals {
        is_arrow_function: true,
        arrow_force_expand,
        arrow_trailing_line_break_if_breaks,
        arrow_trailing_comma_if_breaks,
    }
}

/// Choose between inline and hugged candidate docs for one single hugged argument.
#[allow(clippy::too_many_arguments)]
fn choose_hugged_argument_layout<'ast, InlineDoc, HuggedDoc>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
    force_expand: bool,
    signals: HuggedArrowLayoutSignals,
    inline_format: InlineDoc,
    hugged_format: HuggedDoc,
) -> FormatResult<()>
where
    InlineDoc: Format<DestackFormatContext<'ast>>,
    HuggedDoc: Format<DestackFormatContext<'ast>>,
{
    // forced expansion always selects hugged output
    if force_expand {
        hugged_format.format(f)?;
        return Ok(());
    }

    // context-based default selection
    let is_annotated =
        f.context().has_annotation(argument_id) || f.context().has_annotation(value_id);
    let should_hug = is_annotated || signals.arrow_force_expand;
    if should_hug {
        f.context()
            .increment_counter("profile.jsx.hug.by_context.hug", 1);
        hugged_format.format(f)?;
    } else {
        f.context()
            .increment_counter("profile.jsx.hug.by_context.inline", 1);
        inline_format.format(f)?;
    }

    Ok(())
}

/// Format a single-element argument list with hugging for expandable elements.
///
/// When a single object/array (or arrow function for calls) is the only argument,
/// format as `foo({...})` instead of `foo(\n    {...},\n)`.
/// Returns true if hugging was applied, false if regular list_like should be used.
pub(crate) fn format_hugged<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
    config: HugOptions,
    group_id: Option<GroupId>,
    force_expand: bool,
) -> FormatResult<bool> {
    // only hug single positional arguments
    if arguments.len() != 1 {
        return Ok(false);
    }

    // resolve and normalize the single argument value
    let argument_id = arguments[0];
    let Some(value_id) = argument_value(f.context().tree, argument_id) else {
        return Ok(false);
    };
    let value_id = transparent_inner_expression(f.context(), value_id);

    // guard non-huggable value kinds
    if !is_huggable_expression(f.context().tree, value_id, &config) {
        return Ok(false);
    }

    // multiline empty collections are not stable hugging candidates
    if !force_expand
        && f.context().node_has_newline(value_id)
        && match f.context().tree.get(value_id) {
            Expression::ObjectExpression { properties, .. } => properties.is_empty(),
            Expression::ArrayExpression { elements } => elements.is_empty(),
            _ => false,
        }
    {
        return Ok(false);
    }

    // multiline collection values can opt out of hugging by configuration
    if !force_expand
        && !config.allow_multiline_collection
        && f.context().node_has_newline(value_id)
        && matches!(
            f.context().tree.get(value_id),
            Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
        )
    {
        return Ok(false);
    }

    // collect arrow-specific layout facts once
    let signals = collect_hugged_arrow_layout_signals(f.context(), value_id, &config);
    let trailing_if_breaks = config.trailing_if_breaks;

    // build inline candidate doc
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let inline_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [token(config.open), argument_id])?;
            if config.force_trailing {
                write!(f, [token(",")])?;
            }
            write!(f, [token(config.close)])
        });

        if let Some(group_id) = group_id {
            group(&inline_inner).with_id(Some(group_id)).format(f)
        } else {
            write!(f, [inline_inner])
        }
    });

    // build hugged candidate doc
    let hugged_format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // argument annotations
        if config.handle_annotations {
            if signals.is_arrow_function {
                write!(f, [f.context().block_prefix_annotations(argument_id)])?;
            } else {
                write!(f, [f.context().any_prefix_annotations(argument_id)])?;
            }
        }

        // force expansion when arrow policy requires it
        if signals.arrow_force_expand {
            write!(f, [expand_parent()])?;
        }

        // opening delimiter
        write!(f, [token(config.open)])?;

        // value payload
        match f.context().tree.get(value_id) {
            Expression::ObjectExpression { ty, properties } => {
                if let Some(ty) = ty {
                    write!(f, [ty, space()])?;
                }
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("{"),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(properties)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("}")
                            ]
                        )
                    }))
                    .should_expand(true)]
                )?;
            }
            Expression::ArrayExpression { elements } => {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("["),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(elements)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("]")
                            ]
                        )
                    }))
                    .should_expand(true)]
                )?;
            }
            Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
                if let Some(group_id) = group_id {
                    write!(
                        f,
                        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(f, [declaration_id])
                        }))
                        .with_id(Some(group_id))]
                    )?;
                } else {
                    write!(f, [declaration_id])?;
                }
            }
            _ => {
                write!(f, [value_id])?;
            }
        }

        // trailing comma behavior
        if config.force_trailing {
            write!(f, [token(",")])?;
        } else if signals.arrow_trailing_comma_if_breaks || trailing_if_breaks {
            let comma = token(",");
            let trailing_comma = if let Some(group_id) = group_id {
                if_group_breaks(&comma).with_group_id(Some(group_id))
            } else {
                if_group_breaks(&comma)
            };
            write!(f, [trailing_comma])?;
        }

        // trailing line break behavior for arrow values
        if signals.arrow_trailing_line_break_if_breaks {
            let line_break = hard_line_break();
            let break_doc = if let Some(group_id) = group_id {
                if_group_breaks(&line_break).with_group_id(Some(group_id))
            } else {
                if_group_breaks(&line_break)
            };
            write!(f, [break_doc])?;
        }

        // closing delimiter
        write!(f, [token(config.close)])?;

        // trailing annotations
        if config.handle_annotations {
            write!(
                f,
                [f.context().any_infix_or_postfix_annotations(argument_id)]
            )?;
        }
        Ok(())
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if trailing_if_breaks || group_id.is_some() {
            write!(
                f,
                [group(&hugged_format_inner)
                    .with_id(group_id)
                    .should_expand(signals.arrow_force_expand)]
            )?;
        } else {
            write!(f, [hugged_format_inner])?;
        }
        Ok(())
    });

    // choose final candidate
    choose_hugged_argument_layout(
        f,
        argument_id,
        value_id,
        force_expand,
        signals,
        inline_format,
        hugged_format,
    )?;

    Ok(true)
}
