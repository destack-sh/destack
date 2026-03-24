use std::collections::HashSet;

use destack_source::{EnclosingSpan, FileId, ModuleId, NodeSpanType, Span};
use {destack_ast as ast, destack_dir as dir};

use crate::common::{
    QueryContext, block_statement_position, enclosing_missing_expression,
    enclosing_spans_at_cursor_boundary, enclosing_spans_with_previous,
    extract_string_literal_prefix, get_module_by_file_id, import_clause_bounds,
    missing_declarator_value_at_cursor, offset_is_in_ast_type_side_span,
    previous_significant_token, resolve_expression_symbol, sorted_enclosing_spans,
    span_for_dir_node, token_span_at_cursor_offset, token_text, visible_symbols,
};
use destack_workspace::Session;

/// Describes the context for a completion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
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
        /// The expected type when one is available.
        expected_type: Option<dir::LocalTypeId>,
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
pub struct TokenAtCursor {
    /// The token text.
    pub text: String,
    /// The start offset of the token.
    pub start: u32,
    /// The end offset of the token.
    pub end: u32,
}

/// The detected completion context plus token data.
#[derive(Debug, Clone)]
pub struct ContextResult {
    /// The completion context.
    pub context: CompletionContext,
    /// The token at the cursor when available.
    pub token: Option<TokenAtCursor>,
}

/// A scope and mark resolved for a cursor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeAtOffset {
    /// The scope id.
    pub scope_id: dir::LocalScopeId,
    /// The scope mark within the scope.
    pub scope_mark: dir::LocalScopeMark,
}

/// Create an unknown completion context result.
fn unknown_context() -> ContextResult {
    // return an unknown context result
    ContextResult {
        context: CompletionContext::Unknown,
        token: None,
    }
}

/// Check whether the cursor is after a specific keyword token.
fn is_after_keyword(ctx: &QueryContext<'_>, source: &str, offset: u32, keyword: &str) -> bool {
    // resolve the previous token before the cursor
    let Some(token) = previous_significant_token(ctx, offset) else {
        return false;
    };

    // require an identifier token matching the keyword text
    if token.token.ty != ast::TokenType::Identifier {
        return false;
    }

    token_text(source, token.span) == Some(keyword)
}

/// Check whether the cursor is immediately after a dot.
fn is_after_dot(ctx: &QueryContext<'_>, offset: u32) -> bool {
    // resolve the previous token before the cursor
    let Some(token) = previous_significant_token(ctx, offset) else {
        return false;
    };

    token.token.ty == ast::TokenType::Dot
}

/// Resolve the member access dot before the given offset when present.
fn member_access_dot_before_offset(ctx: &QueryContext<'_>, offset: u32) -> Option<ast::TokenSpan> {
    let previous = previous_significant_token(ctx, offset)?;

    if previous.token.ty == ast::TokenType::Dot {
        return Some(previous);
    }

    if previous.token.ty != ast::TokenType::Identifier {
        return None;
    }

    let dot = previous_significant_token(ctx, previous.span.start)?;
    if dot.token.ty != ast::TokenType::Dot {
        return None;
    }

    Some(dot)
}

/// Resolve the receiver token before one member access dot.
fn receiver_token_before_member_access_dot(
    ctx: &QueryContext<'_>,
    dot: ast::TokenSpan,
) -> Option<ast::TokenSpan> {
    let mut receiver_token = previous_significant_token(ctx, dot.span.start)?;

    // optional chaining inserts `?` before `.`
    if receiver_token.token.ty == ast::TokenType::Maybe {
        receiver_token = previous_significant_token(ctx, receiver_token.span.start)?;
    }

    Some(receiver_token)
}

/// Build a value position context from an optional scope.
fn value_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    // return the value position context
    CompletionContext::ValuePosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Build a type position context from an optional scope.
fn type_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    // return the type position context
    CompletionContext::TypePosition {
        scope_id: scope.map(|scope| scope.scope_id),
        scope_mark: scope.map(|scope| scope.scope_mark),
    }
}

/// Detect member access context near the cursor.
fn detect_member_access_context(
    session: &Session,
    ctx: &QueryContext<'_>,
    token: &Option<TokenAtCursor>,
    offset: u32,
) -> Option<CompletionContext> {
    let cursor_position = offset.saturating_sub(1);

    // detect member access inside an existing member name token
    if let Some(context) = member_access_context_from_member_name(ctx, cursor_position) {
        return Some(context);
    }

    // resolve member access context when immediately after dot
    if is_after_dot(ctx, offset)
        && let Some(dot) = member_access_dot_before_offset(ctx, offset)
        && let Some(receiver_token) = receiver_token_before_member_access_dot(ctx, dot)
    {
        let receiver_position = receiver_token.span.end.saturating_sub(1);
        let offset_context = member_access_context_at_offset(ctx, receiver_position);
        let token_context = member_access_context_from_tokens(session, ctx, offset);
        return merge_member_access_contexts(offset_context, token_context);
    }

    // resolve member access when the cursor is inside a member name
    if let Some(token_at_cursor) = token.as_ref()
        && let Some(dot) = member_access_dot_before_offset(ctx, token_at_cursor.start)
        && let Some(receiver_token) = receiver_token_before_member_access_dot(ctx, dot)
    {
        let receiver_position = receiver_token.span.end.saturating_sub(1);
        if let Some(context) = member_access_context_at_offset(ctx, receiver_position) {
            return Some(context);
        }
    }

    member_access_context_from_tokens(session, ctx, offset)
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
        let receiver_symbol = get_expression_symbol(dir_tree, *left);
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
    // only suppress naked missing-expression holes
    if token.is_some() {
        return false;
    }

    enclosing_missing_expression(ctx, offset).is_some()
        || missing_declarator_value_at_cursor(ctx, offset)
}

