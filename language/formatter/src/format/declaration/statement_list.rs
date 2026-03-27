use std::borrow::Cow;

use crate::Annotation;
use crate::format::annotation::{
    annotation_render_items_matching, expression_needs_statement_terminator,
    write_annotation_render_items,
};
use crate::format::directive::{
    ignore_ranges_for_nodes, node_has_ignore_directive, write_ignored_span,
};
use crate::format::expression::format_expression;
use destack_ast::{
    AnnotationPosition, Block, BlockContext, Declaration, Expression, FunctionKind, FunctionMode,
    IfCondition, IfKind, LocalNodeId, Member, NodeType, Property,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::{FileId, Span};

use crate::format::declaration::dependency as imports;
use crate::{DestackFormatContext, DestackFormatter};

/// Return whether trivia between two expressions contains one explicit blank line.
fn expressions_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    left_expression_id: LocalNodeId<Expression>,
    right_expression_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_expression_id);
    let right_span = context.span(right_expression_id);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };
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
    let mut start = default_start;
    context.visit_annotations(expression_id, |annotation_ids| {
        for annotation_id in annotation_ids {
            let Annotation::Comment { node, position } = context.annotation(*annotation_id) else {
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
    });

    start
}

/// Return the latest end offset for postfix comment annotations on an expression.
fn expression_postfix_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    default_end: u32,
) -> u32 {
    let mut end = default_end;
    context.visit_annotations(expression_id, |annotation_ids| {
        for annotation_id in annotation_ids {
            let annotation = context.annotation(*annotation_id);
            if !matches!(
                annotation.position(),
                AnnotationPosition::BlockPostfix
                    | AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
            ) {
                continue;
            }

            let annotation_span = context.annotation_span(*annotation_id);
            end = end.max(annotation_span.end);
        }
    });

    end
}

/// Write postfix annotations for one block expression.
fn write_expression_postfix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
    is_ignored: bool,
    use_statement_inner_annotations: bool,
) -> FormatResult<()> {
    if use_statement_inner_annotations && let Expression::Statement(statement_id) = expression {
        let statement_expression = f.context().tree.get(*statement_id);
        let statement_is_ignored = node_has_ignore_directive(f.context(), *statement_id);
        let if_chain_handles_annotations = matches!(
            statement_expression,
            Expression::If {
                kind: IfKind::If,
                ..
            }
        );
        if if_chain_handles_annotations || statement_is_ignored {
            return Ok(());
        }

        return write!(
            f,
            [f.context().any_infix_or_postfix_annotations(*statement_id)]
        );
    }

    let if_chain_handles_annotations = matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        }
    );
    if if_chain_handles_annotations || is_ignored {
        return Ok(());
    }

    write!(
        f,
        [f.context().any_infix_or_postfix_annotations(expression_id)]
    )
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

