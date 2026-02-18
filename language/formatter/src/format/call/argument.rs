use crate::analysis::scan::{
    next_non_whitespace_token_after_annotation, previous_non_whitespace_token_before_annotation,
};
use crate::collection::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, CommentStyle, Declaration, Expression, FunctionKind, LocalNodeId,
    NodeType, TokenType, TypeBinaryOperator,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

/// Return whether an argument should emit its prefix annotations.
fn argument_should_emit_prefix_annotations(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if argument_has_satisfies_static_seam_prefix_line_comment(context, argument_id) {
        return false;
    }

    if !argument_is_call_or_new(context, argument_id) {
        return true;
    }

    if argument_has_non_blank_prefix_annotation(context, argument_id) {
        return true;
    }

    if argument_has_blank_prefix_annotation_before_separator(context, argument_id) {
        return false;
    }

    if !argument_is_first_in_call_or_new(context, argument_id) {
        return true;
    }

    !argument_has_blank_prefix_annotation(context, argument_id)
}

/// Return one satisfies static seam line comment source for this argument when present.
pub(in crate::format) fn argument_satisfies_static_seam_comment_source(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<String> {
    let seam_comment_id = argument_satisfies_static_seam_comment_id(context, argument_id)?;
    let span = context.annotation_span(seam_comment_id);
    Some(context.span_str(span).trim().to_string())
}

/// Return one satisfies static seam line comment annotation id for this argument when present.
pub(super) fn argument_satisfies_static_seam_comment_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Annotation>> {
    if !argument_is_first_static_argument_of_satisfies_right_path(context, argument_id) {
        return None;
    }

    let Some(annotations) = context.annotations(argument_id) else {
        return None;
    };

    let mut seam_comment_id = None;
    for annotation_id in annotations {
        let Annotation::Comment {
            node,
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
        } = context.annotation(annotation_id)
        else {
            continue;
        };

        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != CommentStyle::Slash {
            continue;
        }

        seam_comment_id = Some(annotation_id);
        break;
    }

    seam_comment_id
}

/// Return whether one argument is the first static argument in a `satisfies` rhs path with multiple arguments.
fn argument_is_first_static_argument_of_satisfies_right_path(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_first_static_argument_of_multi_argument_path(context, argument_id) {
        return false;
    }

    let Some((path_expression_id, path_parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if path_parent_type != NodeType::Expression {
        return false;
    }

    let path_expression_id = LocalNodeId::<Expression>::new(path_expression_id);
    let Some((type_binary_id, type_binary_parent_type)) = context.parent(path_expression_id) else {
        return false;
    };
    if type_binary_parent_type != NodeType::Expression {
        return false;
    }

    let type_binary_id = LocalNodeId::<Expression>::new(type_binary_id);
    let Expression::TypeBinary {
        operator: TypeBinaryOperator::Satisfies,
        right,
        ..
    } = context.tree.get(type_binary_id)
    else {
        return false;
    };

    let right_expression_id = match context.tree.get(*right) {
        Expression::Parenthesized { expression } | Expression::Statement(expression) => *expression,
        _ => *right,
    };

    right_expression_id == path_expression_id
}

/// Return whether one argument is the first static argument of a multi-argument path static list.
fn argument_is_first_static_argument_of_multi_argument_path(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(parent_id);
    let static_arguments = match context.tree.get(expression_id) {
        // keep seam remapping constrained to the formatter path we re-render explicitly
        Expression::Path {
            path,
            static_arguments,
        } if path.segments.len() == 1 => static_arguments.as_ref(),
        _ => None,
    };

    let Some(static_arguments) = static_arguments else {
        return false;
    };
    if static_arguments.len() <= 1 {
        return false;
    }

    static_arguments
        .first()
        .is_some_and(|first| *first == argument_id)
}

/// Return whether this argument has one satisfies static seam prefix line comment.
fn argument_has_satisfies_static_seam_prefix_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_satisfies_static_seam_comment_id(context, argument_id).is_some()
}

/// Return whether an argument belongs to a call or new expression.
fn argument_is_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    matches!(expression, Expression::Call { .. } | Expression::New { .. })
}

/// Return whether an argument is the first in its call/new argument list.
fn argument_is_first_in_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    match expression {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments
            .first()
            .is_some_and(|first| *first == argument_id),
        _ => false,
    }
}