/// Detect the completion context at a given offset.
pub fn detect_completion_context(session: &Session, file_id: FileId, offset: u32) -> ContextResult {
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
    if let Some(context) = detect_member_access_context(session, &ctx, &token, offset) {
        return ContextResult { context, token };
    }

    // check object literal context before type position to avoid comma misclassification
    if let Some(object_context) = detect_object_literal_context(&ctx, offset, session) {
        return ContextResult {
            context: object_context,
            token,
        };
    }

    // check for object literal value position
    if let Some(scope) = detect_object_literal_value_scope(&ctx, offset) {
        return ContextResult {
            context: CompletionContext::ObjectLiteralValue {
                scope_id: Some(scope.scope_id),
                scope_mark: Some(scope.scope_mark),
            },
            token,
        };
    }

    // check for new expression context before call arguments
    if let Some(new_context) = detect_new_expression_context(&ctx, source, offset) {
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
    let in_type_position = detect_type_position(&ctx, source, offset);

    if in_type_position {
        let scope = find_scope_at_offset(&ctx, offset);
        return ContextResult {
            context: type_context_from_scope(scope),
            token,
        };
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

    // default to value position
    let scope = find_scope_at_offset(&ctx, offset);
    ContextResult {
        context: value_context_from_scope(scope),
        token,
    }
}

/// Merge two candidate member access contexts with preference for richer receiver info.
fn merge_member_access_contexts(
    offset_context: Option<CompletionContext>,
    token_context: Option<CompletionContext>,
) -> Option<CompletionContext> {
    let offset_context = offset_context?;
    let Some(token_context) = token_context else {
        return Some(offset_context);
    };

    let CompletionContext::MemberAccess {
        receiver_node,
        receiver_symbol,
        receiver_type,
    } = offset_context
    else {
        return Some(offset_context);
    };

    let CompletionContext::MemberAccess {
        receiver_node: token_receiver_node,
        receiver_symbol: token_receiver_symbol,
        receiver_type: token_receiver_type,
    } = token_context
    else {
        return Some(CompletionContext::MemberAccess {
            receiver_node,
            receiver_symbol,
            receiver_type,
        });
    };

    // prefer token derived context only when the offset context lacks a symbol
    if receiver_symbol.is_none() {
        return Some(CompletionContext::MemberAccess {
            receiver_node: token_receiver_node,
            receiver_symbol: token_receiver_symbol,
            receiver_type: token_receiver_type,
        });
    }

    // keep the richer offset context when it already has semantic receiver info
    Some(CompletionContext::MemberAccess {
        receiver_node,
        receiver_symbol,
        receiver_type,
    })
}

/// Get the target symbol of an expression if it resolves to one.
fn get_expression_symbol(
    dir_tree: &dir::NodeTree,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    // read the expression and extract the target symbol
    let expr = dir_tree.get::<dir::Expression>(expr_id);
    expr.target_symbol()
}

/// Get the type of a receiver expression.
///
/// Tries multiple approaches.
/// First, look up inferred or declared type for the expression node.
/// If the expression is a reference, use `get_type_id_for_symbol` which checks cached value
/// types for symbols and declared or inferred types from the primary declaration.
fn get_receiver_type(
    ctx: &QueryContext<'_>,
    receiver_global: dir::GlobalNodeIdAny,
    receiver_symbol: Option<dir::GlobalSymbolId>,
) -> Option<dir::LocalTypeId> {
    // first try: get type directly from the expression node
    {
        let types = ctx.types();
        if let Some(type_id) = types.get_declared_or_inferred_type_id(receiver_global) {
            return Some(type_id);
        }
    }

    // second try: use get_type_id_for_symbol which checks cached value types
    // and falls back to declared or inferred types from the primary declaration
    if let Some(symbol_id) = receiver_symbol {
        {
            let types = ctx.types();
            let symbols = ctx.symbols();
            if let Some(type_id) = types.get_type_id_for_symbol(symbols, symbol_id) {
                return Some(type_id);
            }
        }

        // third try: resolve from the declaration's annotation or initializer
        if symbol_id.module_id == ctx.module_id {
            let declaration = {
                let symbols = ctx.symbols();
                let symbol = symbols.get_symbol(symbol_id.local_id);
                symbol.primary_declaration
            };

            if let Some(declaration) = declaration {
                let dir_tree = ctx.tree();

                let declarator_id = match declaration.local_id.ty {
                    dir::NodeType::Declarator => declaration.local_id.try_into().ok(),
                    dir::NodeType::Pattern => {
                        let parent_id = dir_tree.get_parent(declaration.local_id.id);
                        if let Some(parent_id) = parent_id
                            && parent_id.ty == dir::NodeType::Declarator
                        {
                            parent_id.try_into_typed().ok()
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(declarator_id) = declarator_id {
                    let declarator = dir_tree.get::<dir::Declarator>(declarator_id).clone();

                    if let Some(ty_expr_id) = declarator.ty {
                        let types = ctx.types();
                        let global_id = ty_expr_id.into_global(ctx.module_id);
                        if let Some(type_id) =
                            types.get_declared_or_inferred_type_id(global_id.into())
                        {
                            return Some(type_id);
                        }
                    }

                    if let Some(value_id) = declarator.value {
                        let types = ctx.types();
                        let global_id = value_id.into_global(ctx.module_id);
                        if let Some(type_id) =
                            types.get_declared_or_inferred_type_id(global_id.into())
                        {
                            return Some(type_id);
                        }
                    }
                } else if declaration.local_id.ty == dir::NodeType::Parameter {
                    let types = ctx.types();
                    let global_id = declaration.local_id.into_global(ctx.module_id);
                    if let Some(type_id) = types.get_declared_or_inferred_type_id(global_id) {
                        return Some(type_id);
                    }
                }
            }
        }
    }

    // return none when no type is available
    None
}

/// Resolve member access context for a receiver position.
fn member_access_context_at_offset(
    ctx: &QueryContext<'_>,
    receiver_position: u32,
) -> Option<CompletionContext> {
    // resolve enclosing spans at the receiver position
    let enclosing = sorted_enclosing_spans(ctx, receiver_position, receiver_position);

    // resolve the dir tree for span to dir lookup
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
            let receiver_symbol = get_expression_symbol(dir_tree, *left);
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

            // resolve the receiver type
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

        // resolve the receiver type
        let receiver_type = get_receiver_type(ctx, receiver_global, receiver_symbol);

        return Some(CompletionContext::MemberAccess {
            receiver_node: actual_node_id,
            receiver_symbol,
            receiver_type,
        });
    }

    partial_context
}

/// Resolve member access context using token lookups.
fn member_access_context_from_tokens(
    session: &Session,
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<CompletionContext> {
    let dot = member_access_dot_before_offset(ctx, offset)?;
    let receiver_token = receiver_token_before_member_access_dot(ctx, dot)?;

    if receiver_token.token.ty != ast::TokenType::Identifier {
        return None;
    }

    // resolve the receiver offset for span lookups
    let receiver_offset = receiver_token.span.end.saturating_sub(1);
    let source_file = session.files.get(ctx.file_id);
    let source = source_file.text();
    let receiver_name = token_text(source, receiver_token.span)?;

    let mut receiver_node = None;
    let mut receiver_symbol = None;
    let mut partial_receiver_node = None;

    // resolve the receiver node from enclosing AST spans
    if receiver_node.is_none() {
        let enclosing = sorted_enclosing_spans(ctx, receiver_offset, receiver_offset);
        let dir_tree = ctx.tree();
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
            let (actual_node_id, actual_expr) =
                unwrap_statement_expression(dir_tree, dir_node_id, expr);

            if let dir::Expression::Member { left, .. } = actual_expr {
                let receiver_local: dir::LocalNodeIdAny = (*left).into();
                let receiver_local_symbol = get_expression_symbol(dir_tree, *left);

                if partial_receiver_node.is_none() {
                    partial_receiver_node = Some(receiver_local);
                }
                if receiver_local_symbol.is_some() {
                    receiver_node = Some(receiver_local);
                    receiver_symbol = receiver_local_symbol;
                    break;
                }

                continue;
            }

            if partial_receiver_node.is_none() {
                partial_receiver_node = Some(actual_node_id);
            }

            let actual_symbol = actual_expr.target_symbol();
            if actual_symbol.is_some() {
                receiver_node = Some(actual_node_id);
                receiver_symbol = actual_symbol;
                break;
            }
        }
    }

    if receiver_node.is_none() {
        receiver_node = partial_receiver_node;
    }

    // resolve the receiver symbol from visible scopes when direct expression symbols are missing
    if receiver_symbol.is_none() {
        let symbols = ctx.symbols();

        if let Some(scope) = find_scope_at_offset(ctx, receiver_offset) {
            receiver_symbol = resolve_member_receiver_symbol_in_scope(
                session,
                symbols,
                receiver_name,
                scope.scope_id,
                scope.scope_mark,
            );
        }

        if receiver_symbol.is_none() {
            receiver_symbol = resolve_member_receiver_symbol_in_scope(
                session,
                symbols,
                receiver_name,
                ctx.dir.namespace_scope,
                dir::LocalScopeMark::end(),
            );
        }
    }
    let receiver_node = receiver_node.or_else(|| {
        if receiver_symbol.is_some() {
            ctx.dir.roots.first().copied().map(Into::into)
        } else {
            None
        }
    })?;

    // resolve receiver type for member completions
    let receiver_global = receiver_node.into_global(ctx.module_id);
    let receiver_type = get_receiver_type(ctx, receiver_global, receiver_symbol);

    Some(CompletionContext::MemberAccess {
        receiver_node,
        receiver_symbol,
        receiver_type,
    })
}

/// Resolve a receiver symbol by name in the visible scope chain.
fn resolve_member_receiver_symbol_in_scope(
    session: &Session,
    symbols: &dir::SymbolTable,
    receiver_name: &str,
    scope_id: dir::LocalScopeId,
    scope_mark: dir::LocalScopeMark,
) -> Option<dir::GlobalSymbolId> {
    // prefer value-space symbols for member access receivers
    for visible in visible_symbols(symbols, scope_id, scope_mark, Some(dir::SymbolSpace::Value)) {
        let dir::StaticKey::Name(name_id) = visible.key else {
            continue;
        };
        if session.strings.get(name_id) != receiver_name {
            continue;
        }

        return Some(dir::GlobalSymbolId {
            module_id: symbols.module_id,
            local_id: visible.id,
        });
    }

    // fall back to any symbol space when no value symbol matches
    for visible in visible_symbols(symbols, scope_id, scope_mark, None) {
        let dir::StaticKey::Name(name_id) = visible.key else {
            continue;
        };
        if session.strings.get(name_id) != receiver_name {
            continue;
        }

        return Some(dir::GlobalSymbolId {
            module_id: symbols.module_id,
            local_id: visible.id,
        });
    }

    None
}

/// Unwrap Statement expressions to get the inner expression.
///
/// Statement expressions wrap another expression with a `;` terminator.
/// When searching for member access context, we want the actual inner expression,
/// not the Statement wrapper (which has type void and no target symbol).
fn unwrap_statement_expression<'a>(
    dir_tree: &'a dir::NodeTree,
    node_id: dir::LocalNodeIdAny,
    expr: &'a dir::Expression,
) -> (dir::LocalNodeIdAny, &'a dir::Expression) {
    // recursively unwrap Statement expressions
    if let dir::Expression::Statement { statement } = expr {
        let inner_node_id: dir::LocalNodeIdAny = (*statement).into();
        let inner_expr = dir_tree.get::<dir::Expression>(*statement);
        return unwrap_statement_expression(dir_tree, inner_node_id, inner_expr);
    }
    (node_id, expr)
}

/// Detect partial identifier at cursor position.
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
        end: span.end,
    })
}

