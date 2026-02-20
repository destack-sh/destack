use super::super::children::{
    expression_has_chain_seam_comment, tree_child_should_inline_braced_expression,
};
use super::attribute::format_tree_attribute_value;
use crate::analysis::scan::{first_non_trivia_token_in_span, last_non_trivia_token_in_span};
use crate::expression::{
    Annotation, AnnotationPosition, Argument, DestackFormatContext, DestackFormatter, Expression,
    FormatResult, IfCondition, IfKind, LocalNodeId, NodeTree, TokenType, argument_value,
    block_indent, group, hard_line_break, soft_block_indent, space, text, token,
    transparent_inner_expression,
};
use destack_ast::{Declaration, FunctionKind, NodeType, ScalarLiteral};
use destack_fir::format::{Buffer, Format};
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

/// Return whether one tree child argument is a braced whitespace scalar.
fn tree_argument_is_braced_whitespace_scalar(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !tree_argument_is_wrapped_in_braces(context, argument_id) {
        return false;
    }

    let Argument::Positional { value, .. } = context.tree.get(argument_id) else {
        return false;
    };

    match context.tree.get(*value) {
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            let content = context.strings.get(*string_id);
            content.chars().all(char::is_whitespace)
        }
        Expression::ScalarLiteral(ScalarLiteral::Character(value)) => value.is_whitespace(),
        _ => false,
    }
}

/// Return whether one tree child argument is a non-whitespace text scalar.
fn tree_argument_is_non_whitespace_text_scalar(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Argument::Positional { value, .. } = context.tree.get(argument_id) else {
        return false;
    };

    match context.tree.get(*value) {
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            let content = context.strings.get(*string_id);
            content.chars().any(|character| !character.is_whitespace())
        }
        Expression::ScalarLiteral(ScalarLiteral::Character(value)) => !value.is_whitespace(),
        _ => false,
    }
}

/// Return whether one braced whitespace tree child should render as text space.
fn tree_whitespace_expression_renders_as_text(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !tree_argument_is_braced_whitespace_scalar(context, argument_id) {
        return false;
    }

    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::TreeExpression {
        elements: Some(elements),
        ..
    } = context.tree.get(parent_expression_id)
    else {
        return false;
    };

    let Some(element_index) = elements
        .iter()
        .position(|element_id| *element_id == argument_id)
    else {
        return false;
    };

    let previous_is_text = element_index
        .checked_sub(1)
        .and_then(|index| elements.get(index).copied())
        .is_some_and(|element_id| tree_argument_is_non_whitespace_text_scalar(context, element_id));
    let next_is_text = elements
        .get(element_index + 1)
        .copied()
        .is_some_and(|element_id| tree_argument_is_non_whitespace_text_scalar(context, element_id));

    previous_is_text || next_is_text
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
                let argument_whitespace_renders_as_text = !f
                    .context()
                    .options
                    .language_type
                    .is_destack()
                    && tree_whitespace_expression_renders_as_text(f.context(), self.argument_id);
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
                    if argument_whitespace_renders_as_text {
                        write!(f, [text(" ")])?;
                    } else if matches!(value_expr, Expression::Stub) {
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
