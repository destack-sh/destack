use destack_source::{EnclosingSpan, Span};
use {destack_ast as ast, destack_dir as dir};

use crate::ast::{
    enclosing_spans_at_cursor, previous_significant_token, sorted_enclosing_spans, span_owns_cursor,
};
use crate::core::{AstQuery, DirQuery};
use crate::dir::{
    ScopeAtOffset, block_scope_at_offset, expression_scope_at_offset, scope_at_offset,
};

use super::CompletionContext;

/// Detect whether the cursor is in a new expression context.
pub(super) fn detect_new_expression_context(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve enclosing spans around the cursor boundary
    let enclosing = enclosing_spans_at_cursor(ast, offset);

    // scan spans for a new expression containing the cursor
    for enc in &enclosing {
        if let Some(context) = new_expression_context_for_span(ast, dir, enc, offset) {
            return Some(context);
        }
    }

    None
}

/// Detect whether the cursor is in a call argument context.
pub(super) fn detect_call_argument_context(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve enclosing spans at the cursor
    let enclosing = sorted_enclosing_spans(ast, offset, offset);

    // bail out when there are no spans
    if enclosing.is_empty() {
        return None;
    }

    let dir_tree = dir.tree();

    // scan spans for a call or new expression argument list
    for enc in &enclosing {
        if let Some(context) = call_argument_context_for_span(ast, dir, dir_tree, enc, offset) {
            return Some(context);
        }
    }

    // fall back to token based separator ownership inside a call
    if let Some(separator) = previous_significant_token(ast, offset)
        && matches!(
            separator.token.ty,
            ast::TokenType::OpenParenthesis | ast::TokenType::Comma
        )
        && let Some(context) =
            call_argument_context_after_separator(ast, dir, dir_tree, offset, separator.span.start)
    {
        return Some(context);
    }

    None
}

/// Build a new expression context from a single enclosing span.
fn new_expression_context_for_span(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<CompletionContext> {
    // only expression spans can own one `new` constructor region
    if ast.tree().get_node_type(enc.idx) != ast::NodeType::Expression {
        return None;
    }

    let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
    let ast_tree = ast.tree();
    let (_expr_id, expr) = unwrap_statement_ast_expression(ast_tree, expr_id);
    let ast::Expression::New { left, .. } = expr else {
        return None;
    };

    // only explicit constructor text belongs to this path
    if matches!(ast_tree.get(*left), ast::Expression::Missing) {
        return None;
    }

    // only the constructor side should classify as one `new` completion position
    let left_span = ast_tree.source_map.get(left.id);
    if !span_owns_cursor(left_span, offset) {
        return None;
    }

    let scope = scope_at_offset(ast, dir, offset);

    Some(CompletionContext::NewExpression {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    })
}

/// Unwrap statement expressions to the underlying inner expression.
fn unwrap_statement_ast_expression(
    ast_tree: &ast::NodeTree,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> (ast::LocalNodeId<ast::Expression>, &ast::Expression) {
    let expr = ast_tree.get(expr_id);
    if let ast::Expression::Statement(inner) = expr {
        return unwrap_statement_ast_expression(ast_tree, *inner);
    }

    (expr_id, expr)
}

/// Build a call argument context from a single enclosing span.
fn call_argument_context_for_span(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    dir_tree: &dir::NodeTree,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<CompletionContext> {
    let dir_node_id = dir_tree.get_node_id_by_source_id(enc.idx)?;
    if dir_node_id.ty != dir::NodeType::Expression {
        return None;
    }

    let Ok(expr_id) = dir_node_id.try_into() else {
        return None;
    };

    let expr: &dir::Expression = dir_tree.get(expr_id);
    let (left, dynamic_arguments) = match expr {
        dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } => (left, dynamic_arguments.as_slice()),
        dir::Expression::New {
            left,
            dynamic_arguments,
            ..
        } => (left, dynamic_arguments.as_slice()),
        _ => return None,
    };

    let left_node_id: dir::LocalNodeIdAny = (*left).into();
    let left_source_id = dir_tree.get_source(left_node_id.id);
    let left_span = ast.tree().source_map.get(left_source_id);
    let call_span = ast.tree().source_map.get(enc.idx);

    // only the argument list belongs to this path
    if !cursor_in_argument_list(
        ast,
        dir_tree,
        dynamic_arguments,
        left_span,
        call_span,
        offset,
    ) {
        return None;
    }

    let scope = expression_scope_at_offset(ast, dir, expr_id, offset);
    Some(CompletionContext::CallArgument {
        scope_id: Some(scope.scope_id),
        scope_mark: Some(scope.scope_mark),
    })
}

/// Build a call argument context from a separator position inside a call.
fn call_argument_context_after_separator(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    dir_tree: &dir::NodeTree,
    offset: u32,
    separator_position: u32,
) -> Option<CompletionContext> {
    let lookup_position = separator_position.saturating_sub(1);
    let enclosing = sorted_enclosing_spans(ast, lookup_position, lookup_position);

    // prefer dir backed call shapes first
    for enc in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };
        if dir_node_id.ty != dir::NodeType::Expression {
            continue;
        }

        let Ok(expr_id) = dir_node_id.try_into() else {
            continue;
        };

        let expr: &dir::Expression = dir_tree.get(expr_id);
        let left = match expr {
            dir::Expression::Call { left, .. } | dir::Expression::New { left, .. } => left,
            _ => continue,
        };

        let left_node_id: dir::LocalNodeIdAny = (*left).into();
        let left_source_id = dir_tree.get_source(left_node_id.id);
        let left_span = ast.tree().source_map.get(left_source_id);

        if separator_position <= left_span.end {
            continue;
        }

        let scope = expression_scope_at_offset(ast, dir, expr_id, offset);
        return Some(CompletionContext::CallArgument {
            scope_id: Some(scope.scope_id),
            scope_mark: Some(scope.scope_mark),
        });
    }

    // fall back to ast shape when partial DIR does not preserve the call
    for enc in &enclosing {
        if ast.tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ast.tree().get(expr_id);

        let left = match expr {
            ast::Expression::Call { left, .. } | ast::Expression::New { left, .. } => left,
            _ => continue,
        };

        let left_span = ast.tree().source_map.get(left.id);
        if separator_position <= left_span.end {
            continue;
        }

        let scope = call_argument_scope_from_offsets(ast, dir, &[offset, lookup_position])?;
        return Some(CompletionContext::CallArgument {
            scope_id: Some(scope.scope_id),
            scope_mark: Some(scope.scope_mark),
        });
    }

    None
}