/// Detect if the cursor is in a type position.
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

    // use token heuristics when side spans are unavailable
    let Some(token) = previous_significant_token(ctx, offset) else {
        return false;
    };

    // check for type annotation delimiters
    if matches!(
        token.token.ty,
        ast::TokenType::Colon | ast::TokenType::Comma | ast::TokenType::LessThan
    ) {
        return true;
    }

    // check for type-position keywords
    if token.token.ty == ast::TokenType::Identifier
        && matches!(
            token_text(source, token.span),
            Some("extends") | Some("implements") | Some("is") | Some("as")
        )
    {
        return true;
    }

    false
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

        // resolve the import span and detect path completions inside the string
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

        // resolve import clause info and type only context
        if let Some(info) = import_clause_info(ctx, expr, offset, source, import_span, main_span) {
            let ast::Expression::Import { target, .. } = expr else {
                continue;
            };
            let ast::ImportTarget::String(target) = target else {
                continue;
            };

            // resolve the target module from the import specifier
            let target_specifier = ctx.ast_context().strings().get(*target).to_string();
            let target_module = resolve_import_target_module(session, ctx, &target_specifier);

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

    // require the token to be inside the import statement span
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

/// Resolve a module id from a relative import target path.
fn resolve_import_target_module(
    session: &Session,
    ctx: &QueryContext<'_>,
    target: &str,
) -> Option<ModuleId> {
    // only resolve relative paths
    if !target.starts_with("./") && !target.starts_with("../") {
        return None;
    }

    // resolve the base path for the current module
    let module = session.modules.get(ctx.module_id);
    let module = module.as_ref();
    let base_path = if let Some(path) = module.path.clone() {
        Some(path)
    } else {
        let uri = module.uri.as_ref();
        let stripped = uri.strip_prefix("file://");
        stripped
            .map(std::path::PathBuf::from)
            .or_else(|| module.uri.to_path_buf())
    }?;

    // resolve via common module lookup
    crate::common::resolve_module_id_for_import_target_path(session, &base_path, target)
}

/// Find the scope at a given offset.
fn find_scope_at_offset(
    ctx: &crate::common::QueryContext<'_>,
    offset: u32,
) -> Option<ScopeAtOffset> {
    // resolve enclosing spans at the cursor and previous byte
    let enclosing = enclosing_spans_with_previous(ctx, offset);

    // resolve the dir tree and symbols for scope lookups
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();

    // first pass: find block scopes at the cursor position
    for enc in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        // for Block nodes inside a function, find the function's owned scope
        if dir_node_id.ty == dir::NodeType::Block {
            let block_id = <dir::LocalNodeIdAny as TryInto<dir::LocalNodeId<dir::Block>>>::try_into(
                dir_node_id,
            );
            if let Ok(block_id) = block_id {
                let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
                let scope_mark =
                    scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark,
                });
            }
        }
    }

    // second pass: try other node types
    for enc in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        // for Expression nodes, use get_scope
        if dir_node_id.ty == dir::NodeType::Expression {
            let expr_id = dir_node_id.try_into();
            if let Ok(expr_id) = expr_id {
                let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
                let scope_mark =
                    scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark,
                });
            }
        }

        // for Declaration nodes, find the owned scope if this is a function/class
        if dir_node_id.ty == dir::NodeType::Declaration {
            let scope_id = find_owned_scope_for_declaration(symbols, dir_node_id.id);
            if let Some(scope_id) = scope_id {
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark: dir::LocalScopeMark::end(),
                });
            }
        }
    }

    // fallback: walk AST parents from the innermost node
    if let Some(start_id) = enclosing.first().map(|enc| enc.idx) {
        for parent_id in ctx.ast_context().parents().walk_parents_by_id(start_id) {
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(parent_id) else {
                if ctx.ast_context().tree().get_node_type(parent_id) == ast::NodeType::Declaration {
                    let scope_id =
                        find_owned_scope_for_ast_declaration(symbols, dir_tree, parent_id);
                    if let Some(scope_id) = scope_id {
                        return Some(ScopeAtOffset {
                            scope_id,
                            scope_mark: dir::LocalScopeMark::end(),
                        });
                    }
                }

                continue;
            };

            if dir_node_id.ty == dir::NodeType::Block {
                let block_id = dir_node_id.try_into();
                if let Ok(block_id) = block_id {
                    let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
                    let scope_mark =
                        scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
                    return Some(ScopeAtOffset {
                        scope_id,
                        scope_mark,
                    });
                }
            }

            if dir_node_id.ty == dir::NodeType::Expression {
                let expr_id = dir_node_id.try_into();
                if let Ok(expr_id) = expr_id {
                    let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
                    let scope_mark =
                        scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
                    return Some(ScopeAtOffset {
                        scope_id,
                        scope_mark,
                    });
                }
            }

            if dir_node_id.ty == dir::NodeType::Declaration {
                let scope_id = find_owned_scope_for_declaration(symbols, dir_node_id.id);
                if let Some(scope_id) = scope_id {
                    return Some(ScopeAtOffset {
                        scope_id,
                        scope_mark: dir::LocalScopeMark::end(),
                    });
                }
            }
        }
    }

    // fall back to the nearest enclosing block span
    let mut best_block = None;
    let mut best_length = u32::MAX;
    for (block_id, _) in dir_tree.iter_nodes_of_type::<dir::Block>() {
        let span = span_for_dir_node(ctx, dir_tree, block_id.into());
        if span.file != ctx.file_id {
            continue;
        }
        if !span.contains(offset) {
            continue;
        }

        let length = span.end.saturating_sub(span.start);
        if length < best_length {
            best_length = length;
            best_block = Some(block_id);
        }
    }

    if let Some(block_id) = best_block {
        let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
        let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
        return Some(ScopeAtOffset {
            scope_id,
            scope_mark,
        });
    }

    None
}

