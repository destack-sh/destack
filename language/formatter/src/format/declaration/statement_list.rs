use std::borrow::Cow;

use crate::Annotation;
use crate::analysis::timing::tags;
use crate::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition,
    collect_ignore_ranges_for_nodes, directive_for_node, write_ignored_span,
};
use crate::expression::format_expression;
use destack_ast::{
    AnnotationPosition, Block, Declaration, Expression, FunctionKind, IfKind, LocalNodeId,
    NodeTree, ScalarLiteral, WhileKind,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::{FileId, Span};

use super::block_policy::block_allows_value_tail;
use super::imports;
use crate::{DestackFormatContext, DestackFormatter};

/// Check whether an expression is a directive prologue string literal.
fn is_directive_expression(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    match tree.get(expression_id) {
        Expression::Statement(inner_id) => is_directive_expression(tree, *inner_id),
        Expression::Parenthesized { expression } => is_directive_expression(tree, *expression),
        Expression::ScalarLiteral(ScalarLiteral::String(_)) => true,
        _ => false,
    }
}

/// Return whether trivia between two expressions contains one explicit blank line.
fn expressions_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    left_expression_id: LocalNodeId<Expression>,
    right_expression_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_expression_id);
    let right_span = context.span(right_expression_id);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let between_span = Span::new(left_span.file, left_span.end, right_span.start);
    context.has_blank_line(between_span)
}

/// Return whether trivia between two offsets contains an explicit blank line.
fn has_blank_line_between_offsets(
    context: &DestackFormatContext<'_>,
    file: FileId,
    start: u32,
    end: u32,
) -> bool {
    if end <= start {
        return false;
    }

    let between_span = Span::new(file, start, end);
    context.has_blank_line(between_span)
}

/// Return the earliest start offset for prefix comment annotations on an expression.
fn expression_prefix_start(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    default_start: u32,
) -> u32 {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return default_start;
    };

    let mut start = default_start;
    for annotation_id in annotation_ids {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            continue;
        };
        if !matches!(
            position,
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            continue;
        }

        let comment_span = context.span(node);
        start = start.min(comment_span.start);
    }

    start
}

/// Return whether an expression or its declaration wrapper has any prefix annotation.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_prefix_annotation(expression_id) {
        return true;
    }

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => context.has_prefix_annotation(*declaration_id),
        Expression::Statement(inner_id) => {
            expression_has_effective_prefix_annotation(context, *inner_id)
        }
        _ => false,
    }
}

/// Return whether an expression or declaration wrapper has one non-comment prefix annotation.
fn expression_has_effective_non_comment_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let has_non_comment_prefix_annotation = context
        .annotations(expression_id)
        .map(|annotation_ids| {
            annotation_ids.into_iter().any(|annotation_id| {
                matches!(
                    context.annotation(annotation_id),
                    Annotation::Decorator {
                        position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);
    if has_non_comment_prefix_annotation {
        return true;
    }

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => context
            .annotations(*declaration_id)
            .map(|annotation_ids| {
                annotation_ids.into_iter().any(|annotation_id| {
                    matches!(
                        context.annotation(annotation_id),
                        Annotation::Decorator {
                            position: AnnotationPosition::BlockPrefix
                                | AnnotationPosition::LinePrefix,
                            ..
                        }
                    )
                })
            })
            .unwrap_or(false),
        Expression::Statement(inner_id) => {
            expression_has_effective_non_comment_prefix_annotation(context, *inner_id)
        }
        _ => false,
    }
}

/// Return whether an expression or its declaration wrapper has a blank prefix annotation.
fn expression_has_effective_blank_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_blank_prefix_annotation(expression_id) {
        return true;
    }

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            context.has_blank_prefix_annotation(*declaration_id)
        }
        Expression::Statement(inner_id) => {
            expression_has_effective_blank_prefix_annotation(context, *inner_id)
        }
        _ => false,
    }
}

/// Return whether an expression or its declaration wrapper has postfix annotations.
fn expression_has_effective_postfix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_postfix_annotation(expression_id) {
        return true;
    }

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => context.has_postfix_annotation(*declaration_id),
        Expression::Statement(inner_id) => {
            expression_has_effective_postfix_annotation(context, *inner_id)
        }
        _ => false,
    }
}

