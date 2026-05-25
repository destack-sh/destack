use destack_dir as dir;

use crate::core::DirQueryContext;
use crate::dir::expression_symbol_target;
use crate::source::{
    member_access_dot_before_offset, receiver_token_before_member_access_dot,
    sorted_enclosing_spans, token_span_at_cursor_offset,
};

use super::{CompletionContext, CursorToken};

/// Detect member access context near the cursor.
pub(super) fn detect_member_access_context(
    ctx: DirQueryContext<'_>,
    token: &Option<CursorToken>,
    offset: u32,
) -> Option<CompletionContext> {
    let cursor_position = offset.saturating_sub(1);

    // detect member access inside an existing member name token
    if let Some(context) = member_access_context_from_member_name(ctx, cursor_position) {
        return Some(context);
    }

    // resolve member access context when immediately after one dot boundary
    if let Some(context) = member_access_context_from_dot(ctx, offset) {
        return Some(context);
    }

    // resolve member access when the cursor is inside a member name
    if let Some(token_at_cursor) = token.as_ref()
        && let Some(context) = member_access_context_from_dot(ctx, token_at_cursor.start)
    {
        return Some(context);
    }

    None
}

/// Detect member access from an existing member name token.
fn member_access_context_from_member_name(
    ctx: DirQueryContext<'_>,
    cursor_position: u32,
) -> Option<CompletionContext> {
    let token_at_cursor = token_span_at_cursor_offset(ctx, cursor_position)?;
    if token_at_cursor.token.ty() != dir::TokenType::Identifier {
        return None;
    }

    let dir_tree = ctx.view();

    // resolve enclosing spans from innermost to outermost
    let enclosing = sorted_enclosing_spans(ctx, cursor_position, cursor_position);

    // scan enclosing spans for one member expression at the cursor
    for enc in &enclosing {
        let main_span = ctx.tree().source_index.get_main(enc.source_id);
        let is_in_member_name = main_span
            .map(|span| span.contains(cursor_position))
            .unwrap_or(true);
        if !is_in_member_name {
            continue;
        }

        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.source_id) else {
            continue;
        };
        if dir_node_id.ty != dir::NodeType::Expression {
            continue;
        }

        let Ok(expr_id) = dir_node_id.try_into() else {
            continue;
        };
        let expr = dir_tree.get::<dir::Expression>(expr_id);

        // use the left operand when inside a member expression
        let dir::Expression::Member { left, .. } = expr else {
            continue;
        };

        let receiver_local: dir::LocalNodeIdAny = (*left).into();
        let receiver_global = receiver_local.into_global(ctx.module_id());
        let receiver_symbol = get_expression_symbol(ctx, *left);
        let receiver_type = get_receiver_type(ctx, receiver_global, receiver_symbol);

        return Some(CompletionContext::MemberAccess {
            receiver_node: receiver_local,
            receiver_symbol,
            receiver_type,
        });
    }

    None
}

/// Get the target symbol of an expression if it resolves to one.
fn get_expression_symbol(
    dir: DirQueryContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    expression_symbol_target(dir, expr_id)
}

/// Get the type of a receiver expression.
fn get_receiver_type(
    ctx: DirQueryContext<'_>,
    receiver_global: dir::GlobalNodeIdAny,
    receiver_symbol: Option<dir::GlobalSymbolId>,
) -> Option<dir::LocalTypeId> {
    let types = ctx.types();
    let node_type_id = types.get_node_type_id(receiver_global);
    let symbol_type_id = receiver_symbol.and_then(|symbol| types.get_symbol_type_id(symbol));

    node_type_id.or(symbol_type_id)
}

/// Resolve member access context for a receiver position.
fn member_access_context_at_offset(
    ctx: DirQueryContext<'_>,
    receiver_position: u32,
) -> Option<CompletionContext> {
    let enclosing = sorted_enclosing_spans(ctx, receiver_position, receiver_position);
    let dir_tree = ctx.view();
    let mut partial_context = None;

    // scan for the nearest enclosing expression
    for enc in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.source_id) else {
            continue;
        };
        if dir_node_id.ty != dir::NodeType::Expression {
            continue;
        }

        let Ok(expr_id) = dir_node_id.try_into() else {
            continue;
        };
        let expr = dir_tree.get::<dir::Expression>(expr_id);

        // prefer the member left operand as the receiver
        if let dir::Expression::Member { left, .. } = expr {
            let receiver_local: dir::LocalNodeIdAny = (*left).into();
            let receiver_global = receiver_local.into_global(ctx.module_id());
            let receiver_symbol = get_expression_symbol(ctx, *left);
            if receiver_symbol.is_none() {
                if partial_context.is_none() {
                    partial_context = Some(CompletionContext::MemberAccess {
                        receiver_node: receiver_local,
                        receiver_symbol,
                        receiver_type: None,
                    });
                }
                continue;
            }

            let receiver_type = get_receiver_type(ctx, receiver_global, receiver_symbol);

            return Some(CompletionContext::MemberAccess {
                receiver_node: receiver_local,
                receiver_symbol,
                receiver_type,
            });
        }

        // otherwise treat the expression itself as the receiver
        let receiver_symbol = expression_symbol_target(ctx, expr_id);
        if receiver_symbol.is_none() {
            if partial_context.is_none() {
                partial_context = Some(CompletionContext::MemberAccess {
                    receiver_node: dir_node_id,
                    receiver_symbol,
                    receiver_type: None,
                });
            }
            continue;
        }

        let receiver_global = dir_node_id.into_global(ctx.module_id());
        let receiver_type = get_receiver_type(ctx, receiver_global, receiver_symbol);

        return Some(CompletionContext::MemberAccess {
            receiver_node: dir_node_id,
            receiver_symbol,
            receiver_type,
        });
    }

    partial_context
}

/// Resolve member access context from one dot owned cursor.
fn member_access_context_from_dot(
    ctx: DirQueryContext<'_>,
    offset: u32,
) -> Option<CompletionContext> {
    let dot = member_access_dot_before_offset(ctx, offset)?;
    let receiver_token = receiver_token_before_member_access_dot(ctx, dot)?;
    let receiver_offset = receiver_token.span.end.saturating_sub(1);

    member_access_context_at_offset(ctx, receiver_offset)
}