/// Detect if we're inside an object literal and return context info.
fn detect_object_literal_context(
    ctx: &QueryContext<'_>,
    offset: u32,
    session: &Session,
) -> Option<CompletionContext> {
    // resolve enclosing spans from innermost to outermost
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // bail out early when there are no enclosing spans
    if enclosing.is_empty() {
        return None;
    }

    // resolve dir tree, symbols, and types for object literal analysis
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();
    let types = ctx.types();

    // look for object expression
    for enc in &enclosing {
        // skip non expression AST nodes
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        // resolve the AST expression node
        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast_context().tree().get(expr_id);

        // require an object literal expression
        let ast::Expression::ObjectExpression { properties, .. } = expr else {
            continue;
        };

        // require a key position inside the object literal
        if !is_object_literal_key_position(&ctx.ast_context().tree(), properties, offset) {
            continue;
        }

        // resolve the dir node for the AST expression
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        // require a dir expression node
        if dir_node_id.ty != dir::NodeType::Expression {
            continue;
        }

        // convert to a typed dir expression id
        let Ok(expr_id) = dir_node_id.try_into() else {
            continue;
        };

        // resolve the dir expression node
        let expr: &dir::Expression = dir_tree.get(expr_id);

        match expr {
            dir::Expression::ObjectExpression { properties } => {
                // extract existing field names
                let existing_fields =
                    extract_property_names(dir_tree, properties, &session.strings);

                // resolve expected type from contextual typing when possible
                let expected_type = expected_type_for_object_literal(
                    ctx,
                    dir_tree,
                    symbols,
                    types,
                    dir_node_id,
                    expr_id,
                );

                // get scope at this position
                let (scope_id, scope_mark) = dir_tree.get_scope::<dir::Expression>(expr_id);

                return Some(CompletionContext::ObjectLiteral {
                    object_node: dir_node_id,
                    expected_type,
                    existing_fields,
                    scope_id: Some(scope_id),
                    scope_mark: Some(scope_mark),
                });
            }
            dir::Expression::TaggedObjectExpression { ty, properties } => {
                // extract existing field names
                let existing_fields =
                    extract_property_names(dir_tree, properties, &session.strings);

                // for tagged object, the type comes from the tag expression
                let ty_node_id: dir::LocalNodeIdAny = (*ty).into();
                let ty_global = ty_node_id.into_global(ctx.module_id);
                let expected_type = types.get_declared_or_inferred_type_id(ty_global);

                // get scope at this position
                let (scope_id, scope_mark) = dir_tree.get_scope::<dir::Expression>(expr_id);

                return Some(CompletionContext::ObjectLiteral {
                    object_node: dir_node_id,
                    expected_type,
                    existing_fields,
                    scope_id: Some(scope_id),
                    scope_mark: Some(scope_mark),
                });
            }
            _ => continue,
        }
    }

    None
}

