use std::collections::HashSet;

use destack_source::{EnclosingSpan, FileId, ModuleId, NodeSpanType, PathExt, Span};
use {destack_ast as ast, destack_dir as dir};

use crate::Session;
use crate::query::common::{QueryContext, get_module_by_file_id};

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

/// Check whether the cursor is immediately after a dot.
fn is_after_dot(source: &str, offset: u32) -> bool {
    // require a nonzero offset so we can read the previous byte
    if offset == 0 {
        return false;
    }

    // check whether the previous byte is a dot
    source
        .as_bytes()
        .get(offset as usize - 1)
        .map(|&byte| byte == b'.')
        .unwrap_or(false)
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

/// Detect the completion context at a given offset.
pub fn detect_completion_context(session: &Session, file_id: FileId, offset: u32) -> ContextResult {
    // resolve the module for this file
    let Some(module) = get_module_by_file_id(session, file_id) else {
        return unknown_context();
    };

    // resolve the query context from the module
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return unknown_context();
    };

    // read the source text for this file
    let source_file = session.files.get(file_id);
    let source = source_file.text();

    // resolve cursor positioning and token context
    let cursor_position = offset.saturating_sub(1);

    // resolve trigger state and token prefix
    let after_dot = is_after_dot(source, offset);
    let token = detect_partial_identifier(source, offset);

    // detect member access inside a member name span
    {
        // resolve enclosing spans from innermost to outermost
        let enclosing = sorted_enclosing_spans(&ctx, cursor_position, cursor_position);

        // scan enclosing spans for a member expression at the cursor
        let dir_tree = ctx.tree();
        for enc in &enclosing {
            let main_span = ctx.ast.tree.source_map.get_main(enc.idx);
            let is_in_member_name = main_span
                .map(|span| span.contains(cursor_position))
                .unwrap_or(true);

            if !is_in_member_name {
                continue;
            }

            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
                continue;
            };

            if dir_node_id.ty == dir::NodeType::Expression {
                let Ok(expr_id) = dir_node_id.try_into() else {
                    continue;
                };
                let expr = dir_tree.get::<dir::Expression>(expr_id);

                // use the left operand when inside a member expression
                if let dir::Expression::Member { left, .. } = expr {
                    let receiver_local: dir::LocalNodeIdAny = (*left).into();
                    let receiver_global = receiver_local.into_global(ctx.module_id);
                    let receiver_symbol = get_expression_symbol(&dir_tree, *left);

                    // resolve the receiver type after releasing the dir tree guard
                    drop(dir_tree);
                    let receiver_type = get_receiver_type(&ctx, receiver_global, receiver_symbol);

                    return ContextResult {
                        context: CompletionContext::MemberAccess {
                            receiver_node: receiver_local,
                            receiver_symbol,
                            receiver_type,
                        },
                        token,
                    };
                }
            }
        }
    }

    // resolve member access context when immediately after dot
    if after_dot {
        // resolve position inside the receiver expression
        let receiver_position = offset.saturating_sub(2);

        // resolve enclosing spans at the receiver position
        let enclosing = sorted_enclosing_spans(&ctx, receiver_position, receiver_position);

        // resolve the dir tree for span to dir lookup
        let dir_tree = ctx.tree();

        // scan for the nearest enclosing expression
        for enc in &enclosing {
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
                continue;
            };

            if dir_node_id.ty == dir::NodeType::Expression {
                let Ok(expr_id) = dir_node_id.try_into() else {
                    continue;
                };
                let expr = dir_tree.get::<dir::Expression>(expr_id);

                // unwrap statement expressions to the inner expression
                let (actual_node_id, actual_expr) =
                    unwrap_statement_expression(&dir_tree, dir_node_id, expr);

                // prefer the member left operand as the receiver
                if let dir::Expression::Member { left, .. } = actual_expr {
                    let receiver_local: dir::LocalNodeIdAny = (*left).into();
                    let receiver_global = receiver_local.into_global(ctx.module_id);
                    let receiver_symbol = get_expression_symbol(&dir_tree, *left);

                    // release the dir tree guard before type queries
                    drop(dir_tree);

                    // resolve the receiver type
                    let receiver_type = get_receiver_type(&ctx, receiver_global, receiver_symbol);

                    return ContextResult {
                        context: CompletionContext::MemberAccess {
                            receiver_node: receiver_local,
                            receiver_symbol,
                            receiver_type,
                        },
                        token,
                    };
                }

                // otherwise treat the expression itself as the receiver
                let receiver_symbol = actual_expr.target_symbol();
                let receiver_global = actual_node_id.into_global(ctx.module_id);

                // release the dir tree guard before type queries
                drop(dir_tree);

                // resolve the receiver type
                let receiver_type = get_receiver_type(&ctx, receiver_global, receiver_symbol);

                return ContextResult {
                    context: CompletionContext::MemberAccess {
                        receiver_node: actual_node_id,
                        receiver_symbol,
                        receiver_type,
                    },
                    token,
                };
            }
        }
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
    if let Some(statement_context) = detect_statement_position(&ctx, source, offset) {
        return ContextResult {
            context: statement_context,
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
    // resolve the type table
    let types = ctx.types();

    // first try: get type directly from the expression node
    if let Some(type_id) = types.get_declared_or_inferred_type_id(receiver_global) {
        return Some(type_id);
    }

    // second try: use get_type_id_for_symbol which checks cached value types
    // and falls back to declared/inferred types from primary declaration
    if let Some(symbol_id) = receiver_symbol {
        let symbols = ctx.symbols();
        if let Some(type_id) = types.get_type_id_for_symbol(&symbols, symbol_id) {
            return Some(type_id);
        }
    }

    // return none when no type is available
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
fn detect_partial_identifier(source: &str, offset: u32) -> Option<TokenAtCursor> {
    // convert the buffer to bytes for scanning
    let bytes = source.as_bytes();
    let offset = offset as usize;

    // find start of identifier (scan backwards)
    let mut start = offset;
    while start > 0 {
        let c = bytes[start - 1];
        if c.is_ascii_alphanumeric() || c == b'_' {
            start -= 1;
        } else {
            break;
        }
    }

    // find end of identifier (scan forwards)
    let mut end = offset;
    while end < bytes.len() {
        let c = bytes[end];
        if c.is_ascii_alphanumeric() || c == b'_' {
            end += 1;
        } else {
            break;
        }
    }

    // return a token span when one is found
    if start < end {
        Some(TokenAtCursor {
            text: String::from_utf8_lossy(&bytes[start..end]).into_owned(),
            start: start as u32,
            end: end as u32,
        })
    } else {
        None
    }
}

/// Detect if the cursor is in a type position.
fn detect_type_position(ctx: &QueryContext<'_>, source: &str, offset: u32) -> bool {
    // resolve the previous offset for edge checks
    let previous_offset = offset.saturating_sub(1);

    // resolve enclosing spans from innermost to outermost
    let enclosing = enclosing_spans_with_previous(ctx, offset);

    // check for type side spans that contain the cursor
    for enc in &enclosing {
        if let Some(span) = ctx
            .ast
            .tree
            .source_map
            .get_side(enc.idx, NodeSpanType::Type)
            && (span.contains(offset) || span.contains(previous_offset))
        {
            return true;
        }
    }

    // check enclosing expressions that are known type expressions
    for enc in &enclosing {
        if ctx.ast.tree.get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast.tree.get(expr_id);

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

    // fall back to textual heuristics when side spans are unavailable
    detect_type_position_by_text(source, offset)
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
        if ctx.ast.tree.get_node_type(enc.idx) != ast::NodeType::Declaration {
            continue;
        }

        let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(enc.idx);
        let declaration = ctx.ast.tree.get(declaration_id);

        let ast::Declaration::Type { value, .. } = declaration else {
            continue;
        };

        let span = ctx.ast.tree.source_map.get(value.id);
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
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // scan enclosing expressions for import nodes under the cursor
    for enc in &enclosing {
        if ctx.ast.tree.get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast.tree.get(expr_id);

        if !matches!(expr, ast::Expression::Import { .. }) {
            continue;
        }

        // resolve the import span and detect path completions inside the string
        let import_span = ctx.ast.tree.source_map.get(enc.idx);
        let main_span = ctx.ast.tree.source_map.get_main(enc.idx);
        if let Some(span) = main_span
            && span.contains(offset)
        {
            let partial_path = extract_string_literal_prefix(source, span, offset);
            return Some(CompletionContext::ImportPath { partial_path });
        }

        // resolve import clause info and type only context
        if let Some(info) = import_clause_info(ctx, expr, offset, source, import_span) {
            let ast::Expression::Import { target, .. } = expr else {
                continue;
            };

            // resolve the target module from the import specifier
            let target_specifier = ctx.ast.strings.get(*target).to_string();
            let target_module = resolve_import_target_module(session, ctx, &target_specifier);

            // detect type only imports by span or line
            let span_is_type_only = source
                .get(import_span.start as usize..import_span.end as usize)
                .map(|text| text.trim_start().starts_with("import type"))
                .unwrap_or(false);
            let line_start = (0..offset as usize)
                .rev()
                .find(|&i| source.as_bytes()[i] == b'\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let line_end = (offset as usize..source.len())
                .find(|&i| source.as_bytes()[i] == b'\n')
                .unwrap_or(source.len());
            let line_is_type_only = source[line_start..line_end]
                .trim_start()
                .starts_with("import type");
            let is_type_only = span_is_type_only || line_is_type_only;

            // enforce type space filtering inside type only imports
            let space_filter = if info.space_filter.is_none() && is_type_only {
                Some(dir::SymbolSpace::Type)
            } else {
                info.space_filter
            };

            return Some(CompletionContext::ImportClause {
                target_module,
                existing_names: info.existing_names,
                space_filter,
            });
        }
    }

    detect_import_context_by_text(session, ctx, source, offset)
}

/// Detect type position from raw text as a fallback for incomplete parses.
fn detect_type_position_by_text(source: &str, offset: u32) -> bool {
    // resolve the byte buffer and cursor position
    let bytes = source.as_bytes();
    let mut pos = offset as usize;

    // skip whitespace backwards
    while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
        pos -= 1;
    }

    // check for type annotation indicators
    if pos > 0 {
        let c = bytes[pos - 1];
        if c == b':' || c == b'<' || c == b',' {
            return true;
        }
    }

    // check for keyword patterns (extends, implements, is)
    let prefix = &source[..pos.min(source.len())];
    let trimmed = prefix.trim_end();
    if trimmed.ends_with("extends")
        || trimmed.ends_with("implements")
        || trimmed.ends_with("is")
        || trimmed.ends_with("as")
    {
        return true;
    }

    // return false when no type hint is detected
    false
}

/// Detect import related context from raw text as a fallback.
fn detect_import_context_by_text(
    session: &Session,
    ctx: &QueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<CompletionContext> {
    // resolve the cursor offset as a usize
    let offset = offset as usize;
    let (statement_start, statement_end) = find_import_statement(source, offset)?;
    let statement = &source[statement_start..statement_end];
    if !statement.trim_start().starts_with("import") {
        return None;
    }

    let cursor_in_stmt = offset.saturating_sub(statement_start);

    if let Some(partial_path) = parse_import_path_by_text(statement, cursor_in_stmt) {
        return Some(CompletionContext::ImportPath { partial_path });
    }

    let target_module = parse_import_target_by_text(statement)
        .and_then(|target| resolve_import_target_module(session, ctx, &target));

    let (existing_names, cursor_is_type, cursor_in_clause) =
        parse_import_clause_by_text(statement, cursor_in_stmt);
    if !cursor_in_clause {
        return None;
    }

    let is_type_only = statement.trim_start().starts_with("import type");
    let space_filter = if is_type_only || cursor_is_type {
        Some(dir::SymbolSpace::Type)
    } else {
        None
    };

    // return the parsed import clause context
    Some(CompletionContext::ImportClause {
        target_module,
        existing_names,
        space_filter,
    })
}

/// Find the bounds of the import statement that contains the cursor.
fn find_import_statement(source: &str, offset: usize) -> Option<(usize, usize)> {
    // find the nearest import keyword before the cursor
    let statement_start = source[..offset].rfind("import")?;
    if source[statement_start..offset].contains(';') {
        return None;
    }

    // find the end of the import statement
    let statement_end = source[statement_start..]
        .find(';')
        .map(|idx| statement_start + idx)
        .unwrap_or(source.len());

    // return the statement bounds
    Some((statement_start, statement_end))
}

/// Parse the target module string from an import statement.
fn parse_import_target_by_text(statement: &str) -> Option<String> {
    // locate the from keyword
    let from_idx = statement.find("from")?;
    let after_from = &statement[from_idx + 4..];

    // locate the opening quote
    let mut quote_idx = None;
    let mut quote_char = '"';
    for (idx, ch) in after_from.char_indices() {
        if ch == '"' || ch == '\'' {
            quote_idx = Some(idx);
            quote_char = ch;
            break;
        }
    }
    let quote_idx = quote_idx?;
    let rest = &after_from[quote_idx + 1..];

    // locate the closing quote
    let end_idx = rest.find(quote_char)?;

    // return the parsed target path
    Some(rest[..end_idx].to_string())
}

/// Parse the partially typed import path from a statement and cursor offset.
fn parse_import_path_by_text(statement: &str, cursor_in_stmt: usize) -> Option<String> {
    // locate the from keyword before the cursor
    let from_idx = statement[..cursor_in_stmt.min(statement.len())].rfind("from")?;
    let after_from = &statement[from_idx + 4..];
    let cursor_after_from = cursor_in_stmt.saturating_sub(from_idx + 4);
    let before_cursor = &after_from[..cursor_after_from.min(after_from.len())];

    // locate the nearest opening quote before the cursor
    let mut opening_idx = None;
    let mut quote_char = '"';
    if let Some(idx) = before_cursor.rfind('"') {
        opening_idx = Some(idx);
        quote_char = '"';
    }
    if let Some(idx) = before_cursor.rfind('\'') {
        let current = opening_idx.unwrap_or(0);
        if opening_idx.is_none() || idx > current {
            opening_idx = Some(idx);
            quote_char = '\'';
        }
    }
    let opening_idx = opening_idx?;
    let opening_in_stmt = from_idx + 4 + opening_idx;
    let path_start = opening_in_stmt + 1;

    // return an empty prefix when cursor is before the path
    if cursor_in_stmt <= path_start {
        return Some(String::new());
    }

    // ignore when cursor is past the closing quote
    let rest = &statement[path_start..];
    if let Some(end_idx) = rest.find(quote_char) {
        let closing_in_stmt = path_start + end_idx;
        if cursor_in_stmt > closing_in_stmt {
            return None;
        }
    }

    // slice the prefix up to the cursor
    let end = cursor_in_stmt.min(statement.len());

    // return the partial path
    statement.get(path_start..end).map(str::to_string)
}

/// Parse import clause names and cursor context from raw text.
fn parse_import_clause_by_text(
    statement: &str,
    cursor_in_stmt: usize,
) -> (Vec<String>, bool, bool) {
    // locate the import clause braces
    let Some(open_idx) = statement.find('{') else {
        return (Vec::new(), false, false);
    };
    let Some(close_idx) = statement.rfind('}') else {
        return (Vec::new(), false, false);
    };
    if close_idx <= open_idx + 1 {
        return (Vec::new(), false, false);
    }

    // ensure the cursor is inside the clause
    let cursor_in_clause = cursor_in_stmt > open_idx && cursor_in_stmt <= close_idx;
    if !cursor_in_clause {
        return (Vec::new(), false, false);
    }

    // seed names from the prefix and prepare state
    let mut existing_names = parse_import_prefix_names(&statement[..open_idx]);
    let mut cursor_is_type = false;
    let mut segment_start = open_idx + 1;

    // walk comma separated segments
    for (idx, ch) in statement[open_idx + 1..close_idx].char_indices() {
        if ch != ',' {
            continue;
        }

        let segment_end = open_idx + 1 + idx;
        let is_cursor_segment = cursor_in_stmt >= segment_start && cursor_in_stmt <= segment_end;
        let segment = statement[segment_start..segment_end].trim();

        if !segment.is_empty() {
            let parsed = parse_import_item_names(segment);
            if is_cursor_segment {
                cursor_is_type = parsed.is_type;
            } else {
                existing_names.extend(parsed.names);
            }
        }

        segment_start = segment_end + 1;
    }

    // handle the final segment after the last comma
    let last_segment = statement[segment_start..close_idx].trim();
    let cursor_in_last = cursor_in_stmt >= segment_start && cursor_in_stmt <= close_idx;
    if !last_segment.is_empty() {
        let parsed = parse_import_item_names(last_segment);
        if cursor_in_last {
            cursor_is_type = parsed.is_type;
        } else {
            existing_names.extend(parsed.names);
        }
    } else if cursor_in_last {
        // allow a trailing type keyword segment
        let prefix = statement[open_idx + 1..cursor_in_stmt.min(close_idx)].trim();
        if prefix == "type" {
            cursor_is_type = true;
        }
    }

    // return parsed names and cursor flags
    (existing_names, cursor_is_type, true)
}

/// Parse names from the import prefix segment.
fn parse_import_prefix_names(prefix: &str) -> Vec<String> {
    // normalize the prefix text
    let prefix = prefix.trim();
    let prefix = prefix.strip_prefix("import").unwrap_or(prefix).trim();
    let prefix = prefix.strip_prefix("type").unwrap_or(prefix).trim();
    if prefix.is_empty() {
        return Vec::new();
    }

    // parse comma separated items
    let mut names = Vec::new();
    for segment in prefix.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        let parsed = parse_import_item_names(segment);
        names.extend(parsed.names);
    }

    // return the collected names
    names
}

/// Parsed names and flags for an import item segment.
struct ParsedImportItem {
    /// The parsed names for the item.
    names: Vec<String>,
    /// Whether the item is marked as type only.
    is_type: bool,
}

/// Parse a clause segment into names and type flag.
fn parse_import_item_names(segment: &str) -> ParsedImportItem {
    // tokenize and clean the segment
    let cleaned = segment
        .split_whitespace()
        .map(clean_import_token)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut tokens = cleaned.iter().map(String::as_str);

    // read the first token
    let Some(first) = tokens.next() else {
        return ParsedImportItem {
            names: Vec::new(),
            is_type: false,
        };
    };

    // handle optional leading type keyword
    let mut is_type = false;
    let mut names = Vec::new();
    let mut current = first;

    if current == "type" {
        is_type = true;
        let Some(next) = tokens.next() else {
            return ParsedImportItem { names, is_type };
        };
        current = next;
    }

    // handle namespace imports
    if current == "*" {
        if let Some(alias) = parse_alias(tokens) {
            names.push(alias);
        }
        return ParsedImportItem { names, is_type };
    }

    // collect the primary name and alias
    names.push(current.to_string());
    if let Some(alias) = parse_alias(tokens) {
        names.push(alias);
    }

    // return parsed names and flags
    ParsedImportItem { names, is_type }
}

/// Parse an alias token sequence from remaining tokens.
fn parse_alias<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Option<String> {
    // scan for an as keyword
    while let Some(token) = tokens.next() {
        if token == "as" {
            return tokens.next().map(str::to_string);
        }
    }

    // return none when no alias is present
    None
}

/// Clean a raw token by trimming punctuation.
fn clean_import_token(token: &str) -> String {
    // strip punctuation characters from both sides
    token
        .trim_matches(|c: char| matches!(c, '{' | '}' | ',' | ';'))
        .to_string()
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
    let module = module.read();
    let base_path = if let Some(path) = module.path.clone() {
        Some(path)
    } else {
        let uri = module.uri.as_ref();
        let stripped = uri.strip_prefix("file://");
        stripped
            .map(std::path::PathBuf::from)
            .or_else(|| module.uri.to_path_buf())
    }?;
    let base_dir = base_path.parent()?;
    let target_path = std::path::Path::new(target);

    // build candidate paths for common extensions
    let mut candidates = Vec::new();
    let mut full_path = base_dir.join(target_path).normalize();
    candidates.push(full_path.clone());

    if full_path.extension().is_none() {
        full_path.set_extension("ds");
        candidates.push(full_path.clone());

        let mut ts_path = base_dir.join(target_path);
        ts_path.set_extension("ts");
        candidates.push(ts_path);
    }

    // resolve candidates by path and uri
    for candidate in candidates {
        if let Some(module_id) = session.modules.get_id_by_path(&candidate) {
            return Some(module_id);
        }

        let module_id = candidate
            .canonicalize()
            .ok()
            .and_then(|canonical| session.modules.get_id_by_path(&canonical));
        if let Some(module_id) = module_id {
            return Some(module_id);
        }

        let uri_prefix = module.uri.as_ref();
        let candidate_uri = if uri_prefix.starts_with("file://") {
            destack_source::Uri::from_string(format!("file://{}", candidate.to_string_lossy()))
        } else {
            destack_source::Uri::from_path(candidate)
        };
        if let Some(module_id) = session.modules.get_id_by_uri(&candidate_uri) {
            return Some(module_id);
        }
    }

    // return none when no candidate resolves
    None
}

/// Find the scope at a given offset.
fn find_scope_at_offset(
    ctx: &crate::query::common::QueryContext<'_>,
    offset: u32,
) -> Option<ScopeAtOffset> {
    // resolve enclosing spans at the cursor and previous byte
    let enclosing = enclosing_spans_with_previous(ctx, offset);

    // bail out early when there are no enclosing spans
    if enclosing.is_empty() {
        return None;
    }

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
                    scope_mark_for_scope_at_offset(ctx, &dir_tree, &symbols, scope_id, offset);
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
                    scope_mark_for_scope_at_offset(ctx, &dir_tree, &symbols, scope_id, offset);
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark,
                });
            }
        }

        // for Declaration nodes, find the owned scope if this is a function/class
        if dir_node_id.ty == dir::NodeType::Declaration {
            let scope_id = find_owned_scope_for_declaration(&symbols, dir_node_id.id);
            if let Some(scope_id) = scope_id {
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark: dir::LocalScopeMark::end(),
                });
            }
        }
    }

    // fallback: walk AST parents from the innermost node
    let start_id = enclosing.first().map(|enc| enc.idx)?;

    for parent_id in ctx.ast.parents.walk_parents_by_id(start_id) {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(parent_id) else {
            if ctx.ast.tree.get_node_type(parent_id) == ast::NodeType::Declaration {
                let scope_id = find_owned_scope_for_ast_declaration(&symbols, &dir_tree, parent_id);
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
                    scope_mark_for_scope_at_offset(ctx, &dir_tree, &symbols, scope_id, offset);
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
                    scope_mark_for_scope_at_offset(ctx, &dir_tree, &symbols, scope_id, offset);
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark,
                });
            }
        }

        if dir_node_id.ty == dir::NodeType::Declaration {
            let scope_id = find_owned_scope_for_declaration(&symbols, dir_node_id.id);
            if let Some(scope_id) = scope_id {
                return Some(ScopeAtOffset {
                    scope_id,
                    scope_mark: dir::LocalScopeMark::end(),
                });
            }
        }
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
        if ctx.ast.tree.get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        // resolve the AST expression node
        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast.tree.get(expr_id);

        // require an object literal expression
        let ast::Expression::ObjectExpression { properties, .. } = expr else {
            continue;
        };

        // require a key position inside the object literal
        if !is_object_literal_key_position(&ctx.ast.tree, properties, offset) {
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
                    extract_property_names(&dir_tree, properties, &session.strings);

                // resolve expected type from contextual typing when possible
                let expected_type = expected_type_for_object_literal(
                    ctx,
                    &dir_tree,
                    &symbols,
                    &types,
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
                    extract_property_names(&dir_tree, properties, &session.strings);

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
                            let left_expression = dir_tree.get(*left);
                            if let Some(target_symbol) = left_expression.target_symbol() {
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
        if ctx.ast.tree.get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        // resolve the AST expression node
        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast.tree.get(expr_id);

        // require an object literal expression
        let ast::Expression::ObjectExpression { properties, .. } = expr else {
            continue;
        };

        // skip key positions inside object literals
        if is_object_literal_key_position(&ctx.ast.tree, properties, offset) {
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
        let scope_mark = scope_mark_for_scope_at_offset(ctx, &dir_tree, &symbols, scope_id, offset);
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

    // bail out when there are no spans
    if enclosing.is_empty() {
        return None;
    }

    // load dir tables for node inspection
    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();

    // scan spans for a new expression containing the cursor
    for enc in &enclosing {
        // try to build a context for this span
        if let Some(context) =
            new_expression_context_for_span(ctx, &dir_tree, &symbols, enc, offset)
        {
            return Some(context);
        }
    }

    // fall back to new keyword text detection
    if is_after_new_keyword(source, offset) {
        return Some(CompletionContext::NewExpression {
            scope_id: None,
            scope_mark: None,
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
    let left_span = ctx.ast.tree.source_map.get(left_source_id);

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
    let file_id = ctx.file_id;

    // scan spans for a call or new expression argument list
    for enc in &enclosing {
        // try to build a context for this span
        if let Some(context) =
            call_argument_context_for_span(ctx, &dir_tree, &symbols, file_id, enc, offset)
        {
            return Some(context);
        }
    }

    None
}

/// Build a call argument context from a single enclosing span.
fn call_argument_context_for_span(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    file_id: FileId,
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
    let left_span = ctx.ast.tree.source_map.get(left_source_id);
    let call_span = ctx.ast.tree.source_map.get(enc.idx);

    // ensure the cursor is within the argument list
    if !cursor_in_argument_list(
        ctx,
        file_id,
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

/// Detect whether the cursor is at a statement position.
fn detect_statement_position(
    ctx: &QueryContext<'_>,
    source: &str,
    offset: u32,
) -> Option<CompletionContext> {
    // check block based statement gaps first
    if let Some(scope) = statement_position_from_block(ctx, offset) {
        return Some(statement_context_from_scope(Some(scope)));
    }

    // fall back to text heuristics
    if is_statement_position_by_text(source, offset) {
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
    // resolve enclosing spans at the cursor
    let enclosing = sorted_enclosing_spans(ctx, offset, offset);

    // bail out when there are no spans
    if enclosing.is_empty() {
        return None;
    }

    // scan for the nearest block that does not contain an expression at the cursor
    for enc in &enclosing {
        if statement_gap_in_block(ctx, enc, offset) {
            return scope_from_block_span(ctx, enc, offset);
        }
    }

    None
}

/// Check whether the cursor is in a statement gap within a block.
fn statement_gap_in_block(ctx: &QueryContext<'_>, enc: &EnclosingSpan, offset: u32) -> bool {
    // only consider block nodes
    if ctx.ast.tree.get_node_type(enc.idx) != ast::NodeType::Block {
        return false;
    }

    // resolve the block node
    let block_id = ast::LocalNodeId::<ast::Block>::new(enc.idx);
    let block = ctx.ast.tree.get(block_id);

    // treat empty blocks as statement positions
    if block.expressions.is_empty() {
        return true;
    }

    // check whether the cursor is inside any expression span
    for expr_id in &block.expressions {
        let span = ctx.ast.tree.source_map.get(expr_id.id);
        if span.contains(offset) {
            return false;
        }
    }

    true
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
    let scope_mark = scope_mark_for_scope_at_offset(ctx, &dir_tree, &symbols, scope_id, offset);

    Some(ScopeAtOffset {
        scope_id,
        scope_mark,
    })
}

/// Check statement position using text heuristics.
fn is_statement_position_by_text(source: &str, offset: u32) -> bool {
    // treat start of file as statement position
    if offset == 0 {
        return true;
    }

    // find the previous non whitespace byte
    let prev = previous_non_whitespace(source, offset);
    let Some(prev_char) = prev else {
        return true;
    };

    matches!(prev_char, '{' | '}' | ';')
}

/// Find the previous non whitespace character before an offset.
fn previous_non_whitespace(source: &str, offset: u32) -> Option<char> {
    // scan backwards in the source bytes
    let bytes = source.as_bytes();
    let mut pos = offset as usize;

    while pos > 0 {
        let c = bytes[pos - 1];
        if !c.is_ascii_whitespace() {
            return Some(c as char);
        }

        pos -= 1;
    }

    // return none when no non whitespace character exists
    None
}

/// Check whether the cursor is inside a call argument list.
fn cursor_in_argument_list(
    ctx: &QueryContext<'_>,
    file_id: FileId,
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
            let span = get_argument_span(ctx, file_id, dir_tree, arg_node_id);
            min_start = min_start.min(span.start);
            max_end = max_end.max(span.end);
        }

        if offset >= min_start && offset <= max_end {
            return true;
        }
    }

    offset > left_span.end && offset <= call_span.end
}

/// Resolve the span for an argument node.
fn get_argument_span(
    ctx: &QueryContext<'_>,
    file_id: FileId,
    dir_tree: &dir::NodeTree,
    node_id: dir::LocalNodeIdAny,
) -> Span {
    // resolve the source span for the argument node
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ctx.ast.tree.source_map.get(source_id);

    Span::new(file_id, ast_span.start, ast_span.end)
}

/// Check whether the cursor is positioned after a new keyword.
fn is_after_new_keyword(source: &str, offset: u32) -> bool {
    // validate the offset against the source bounds
    let offset = offset as usize;
    if offset == 0 || offset > source.len() {
        return false;
    }

    // scan backward to the start of the current token
    let bytes = source.as_bytes();
    let mut start = offset;
    while start > 0 {
        let c = bytes[start - 1];
        if c.is_ascii_alphanumeric() || c == b'_' {
            start -= 1;
        } else {
            break;
        }
    }

    // skip whitespace before the token
    let mut pos = start;
    while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
        pos -= 1;
    }

    // ensure there is room for the keyword
    if pos < 3 {
        return false;
    }

    // match the keyword text
    let keyword = &source[pos - 3..pos];
    if keyword != "new" {
        return false;
    }

    // ensure the keyword is not part of a larger identifier
    if pos > 3 {
        let prev = bytes[pos - 4];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return false;
        }
    }

    true
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
    strings: &destack_base::StringPool,
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
) -> Option<ImportClauseInfo> {
    // require an import expression
    let ast::Expression::Import { items, kind, .. } = expr else {
        return None;
    };

    // resolve the import text and cursor position
    let import_text = source.get(import_span.start as usize..import_span.end as usize)?;
    let cursor_in_span = offset.saturating_sub(import_span.start) as usize;
    let (mut existing_names, cursor_is_type_text, cursor_in_clause_text) =
        parse_import_clause_by_text(import_text, cursor_in_span);

    let mut in_item_kind = None;
    let mut min_start: Option<u32> = None;
    let mut max_end: Option<u32> = None;

    for item_id in items {
        let item = ctx.ast.tree.get(*item_id);
        let span = ctx.ast.tree.source_map.get(item_id.id);

        min_start = Some(min_start.map_or(span.start, |start| start.min(span.start)));
        max_end = Some(max_end.map_or(span.end, |end| end.max(span.end)));

        if span.contains(offset) {
            in_item_kind = item.kind;
            continue;
        }

        if let Some(name_id) = item.name {
            existing_names.push(ctx.ast.strings.get(name_id).to_string());
        }
        if let Some(alias_id) = item.alias {
            existing_names.push(ctx.ast.strings.get(alias_id).to_string());
        }
    }

    let cursor_in_items = if let (Some(start), Some(end)) = (min_start, max_end) {
        offset >= start && offset <= end
    } else {
        false
    };
    let cursor_in_clause = cursor_in_clause_text || cursor_in_items;
    if !cursor_in_clause && in_item_kind.is_none() {
        return None;
    }

    let space_filter = match kind {
        ast::DependencyKind::Type => Some(dir::SymbolSpace::Type),
        ast::DependencyKind::Value => match in_item_kind {
            Some(ast::DependencyKind::Type) => Some(dir::SymbolSpace::Type),
            Some(ast::DependencyKind::Value) => Some(dir::SymbolSpace::Value),
            None if cursor_is_type_text => Some(dir::SymbolSpace::Type),
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
        if ctx.ast.tree.get_node_type(enc.idx) != ast::NodeType::Expression {
            continue;
        }

        let expr_id = ast::LocalNodeId::<ast::Expression>::new(enc.idx);
        let expr = ctx.ast.tree.get(expr_id);

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

/// Extract a prefix from a string literal span at a cursor offset.
fn extract_string_literal_prefix(source: &str, span: destack_source::Span, offset: u32) -> String {
    // clamp the span within the source
    let start = span.start as usize;
    let end = span.end.min(source.len() as u32) as usize;
    if start >= end {
        return String::new();
    }

    // extract the literal text and cursor position
    let literal = &source[start..end];
    let offset_in_literal = offset.saturating_sub(span.start) as usize;

    // compute the content boundaries inside quotes
    let (content_start, content_end) = match literal.as_bytes().first().copied() {
        Some(b'"') | Some(b'\'') => {
            let end = literal
                .as_bytes()
                .last()
                .copied()
                .filter(|b| *b == b'"' || *b == b'\'')
                .map(|_| literal.len().saturating_sub(1))
                .unwrap_or(literal.len());
            (1, end)
        }
        _ => (0, literal.len()),
    };

    if offset_in_literal <= content_start {
        return String::new();
    }

    // return the prefix up to the cursor
    let prefix_end = offset_in_literal.min(content_end);
    literal[content_start..prefix_end].to_string()
}

/// Collect enclosing spans and sort from innermost to outermost.
fn sorted_enclosing_spans(ctx: &QueryContext<'_>, start: u32, end: u32) -> Vec<EnclosingSpan> {
    // collect enclosing spans from the source map
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(start, end);

    // sort by span length so innermost spans come first
    enclosing.sort_by_key(|span| span.length);

    enclosing
}

/// Collect enclosing spans at the cursor and previous byte.
fn enclosing_spans_with_previous(ctx: &QueryContext<'_>, offset: u32) -> Vec<EnclosingSpan> {
    // collect enclosing spans at the cursor position
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);

    // include enclosing spans at the previous byte for boundary cases
    if offset > 0 {
        let previous_offset = offset - 1;
        let mut previous = ctx
            .ast
            .tree
            .source_map
            .get_enclosing_spans(previous_offset, previous_offset);
        enclosing.append(&mut previous);
    }

    // deduplicate by span index when we have overlapping collections
    if !enclosing.is_empty() {
        let mut seen = HashSet::new();
        enclosing.retain(|span| seen.insert(span.idx));
    }

    // sort by span length so innermost spans come first
    enclosing.sort_by_key(|span| span.length);

    enclosing
}

/// Deduplicate names while preserving their first occurrence order.
fn deduplicate_names(names: &mut Vec<String>) {
    // retain the first occurrence of each name
    let mut deduped = HashSet::new();
    names.retain(|name| deduped.insert(name.clone()));
}
