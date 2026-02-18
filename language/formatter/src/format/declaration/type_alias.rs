use super::dispatch::format_declaration_export_modifier;
use crate::analysis::scan::next_non_whitespace_after_annotation;
use crate::collection::list_like;
use crate::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::expression::{format_expression, is_expression_breakable, source_min_inline_char_len};
use crate::operator::is_type_context;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Comment, CommentStyle, Declaration, DeclarationDescriptor, DeclarationKind,
    Expression, IfKind, Keyword, LocalNodeId, Mutability, Parameter, TypeKind,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

/// Store inline block-prefix comment metadata for type grouping expressions.
#[derive(Debug, Clone)]
struct InlineTypePrefixCommentCluster {
    /// The target expression id whose leading comments are emitted inline.
    expression_id: LocalNodeId<Expression>,
    /// The annotation ids in source order.
    annotation_ids: Vec<LocalNodeId<Annotation>>,
}

/// Return inline prefix comment cluster metadata for type grouping expressions.
fn single_line_type_grouping_prefix_comment_cluster(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<InlineTypePrefixCommentCluster> {
    if !is_type_context(context, expression_id) {
        return None;
    }

    let mut current_id = expression_id;
    loop {
        let Some(annotations) = context.annotations(current_id) else {
            let next_id = match context.tree.get(current_id) {
                Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                    Some(*expression)
                }
                Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => {
                    Some(*left)
                }
                _ => None,
            }?;
            current_id = next_id;
            continue;
        };

        let mut cluster = Vec::new();
        for annotation_id in annotations {
            let annotation = context.annotation(annotation_id);
            let Annotation::Comment {
                node: comment_id,
                position,
                ..
            } = annotation
            else {
                break;
            };
            if !matches!(
                position,
                AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
            ) {
                break;
            }

            let comment = context.tree.get::<Comment>(comment_id);
            if comment.style != CommentStyle::Star {
                break;
            }
            let comment_span = context.annotation_span(annotation_id);
            let comment_source = context.span_str(comment_span);
            if comment_source.trim_start().starts_with("/**") || comment_source.contains('\n') {
                return None;
            }

            if let Some(previous_annotation_id) = cluster.last().copied() {
                let previous_span = context.annotation_span(previous_annotation_id);
                let current_span = context.annotation_span(annotation_id);
                let between_span =
                    Span::new(previous_span.file, previous_span.end, current_span.start);
                if context.has_newline(between_span) {
                    return None;
                }
            }
            cluster.push(annotation_id);
        }

        if cluster.is_empty() {
            return None;
        }

        let last_annotation_id = *cluster.last()?;
        let next_character = next_non_whitespace_after_annotation(context, last_annotation_id);
        if !matches!(next_character, Some('|' | '&')) {
            return None;
        }
        return Some(InlineTypePrefixCommentCluster {
            expression_id: current_id,
            annotation_ids: cluster,
        });
    }
}

/// Return whether an expression has a doc-like block prefix annotation.
fn expression_has_doc_like_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };

    annotation_ids.into_iter().any(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        match annotation {
            Annotation::Doc {
                position: AnnotationPosition::BlockPrefix,
                ..
            } => true,
            Annotation::Comment {
                position: AnnotationPosition::BlockPrefix,
                ..
            } => {
                let annotation_span = context.annotation_span(annotation_id);
                let annotation_source = context.span_str(annotation_span);
                annotation_source.trim_start().starts_with("/**")
            }
            _ => false,
        }
    })
}

/// Return whether an expression or its transparent left spine has a prefix annotation.
fn expression_has_prefix_annotation_in_left_spine(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;
    loop {
        if context.has_prefix_annotation(current_id) {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                Some(*expression)
            }
            Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => Some(*left),
            _ => None,
        };

        let Some(next_id) = next_id else {
            return false;
        };
        current_id = next_id;
    }
}