/// Resolve contextual type information for an object literal.
fn expected_type_for_object_literal(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
    node_id: dir::LocalNodeIdAny,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalTypeId> {
    // resolve the parent node
    let parent = dir_tree.get_parent(node_id.id);

    // prefer annotated declarators when the object literal is a binding value
    if let Some(parent) = parent {
        // handle declarator parents first
        if parent.ty == dir::NodeType::Declarator {
            // resolve the declarator id and node
            let declarator_id = dir::LocalNodeId::<dir::Declarator>::try_from(parent).ok();
            if let Some(declarator_id) = declarator_id {
                // read the declarator node
                let declarator = dir_tree.get(declarator_id);

                // require the object literal to be the declarator value
                if declarator.value == Some(expr_id) {
                    // resolve the explicit type annotation when available
                    let ty_expr_id = declarator.ty;
                    if let Some(ty_expr_id) = ty_expr_id {
                        // resolve the annotated type
                        let ty_node_id: dir::LocalNodeIdAny = ty_expr_id.into();
                        let ty_global = ty_node_id.into_global(ctx.module_id);
                        let ty = types.get_declared_or_inferred_type_id(ty_global);
                        if let Some(ty) = ty {
                            return Some(ty);
                        }
                    }

                    // fall back to the binding symbol type when possible
                    let pattern = dir_tree.get(declarator.pattern);
                    if let Some(symbol_id) = pattern.symbol() {
                        // resolve the symbol type from the DIR table
                        let global_symbol = dir::GlobalSymbolId {
                            module_id: ctx.module_id,
                            local_id: symbol_id,
                        };
                        let ty = types.get_type_id_for_symbol(symbols, global_symbol);
                        if let Some(ty) = ty {
                            return Some(ty);
                        }
                    }
                }
            }
        }

        // prefer assignment target types when the object literal is assigned
        if parent.ty == dir::NodeType::Expression {
            // resolve the parent expression node
            let parent_expr_id = dir::LocalNodeId::<dir::Expression>::try_from(parent).ok();
            if let Some(parent_expr_id) = parent_expr_id {
                let parent_expr = dir_tree.get(parent_expr_id);

                // check assignment expressions that use the object literal as the right side
                match parent_expr {
                    dir::Expression::Assign { left, right }
                    | dir::Expression::AssignBinary { left, right, .. } => {
                        if *right == expr_id {
                            // prefer the resolved symbol type for assignments
                            if let Some(target_symbol) = resolve_expression_symbol(ctx, *left) {
                                let ty = types.get_type_id_for_symbol(symbols, target_symbol);
                                if let Some(ty) = ty {
                                    return Some(ty);
                                }
                            }

                            // fall back to the declared or inferred type of the left expression
                            let left_node_id: dir::LocalNodeIdAny = (*left).into();
                            let left_global = left_node_id.into_global(ctx.module_id);
                            let ty = types.get_declared_or_inferred_type_id(left_global);
                            if let Some(ty) = ty {
                                return Some(ty);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // fall back to the inferred type of the expression node
    let global_node_id = node_id.into_global(ctx.module_id);
    types.get_declared_or_inferred_type_id(global_node_id)
}

/// Detect object literal value position and return a scope.
fn detect_object_literal_value_scope(ctx: &QueryContext<'_>, offset: u32) -> Option<ScopeAtOffset> {
    // resolve enclosing spans from innermost to outermost
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // bail out early when there are no enclosing spans
    if enclosing.is_empty() {
        return None;
    }

    // resolve the dir tree and symbols for scope analysis
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();

    // scan enclosing expressions to detect object literal value positions
    for enc in &enclosing {
        // skip non expression AST nodes
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        // resolve the AST expression node
        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast_context().tree().get(expr_id);

        // require an object literal expression
        let ast::Expression::ObjectExpression { properties, .. } = expr else {
            continue;
        };

        // skip key positions inside object literals
        if is_object_literal_key_position(&ctx.ast_context().tree(), properties, offset) {
            continue;
        }

        // resolve the DIR node for the expression
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        // require a DIR expression node
        if dir_node_id.ty != dir::NodeType::Expression {
            continue;
        }

        // convert to a typed expression id
        let Ok(expr_id) = dir_node_id.try_into() else {
            continue;
        };

        // resolve the scope and return the value context
        let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
        let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
        return Some(ScopeAtOffset {
            scope_id,
            scope_mark,
        });
    }

    None
}

/// Detect whether the cursor is in a new expression context.
fn detect_new_expression_context(
    ctx: &QueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve enclosing spans at the cursor
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // load dir tables for node inspection
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();

    // scan spans for a new expression containing the cursor
    for enc in &enclosing {
        // try to build a context for this span
        if let Some(context) = new_expression_context_for_span(ctx, dir_tree, symbols, enc, offset)
        {
            return Some(context);
        }
    }

    // fall back to the nearest keyword token
    if is_after_keyword(ctx, source, offset, "new") {
        let scope = find_scope_at_offset(ctx, offset);
        return Some(CompletionContext::NewExpression {
            scope_id: scope.map(|scope| scope.scope_id),
            scope_mark: scope.map(|scope| scope.scope_mark),
        });
    }

    None
}

/// Build a new expression context from a single enclosing span.
fn new_expression_context_for_span(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve the dir node id for the span
    let dir_node_id = dir_tree.get_node_id_by_source_id(enc.idx)?;

    // skip nodes that are not expressions
    if dir_node_id.ty != dir::NodeType::Expression {
        return None;
    }

    // resolve the expression node id
    let Ok(expr_id) = dir_node_id.try_into() else {
        return None;
    };

    // narrow to new expressions
    let expr: &dir::Expression = dir_tree.get(expr_id);
    let dir::Expression::New { left, .. } = expr else {
        return None;
    };

    // ensure the cursor is inside the constructor region
    let left_node_id: dir::LocalNodeIdAny = (*left).into();
    let left_source_id = dir_tree.get_source(left_node_id.id);
    let left_span = ctx.ast_context().tree().source_map.get(left_source_id);

    if offset > left_span.end {
        return None;
    }

    // derive scope information for the result
    let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
    let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);

    Some(CompletionContext::NewExpression {
        scope_id: Some(scope_id),
        scope_mark: Some(scope_mark),
    })
}

/// Detect whether the cursor is in a call argument context.
fn detect_call_argument_context(ctx: &QueryContext<'_>, offset: u32) -> Option<CompletionContext> {
    // resolve enclosing spans at the cursor
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // bail out when there are no spans
    if enclosing.is_empty() {
        return None;
    }

    // load dir tables for node inspection
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();
    // scan spans for a call or new expression argument list
    for enc in &enclosing {
        // try to build a context for this span
        if let Some(context) = call_argument_context_for_span(ctx, dir_tree, symbols, enc, offset) {
            return Some(context);
        }
    }

    // fall back when the cursor is directly after `(` or `,` inside a call
    if let Some(separator) = previous_significant_token(ctx, offset)
        && matches!(
            separator.token.ty,
            ast::TokenType::OpenParenthesis | ast::TokenType::Comma
        )
        && let Some(context) = call_argument_context_after_separator(
            ctx,
            dir_tree,
            symbols,
            offset,
            separator.span.start,
        )
    {
        return Some(context);
    }

    None
}

/// Build a call argument context from a single enclosing span.
fn call_argument_context_for_span(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve the dir node id for the span
    let dir_node_id = dir_tree.get_node_id_by_source_id(enc.idx)?;

    // skip nodes that are not expressions
    if dir_node_id.ty != dir::NodeType::Expression {
        return None;
    }

    // resolve the expression node id
    let Ok(expr_id) = dir_node_id.try_into() else {
        return None;
    };

    // restrict to call and new expressions
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

    // compute spans used for argument range checks
    let left_node_id: dir::LocalNodeIdAny = (*left).into();
    let left_source_id = dir_tree.get_source(left_node_id.id);
    let left_span = ctx.ast_context().tree().source_map.get(left_source_id);
    let call_span = ctx.ast_context().tree().source_map.get(enc.idx);

    // ensure the cursor is within the argument list
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

    // derive scope information for the result
    let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
    let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);

    Some(CompletionContext::CallArgument {
        scope_id: Some(scope_id),
        scope_mark: Some(scope_mark),
    })
}

/// Build a call argument context from a separator position inside a call.
fn call_argument_context_after_separator(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    offset: u32,
    separator_position: u32,
) -> Option<CompletionContext> {
    let lookup_position = separator_position.saturating_sub(1);
    let enclosing = sorted_enclosing_spans(ctx, lookup_position, lookup_position);

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

        let (scope_id, _) = dir_tree.get_scope::<dir::Expression>(expr_id);
        let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);

        return Some(CompletionContext::CallArgument {
            scope_id: Some(scope_id),
            scope_mark: Some(scope_mark),
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

        let scope = find_block_scope_at_offset(ctx, offset)
            .or_else(|| find_block_scope_at_offset(ctx, lookup_position))
            .or_else(|| find_scope_at_offset(ctx, lookup_position))
            .or_else(|| find_scope_at_offset(ctx, separator_position))
            .or_else(|| find_scope_at_offset(ctx, offset));
        let scope_mark = scope.map(|scope| {
            if scope.scope_mark == dir::LocalScopeMark(0) {
                dir::LocalScopeMark::end()
            } else {
                scope.scope_mark
            }
        });
        return Some(CompletionContext::CallArgument {
            scope_id: scope.map(|scope| scope.scope_id),
            scope_mark,
        });
    }

    None
}

/// Find the nearest enclosing block or owned declaration scope at an offset.
fn find_block_scope_at_offset(ctx: &QueryContext<'_>, offset: u32) -> Option<ScopeAtOffset> {
    let enclosing = enclosing_spans_with_previous(ctx, offset);
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();

    // prefer block scopes for statement and argument positions
    for enc in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        if dir_node_id.ty != dir::NodeType::Block {
            continue;
        }

        let Ok(block_id) = dir_node_id.try_into_typed() else {
            continue;
        };

        let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
        let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
        return Some(ScopeAtOffset {
            scope_id,
            scope_mark,
        });
    }

    // otherwise fall back to owned declaration scopes
    for enc in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        if dir_node_id.ty != dir::NodeType::Declaration {
            continue;
        }

        let Some(scope_id) = find_owned_scope_for_declaration(symbols, dir_node_id.id) else {
            continue;
        };

        return Some(ScopeAtOffset {
            scope_id,
            scope_mark: dir::LocalScopeMark::end(),
        });
    }

    // fall back to the smallest ast block that still contains the offset
    let mut best_ast_block = None;
    let mut best_ast_block_length = u32::MAX;

    for block_id in ctx.ast_context().tree().iter_nodes::<ast::Block>() {
        let span = ctx.ast_context().tree().source_map.get(block_id.id);
        if span.file != ctx.file_id || !span.contains(offset) {
            continue;
        }

        let length = span.end.saturating_sub(span.start);
        if length >= best_ast_block_length {
            continue;
        }

        best_ast_block_length = length;
        best_ast_block = Some(block_id);
    }

    if let Some(block_id) = best_ast_block
        && let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(block_id.id)
        && dir_node_id.ty == dir::NodeType::Block
        && let Ok(block_id) = dir_node_id.try_into_typed()
    {
        let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
        let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
        return Some(ScopeAtOffset {
            scope_id,
            scope_mark,
        });
    }

    // fall back to ast parents for damaged span stacks
    if let Some(start_id) = enclosing.first().map(|enc| enc.idx) {
        for parent_id in ctx.ast_context().parents().walk_parents_by_id(start_id) {
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(parent_id) else {
                if ctx.ast_context().tree().get_node_type(parent_id) == ast::NodeType::Declaration {
                    let Some(scope_id) =
                        find_owned_scope_for_ast_declaration(symbols, dir_tree, parent_id)
                    else {
                        continue;
                    };

                    return Some(ScopeAtOffset {
                        scope_id,
                        scope_mark: dir::LocalScopeMark::end(),
                    });
                }

                continue;
            };

            if dir_node_id.ty == dir::NodeType::Block {
                let Ok(block_id) = dir_node_id.try_into_typed() else {
                    continue;
                };

                let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
                let scope_mark =
                    scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark,
                });
            }

            if dir_node_id.ty != dir::NodeType::Declaration {
                continue;
            }

            let Some(scope_id) = find_owned_scope_for_declaration(symbols, dir_node_id.id) else {
                continue;
            };

            return Some(ScopeAtOffset {
                scope_id,
                scope_mark: dir::LocalScopeMark::end(),
            });
        }
    }

    // fall back to the smallest block span that still contains the offset
    let mut best_block = None;
    let mut best_length = u32::MAX;

    for (block_id, _) in dir_tree.iter_nodes_of_type::<dir::Block>() {
        let span = span_for_dir_node(ctx, dir_tree, block_id.into());
        if span.file != ctx.file_id || !span.contains(offset) {
            continue;
        }

        let length = span.end.saturating_sub(span.start);
        if length >= best_length {
            continue;
        }

        best_length = length;
        best_block = Some(block_id);
    }

    if let Some(block_id) = best_block {
        let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
        let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);
        return Some(ScopeAtOffset {
            scope_id,
            scope_mark,
        });
    }

    None
}

/// Detect whether the cursor is at a statement position.
fn detect_statement_position(ctx: &QueryContext<'_>, offset: u32) -> Option<CompletionContext> {
    // treat the start of the file as a statement position
    if offset == 0 {
        let scope = find_scope_at_offset(ctx, offset);
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
        let scope = find_scope_at_offset(ctx, offset);
        return Some(statement_context_from_scope(scope));
    }

    None
}

/// Build a statement position context from an optional scope.
fn statement_context_from_scope(scope: Option<ScopeAtOffset>) -> CompletionContext {
    // return the statement position context
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
            return scope_from_block_span(ctx, enc, offset);
        }
    }

    None
}