/// Return whether an expression or its declaration wrapper has blank postfix annotations.
fn expression_has_effective_blank_postfix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let has_blank_postfix_annotation = context
        .annotations(expression_id)
        .map(|annotation_ids| {
            annotation_ids.into_iter().any(|annotation_id| {
                matches!(
                    context.annotation(annotation_id),
                    Annotation::Blank {
                        position: AnnotationPosition::BlockPostfix
                            | AnnotationPosition::LinePostfix
                            | AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);
    if has_blank_postfix_annotation {
        return true;
    }

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => context
            .annotations(*declaration_id)
            .map(|annotation_ids| {
                annotation_ids.into_iter().any(|annotation_id| {
                    matches!(
                        context.annotation(annotation_id),
                        Annotation::Blank {
                            position: AnnotationPosition::BlockPostfix
                                | AnnotationPosition::LinePostfix
                                | AnnotationPosition::LinePostfixBoundary,
                            ..
                        }
                    )
                })
            })
            .unwrap_or(false),
        Expression::Statement(inner_id) => {
            expression_has_effective_blank_postfix_annotation(context, *inner_id)
        }
        _ => false,
    }
}

/// Format a block inline with zero or one expression (including label and infix annotations).
/// Format block contents with compact inner spacing.
#[inline]
pub(crate) fn format_block_body_narrow<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    debug_assert!(block.expressions.len() <= 1);

    // body
    if block.expressions.is_empty() {
        write!(f, [token("{"), token("}")])?;
    } else {
        write!(
            f,
            [
                token("{"),
                soft_line_break_or_space(),
                soft_block_indent(&format_args![
                    &block.expressions[0],
                    f.context().block_infix_annotations(block_id)
                ]),
                soft_line_break_or_space(),
                token("}")
            ]
        )?;
    }
    Ok(())
}

/// Format a block multiline with multiple expressions (including label and infix annotations).
/// Format block contents with expanded inner spacing.
#[inline]
pub(crate) fn format_block_body_wide<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    let allow_value_tail = block_allows_value_tail(f.context(), block_id);
    // body
    write!(
        f,
        [
            token("{"),
            hard_line_break(),
            soft_block_indent(&format_with(|f| format_block_of_statements(
                f,
                &block.expressions,
                allow_value_tail,
            ))),
            hard_line_break(),
            block_indent(&f.context().block_infix_annotations(block_id)),
            token("}"),
        ]
    )
}

/// Format a block statement body with statement-level spacing and ignore handling.
pub(crate) fn format_block_of_statements<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
    allow_value_tail: bool,
) -> FormatResult<()> {
    let _timing = f.context().timing_scope(tags::FORMAT_BLOCK_STATEMENTS);
    let organize = f.context().options.organize_imports.is_enabled();
    let tree = f.context().tree;
    let strings = f.context().strings;
    // ignore ranges: only compute when the file may contain ignore directives
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        collect_ignore_ranges_for_nodes(f.context(), expressions, comment_tokens)
    } else {
        std::collections::HashMap::new()
    };
    let has_ignore_ranges = !ignore_ranges.is_empty();

    // find contiguous import section at the start
    let import_count = expressions
        .iter()
        .take_while(|&&expr_id| imports::is_import(expr_id, tree))
        .count();

    // prepare the expression list (potentially with sorted imports)
    let sorted_imports: Vec<LocalNodeId<Expression>>;
    let effective_expressions: Cow<'_, [LocalNodeId<Expression>]> =
        if organize && import_count > 1 && !has_ignore_ranges {
            sorted_imports = imports::sort_imports(&expressions[..import_count], tree, strings);
            Cow::Owned(
                sorted_imports
                    .iter()
                    .copied()
                    .chain(expressions[import_count..].iter().copied())
                    .collect(),
            )
        } else {
            Cow::Borrowed(expressions)
        };

    let directive_count = effective_expressions
        .iter()
        .take_while(|&&expr_id| is_directive_expression(tree, expr_id))
        .count();
    let insert_blank_after_directive_prologue = if directive_count == 0 {
        false
    } else {
        let last_directive_expression = effective_expressions[directive_count - 1];
        !expression_has_effective_prefix_annotation(f.context(), last_directive_expression)
            && !expression_has_effective_postfix_annotation(f.context(), last_directive_expression)
    };

    let mut prev_was_import = false;
    let mut prev_import_id: Option<LocalNodeId<Expression>> = None;
    let mut previous_output_end: Option<(FileId, u32)> = None;
    let mut skip_until: Option<u32> = None;

    for (i, &expression_id) in effective_expressions.iter().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let is_import_expr = imports::is_import(expression_id, tree);
        let ignore_range = ignore_ranges.get(&expression_id.id).copied();
        let has_ignore_range = ignore_range.is_some();

        let expression_span = f.context().span(expression_id);

        if let Some(skip_end) = skip_until {
            if expression_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between expressions
        if i > 0 {
            let previous_expression_id = effective_expressions[i - 1];
            let has_blank_prefix_annotation =
                expression_has_effective_blank_prefix_annotation(f.context(), expression_id);
            let has_non_comment_prefix_annotation =
                expression_has_effective_non_comment_prefix_annotation(f.context(), expression_id);
            let previous_has_postfix_annotation =
                expression_has_effective_postfix_annotation(f.context(), previous_expression_id);
            let previous_has_blank_postfix_annotation =
                expression_has_effective_blank_postfix_annotation(
                    f.context(),
                    previous_expression_id,
                );
            let source_has_blank_line_between = if has_ignore_range {
                if let Some((previous_file, previous_end)) = previous_output_end {
                    if previous_file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_end,
                            range_start,
                        )
                    }
                } else {
                    let previous_span = f.context().span(previous_expression_id);
                    if previous_span.file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_span.end,
                            range_start,
                        )
                    }
                }
            } else {
                expressions_have_blank_line_between(
                    f.context(),
                    previous_expression_id,
                    expression_id,
                )
            };
            let uses_source_blank_line_without_leading_break = source_has_blank_line_between
                && !has_blank_prefix_annotation
                && !has_non_comment_prefix_annotation
                && !previous_has_postfix_annotation
                && !has_ignore_range;
            if !has_ignore_range {
                if !has_blank_prefix_annotation
                    && !uses_source_blank_line_without_leading_break
                    && !previous_has_blank_postfix_annotation
                {
                    write!(f, [hard_line_break()])?;
                }

                // determine if we need an extra blank line
                let needs_blank = if directive_count > 0 && i == directive_count {
                    insert_blank_after_directive_prologue && !has_blank_prefix_annotation
                } else if organize && prev_was_import && is_import_expr {
                    // check if different import groups
                    prev_import_id.is_some_and(|prev_id| {
                        imports::should_insert_blank_between(
                            prev_id,
                            expression_id,
                            f.context().tree,
                            f.context().strings,
                        )
                    })
                } else if prev_was_import && !is_import_expr {
                    // blank line after import section (if not already present)
                    !has_blank_prefix_annotation && !previous_has_blank_postfix_annotation
                } else if source_has_blank_line_between {
                    !has_blank_prefix_annotation
                        && !has_non_comment_prefix_annotation
                        && !previous_has_postfix_annotation
                } else {
                    false
                };

                if needs_blank {
                    write!(f, [empty_line()])?;
                }
            }
        }

        if let Some(range_span) = ignore_range {
            let prefix_start = if let Some((previous_file, previous_end)) = previous_output_end {
                if previous_file == range_span.file {
                    previous_end
                } else {
                    expression_prefix_start(f.context(), expression_id, expression_span.start)
                }
            } else if i > 0 {
                let previous_expression_id = effective_expressions[i - 1];
                let previous_span = f.context().span(previous_expression_id);
                if previous_span.file == range_span.file {
                    previous_span.end
                } else {
                    expression_prefix_start(f.context(), expression_id, expression_span.start)
                }
            } else {
                expression_prefix_start(f.context(), expression_id, expression_span.start)
            };
            if prefix_start < range_span.start {
                let prefix_span = Span::new(range_span.file, prefix_start, range_span.start);
                write_ignored_span(f, prefix_span)?;
            }

            write_ignored_span(f, range_span)?;
            skip_until = Some(range_span.end);
            prev_was_import = false;
            prev_import_id = None;
            previous_output_end = Some((range_span.file, range_span.end));
            continue;
        }

        let directive = directive_for_node(f.context(), expression_id);

        // expression itself (with prefix annotations)
        // lambda declaration line prefix comments are handled in declaration formatting
        let is_lambda_declaration_expression = matches!(
            expression,
            Expression::Declaration(declaration_id)
                if matches!(
                    tree.get(*declaration_id),
                    Declaration::Function { signature, .. }
                        if signature.kind == FunctionKind::Lambda
                )
        );
        if is_lambda_declaration_expression {
            write!(f, [f.context().block_prefix_annotations(expression_id)])?;
        } else {
            write!(f, [f.context().any_prefix_annotations(expression_id)])?;
        }
        format_expression(f, expression_id, expression, directive)?;

        // add statement terminators for statement-context expression forms
        let is_expression_context_tail = allow_value_tail && i + 1 == effective_expressions.len();
        let needs_statement_terminator = !is_expression_context_tail
            && (matches!(
                expression,
                Expression::Import { .. } | Expression::Let { .. } | Expression::Using { .. }
            ) || matches!(
                expression,
                Expression::While {
                    kind: WhileKind::DoWhile,
                    ..
                }
            ) || matches!(
                expression,
                Expression::Declaration(declaration_id)
                    if matches!(
                        tree.get(*declaration_id),
                        Declaration::Function {
                            descriptor,
                            signature,
                            ..
                        }
                        if descriptor.name.is_none() && signature.kind == FunctionKind::Lambda
                    )
            ) || (!matches!(expression, Expression::Statement(_))
                && !matches!(expression, Expression::Stub | Expression::Error)
                && !expression.ends_statement_on_newline()));
        if needs_statement_terminator {
            write!(f, [token(";")])?;
        }

        // postfix annotations
        let if_chain_handles_annotations = matches!(
            expression,
            Expression::If {
                kind: IfKind::If,
                ..
            }
        );
        if !if_chain_handles_annotations
            && !matches!(
                directive,
                Some(FormatterDirective {
                    kind: FormatterDirectiveKind::IgnoreFormat,
                    position: FormatterDirectivePosition::Postfix { .. },
                })
            )
        {
            write!(
                f,
                [f.context().any_infix_or_postfix_annotations(expression_id)]
            )?;
        }

        prev_was_import = is_import_expr;
        if is_import_expr {
            prev_import_id = Some(expression_id);
        }
        previous_output_end = Some((expression_span.file, expression_span.end));
    }
    Ok(())
}