/// Return whether an expression or its declaration wrapper has blank postfix annotations.
fn expression_has_effective_blank_postfix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let has_blank_postfix_annotation = context
        .visit_annotations(expression_id, |annotation_ids| {
            annotation_ids.iter().copied().any(|annotation_id| {
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
            .visit_annotations(*declaration_id, |annotation_ids| {
                annotation_ids.iter().copied().any(|annotation_id| {
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
    let organize = f.context().options.organize_imports.is_enabled();
    let tree = f.context().tree;
    let strings = f.context().strings;
    // ignore ranges: only compute when the file may contain ignore directives
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        ignore_ranges_for_nodes(f.context(), expressions, comment_tokens)
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

    let mut prev_was_import = false;
    let mut prev_import_id: Option<LocalNodeId<Expression>> = None;
    let mut previous_output_end: Option<(FileId, u32)> = None;
    let mut skip_until: Option<u32> = None;

    for (i, &expression_id) in effective_expressions.iter().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let is_import_expr = imports::is_import(expression_id, tree);
        let ignore_range = ignore_ranges.get(&expression_id.id).copied();
        let has_ignore_range = ignore_range.is_some();
        let is_ignored = node_has_ignore_directive(f.context(), expression_id);

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
            let previous_has_blank_postfix_annotation =
                expression_has_effective_blank_postfix_annotation(
                    f.context(),
                    previous_expression_id,
                );
            let has_blank_prefix_annotation =
                expression_has_effective_blank_prefix_annotation(f.context(), expression_id);
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
                && !previous_has_blank_postfix_annotation
                && !has_ignore_range;
            if !has_ignore_range {
                if !has_blank_prefix_annotation
                    && !uses_source_blank_line_without_leading_break
                    && !previous_has_blank_postfix_annotation
                {
                    write!(f, [hard_line_break()])?;
                }

                // determine if we need an extra blank line
                let needs_blank = if organize && prev_was_import && is_import_expr {
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
                    !has_blank_prefix_annotation && !previous_has_blank_postfix_annotation
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
            let prefix_items =
                annotation_render_items_matching(f.context(), expression_id, |position| {
                    position == AnnotationPosition::BlockPrefix
                });
            write_annotation_render_items(f, &prefix_items)?;
        } else {
            write!(f, [f.context().any_prefix_annotations(expression_id)])?;
        }
        format_expression(f, expression_id, expression, is_ignored)?;

        // add statement terminators for statement-context expression forms
        let is_expression_context_tail = allow_value_tail && i + 1 == effective_expressions.len();
        if expression_needs_statement_terminator(
            f.context(),
            expression,
            is_expression_context_tail,
        ) {
            write!(f, [token(";")])?;
        }

        write_expression_postfix_annotations(f, expression_id, expression, is_ignored, false)?;

        prev_was_import = is_import_expr;
        if is_import_expr {
            prev_import_id = Some(expression_id);
        }
        let expression_output_end =
            expression_postfix_end(f.context(), expression_id, expression_span.end);
        previous_output_end = Some((expression_span.file, expression_output_end));
    }
    Ok(())
}

/// Return true when the final expression in this block is value-position.
pub(crate) fn block_allows_value_tail(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    if block.context != BlockContext::Expression {
        return false;
    }

    let Some((block_expression_id, block_expression_type)) = context.parent(block_id) else {
        return false;
    };
    if block_expression_type == NodeType::MatchCase {
        return true;
    }
    if block_expression_type != NodeType::Expression {
        return false;
    }

    let block_expression_id = LocalNodeId::<Expression>::new(block_expression_id);
    let Expression::Block(inner_block_id) = context.tree.get(block_expression_id) else {
        return false;
    };
    if *inner_block_id != block_id {
        return false;
    }

    !expression_is_in_statement_position(context, block_expression_id)
}

/// Return true when this expression is in statement position.
fn expression_is_in_statement_position(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return true;
    };

    match parent_type {
        NodeType::Expression => {
            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let parent_expression = context.tree.get(parent_expression_id);
            if matches!(
                parent_expression,
                Expression::Statement(inner_expression_id) if *inner_expression_id == expression_id
            ) {
                return true;
            }

            let should_inherit_parent_position = match parent_expression {
                Expression::If {
                    condition,
                    then_expression,
                    else_expression,
                    ..
                } => {
                    let branch_inherits_statement_position =
                        if_condition_inherits_statement_position(context, condition);

                    branch_inherits_statement_position
                        && (then_expression.id == expression_id.id
                            || else_expression.as_ref().is_some_and(|else_expression| {
                                else_expression.id == expression_id.id
                            }))
                }
                Expression::While { body, .. }
                | Expression::ForEach { body, .. }
                | Expression::For { body, .. }
                | Expression::Loop { body } => body.id == expression_id.id,
                Expression::Try {
                    try_expression,
                    catch_expression,
                    finally_expression,
                    ..
                } => {
                    try_expression.id == expression_id.id
                        || catch_expression
                            .as_ref()
                            .is_some_and(|catch_expression| catch_expression.id == expression_id.id)
                        || finally_expression
                            .as_ref()
                            .is_some_and(|finally_expression| {
                                finally_expression.id == expression_id.id
                            })
                }
                Expression::Labelled { body, .. } => body.id == expression_id.id,
                _ => false,
            };

            should_inherit_parent_position
                && expression_is_in_statement_position(context, parent_expression_id)
        }
        NodeType::Block => expression_is_in_statement_position_inside_parent_block(
            context,
            LocalNodeId::<Block>::new(parent_id),
            expression_id,
        ),
        NodeType::Declaration => expression_is_in_statement_position_inside_parent_declaration(
            context,
            LocalNodeId::<Declaration>::new(parent_id),
            expression_id,
        ),
        NodeType::Member => expression_is_in_statement_position_inside_parent_member(
            context,
            LocalNodeId::<Member>::new(parent_id),
            expression_id,
        ),
        NodeType::Property => expression_is_in_statement_position_inside_parent_property(
            context,
            LocalNodeId::<Property>::new(parent_id),
            expression_id,
        ),
        _ => false,
    }
}

/// Return true when one condition should inherit statement-position from its parent `if`.
fn if_condition_inherits_statement_position(
    context: &DestackFormatContext<'_>,
    condition: &IfCondition,
) -> bool {
    match condition {
        // `if let` branches should preserve expression tails
        IfCondition::Let { .. } => false,
        // `if (comptime ...)` branches should preserve expression tails
        IfCondition::Expression { condition } => {
            !matches!(context.tree.get(*condition), Expression::Comptime { .. })
        }
    }
}

/// Return true when one child expression is statement-position inside one parent block.
fn expression_is_in_statement_position_inside_parent_block(
    context: &DestackFormatContext<'_>,
    parent_block_id: LocalNodeId<Block>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_block = context.tree.get(parent_block_id);

    if parent_block.context == BlockContext::Statement {
        return true;
    }

    let is_last_expression = parent_block
        .expressions
        .last()
        .is_some_and(|last_expression_id| *last_expression_id == expression_id);
    if !is_last_expression {
        return true;
    }

    !block_allows_value_tail(context, parent_block_id)
}

/// Return true when one child expression is statement-position inside one parent declaration.
fn expression_is_in_statement_position_inside_parent_declaration(
    context: &DestackFormatContext<'_>,
    parent_declaration_id: LocalNodeId<Declaration>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_declaration = context.tree.get(parent_declaration_id);

    match parent_declaration {
        Declaration::Function {
            body, signature, ..
        } => body.as_ref().is_some_and(|body_expression_id| {
            if body_expression_id.id != expression_id.id {
                return false;
            }

            function_body_is_statement_position(signature.mode)
                && !function_has_self_return_type(context, signature.return_type)
        }),
        Declaration::Global { expressions, .. } | Declaration::Namespace { expressions, .. } => {
            expressions.contains(&expression_id)
        }
        _ => false,
    }
}

/// Return true when one child expression is statement-position inside one parent member.
fn expression_is_in_statement_position_inside_parent_member(
    context: &DestackFormatContext<'_>,
    parent_member_id: LocalNodeId<Member>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_member = context.tree.get(parent_member_id);

    match parent_member {
        Member::Method { body, .. } => body
            .as_ref()
            .is_some_and(|body_expression_id| body_expression_id.id == expression_id.id),
        Member::StaticBlock { body, .. } | Member::ComptimeBlock { body, .. } => {
            body.id == expression_id.id
        }
        _ => false,
    }
}

/// Return true when one child expression is statement-position inside one parent property method.
fn expression_is_in_statement_position_inside_parent_property(
    context: &DestackFormatContext<'_>,
    parent_property_id: LocalNodeId<Property>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_property = context.tree.get(parent_property_id);

    match parent_property {
        Property::Method { body, .. } => body
            .as_ref()
            .is_some_and(|body_expression_id| body_expression_id.id == expression_id.id),
        _ => false,
    }
}

/// Return true when one function-like body should be statement-position.
fn function_body_is_statement_position(mode: Option<FunctionMode>) -> bool {
    if matches!(mode, Some(FunctionMode::Constructor | FunctionMode::Setter)) {
        return true;
    }

    true
}

/// Return true when one function return type is exactly `Self`.
fn function_has_self_return_type(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(return_type_id) = return_type else {
        return false;
    };

    expression_is_self_type_path(context, return_type_id)
}

/// Return true when one expression is a simple `Self` type path.
fn expression_is_self_type_path(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Path { path, .. } => {
            path.segments.len() == 1 && context.strings.get(path.segments[0]) == "Self"
        }
        Expression::Parenthesized { expression } => {
            expression_is_self_type_path(context, *expression)
        }
        _ => false,
    }
}