/// Return whether an argument has a non-blank prefix annotation.
fn argument_has_non_blank_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations
        .iter()
        .any(|annotation_id| match context.annotation(*annotation_id) {
            Annotation::Blank { .. } => false,
            Annotation::Doc { position, .. }
            | Annotation::Comment { position, .. }
            | Annotation::Decorator { position, .. } => matches!(
                position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ),
        })
}

/// Return whether an argument has a blank prefix annotation.
fn argument_has_blank_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        matches!(
            context.annotation(*annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether an argument has a blank prefix annotation before a separator.
fn argument_has_blank_prefix_annotation_before_separator(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        let Annotation::Blank {
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
            ..
        } = context.annotation(*annotation_id)
        else {
            return false;
        };

        next_non_whitespace_token_after_annotation(context, *annotation_id)
            .is_some_and(|token| token.token.ty == TokenType::Comma)
    })
}

/// Return whether a lambda argument has an inline prefix comment that must break.
fn argument_prefix_lambda_comment_needs_forced_break(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(*annotation_id) else {
            return false;
        };
        if position != AnnotationPosition::BlockPrefix {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        let previous_token =
            previous_non_whitespace_token_before_annotation(context, *annotation_id);
        let next_token = next_non_whitespace_token_after_annotation(context, *annotation_id);
        previous_token.is_some_and(|token| {
            matches!(
                token.token.ty,
                TokenType::OpenParenthesis
                    | TokenType::OpenBracket
                    | TokenType::OpenBrace
                    | TokenType::LessThan
            )
        }) && next_token.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
    })
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: LocalNodeId<Argument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // fast path: annotation free positional and spread arguments dominate call sites
        // and don't need the expensive prefix and trailing annotation checks
        let has_argument_annotation = f.context().has_annotation(node_id);
        if !has_argument_annotation {
            match self {
                Argument::Named {
                    modifiers: None,
                    name,
                    value,
                } => {
                    write!(f, [*name, token(":"), space(), *value])?;
                    return Ok(());
                }
                Argument::Labeled {
                    modifiers: None,
                    label,
                    value,
                } => {
                    write!(f, [*label, token(":"), space(), *value])?;
                    return Ok(());
                }
                Argument::Positional {
                    modifiers: None,
                    value,
                } => {
                    write!(f, [*value])?;
                    return Ok(());
                }
                Argument::Spread {
                    modifiers: None,
                    label: None,
                    value,
                } => {
                    write!(f, [token("..."), *value])?;
                    return Ok(());
                }
                Argument::Spread {
                    modifiers: None,
                    label: Some(label),
                    value,
                } => {
                    write!(f, [token("..."), *label, token(":"), space(), *value])?;
                    return Ok(());
                }
                _ => {}
            }
        }

        // lambda argument comments are handled at the lambda arrow site
        let should_emit_prefix_annotations =
            argument_should_emit_prefix_annotations(f.context(), node_id);
        let should_preserve_blank_line_before_prefix_comment =
            argument_prefix_comment_has_leading_blank_line_after_separator(f.context(), node_id);
        let has_argument_prefix_comment =
            argument_has_prefix_comment_annotation(f.context(), node_id);
        let has_lambda_value = argument_contains_lambda_value(f.context(), self);
        let force_break_after_lambda_prefix_comment = has_lambda_value
            && argument_prefix_lambda_comment_needs_forced_break(f.context(), node_id);

        if has_lambda_value {
            write!(f, [f.context().block_prefix_annotations(node_id)])?;
        } else if should_emit_prefix_annotations {
            if should_preserve_blank_line_before_prefix_comment && has_argument_prefix_comment {
                write!(f, [hard_line_break()])?;
            }
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

        match self {
            Argument::Named {
                modifiers,
                name,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // name
                write!(f, [name])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if should_preserve_blank_line_before_prefix_comment && !has_argument_prefix_comment
                {
                    write!(f, [hard_line_break()])?;
                }
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Labeled {
                modifiers,
                label,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // label
                write!(f, [label])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if should_preserve_blank_line_before_prefix_comment && !has_argument_prefix_comment
                {
                    write!(f, [hard_line_break()])?;
                }
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Positional { modifiers, value } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // value
                if force_break_after_lambda_prefix_comment {
                    write!(f, [hard_line_break()])?;
                }
                if should_preserve_blank_line_before_prefix_comment && !has_argument_prefix_comment
                {
                    write!(f, [hard_line_break()])?;
                }
                write!(f, [value])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            }
            Argument::Spread {
                modifiers,
                label,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
                write!(f, [token("...")])?;
                if let Some(label) = label {
                    write!(f, [label])?;
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                    if should_preserve_blank_line_before_prefix_comment
                        && !has_argument_prefix_comment
                    {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [token(":"), space(), value])?;
                } else {
                    if force_break_after_lambda_prefix_comment {
                        write!(f, [hard_line_break()])?;
                    }
                    if should_preserve_blank_line_before_prefix_comment
                        && !has_argument_prefix_comment
                    {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [value])?;
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

/// Return whether the argument itself has a prefix comment annotation.
fn argument_has_prefix_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        matches!(
            context.annotation(*annotation_id),
            Annotation::Comment {
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether an argument has a prefix comment separated by a blank line after a comma.
fn argument_prefix_comment_has_leading_blank_line_after_separator(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_call_or_new(context, argument_id) {
        return false;
    }
    let Some(prefix_comment_start) =
        first_prefix_comment_annotation_start_for_argument(context, argument_id)
    else {
        return false;
    };
    let Some(previous_argument_id) = previous_dynamic_argument_in_call_or_new(context, argument_id)
    else {
        return false;
    };

    let previous_span = context.span(previous_argument_id);
    if previous_span.end >= prefix_comment_start {
        return false;
    }

    let between = context.span_str(Span::new(
        previous_span.file,
        previous_span.end,
        prefix_comment_start,
    ));
    between
        .chars()
        .filter(|character| *character == '\n')
        .count()
        >= 2
}

/// Return the start offset of the first prefix comment on an argument or its value.
fn first_prefix_comment_annotation_start_for_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<u32> {
    let argument_annotation_start = context.annotations(argument_id).and_then(|annotations| {
        annotations.iter().find_map(|annotation_id| {
            let annotation = context.annotation(*annotation_id);
            let is_prefix_comment = matches!(
                annotation,
                Annotation::Comment {
                    position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                    ..
                }
            );
            if is_prefix_comment {
                Some(context.annotation_span(*annotation_id).start)
            } else {
                None
            }
        })
    });

    let argument = context.tree.get(argument_id);
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };
    let value_annotation_start = context.annotations(value_id).and_then(|annotations| {
        annotations.iter().find_map(|annotation_id| {
            let annotation = context.annotation(*annotation_id);
            let is_prefix_comment = matches!(
                annotation,
                Annotation::Comment {
                    position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                    ..
                }
            );
            if is_prefix_comment {
                Some(context.annotation_span(*annotation_id).start)
            } else {
                None
            }
        })
    });

    match (argument_annotation_start, value_annotation_start) {
        (Some(argument_start), Some(value_start)) => Some(argument_start.min(value_start)),
        (Some(argument_start), None) => Some(argument_start),
        (None, Some(value_start)) => Some(value_start),
        (None, None) => None,
    }
}

/// Return the previous dynamic argument in a call or new expression.
fn previous_dynamic_argument_in_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Argument>> {
    let (parent_id, parent_type) = context.parent(argument_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    let arguments = match expression {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments,
        _ => return None,
    };

    let index = arguments
        .iter()
        .position(|argument| *argument == argument_id)?;
    index
        .checked_sub(1)
        .and_then(|index| arguments.get(index).copied())
}

/// Return whether this argument wraps a lambda declaration expression.
fn argument_contains_lambda_value(context: &DestackFormatContext<'_>, argument: &Argument) -> bool {
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

#[cfg(test)]
mod tests {
    use crate::collection::list::{raw_ends_with_separator, strip_trailing_comments};
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_argument_named() {
        assert_format!(
            "x: 1",
            "x: 1",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named_shorthand() {
        assert_format!(
            "x",
            "x",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_positional() {
        assert_format!(
            "1",
            "1",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_strip_trailing_comments_keeps_leading_ignore_block_comment() {
        let raw = "/* biome-ignore format: keep */\nsomeProperty:    alias,";
        let stripped = strip_trailing_comments(raw);

        assert_eq!(stripped, raw);
    }

    #[test]
    fn test_raw_ends_with_separator_for_ignored_field_with_leading_comment() {
        let raw = "/* biome-ignore format: keep */\nsomeProperty:    alias,";
        assert!(raw_ends_with_separator(raw, ","));
    }

    #[test]
    fn test_raw_ends_with_separator_with_trailing_line_comment() {
        let raw = "someProperty: alias, // keep";
        assert!(raw_ends_with_separator(raw, ","));
    }
}
