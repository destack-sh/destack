use std::collections::HashSet;

use destack_artifact::ImportEdgeKind;
use destack_source::{EnclosingSpan, FileId, ModuleId, Span};
use destack_workspace::Session;
use {destack_ast as ast, destack_dir as dir};

use crate::common::{
    ExpressionSlotPosition, ObjectLiteralCursorContext, QueryContext, ScopeAtOffset,
    block_statement_position, enclosing_missing_expression, enclosing_spans_at_cursor_boundary,
    enclosing_spans_with_previous, expression_scope_at_offset, expression_slot_position,
    extract_string_literal_prefix, get_module_by_file_id, import_clause_bounds,
    is_inside_object_literal_expression, member_access_dot_before_offset,
    missing_declarator_value_at_cursor, object_literal_cursor_context,
    offset_is_in_ast_type_side_span, previous_significant_token,
    receiver_token_before_member_access_dot, resolve_expression_symbol, scope_at_offset,
    scope_from_block_span, sorted_enclosing_spans, span_for_dir_node, span_owns_cursor_boundary,
    token_span_at_cursor_offset, token_text,
};

/// Describes the context for a completion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompletionContext {
    /// Member access context such as `value.`.
    MemberAccess {
        /// The receiver node id.
        receiver_node: dir::LocalNodeIdAny,
        /// The resolved receiver symbol when available.
        receiver_symbol: Option<dir::GlobalSymbolId>,
        /// The resolved receiver type when available.
        receiver_type: Option<dir::LocalTypeId>,
    },
    /// Type position context such as annotations or type expressions.
    TypePosition {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Value position context such as expressions and statements.
    ValuePosition {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Statement position context such as the start of a line in a block.
    StatementPosition {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Object literal key context inside `Type { ... }`.
    ObjectLiteral {
        /// The object expression node id.
        object_node: dir::LocalNodeIdAny,
        /// The contextual type when one is available.
        contextual_type: Option<dir::LocalTypeId>,
        /// Field names already present in the literal.
        existing_fields: Vec<String>,
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Object literal value context inside `Type { key: value }`.
    ObjectLiteralValue {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Call argument context inside `call(...)`.
    CallArgument {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// New expression context inside `new ...`.
    NewExpression {
        /// The scope id if one is available.
        scope_id: Option<dir::LocalScopeId>,
        /// The scope mark for ordering visible symbols.
        scope_mark: Option<dir::LocalScopeMark>,
    },
    /// Import path context inside string literals.
    ImportPath {
        /// The partial path typed so far.
        partial_path: String,
    },
    /// Import clause context inside `{ ... }` import lists.
    ImportClause {
        /// The target module id when one is available.
        target_module: Option<ModuleId>,
        /// Names already present in the import clause.
        existing_names: Vec<String>,
        /// Optional namespace filter for the clause.
        space_filter: Option<dir::SymbolSpace>,
    },
    /// Explicitly suppressed completion context.
    Suppressed,
    /// Fallback context when no other context matches.
    Unknown,
}

/// A token extracted at the cursor for prefix matching.
#[derive(Debug, Clone)]
pub(crate) struct TokenAtCursor {
    /// The token text.
    pub(crate) text: String,
    /// The start offset of the token.
    pub(crate) start: u32,
}

/// The detected completion context plus token data.
#[derive(Debug, Clone)]
pub(crate) struct ContextResult {
    /// The completion context.
    pub(crate) context: CompletionContext,
    /// The token at the cursor when available.
    pub(crate) token: Option<TokenAtCursor>,
}

/// Create an unknown completion context result.
fn unknown_context() -> ContextResult {
    ContextResult {
        context: CompletionContext::Unknown,
        token: None,
    }
}

/// Build a value position context from an optional scope.
fn value_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    CompletionContext::ValuePosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Build a type position context from an optional scope.
fn type_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    CompletionContext::TypePosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Build a completion context from a structural expression slot position.
fn context_from_expression_slot_position(
    position: ExpressionSlotPosition,
    scope: Option<ScopeAtOffset>,
) -> CompletionContext {
    match position {
        ExpressionSlotPosition::Constructor => CompletionContext::NewExpression {
            scope_id: scope.map(|scope| scope.scope_id),
            scope_mark: scope.map(|scope| scope.scope_mark),
        },
        ExpressionSlotPosition::Type => type_context_from_scope(scope),
        ExpressionSlotPosition::Value => value_context_from_scope(scope),
    }
}

/// Detect member access context near the cursor.
fn detect_member_access_context(
    ctx: &QueryContext<'_>,
    token: &Option<TokenAtCursor>,
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
    ctx: &QueryContext<'_>,
    cursor_position: u32,
) -> Option<CompletionContext> {
    let token_at_cursor = token_span_at_cursor_offset(ctx, cursor_position)?;
    if token_at_cursor.token.ty != ast::TokenType::Identifier {
        return None;
    }

    let ast = ctx.ast_context();
    let dir_tree = ctx.tree();

    // resolve enclosing spans from innermost to outermost
    let enclosing = sorted_enclosing_spans(ctx, cursor_position, cursor_position);

    // scan enclosing spans for one member expression at the cursor
    for enc in &enclosing {
        let main_span = ast.tree().source_map.get_main(enc.idx);
        let is_in_member_name = main_span
            .map(|span| span.contains(cursor_position))
            .unwrap_or(true);
        if !is_in_member_name {
            continue;
        }

        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
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
        let receiver_global = receiver_local.into_global(ctx.module_id);
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

/// Detect whether completion should stay suppressed at the cursor.
fn is_suppressed_completion_position(
    ctx: &QueryContext<'_>,
    token: &Option<TokenAtCursor>,
    offset: u32,
) -> bool {
    if token.is_some() {
        return false;
    }

    enclosing_missing_expression(ctx, offset).is_some()
        && expression_slot_position(ctx, offset).is_none()
}

/// Detect the completion context at a given offset.
pub(crate) fn detect_completion_context(
    session: &Session,
    file_id: FileId,
    offset: u32,
) -> ContextResult {
    // resolve the module for this file
    let Some(module) = get_module_by_file_id(session, file_id) else {
        return unknown_context();
    };

    // resolve the query context from the module
    let module = module.as_ref();
    let Some(ctx) = crate::query_context(session, module) else {
        return unknown_context();
    };

    // read the source text for this file
    let source_file = session.files.get(ctx.file_id);
    let source = source_file.text();

    // resolve token prefix at the cursor
    let token = detect_partial_identifier(&ctx, source, offset);

    // member access stays first because it is the most specific value-position context
    if let Some(context) = detect_member_access_context(&ctx, &token, offset) {
        return ContextResult { context, token };
    }

    // check object literal context before type position to avoid comma misclassification
    if let Some(object_context) = object_literal_cursor_context(&ctx, offset, session) {
        let context = match object_context {
            ObjectLiteralCursorContext::Key(object_context) => CompletionContext::ObjectLiteral {
                object_node: object_context.object_node,
                contextual_type: object_context.contextual_type,
                existing_fields: object_context.existing_fields,
                scope_id: Some(object_context.scope.scope_id),
                scope_mark: Some(object_context.scope.scope_mark),
            },
            ObjectLiteralCursorContext::Value(scope) => CompletionContext::ObjectLiteralValue {
                scope_id: Some(scope.scope_id),
                scope_mark: Some(scope.scope_mark),
            },
        };

        return ContextResult { context, token };
    }

    // check for explicit constructor typing before call arguments
    if let Some(new_context) = detect_new_expression_context(&ctx, offset) {
        return ContextResult {
            context: new_context,
            token,
        };
    }

    // check for call argument context
    if let Some(call_context) = detect_call_argument_context(&ctx, offset) {
        return ContextResult {
            context: call_context,
            token,
        };
    }

    // check for import context
    if let Some(import_context) = detect_import_context(session, &ctx, source, offset) {
        return ContextResult {
            context: import_context,
            token,
        };
    }

    // check for type position via ast spans
    if detect_type_position(&ctx, source, offset) {
        let scope = scope_at_offset(&ctx, offset);
        return ContextResult {
            context: type_context_from_scope(scope),
            token,
        };
    }

    // treat structurally classified missing expression slots as real completion positions
    if let Some(position) = expression_slot_position(&ctx, offset) {
        let scope = scope_at_offset(&ctx, offset);
        let context = context_from_expression_slot_position(position, scope);

        return ContextResult { context, token };
    }

    // check for statement position
    if let Some(statement_context) = detect_statement_position(&ctx, offset) {
        return ContextResult {
            context: statement_context,
            token,
        };
    }

    // keep naked missing expression holes suppressed until a richer context claims them
    if is_suppressed_completion_position(&ctx, &token, offset) {
        return ContextResult {
            context: CompletionContext::Suppressed,
            token,
        };
    }

    let scope = scope_at_offset(&ctx, offset);
    ContextResult {
        context: value_context_from_scope(scope),
        token,
    }
}

/// Get the target symbol of an expression if it resolves to one.
fn get_expression_symbol(
    ctx: &QueryContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    resolve_expression_symbol(ctx, expr_id)
}

/// Keep one concrete type id and skip unevaluated placeholders.
fn concrete_type_id(
    types: &dir::TypeTable,
    type_id: Option<dir::LocalTypeId>,
) -> Option<dir::LocalTypeId> {
    let type_id = type_id?;
    let ty = types.get_type(type_id);
    if ty.is_unevaluated() {
        return None;
    }

    Some(type_id)
}

/// Get the type of a receiver expression.
///
/// Prefer the compiler recorded member lookup type and fall back to existing type tables.
fn get_receiver_type(
    ctx: &QueryContext<'_>,
    receiver_global: dir::GlobalNodeIdAny,
    receiver_symbol: Option<dir::GlobalSymbolId>,
) -> Option<dir::LocalTypeId> {
    let types = ctx.types();
    let symbol_type_id = receiver_symbol
        .and_then(|receiver_symbol| types.get_type_id_for_symbol(ctx.symbols(), receiver_symbol));

    concrete_type_id(
        types,
        types
            .get_member_receiver_type_id_for_node(receiver_global)
            .or_else(|| types.get_declared_or_inferred_type_id(receiver_global))
            .or(symbol_type_id),
    )
}

/// Resolve member access context for a receiver position.
fn member_access_context_at_offset(
    ctx: &QueryContext<'_>,
    receiver_position: u32,
) -> Option<CompletionContext> {
    let enclosing = sorted_enclosing_spans(ctx, receiver_position, receiver_position);
    let dir_tree = ctx.tree();
    let mut partial_context = None;

    // scan for the nearest enclosing expression
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
        let expr = dir_tree.get::<dir::Expression>(expr_id);

        // unwrap statement expressions to the inner expression
        let (actual_node_id, actual_expr) =
            unwrap_statement_expression(dir_tree, dir_node_id, expr);

        // prefer the member left operand as the receiver
        if let dir::Expression::Member { left, .. } = actual_expr {
            let receiver_local: dir::LocalNodeIdAny = (*left).into();
            let receiver_global = receiver_local.into_global(ctx.module_id);
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
        let receiver_symbol = actual_expr.target_symbol();
        if receiver_symbol.is_none() {
            if partial_context.is_none() {
                partial_context = Some(CompletionContext::MemberAccess {
                    receiver_node: actual_node_id,
                    receiver_symbol,
                    receiver_type: None,
                });
            }
            continue;
        }

        let receiver_global = actual_node_id.into_global(ctx.module_id);
        let receiver_type = get_receiver_type(ctx, receiver_global, receiver_symbol);

        return Some(CompletionContext::MemberAccess {
            receiver_node: actual_node_id,
            receiver_symbol,
            receiver_type,
        });
    }

    partial_context
}

/// Resolve member access context from one dot-owned cursor boundary.
fn member_access_context_from_dot(
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<CompletionContext> {
    let dot = member_access_dot_before_offset(ctx, offset)?;
    let receiver_token = receiver_token_before_member_access_dot(ctx, dot)?;
    let receiver_offset = receiver_token.span.end.saturating_sub(1);

    member_access_context_at_offset(ctx, receiver_offset)
}

/// Unwrap statement expressions to get the inner expression.
///
/// Statement expressions wrap another expression with a `;` terminator.
/// When searching for member access context, we want the actual inner expression,
/// not the Statement wrapper, which has type void and no target symbol.
fn unwrap_statement_expression<'a>(
    dir_tree: &'a dir::NodeTree,
    node_id: dir::LocalNodeIdAny,
    expr: &'a dir::Expression,
) -> (dir::LocalNodeIdAny, &'a dir::Expression) {
    if let dir::Expression::Statement { statement } = expr {
        let inner_node_id: dir::LocalNodeIdAny = (*statement).into();
        let inner_expr = dir_tree.get::<dir::Expression>(*statement);
        return unwrap_statement_expression(dir_tree, inner_node_id, inner_expr);
    }

    (node_id, expr)
}

/// Detect partial identifier at the cursor position.
fn detect_partial_identifier(
    ctx: &QueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<TokenAtCursor> {
    // find the token under the cursor or immediately before it
    let token = match token_span_at_cursor_offset(ctx, offset) {
        Some(token) if token.token.ty == ast::TokenType::Identifier => token,
        _ => {
            let token = previous_significant_token(ctx, offset)?;
            if token.token.ty != ast::TokenType::Identifier {
                return None;
            }

            if token.span.end != offset {
                return None;
            }

            token
        }
    };

    // resolve the token span and prefix end
    let span = token.span;
    let prefix_end = offset.saturating_sub(span.start) as usize;

    // slice the token text from the source
    let text = token_text(source, span)?;
    let prefix_end = prefix_end.min(text.len());

    Some(TokenAtCursor {
        text: text[..prefix_end].to_string(),
        start: span.start,
    })
}

// ================================================================================
// call and new contexts
// ================================================================================

/// Detect whether the cursor is in a new expression context.
fn detect_new_expression_context(ctx: &QueryContext<'_>, offset: u32) -> Option<CompletionContext> {
    // resolve enclosing spans around the cursor boundary
    let enclosing = enclosing_spans_at_cursor_boundary(ctx, offset);

    // scan spans for a new expression containing the cursor
    for enc in &enclosing {
        if let Some(context) = new_expression_context_for_span(ctx, enc, offset) {
            return Some(context);
        }
    }

    None
}

/// Detect whether the cursor is in a call argument context.
fn detect_call_argument_context(ctx: &QueryContext<'_>, offset: u32) -> Option<CompletionContext> {
    // resolve enclosing spans at the cursor
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // bail out when there are no spans
    if enclosing.is_empty() {
        return None;
    }

    let dir_tree = ctx.tree();

    // scan spans for a call or new expression argument list
    for enc in &enclosing {
        if let Some(context) = call_argument_context_for_span(ctx, dir_tree, enc, offset) {
            return Some(context);
        }
    }

    // fall back to token-based separator ownership inside a call
    if let Some(separator) = previous_significant_token(ctx, offset)
        && matches!(
            separator.token.ty,
            ast::TokenType::OpenParenthesis | ast::TokenType::Comma
        )
        && let Some(context) =
            call_argument_context_after_separator(ctx, dir_tree, offset, separator.span.start)
    {
        return Some(context);
    }

    None
}

/// Build a new expression context from a single enclosing span.
fn new_expression_context_for_span(
    ctx: &QueryContext<'_>,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<CompletionContext> {
    // only expression spans can own one `new` constructor region
    if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
        return None;
    }

    let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
    let ast_tree = ctx.ast_context().tree();
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
    if !span_owns_cursor_boundary(left_span, offset) {
        return None;
    }

    let scope = scope_at_offset(ctx, offset);

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
    ctx: &QueryContext<'_>,
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
    let left_span = ctx.ast_context().tree().source_map.get(left_source_id);
    let call_span = ctx.ast_context().tree().source_map.get(enc.idx);

    // only the argument list belongs to this path
    if !cursor_in_argument_list(
        ctx,
        dir_tree,
        dynamic_arguments,
        left_span,
        call_span,
        offset,
    ) {
        return None;
    }

    let scope = expression_scope_at_offset(ctx, expr_id, offset);
    Some(CompletionContext::CallArgument {
        scope_id: Some(scope.scope_id),
        scope_mark: Some(scope.scope_mark),
    })
}

/// Build a call argument context from a separator position inside a call.
fn call_argument_context_after_separator(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    offset: u32,
    separator_position: u32,
) -> Option<CompletionContext> {
    let lookup_position = separator_position.saturating_sub(1);
    let enclosing = sorted_enclosing_spans(ctx, lookup_position, lookup_position);

    // prefer dir-backed call shapes first
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
        let left_span = ctx.ast_context().tree().source_map.get(left_source_id);

        if separator_position <= left_span.end {
            continue;
        }

        let scope = expression_scope_at_offset(ctx, expr_id, offset);
        return Some(CompletionContext::CallArgument {
            scope_id: Some(scope.scope_id),
            scope_mark: Some(scope.scope_mark),
        });
    }

    // fall back to ast shape when partial DIR does not preserve the call
    for enc in &enclosing {
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast_context().tree().get(expr_id);

        let left = match expr {
            ast::Expression::Call { left, .. } | ast::Expression::New { left, .. } => left,
            _ => continue,
        };

        let left_span = ctx.ast_context().tree().source_map.get(left.id);
        if separator_position <= left_span.end {
            continue;
        }

        let scope = call_argument_scope_from_offsets(ctx, &[offset, lookup_position])?;
        return Some(CompletionContext::CallArgument {
            scope_id: Some(scope.scope_id),
            scope_mark: Some(scope.scope_mark),
        });
    }

    None
}

/// Resolve a call-argument scope from a small set of nearby offsets.
fn call_argument_scope_from_offsets(
    ctx: &QueryContext<'_>,
    offsets: &[u32],
) -> Option<ScopeAtOffset> {
    for &offset in offsets {
        if let Some(scope) = crate::common::block_scope_at_offset(ctx, offset) {
            return Some(normalize_call_argument_scope(scope));
        }
    }

    for &offset in offsets {
        if let Some(scope) = scope_at_offset(ctx, offset) {
            return Some(normalize_call_argument_scope(scope));
        }
    }

    None
}

/// Normalize the fallback scope used for one call-argument position.
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
    ctx: &QueryContext<'_>,
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
            let span = span_for_dir_node(ctx, dir_tree, arg_node_id);
            min_start = min_start.min(span.start);
            max_end = max_end.max(span.end);
        }

        if offset >= min_start && offset <= max_end {
            return true;
        }
    }

    offset > left_span.end && offset <= call_span.end
}

// ================================================================================
// import contexts
// ================================================================================

/// Detect import related context.
fn detect_import_context(
    session: &Session,
    ctx: &QueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve enclosing spans from innermost to outermost
    let enclosing = enclosing_spans_at_cursor_boundary(ctx, offset);

    // scan enclosing expressions for import nodes under the cursor
    for enc in &enclosing {
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let mut expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let mut expr = ctx.ast_context().tree().get(expr_id);

        // unwrap transparent statement wrappers around recovered imports
        if let ast::Expression::Statement(inner) = expr {
            expr_id = *inner;
            expr = ctx.ast_context().tree().get(expr_id);
        }

        if !matches!(expr, ast::Expression::Import { .. }) {
            continue;
        }

        // detect path completions inside the import string
        let import_span = ctx.ast_context().tree().source_map.get(expr_id.id);
        let main_span = ctx.ast_context().tree().source_map.get_main(expr_id.id);
        if let Some(span) = main_span
            && span.contains(offset)
        {
            let partial_path = extract_string_literal_prefix(source, span, offset);
            return Some(CompletionContext::ImportPath { partial_path });
        }

        if let Some(span) = import_path_span_from_tokens(ctx, import_span, offset) {
            let partial_path = extract_string_literal_prefix(source, span, offset);
            return Some(CompletionContext::ImportPath { partial_path });
        }

        // detect import clause completions inside the brace list
        if let Some(info) = import_clause_info(ctx, expr, offset, source, import_span, main_span) {
            let ast::Expression::Import { target, .. } = expr else {
                continue;
            };
            let ast::ImportTarget::String(target) = target else {
                continue;
            };

            let target_module = resolved_import_target_module(session, ctx, *target);

            return Some(CompletionContext::ImportClause {
                target_module,
                existing_names: info.existing_names,
                space_filter: info.space_filter,
            });
        }
    }

    None
}

/// Resolve a string literal span for an import path at the cursor.
fn import_path_span_from_tokens(
    ctx: &QueryContext<'_>,
    import_span: Span,
    offset: u32,
) -> Option<Span> {
    // find the token under the cursor
    let token = token_span_at_cursor_offset(ctx, offset)?;

    // require the token to stay inside the import statement span
    if token.span.start < import_span.start || token.span.end > import_span.end {
        return None;
    }

    // require a string literal token
    if token.token.ty != ast::TokenType::Literal {
        return None;
    }

    if !matches!(token.token.literal, Some(ast::LiteralType::String { .. })) {
        return None;
    }

    Some(token.span)
}

/// Resolve one import target module from compiler-resolved import edges.
fn resolved_import_target_module(
    session: &Session,
    ctx: &QueryContext<'_>,
    target: destack_core::StringId,
) -> Option<ModuleId> {
    let specifier = ctx.ast_context().strings().get(target);
    let target_id = session.strings.intern(&specifier);
    let cache_key = (Some(ctx.module_id), target_id, ImportEdgeKind::Import, None);
    let targets = ctx
        .dir_resolved()
        .imported_modules
        .get(&cache_key)
        .copied()?;

    targets
        .value
        .or(targets.ty)
        .and_then(|target| target.module_id())
}

/// Parsed import clause info for completion.
struct ImportClauseInfo {
    /// Existing names in the clause.
    existing_names: Vec<String>,
    /// Optional symbol space filter.
    space_filter: Option<dir::SymbolSpace>,
}

/// Extract import clause info at the given offset.
fn import_clause_info(
    ctx: &QueryContext<'_>,
    expr: &ast::Expression,
    offset: u32,
    source: &str,
    import_span: Span,
    target_span: Option<Span>,
) -> Option<ImportClauseInfo> {
    // require an import expression
    let ast::Expression::Import { items, kind, .. } = expr else {
        return None;
    };

    // resolve the import clause braces before the target string
    let bounds = import_clause_bounds(ctx, import_span, target_span)?;
    let open_brace = bounds.open_brace;
    let end_boundary = bounds.end_boundary;

    // detect cursor inside the clause braces
    let cursor_in_clause = offset >= open_brace.end && offset <= end_boundary.start;

    // collect existing names and detect item kind under the cursor
    let mut existing_names = Vec::new();
    let mut in_item_kind = None;

    for item_id in items {
        let item = ctx.ast_context().tree().get(*item_id);
        let span = ctx.ast_context().tree().source_map.get(item_id.id);

        let (item_kind, item_name, item_alias) = match item {
            ast::DependencyItem::Item {
                kind, name, alias, ..
            } => (*kind, *name, *alias),
            ast::DependencyItem::Error => continue,
        };

        if span.contains(offset) {
            in_item_kind = item_kind;
            continue;
        }

        if let Some(name_id) = item_name {
            existing_names.push(
                ctx.ast_context()
                    .strings()
                    .get(name_id.string())
                    .to_string(),
            );
        }

        if let Some(alias_id) = item_alias {
            existing_names.push(ctx.ast_context().strings().get(alias_id).to_string());
        }
    }

    if !cursor_in_clause && in_item_kind.is_none() {
        return None;
    }

    // detect cursor after a type keyword inside the clause
    let cursor_is_type = if cursor_in_clause {
        if let Some(token) = previous_significant_token(ctx, offset) {
            let token_in_clause =
                token.span.start >= open_brace.start && token.span.end <= end_boundary.end;
            token_in_clause
                && token.token.ty == ast::TokenType::Identifier
                && token_text(source, token.span) == Some("type")
        } else {
            false
        }
    } else {
        false
    };

    let space_filter = match kind {
        ast::DependencyKind::Type => Some(dir::SymbolSpace::Type),
        ast::DependencyKind::Value => match in_item_kind {
            Some(ast::DependencyKind::Type) => Some(dir::SymbolSpace::Type),
            Some(ast::DependencyKind::Value) => Some(dir::SymbolSpace::Value),
            None if cursor_is_type => Some(dir::SymbolSpace::Type),
            None => None,
        },
    };

    // deduplicate existing names to keep behavior stable
    deduplicate_names(&mut existing_names);

    Some(ImportClauseInfo {
        existing_names,
        space_filter,
    })
}

/// Deduplicate names while preserving their first occurrence order.
fn deduplicate_names(names: &mut Vec<String>) {
    // retain the first occurrence of each name
    let mut deduped = HashSet::new();
    names.retain(|name| deduped.insert(name.clone()));
}

// ================================================================================
// statement contexts
// ================================================================================

/// Detect whether the cursor is at a statement position.
fn detect_statement_position(ctx: &QueryContext<'_>, offset: u32) -> Option<CompletionContext> {
    // treat the start of the file as a statement position
    if offset == 0 {
        let scope = scope_at_offset(ctx, offset);
        return Some(statement_context_from_scope(scope));
    }

    // missing declarator initializer holes are not statement gaps
    if missing_declarator_value_at_cursor(ctx, offset) {
        return None;
    }

    // check block based statement gaps first
    if let Some(scope) = statement_position_from_block(ctx, offset) {
        return Some(statement_context_from_scope(Some(scope)));
    }

    // fall back to token-based statement boundaries
    if let Some(token) = previous_significant_token(ctx, offset)
        && matches!(
            token.token.ty,
            ast::TokenType::Semicolon | ast::TokenType::OpenBrace | ast::TokenType::CloseBrace
        )
    {
        let scope = scope_at_offset(ctx, offset);
        return Some(statement_context_from_scope(scope));
    }

    None
}

/// Build a statement position context from an optional scope.
fn statement_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    CompletionContext::StatementPosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Resolve a statement position inside a block expression.
fn statement_position_from_block(ctx: &QueryContext<'_>, offset: u32) -> Option<ScopeAtOffset> {
    // resolve enclosing spans at the cursor boundary
    let mut enclosing = enclosing_spans_with_previous(ctx, offset);
    enclosing.sort_by_key(|enc| enc.length);

    // bail out when there are no spans
    if enclosing.is_empty() {
        return None;
    }

    // scan for the nearest block that opens one statement position
    for enc in &enclosing {
        if block_statement_position(ctx, enc, offset).is_some() {
            return scope_from_block_span(ctx, enc.idx, offset);
        }
    }

    None
}

// ================================================================================
// type contexts
// ================================================================================

/// Detect whether the cursor is in a type position.
fn detect_type_position(ctx: &QueryContext<'_>, source: &str, offset: u32) -> bool {
    // resolve enclosing spans from innermost to outermost
    let enclosing = enclosing_spans_with_previous(ctx, offset);

    // check for type side spans that contain the cursor
    if offset_is_in_ast_type_side_span(ctx, offset) {
        return true;
    }

    // check enclosing expressions that are known type expressions
    for enc in &enclosing {
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast_context().tree().get(expr_id);

        if is_type_expression(expr) {
            return true;
        }
    }

    // check type declarations for their value expression spans
    if is_type_declaration_value_position(ctx, &enclosing, offset) {
        return true;
    }

    // avoid treating object literal values as type positions
    if is_inside_object_literal_expression(ctx, offset) {
        return false;
    }

    token_suggests_type_position(ctx, source, offset)
}

/// Check whether the preceding token still suggests a type position.
fn token_suggests_type_position(ctx: &QueryContext<'_>, source: &str, offset: u32) -> bool {
    let Some(token) = previous_significant_token(ctx, offset) else {
        return false;
    };

    // delimiters often introduce one following type position
    if matches!(
        token.token.ty,
        ast::TokenType::Colon | ast::TokenType::Comma | ast::TokenType::LessThan
    ) {
        return true;
    }

    // type relation keywords also introduce one following type position
    token.token.ty == ast::TokenType::Identifier
        && matches!(
            token_text(source, token.span),
            Some("extends") | Some("implements") | Some("is") | Some("as")
        )
}

/// Check whether the cursor is inside a type declaration value expression.
fn is_type_declaration_value_position(
    ctx: &QueryContext<'_>,
    enclosing: &[EnclosingSpan],
    offset: u32,
) -> bool {
    // resolve the previous offset for span checks
    let previous_offset = offset.saturating_sub(1);

    // scan enclosing declarations for type values
    for enc in enclosing {
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Declaration {
            continue;
        }

        let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(enc.idx);
        let declaration = ctx.ast_context().tree().get(declaration_id);

        let ast::Declaration::Type { value, .. } = declaration else {
            continue;
        };

        let span = ctx.ast_context().tree().source_map.get(value.id);
        if span.contains(offset) || span.contains(previous_offset) {
            return true;
        }
    }

    false
}

/// Check whether the expression is one known type expression form.
fn is_type_expression(expr: &ast::Expression) -> bool {
    // match the known type expression variants
    matches!(
        expr,
        ast::Expression::TypeLiteral(_)
            | ast::Expression::TypeUnary { .. }
            | ast::Expression::TypeBinary { .. }
            | ast::Expression::TypeConditional { .. }
            | ast::Expression::TypeMapped { .. }
            | ast::Expression::TypeIndex { .. }
            | ast::Expression::TypeTemplateLiteral { .. }
            | ast::Expression::TypeImport { .. }
            | ast::Expression::TypeInfer { .. }
            | ast::Expression::TypePredicate { .. }
    )
}
