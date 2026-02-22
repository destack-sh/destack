use crate::format::analysis::scan::next_non_whitespace_after_annotation;
use crate::format::collection::list_like;
use crate::format::declaration::dispatch::format_declaration_export_modifier;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::{
    BinaryOperator, flatten_binary_expression, format_expression, is_expression_breakable,
};
use crate::format::operator::{is_type_context, union_has_leading_pipe_token};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
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
    /// Whether each annotation starts after a source newline relative to the previous one.
    annotation_breaks_before: Vec<bool>,
}

impl InlineTypePrefixCommentCluster {
    /// Return whether the cluster contains source newline breaks between comments.
    fn has_multiline_breaks(&self) -> bool {
        self.annotation_breaks_before
            .iter()
            .copied()
            .skip(1)
            .any(|has_break| has_break)
    }
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
        let mut annotation_breaks_before = Vec::new();
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
            if context.has_newline(comment_span) {
                return None;
            }

            let has_newline_before = if let Some(previous_annotation_id) = cluster.last().copied() {
                let previous_span = context.annotation_span(previous_annotation_id);
                let current_span = context.annotation_span(annotation_id);
                let between_span =
                    Span::new(previous_span.file, previous_span.end, current_span.start);
                context.has_newline(between_span)
            } else {
                false
            };

            cluster.push(annotation_id);
            annotation_breaks_before.push(has_newline_before);
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
            annotation_breaks_before,
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
        matches!(
            context.annotation(annotation_id),
            Annotation::Doc {
                position: AnnotationPosition::BlockPrefix,
                ..
            }
        )
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

/// Format one type alias value using leading-pipe union style when applicable.
fn format_leading_pipe_type_alias_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let operands =
        flatten_binary_expression(f.context().tree, value_id, BinaryOperator::ElementwiseOr);
    if operands.len() <= 1 {
        write!(f, [space(), value_id])?;
        return Ok(());
    }

    for operand in operands {
        write!(
            f,
            [hard_line_break(), token("|"), space(), operand.expression]
        )?;
    }

    Ok(())
}

/// Return whether one expression is a union-like type alias value candidate.
fn value_is_union_like_type_alias_expression(
    tree: &destack_ast::NodeTree,
    value_expression: &Expression,
) -> bool {
    match value_expression {
        Expression::Binary { operator, .. } => *operator == BinaryOperator::ElementwiseOr,
        Expression::Parenthesized { expression } | Expression::Statement(expression) => matches!(
            tree.get(*expression),
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr,
                ..
            }
        ),
        _ => false,
    }
}

/// Format a type alias declaration.
pub(crate) fn format_type_alias_declaration<'ast>(
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
        write!(f, [header, space(), token("=")])?;
        if let Some(cluster) = inline_prefix_comment_cluster.as_ref() {
            if cluster.has_multiline_breaks() {
                write!(
                    f,
                    [indent(&format_with(
                        |f: &mut DestackFormatter<'ast, '_>| {
                            write!(f, [hard_line_break()])?;
                            for (index, annotation_id) in
                                cluster.annotation_ids.iter().copied().enumerate()
                            {
                                if index > 0 {
                                    if cluster.annotation_breaks_before[index] {
                                        write!(f, [hard_line_break()])?;
                                    } else {
                                        write!(f, [space()])?;
                                    }
                                }

                                let annotation = f.context().annotation(annotation_id);
                                annotation.format_node(annotation_id, f)?;
                            }

                            write!(f, [space()])?;
                            format_expression_without_prefix_annotations(f, cluster.expression_id)?;
                            Ok(())
                        }
                    ))]
                )?;
            } else {
                write!(f, [space()])?;
                for (index, annotation_id) in cluster.annotation_ids.iter().copied().enumerate() {
                    if index > 0 {
                        write!(f, [space()])?;
                    }
                    let annotation = f.context().annotation(annotation_id);
                    annotation.format_node(annotation_id, f)?;
                }
                write!(f, [space()])?;
                format_expression_without_prefix_annotations(f, cluster.expression_id)?;
            }
        } else {
            write!(f, [space()])?;
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
    let format_hard_break = format_with(|f| {
        write!(
            f,
            [group(&format_args![
                header,
                space(),
                token("="),
                indent(&format_args![format_with(|f| {
                    format_leading_pipe_type_alias_value(f, value_id)
                })])
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

    let tree = f.context().tree;
    let value_expression = tree.get(value_id);
    let value_has_prefix_annotation =
        expression_has_prefix_annotation_in_left_spine(f.context(), value_id);
    let value_has_doc_like_block_prefix_annotation =
        expression_has_doc_like_block_prefix_annotation(f.context(), value_id);
    let should_break_template_literal_type_after_equals = match value_expression {
        Expression::TypeTemplateLiteral { spans, .. } => spans.iter().any(|span_id| {
            matches!(
                tree.get(*span_id),
                Expression::TypeConditional { .. }
                    | Expression::If {
                        kind: IfKind::Ternary,
                        ..
                    }
            )
        }),
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
    let value_has_leading_pipe_type_union =
        value_is_union_like_type_alias_expression(tree, value_expression)
            && is_type_context(f.context(), value_id)
            && union_has_leading_pipe_token(f.context(), value_id);

    if inline_prefix_comment_cluster.is_some() {
        format_inline.format(f)?;
    } else if value_has_leading_pipe_type_union && !value_has_prefix_annotation {
        format_hard_break.format(f)?;
    } else if should_break_after_equals || should_break_template_literal_type_after_equals {
        format_soft_break.format(f)?;
    } else if is_expression_breakable(tree, tree.get(value_id)) {
        if value_has_prefix_annotation && value_has_doc_like_block_prefix_annotation {
            format_inline.format(f)?;
        } else if value_prefers_inline_after_equals {
            format_inline.format(f)?;
        } else {
            format_inline_expanded.format(f)?;
        }
    } else {
        if value_has_prefix_annotation || value_prefers_inline_after_equals {
            format_inline.format(f)?;
        } else {
            format_soft_break.format(f)?;
        }
    }

    // type alias declarations need trailing semicolon (like const/let)
    write!(f, [token(";")])?;

    Ok(())
}