/// Resolve a call argument scope from a small set of nearby offsets.
fn call_argument_scope_from_offsets(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offsets: &[u32],
) -> Option<ScopeAtOffset> {
    for &offset in offsets {
        if let Some(scope) = block_scope_at_offset(ast, dir, offset) {
            return Some(normalize_call_argument_scope(scope));
        }
    }

    for &offset in offsets {
        if let Some(scope) = scope_at_offset(ast, dir, offset) {
            return Some(normalize_call_argument_scope(scope));
        }
    }

    None
}

/// Normalize the fallback scope used for one call argument position.
fn normalize_call_argument_scope(scope: ScopeAtOffset) -> ScopeAtOffset {
    let scope_mark = if scope.scope_mark == dir::LocalScopeMark(0) {
        dir::LocalScopeMark::end()
    } else {
        scope.scope_mark
    };

    ScopeAtOffset {
        scope_id: scope.scope_id,
        scope_mark,
    }
}

/// Check whether the cursor is inside a call argument list.
fn cursor_in_argument_list(
    ast: AstQuery<'_>,
    dir_tree: &dir::NodeTree,
    arguments: &[dir::LocalNodeId<dir::Argument>],
    left_span: Span,
    call_span: Span,
    offset: u32,
) -> bool {
    // check argument spans when arguments are present
    if !arguments.is_empty() {
        let mut min_start = u32::MAX;
        let mut max_end = 0u32;

        for argument_id in arguments {
            let arg_node_id: dir::LocalNodeIdAny = (*argument_id).into();
            let span = crate::ast::span_for_dir_node(ast, dir_tree, arg_node_id);
            min_start = min_start.min(span.start);
            max_end = max_end.max(span.end);
        }

        if offset >= min_start && offset <= max_end {
            return true;
        }
    }

    offset > left_span.end && offset <= call_span.end
}