/// Resolve a scope for a block span.
fn scope_from_block_span(
    ctx: &QueryContext<'_>,
    enc: &EnclosingSpan,
    offset: u32,
) -> Option<ScopeAtOffset> {
    // resolve the dir node for the block span
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();

    let dir_node_id = dir_tree.get_node_id_by_source_id(enc.idx)?;

    // ensure the dir node is a block
    if dir_node_id.ty != dir::NodeType::Block {
        return None;
    }

    // resolve scope information for the block
    let Ok(block_id) = dir_node_id.try_into() else {
        return None;
    };

    let (scope_id, _) = dir_tree.get_scope::<dir::Block>(block_id);
    let scope_mark = scope_mark_for_scope_at_offset(ctx, dir_tree, symbols, scope_id, offset);

    Some(ScopeAtOffset {
        scope_id,
        scope_mark,
    })
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

        for arg_id in arguments {
            let arg_node_id: dir::LocalNodeIdAny = (*arg_id).into();
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

/// Resolve the scope mark at a cursor position within a scope.
fn scope_mark_for_scope_at_offset(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    scope_id: dir::LocalScopeId,
    offset: u32,
) -> dir::LocalScopeMark {
    // resolve the scope data
    let scope = symbols.get_scope_by_id(scope_id);
    if scope.named_symbols.is_empty() {
        return dir::LocalScopeMark(0);
    }

    let mut mark_index = 0u32;

    for (index, (_, symbol_id)) in scope.named_symbols.iter().enumerate() {
        let symbol = symbols.get_symbol(*symbol_id);
        let Some(decl) = symbol.primary_declaration else {
            mark_index = (index + 1) as u32;
            continue;
        };

        let source_id = dir_tree.get_source(decl.local_id.id);
        let span = ctx
            .ast
            .tree
            .source_map
            .get_side_or_main_or_enclosing(source_id, NodeSpanType::Main);

        if span.start <= offset {
            mark_index = (index + 1) as u32;
        }
    }

    dir::LocalScopeMark(mark_index)
}

/// Extract property names from object literal properties.
fn extract_property_names(
    dir_tree: &dir::NodeTree,
    properties: &[dir::LocalNodeId<dir::Property>],
    strings: &destack_core::StringPool,
) -> Vec<String> {
    // collect property names
    let mut names = Vec::new();
    for &prop_id in properties {
        let prop = dir_tree.get::<dir::Property>(prop_id);
        match prop {
            dir::Property::Field { key, .. } => {
                // extract name from key if it's a static name
                if let Some(dir::DynamicKey::Name(name_id)) = key {
                    names.push(strings.get(*name_id).to_string());
                } else if let Some(dir::DynamicKey::Number(name_id)) = key {
                    names.push(strings.get(*name_id).to_string());
                }
                // expression and NamedExpression keys are dynamic, skip them
            }
            dir::Property::Method { key, .. } => {
                if let Some(dir::DynamicKey::Name(name_id)) = key {
                    names.push(strings.get(*name_id).to_string());
                }
            }
            dir::Property::Spread { .. } => {
                // spread properties don't have a single name
            }
            dir::Property::Error { .. } => {}
        }
    }

    // return the collected names
    names
}

/// Find the scope that owns a declaration id.
fn find_owned_scope_for_declaration(
    symbols: &dir::SymbolTable,
    declaration_id: u32,
) -> Option<dir::LocalScopeId> {
    // scan scopes for a matching declaration owner
    for (idx, scope) in symbols.scopes().enumerate() {
        if let Some(owner_id) = scope.owner_id {
            let owner = symbols.get_symbol(owner_id);
            if let Some(decl) = owner.primary_declaration
                && decl.local_id.id == declaration_id
            {
                return Some(dir::LocalScopeId::new(idx as u32));
            }
        }
    }

    None
}

/// Find the scope that owns an AST declaration id.
fn find_owned_scope_for_ast_declaration(
    symbols: &dir::SymbolTable,
    dir_tree: &dir::NodeTree,
    ast_id: u32,
) -> Option<dir::LocalScopeId> {
    // scan scopes for a matching AST owner
    for (idx, scope) in symbols.scopes().enumerate() {
        if let Some(owner_id) = scope.owner_id {
            let owner = symbols.get_symbol(owner_id);
            if let Some(decl) = owner.primary_declaration
                && dir_tree.get_source(decl.local_id.id) == ast_id
            {
                return Some(dir::LocalScopeId::new(idx as u32));
            }
        }
    }

    None
}

/// Check if the cursor is inside a type expression.
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

/// Check if the cursor is in a property key position of an object literal.
fn is_object_literal_key_position(
    ast_tree: &ast::NodeTree,
    properties: &[ast::LocalNodeId<ast::Property>],
    offset: u32,
) -> bool {
    // check for any property key span hit
    for property_id in properties {
        if let Some(span) = ast_tree.source_map.get_main(property_id.id)
            && span.contains(offset)
        {
            return true;
        }
    }

    // avoid property values
    if is_object_literal_value_position(ast_tree, properties, offset) {
        return false;
    }

    // avoid property spans when not on keys
    for property_id in properties {
        let span = ast_tree.source_map.get(property_id.id);
        if span.contains(offset) {
            return false;
        }
    }

    true
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

/// Check if the cursor is inside an object literal expression.
fn is_inside_object_literal_expression(ctx: &QueryContext<'_>, offset: u32) -> bool {
    // resolve enclosing spans from innermost to outermost
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // bail out early when there are no enclosing spans
    if enclosing.is_empty() {
        return false;
    }

    // scan enclosing expressions for object literal nodes
    for enc in &enclosing {
        if ctx.ast_context().tree().get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast_context().tree().get(expr_id);

        let ast::Expression::ObjectExpression { .. } = expr else {
            continue;
        };

        return true;
    }

    false
}

/// Check if the cursor is inside a value span of an object literal.
fn is_object_literal_value_position(
    ast_tree: &ast::NodeTree,
    properties: &[ast::LocalNodeId<ast::Property>],
    offset: u32,
) -> bool {
    // compute the cursor location inside the literal
    let cursor = offset.saturating_sub(1);

    // scan property value spans
    for property_id in properties {
        let property = ast_tree.get(*property_id);

        match property {
            ast::Property::Field { value, default, .. } => {
                if let Some(value_id) = value {
                    let span = ast_tree.source_map.get(value_id.id);
                    if span.contains(cursor) {
                        return true;
                    }
                }

                if let Some(default_id) = default {
                    let span = ast_tree.source_map.get(default_id.id);
                    if span.contains(cursor) {
                        return true;
                    }
                }
            }
            ast::Property::Method { body, .. } => {
                if let Some(body_id) = body {
                    let span = ast_tree.source_map.get(body_id.id);
                    if span.contains(cursor) {
                        return true;
                    }
                }
            }
            ast::Property::Spread { value, .. } => {
                let span = ast_tree.source_map.get(value.id);
                if span.contains(cursor) {
                    return true;
                }
            }
            ast::Property::Error => {}
        }
    }

    // fall back to property spans outside keys
    for property_id in properties {
        let span = ast_tree.source_map.get(property_id.id);
        if !span.contains(cursor) {
            continue;
        }

        let key_span = ast_tree.source_map.get_main(property_id.id);
        let is_in_key = key_span
            .map(|key_span| key_span.contains(cursor))
            .unwrap_or(false);

        if !is_in_key {
            return true;
        }
    }

    // return false when no value span matches
    false
}

/// Deduplicate names while preserving their first occurrence order.
fn deduplicate_names(names: &mut Vec<String>) {
    // retain the first occurrence of each name
    let mut deduped = HashSet::new();
    names.retain(|name| deduped.insert(name.clone()));
}