/// Format an expression while omitting its own prefix annotations.
fn format_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let directive = directive_for_node(f.context(), expression_id);
    let expression = f.context().tree.get(expression_id);

    format_expression(f, expression_id, expression, directive)?;

    if !matches!(
        directive,
        Some(FormatterDirective {
            kind: FormatterDirectiveKind::IgnoreFormat,
            position: FormatterDirectivePosition::Postfix { .. },
        })
    ) {
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(expression_id)]
        )?;
    }

    Ok(())
}
/// Format a type alias declaration.
pub(super) fn format_type_alias_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: TypeKind,
    mutability: Option<Mutability>,
    static_parameters: &Option<Vec<LocalNodeId<Parameter>>>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let header = format_with(|f| {
        // export
        format_declaration_export_modifier(f, node_id, descriptor)?;

        // kind
        if descriptor.kind == DeclarationKind::Declaration {
            write!(f, [Keyword::Declare, space()])?;
        }

        // keyword
        if mutability == Some(Mutability::Immutable) {
            // for readonly type expression
            write!(f, [Keyword::Readonly])?;
        } else if kind == TypeKind::Structural {
            write!(f, [Keyword::Type])?;
        } else {
            write!(f, [Keyword::Newtype])?;
        }

        // name
        if let Some(name) = descriptor.name {
            write!(f, [space(), name])?;
        }

        // static parameters
        if let Some(static_parameters) = static_parameters {
            write!(f, [list_like("<", ">", ",", static_parameters)])?;
            write!(
                f,
                [f.context().declaration_generic_head_annotations(node_id)]
            )?;
        }

        Ok(())
    });

    let inline_prefix_comment_cluster =
        single_line_type_grouping_prefix_comment_cluster(f.context(), value_id);

    // prefer keeping the value on a single line
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("="), space()])?;
        if let Some(cluster) = inline_prefix_comment_cluster.as_ref() {
            for (index, annotation_id) in cluster.annotation_ids.iter().copied().enumerate() {
                if index > 0 {
                    write!(f, [space()])?;
                }
                let annotation_span = f.context().annotation_span(annotation_id);
                let annotation_source = f.context().span_str(annotation_span);
                write!(f, [text(annotation_source.trim())])?;
            }
            write!(f, [space()])?;
            format_expression_without_prefix_annotations(f, cluster.expression_id)?;
        } else {
            write!(f, [value_id])?;
        }
        Ok(())
    });

    let format_soft_break = format_with(|f| {
        write!(
            f,
            [group(&format_args![
                header,
                space(),
                token("="),
                indent(&format_args![soft_line_break_or_space(), value_id])
            ])]
        )
    });

    // expand inline if breakable (like let x = [\n ... ])
    let format_inline_expanded = format_with(|f| {
        write!(
            f,
            [
                header,
                space(),
                token("="),
                space(),
                fits_expanded(&group(&value_id).should_expand(true)),
            ]
        )
    });

    // expand and indent the value
    let format_indented = format_with(|f| {
        group(&format_args![
            header,
            space(),
            token("="),
            block_indent(&value_id)
        ])
        .format(f)
    });

    let tree = f.context().tree;
    let value_expression = tree.get(value_id);
    let value_has_prefix_annotation =
        expression_has_prefix_annotation_in_left_spine(f.context(), value_id);
    let value_has_doc_like_block_prefix_annotation =
        expression_has_doc_like_block_prefix_annotation(f.context(), value_id);
    let declaration_span = f.context().span(node_id);
    let value_span = f.context().span(value_id);
    let leading_value_span = Span::new(value_span.file, declaration_span.start, value_span.start);
    let inline_header_len = f.context().span_char_len(leading_value_span);
    let inline_value_len = f.context().span_char_len(value_span);
    let inline_total_len = inline_header_len.saturating_add(inline_value_len);
    let inline_header_min_len =
        source_min_inline_char_len(f.context().span_str(leading_value_span));
    let inline_value_min_len = source_min_inline_char_len(f.context().span_str(value_span));
    let inline_total_min_len = inline_header_min_len.saturating_add(inline_value_min_len);
    let line_width = usize::from(f.context().options.line_width);
    let value_has_newline = f.context().has_newline(value_span);
    let inline_is_impossible = inline_total_min_len > line_width;
    let should_break_template_literal_type_after_equals = match value_expression {
        Expression::TypeTemplateLiteral { spans, .. } => {
            let has_conditional_interpolation = spans.iter().any(|span_id| {
                matches!(
                    tree.get(*span_id),
                    Expression::TypeConditional { .. }
                        | Expression::If {
                            kind: IfKind::Ternary,
                            ..
                        }
                )
            });
            has_conditional_interpolation && inline_total_len > line_width
        }
        _ => false,
    };
    let should_break_after_equals = match value_expression {
        Expression::TypeConditional { left, .. } => {
            !matches!(tree.get(*left), Expression::Parenthesized { .. })
        }
        _ => false,
    };
    let value_prefers_inline_after_equals = match value_expression {
        Expression::Parenthesized { .. } => true,
        Expression::Index { left, .. } | Expression::TypeIndex { left, .. } => {
            matches!(tree.get(*left), Expression::Parenthesized { .. })
        }
        _ => false,
    };
    if inline_prefix_comment_cluster.is_some() {
        format_inline.format(f)?;
    } else if should_break_after_equals || should_break_template_literal_type_after_equals {
        format_soft_break.format(f)?;
    } else if is_expression_breakable(tree, tree.get(value_id)) {
        if !value_has_prefix_annotation && !value_has_newline && inline_total_len <= line_width {
            format_inline.format(f)?;
        } else if value_has_prefix_annotation {
            if value_has_doc_like_block_prefix_annotation {
                format_inline.format(f)?;
            } else {
                let can_inline_prefixed_value =
                    !value_has_newline && inline_total_len <= line_width;
                if can_inline_prefixed_value {
                    format_inline.format(f)?;
                } else {
                    // keep `=` inline and let the value shape decide line breaks
                    format_inline.format(f)?;
                }
            }
        } else if inline_is_impossible {
            // long conditional-like type values can skip best fitting probes
            format_inline_expanded.format(f)?;
        } else {
            let can_inline = !value_has_newline && inline_total_len <= line_width;
            if can_inline {
                format_inline.format(f)?;
            } else {
                format_inline_expanded.format(f)?;
            }
        }
    } else if inline_is_impossible {
        format_indented.format(f)?;
    } else {
        let can_inline = inline_total_len <= line_width
            && (!value_has_newline || value_prefers_inline_after_equals);
        if can_inline {
            format_inline.format(f)?;
        } else {
            format_indented.format(f)?;
        }
    }

    // type alias declarations need trailing semicolon (like const/let)
    write!(f, [token(";")])?;

    Ok(())
}
