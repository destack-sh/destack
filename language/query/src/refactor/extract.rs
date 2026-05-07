use destack_dir::{self as dir};
use destack_source::Span;

use crate::ast::{line_start_for_offset, span_contains_span, span_for_dir_node};
use crate::core::QueryContext;

/// Check whether an expression can be extracted into a refactor target.
pub(crate) fn is_extractable_expression(expression: &dir::Expression) -> bool {
    !matches!(
        expression,
        dir::Expression::Let { .. }
            | dir::Expression::LetElse { .. }
            | dir::Expression::Using { .. }
            | dir::Expression::Declaration { .. }
            | dir::Expression::Block { .. }
            | dir::Expression::Import { .. }
            | dir::Expression::ReExport { .. }
            | dir::Expression::Export { .. }
            | dir::Expression::ExportNamespace { .. }
    )
}

/// Resolve the extractable expression at a selection span.
pub(crate) fn resolve_extract_expression(
    ctx: &QueryContext,
    selection: Span,
) -> Option<(dir::LocalNodeId<dir::Expression>, Span)> {
    // scan expressions for the smallest span that contains the selection
    let dir_tree = ctx.dir().tree();
    let mut best: Option<(dir::LocalNodeId<dir::Expression>, Span)> = None;
    let mut best_len = u32::MAX;

    for (expr_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let span = span_for_dir_node(ctx.ast(), dir_tree, expr_id.into());
        if !span_contains_span(span, selection) {
            continue;
        }

        if !is_extractable_expression(expression) {
            continue;
        }

        let length = span.end.saturating_sub(span.start);
        if length < best_len {
            best = Some((expr_id, span));
            best_len = length;
        }
    }

    best
}

/// Resolve the statement span that owns an expression.
pub(crate) fn statement_span_for_expression(
    ctx: &QueryContext,
    expr_id: dir::LocalNodeId<dir::Expression>,
    fallback: Span,
) -> Span {
    // walk parent expressions until we find a statement boundary
    let dir_tree = ctx.dir().tree();
    let mut current = dir::LocalNodeIdAny::from(expr_id);

    while let Some(parent) = dir_tree.get_parent(current.id) {
        if parent.ty == dir::NodeType::Expression {
            let Ok(parent_id) = parent.try_into() else {
                current = parent;
                continue;
            };
            let parent_expression = dir_tree.get::<dir::Expression>(parent_id);
            if matches!(
                parent_expression,
                dir::Expression::Let { .. }
                    | dir::Expression::LetElse { .. }
                    | dir::Expression::Using { .. }
                    | dir::Expression::Declaration { .. }
            ) {
                return span_for_dir_node(ctx.ast(), dir_tree, parent);
            }
        }

        current = parent;
    }

    fallback
}

/// Resolve the line start offset and indentation for a source position.
pub(crate) fn line_start_and_indent(source: &str, offset: u32) -> (u32, String) {
    // locate the start of the line
    let line_start = line_start_for_offset(source, offset as usize);

    // collect indentation from the line prefix
    let mut indent = String::new();
    for ch in source[line_start..offset as usize].chars() {
        if ch.is_whitespace() && ch != '\n' && ch != '\r' {
            indent.push(ch);
        } else {
            break;
        }
    }

    (line_start as u32, indent)
}

/// Clean expression text for insertion in generated code.
pub(crate) fn clean_expression_text(text: &str) -> String {
    // trim whitespace and trailing semicolons
    let trimmed = text.trim();
    let trimmed = trimmed.strip_suffix(';').unwrap_or(trimmed).trim_end();

    trimmed.to_string()
}
