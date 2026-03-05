use destack_base::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, DynamicKey, EnumField, Expression,
    GlobalSymbolId, LocalNodeIdAny, Member, NodeType, Parameter, Pattern, PatternField,
    SymbolSpace,
};
use destack_source::{FileId, NodeSpanType, Span};
use {destack_ast as ast, destack_dir as dir};

use super::resolve::global_symbol;
use super::{
    QueryContext, get_dir_node_main_span, get_dir_node_span, get_module_by_file_id,
    token_at_offset, token_span_at_offset,
};
use crate::Session;
use crate::program::{ModuleAst, ModuleDir};

pub(crate) use super::resolve::{
    dependency_item_matches_name, matches_symbol_space_filter, owned_scope_for_symbol,
    resolve_member_access_symbol, resolve_nominal_symbol_from_initializer,
    resolve_nominal_symbol_from_type_expression, resolve_type_symbol_from_dependency_symbol,
    resolve_type_symbol_from_module,
};

/// Result of finding a symbol at an offset.
#[derive(Debug, Clone)]
pub struct SymbolAtOffset {
    /// The symbol that was referenced.
    pub symbol_id: GlobalSymbolId,
    /// The DIR node that contains the reference.
    pub node_id: LocalNodeIdAny,
    /// The span of the reference.
    pub span: Span,
}

/// Find the symbol referenced at a given offset.
pub fn find_symbol_at_offset(
    session: &Session,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    find_symbol_at_offset_impl(session, file_id, offset)
}

/// Find the symbol for hover at a given offset.
///
/// This resolves normal symbol tokens first and then declaration modifier
/// keywords (`export`, `declare`, `abstract`, etc.) to their declaration symbol.
pub(crate) fn find_symbol_for_hover_at_offset(
    session: &Session,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    find_symbol_at_offset(session, file_id, offset)
        .or_else(|| declaration_modifier_symbol_at_offset(session, file_id, offset))
}

/// Resolve a symbol from structural AST and DIR mappings.
fn find_symbol_at_offset_impl(
    session: &Session,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve the module and query context
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // find AST nodes at the offset
    let enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    // sort by length (smallest first) to get most specific node
    let mut enclosing = enclosing;

    // prefer the most specific span when multiple nodes tie
    enclosing.sort_by_key(|span| (span.length, -(span.idx as i64)));

    // skip doc and comment tokens before resolving symbols
    let mut is_comment_token = |token: &ast::TokenSpan| {
        if token.span.file != ctx.file_id {
            return false;
        }
        matches!(
            token.token.ty,
            ast::TokenType::DocLineComment
                | ast::TokenType::DocBlockComment
                | ast::TokenType::LineComment
                | ast::TokenType::BlockComment
        ) && token.span.contains(offset)
    };

    if ctx.ast.tokens.iter().any(&mut is_comment_token)
        || ctx.ast.side_tokens.iter().any(&mut is_comment_token)
    {
        return None;
    }

    // check if we're in a doc/comment first (these should not return symbols)
    for enclosing_span in &enclosing {
        let node_type = ctx.ast.tree.get_node_type(enclosing_span.idx);
        if matches!(node_type, ast::NodeType::Doc | ast::NodeType::Comment) {
            return None;
        }
    }

    let dir_tree = ctx.tree();

    // check static parameters first to avoid capturing the enclosing declaration
    if let Some(result) = static_parameter_symbol_at_offset(session, &ctx, offset) {
        return Some(result);
    }

    // scan member access expressions first to lock onto the member name span
    for (expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expr else {
            continue;
        };

        // resolve member symbols when the cursor is on the member name
        if let Some(result) =
            member_symbol_at_offset(session, &ctx, expr_id, expr_id.into(), *left, *name, offset)
        {
            return Some(result);
        }
    }

    // try each AST node from smallest to largest
    for enclosing_span in &enclosing {
        // try to get the DIR node for this AST node
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
            continue;
        };

        match dir_node_id.ty {
            // check if it's an expression with a target_symbol (reference)
            NodeType::Expression => {
                let Ok(expr_id) = dir_node_id.try_into() else {
                    continue;
                };
                let expr = dir_tree.get::<Expression>(expr_id);
                let mut skip_expression_target_symbol = false;

                // resolve member symbols when the cursor is on the member name
                if let Expression::Member { left, name, .. } = expr {
                    let member_name = session.strings.get(*name);
                    let is_member_name_token = token_at_offset(session, ctx.file_id, offset)
                        .as_deref()
                        .is_some_and(|token| member_name == token);

                    if !is_member_name_token {
                        skip_expression_target_symbol = true;
                    }

                    let result = member_symbol_at_offset(
                        session,
                        &ctx,
                        expr_id,
                        dir_node_id,
                        *left,
                        *name,
                        offset,
                    );
                    if let Some(result) = result {
                        return Some(result);
                    }

                    // use the recorded member target when the cursor is on the member name
                    if is_member_name_token && let Some(target_symbol) = expr.target_symbol() {
                        let span = get_member_access_name_span(&ctx, expr_id)
                            .or_else(|| {
                                token_span_at_offset(session, ctx.file_id, offset)
                                    .map(|token| token.span)
                            })
                            .unwrap_or_else(|| {
                                Span::new(
                                    ctx.file_id,
                                    enclosing_span.span.start,
                                    enclosing_span.span.end,
                                )
                            });

                        return Some(SymbolAtOffset {
                            symbol_id: target_symbol,
                            node_id: dir_node_id,
                            span,
                        });
                    }

                    // treat offsets before the member name as receiver positions
                    if let Some(name_span) = get_member_access_name_span(&ctx, expr_id)
                        && offset < name_span.start
                    {
                        skip_expression_target_symbol = true;

                        if let Some(receiver_symbol) =
                            dir_tree.get::<Expression>(*left).target_symbol()
                        {
                            let receiver_span =
                                get_dir_node_main_span(ctx.ast, ctx.dir, (*left).into())
                                    .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, (*left).into()))
                                    .or_else(|| {
                                        get_dir_node_span(ctx.ast, ctx.dir, dir_node_id).and_then(
                                            |member_span| {
                                                let receiver_end =
                                                    name_span.start.saturating_sub(1);
                                                (member_span.start <= receiver_end).then_some(
                                                    Span::new(
                                                        member_span.file,
                                                        member_span.start,
                                                        receiver_end,
                                                    ),
                                                )
                                            },
                                        )
                                    });

                            if let Some(receiver_span) = receiver_span
                                && offset_matches_symbol_span(offset, receiver_span)
                            {
                                return Some(SymbolAtOffset {
                                    symbol_id: receiver_symbol,
                                    node_id: (*left).into(),
                                    span: receiver_span,
                                });
                            }
                        }
                    }

                    // check if the cursor is on the left expression and resolve that symbol
                    if let Some(left_span) = get_dir_node_span(ctx.ast, ctx.dir, (*left).into())
                        && offset >= left_span.start
                        && offset <= left_span.end
                    {
                        let left_expr = dir_tree.get::<Expression>(*left);
                        if let Some(target_symbol) = left_expr.target_symbol() {
                            return Some(SymbolAtOffset {
                                symbol_id: target_symbol,
                                node_id: (*left).into(),
                                span: left_span,
                            });
                        }
                    }
                }

                if !skip_expression_target_symbol && let Some(target_symbol) = expr.target_symbol()
                {
                    let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                        .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                        .unwrap_or_else(|| {
                            Span::new(
                                ctx.file_id,
                                enclosing_span.span.start,
                                enclosing_span.span.end,
                            )
                        });
                    if !offset_matches_symbol_span(offset, span) {
                        continue;
                    }

                    return Some(SymbolAtOffset {
                        symbol_id: target_symbol,
                        node_id: dir_node_id,
                        span,
                    });
                }

                // resolve unresolved type references through type-only imports
                if let Some(target_symbol) =
                    unresolved_type_symbol_for_expression(session, &ctx, expr)
                {
                    let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                        .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                        .unwrap_or_else(|| {
                            Span::new(
                                ctx.file_id,
                                enclosing_span.span.start,
                                enclosing_span.span.end,
                            )
                        });
                    if !offset_matches_symbol_span(offset, span) {
                        continue;
                    }

                    return Some(SymbolAtOffset {
                        symbol_id: target_symbol,
                        node_id: dir_node_id,
                        span,
                    });
                }
            }
            // check if it's a pattern (variable binding definition)
            NodeType::Pattern => {
                let Ok(pattern_id) = dir_node_id.try_into() else {
                    continue;
                };
                let pattern = dir_tree.get::<Pattern>(pattern_id);
                if let Some(local_symbol) = pattern.symbol() {
                    let symbol_id = global_symbol(ctx.module_id, local_symbol);
                    let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                        .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                        .unwrap_or_else(|| {
                            Span::new(
                                ctx.file_id,
                                enclosing_span.span.start,
                                enclosing_span.span.end,
                            )
                        });
                    if !offset_matches_symbol_span(offset, span) {
                        continue;
                    }

                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span,
                    });
                }
            }
            // check if it's a pattern field (destructuring binding definition)
            NodeType::PatternField => {
                let Ok(field_id) = dir_node_id.try_into() else {
                    continue;
                };
                let field = dir_tree.get::<PatternField>(field_id);
                if let Some(local_symbol) = field.symbol() {
                    let symbol_id = global_symbol(ctx.module_id, local_symbol);
                    let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                        .unwrap_or_else(|| {
                            Span::new(
                                ctx.file_id,
                                enclosing_span.span.start,
                                enclosing_span.span.end,
                            )
                        });

                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span,
                    });
                }

                if let Some(symbol_at) =
                    pattern_field_symbol_at_offset(session, &ctx, &dir_tree, field_id, offset)
                {
                    return Some(symbol_at);
                }
            }
            // check if it's a declaration (function/struct/class definition)
            NodeType::Declaration => {
                let Ok(declaration_id): Result<dir::LocalNodeId<Declaration>, _> =
                    dir_node_id.try_into()
                else {
                    continue;
                };
                let declaration = dir_tree.get::<Declaration>(declaration_id);
                let local_symbol = declaration.symbol();
                let symbol_id = global_symbol(ctx.module_id, local_symbol);
                let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                    .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                    .unwrap_or_else(|| {
                        Span::new(
                            ctx.file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        )
                    });
                if !offset_matches_symbol_span(offset, span) {
                    continue;
                }

                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span,
                });
            }
            // check if it's a member (class/struct field or method)
            NodeType::Member => {
                let Ok(member_id): Result<dir::LocalNodeId<Member>, _> = dir_node_id.try_into()
                else {
                    continue;
                };
                let member = dir_tree.get::<Member>(member_id);
                let local_symbol = member.symbol();
                let symbol_id = global_symbol(ctx.module_id, local_symbol);
                let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                    .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                    .unwrap_or_else(|| {
                        Span::new(
                            ctx.file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        )
                    });
                if !offset_matches_symbol_span(offset, span) {
                    continue;
                }

                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span,
                });
            }
            // check if it's an enum field
            NodeType::EnumField => {
                let Ok(field_id) = dir_node_id.try_into() else {
                    continue;
                };
                let field = dir_tree.get::<EnumField>(field_id);
                let symbol_id = global_symbol(ctx.module_id, field.symbol);
                let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                    .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                    .unwrap_or_else(|| {
                        Span::new(
                            ctx.file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        )
                    });
                if !offset_matches_symbol_span(offset, span) {
                    continue;
                }

                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span,
                });
            }
            // check if it's a parameter
            NodeType::Parameter => {
                let Ok(param_id) = dir_node_id.try_into() else {
                    continue;
                };
                let param = dir_tree.get::<Parameter>(param_id);
                let local_symbol = param.symbol();
                let symbol_id = global_symbol(ctx.module_id, local_symbol);
                let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                    .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                    .unwrap_or_else(|| {
                        Span::new(
                            ctx.file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        )
                    });
                if !offset_matches_symbol_span(offset, span) {
                    continue;
                }

                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span,
                });
            }
            // check if it's a dependency item (import or re export specifier)
            NodeType::DependencyItem => {
                let Ok(item_id) = dir_node_id.try_into() else {
                    continue;
                };
                let item = dir_tree.get::<DependencyItem>(item_id);
                let ast_node_id = dir_tree.get_source(item_id.id);

                // prefer alias spans as local binding targets in aliased imports
                if let Some(alias_side_span) = ctx
                    .ast
                    .tree
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
                    .map(|span| Span::new(ctx.file_id, span.start, span.end))
                    && offset_matches_symbol_span(offset, alias_side_span)
                    && let Some(symbol_id) =
                        resolve_dependency_item_symbol(session, &ctx, item, false)
                {
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span: alias_side_span,
                    });
                }

                // prefer imported name spans as target symbol references in aliased imports
                if let Some(name_side_span) = ctx
                    .ast
                    .tree
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Type)
                    .map(|span| Span::new(ctx.file_id, span.start, span.end))
                    && offset_matches_symbol_span(offset, name_side_span)
                    && let Some(symbol_id) =
                        resolve_dependency_item_symbol(session, &ctx, item, true)
                {
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span: name_side_span,
                    });
                }

                let Some(symbol_id) = resolve_dependency_item_symbol(session, &ctx, item, false)
                else {
                    continue;
                };

                let span = get_dir_node_main_span(ctx.ast, ctx.dir, dir_node_id)
                    .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, dir_node_id))
                    .unwrap_or_else(|| {
                        Span::new(
                            ctx.file_id,
                            enclosing_span.span.start,
                            enclosing_span.span.end,
                        )
                    });
                if !offset_matches_symbol_span(offset, span) {
                    continue;
                }

                return Some(SymbolAtOffset {
                    symbol_id,
                    node_id: dir_node_id,
                    span,
                });
            }
            _ => {}
        }
    }

    // fall back to type-only import bindings in explicit type contexts
    type_import_symbol_for_type_context(session, &ctx, offset)
}

/// Resolve a dependency item symbol for either local alias or imported name contexts.
fn resolve_dependency_item_symbol(
    _session: &Session,
    ctx: &QueryContext<'_>,
    item: &DependencyItem,
    prefer_target_symbol: bool,
) -> Option<GlobalSymbolId> {
    // prefer direct target symbols when requested
    if prefer_target_symbol && let Some(target_symbol) = item.target_symbol() {
        return Some(target_symbol);
    }

    // resolve local symbols before target symbols for local alias contexts
    if let Some(local_symbol) = item.symbol() {
        return Some(global_symbol(ctx.module_id, local_symbol));
    }
    if let Some(target_symbol) = item.target_symbol() {
        return Some(target_symbol);
    }

    // unresolved dependency items are treated as not-ready
    None
}

/// Resolve a declaration symbol from a declaration modifier keyword.
fn declaration_modifier_symbol_at_offset(
    session: &Session,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve the non-trivia token at the cursor
    let token = token_span_at_offset(session, file_id, offset)?;
    if token.token.ty != ast::TokenType::Identifier {
        return None;
    }

    let source_file = session.files.get(file_id);
    let token_text = source_file.span_str(token.span);
    if !is_declaration_modifier_keyword(token_text) {
        return None;
    }

    // resolve query context for declaration lookup
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let dir_tree = ctx.tree();

    // walk enclosing ast declarations from inner to outer
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);
    enclosing.sort_by_key(|span| span.length);

    for enclosing_span in enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
            continue;
        };

        if dir_node_id.ty != NodeType::Declaration {
            continue;
        }

        // only treat modifier positions before the declaration name
        let Some(name_span) = ctx.ast.tree.source_map.get_main(enclosing_span.idx) else {
            continue;
        };
        if token.span.start >= name_span.start {
            continue;
        }

        let Ok(declaration_id): Result<dir::LocalNodeId<Declaration>, _> = dir_node_id.try_into()
        else {
            continue;
        };
        let declaration = dir_tree.get::<Declaration>(declaration_id);
        let symbol_id = global_symbol(ctx.module_id, declaration.symbol());

        return Some(SymbolAtOffset {
            symbol_id,
            node_id: dir_node_id,
            span: token.span,
        });
    }

    None
}

/// Check whether a keyword can act as a declaration modifier.
fn is_declaration_modifier_keyword(keyword: &str) -> bool {
    matches!(
        keyword,
        "export" | "declare" | "abstract" | "async" | "readonly" | "static"
    )
}

/// Resolve unresolved expression references through type-only imports.
fn unresolved_type_symbol_for_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression: &Expression,
) -> Option<GlobalSymbolId> {
    let path = match expression {
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => path,
        _ => return None,
    };

    let name_id = path.last_segment()?;
    resolve_type_import_symbol_by_name(session, ctx, name_id)
}

/// Resolve a type symbol from imports with local bindings preferred.
fn resolve_type_import_symbol_by_name(
    _session: &Session,
    ctx: &QueryContext<'_>,
    name_id: StringId,
) -> Option<GlobalSymbolId> {
    // prefer local dependency symbols for declaration-sensitive queries
    let dir_tree = ctx.tree();
    for item_id in dir_tree.iter_node_ids_of_type::<DependencyItem>() {
        let item = dir_tree.get::<DependencyItem>(item_id);
        let (kind, mode, name, alias) = match item {
            DependencyItem::Remote {
                kind,
                mode,
                name,
                alias,
                ..
            }
            | DependencyItem::UnresolvedRemote {
                kind,
                mode,
                name,
                alias,
                ..
            } => (*kind, *mode, *name, *alias),
            _ => continue,
        };

        if kind != DependencyKind::Type {
            continue;
        }

        if !dependency_item_matches_name(name_id, name, alias, mode, false) {
            continue;
        }

        if let Some(local_symbol) = item.symbol() {
            return Some(global_symbol(ctx.module_id, local_symbol));
        }

        if let Some(target_symbol) = item.target_symbol() {
            return Some(target_symbol);
        }
    }

    None
}

/// Resolve a type import symbol at the cursor when the cursor is in a type context.
fn type_import_symbol_for_type_context(
    session: &Session,
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // require an identifier token at the cursor
    let token = token_span_at_offset(session, ctx.file_id, offset)?;
    if token.token.ty != ast::TokenType::Identifier {
        return None;
    }

    // require that at least one enclosing node contributes a type side span
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);
    enclosing.sort_by_key(|span| span.length);
    let in_type_context = enclosing.iter().any(|span| {
        ctx.ast
            .tree
            .source_map
            .get_side(span.idx, NodeSpanType::Type)
            .is_some_and(|type_span| type_span.contains(offset))
    });
    if !in_type_context {
        return None;
    }

    // resolve the token text to candidate import bindings
    let source_file = session.files.get(ctx.file_id);
    let token_name = source_file.span_str(token.span);
    let dir_tree = ctx.tree();
    for item_id in dir_tree.iter_node_ids_of_type::<DependencyItem>() {
        let item = dir_tree.get::<DependencyItem>(item_id);
        let (kind, name, alias) = match item {
            DependencyItem::Remote {
                kind, name, alias, ..
            }
            | DependencyItem::UnresolvedRemote {
                kind, name, alias, ..
            } => (*kind, *name, *alias),
            _ => continue,
        };

        if kind != DependencyKind::Type {
            continue;
        }

        let matches_alias = alias
            .map(|alias| session.strings.get(alias) == token_name)
            .unwrap_or(false);
        let matches_name = alias.is_none()
            && name
                .map(|name| session.strings.get(name.string()) == token_name)
                .unwrap_or(false);
        if !matches_alias && !matches_name {
            continue;
        }

        let Some(symbol_name_id) = alias.or(name.map(|name| name.string())) else {
            continue;
        };
        let Some(symbol_id) = resolve_type_import_symbol_by_name(session, ctx, symbol_name_id)
        else {
            continue;
        };

        return Some(SymbolAtOffset {
            symbol_id,
            node_id: item_id.into(),
            span: token.span,
        });
    }

    None
}

/// Resolve a static parameter symbol at the given offset.
fn static_parameter_symbol_at_offset(
    session: &Session,
    ctx: &QueryContext<'_>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve identifier text at the cursor when available
    let token_name = token_at_offset(session, ctx.file_id, offset);

    // scan declarations with static parameters for a matching name span
    let dir_tree = ctx.tree();
    for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
        let Some(parameters) = declaration.static_parameters() else {
            continue;
        };

        for parameter_id in parameters {
            let ast_node_id = dir_tree.get_source(parameter_id.id);
            let main_span = ctx
                .ast
                .tree
                .get_main_span_by_id(ast_node_id)
                .unwrap_or_else(|| ctx.ast.tree.source_map.get_main_or_enclosing(ast_node_id));
            let parameter = dir_tree.get::<Parameter>(*parameter_id);
            let span = Span::new(ctx.file_id, main_span.start, main_span.end);
            if !offset_matches_symbol_span(offset, span) {
                continue;
            }

            if let Some(token_name) = token_name.as_deref()
                && let Some(parameter_name) = static_parameter_name(session, parameter)
                && parameter_name != token_name
            {
                continue;
            }

            let symbol_id = global_symbol(ctx.module_id, parameter.symbol());
            return Some(SymbolAtOffset {
                symbol_id,
                node_id: (*parameter_id).into(),
                span,
            });
        }
    }

    None
}

/// Resolve the declared name for a static parameter when available.
fn static_parameter_name(session: &Session, parameter: &Parameter) -> Option<String> {
    match parameter {
        Parameter::Named { name, .. } | Parameter::VariadicNamed { name, .. } => {
            Some(session.strings.get(*name).to_string())
        }
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => None,
    }
}

/// Resolve a binding symbol inside a pattern field at the cursor.
fn pattern_field_symbol_at_offset(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    field_id: dir::LocalNodeId<PatternField>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let field = dir_tree.get::<PatternField>(field_id);

    match field {
        PatternField::Named { pattern, .. }
        | PatternField::Computed { pattern, .. }
        | PatternField::Spread { pattern, .. } => {
            let pattern = *pattern.as_ref()?;
            pattern_symbol_at_offset(session, ctx, dir_tree, pattern, offset)
        }
        PatternField::Positional { pattern, .. } => {
            pattern_symbol_at_offset(session, ctx, dir_tree, *pattern, offset)
        }
        PatternField::Alias { symbol, .. } => {
            let node_id = field_id.into();
            let span = get_dir_node_main_span(ctx.ast, ctx.dir, node_id)
                .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, node_id))?;
            if !offset_matches_symbol_span(offset, span) {
                return None;
            }

            Some(SymbolAtOffset {
                symbol_id: global_symbol(ctx.module_id, *symbol),
                node_id,
                span,
            })
        }
        PatternField::Elision => None,
    }
}

/// Resolve a binding symbol inside a pattern at the cursor.
fn pattern_symbol_at_offset(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    pattern_id: dir::LocalNodeId<Pattern>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let pattern = dir_tree.get::<Pattern>(pattern_id);

    match pattern {
        Pattern::Binding {
            symbol, pattern, ..
        } => {
            let node_id = pattern_id.into();
            let span = get_dir_node_main_span(ctx.ast, ctx.dir, node_id)
                .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, node_id))?;
            if offset_matches_symbol_span(offset, span) {
                return Some(SymbolAtOffset {
                    symbol_id: global_symbol(ctx.module_id, *symbol),
                    node_id,
                    span,
                });
            }

            if let Some(inner_pattern) = pattern {
                return pattern_symbol_at_offset(session, ctx, dir_tree, *inner_pattern, offset);
            }

            None
        }
        Pattern::Must(inner)
        | Pattern::ReferenceOf { right: inner, .. }
        | Pattern::ValueOf { right: inner, .. } => {
            pattern_symbol_at_offset(session, ctx, dir_tree, *inner, offset)
        }
        Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::Array { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                if let Some(symbol_at) =
                    pattern_field_symbol_at_offset(session, ctx, dir_tree, *field_id, offset)
                {
                    return Some(symbol_at);
                }
            }

            None
        }
        Pattern::Union { patterns } => {
            for pattern_id in patterns {
                if let Some(symbol_at) =
                    pattern_symbol_at_offset(session, ctx, dir_tree, *pattern_id, offset)
                {
                    return Some(symbol_at);
                }
            }

            None
        }
        Pattern::Wildcard | Pattern::Expression { .. } => None,
    }
}

/// Check whether a cursor offset should resolve to a symbol span.
fn offset_matches_symbol_span(offset: u32, span: Span) -> bool {
    if offset >= span.start && offset <= span.end {
        return true;
    }

    offset.saturating_add(1) >= span.start && offset < span.end
}

/// Resolve a member access symbol when the cursor is on the member name.
fn member_symbol_at_offset(
    session: &Session,
    ctx: &QueryContext<'_>,
    expr_id: dir::LocalNodeId<Expression>,
    node_id: LocalNodeIdAny,
    left: dir::LocalNodeId<Expression>,
    name: StringId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve the precise member name span
    let name_span = get_member_access_name_span(ctx, expr_id)?;
    if offset < name_span.start || offset > name_span.end {
        return None;
    }

    // require the token text to match the member identifier
    let member_name = session.strings.get(name);
    let source_file = session.files.get(ctx.file_id);
    let token_name = source_file.span_str(name_span);
    if member_name != token_name {
        return None;
    }

    // resolve the member symbol id
    let symbol_id = resolve_member_access_symbol(session, ctx, expr_id, left, name)?;

    // return the resolved member symbol at the cursor
    Some(SymbolAtOffset {
        symbol_id,
        node_id,
        span: name_span,
    })
}

/// Resolve the span for a member access name inside its expression span.
pub(crate) fn get_member_access_name_span(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    // prefer the AST main span for member names
    let ast_node_id = ctx.tree().get_source(expression_id.id);
    if let Some(main_span) = ctx.ast.tree.get_main_span_by_id(ast_node_id) {
        return Some(main_span);
    }

    // resolve the expression span from DIR or AST maps
    let expression_span = get_dir_node_span(ctx.ast, ctx.dir, expression_id.into())
        .unwrap_or_else(|| ctx.ast.tree.source_map.get(ast_node_id));

    // prefer the last identifier token in the member expression
    let mut last_identifier = None;
    for token in &ctx.ast.tokens {
        if token.span.file != ctx.file_id {
            continue;
        }
        if token.span.start < expression_span.start || token.span.end > expression_span.end {
            continue;
        }
        if !matches!(
            token.token.ty,
            ast::TokenType::Identifier | ast::TokenType::InvalidIdentifier
        ) {
            continue;
        }

        last_identifier = Some(token.span);
    }
    if let Some(last_identifier) = last_identifier {
        return Some(last_identifier);
    }

    // fall back to the AST main span when no identifier token is found
    None
}

/// Get the canonical symbol for a given symbol id.
///
/// Follows the canonical_symbol chain to get the original definition.
/// For imports, this returns the imported symbol.
/// For regular symbols, this returns the same symbol_id.
pub fn get_canonical_symbol(session: &Session, symbol_id: GlobalSymbolId) -> GlobalSymbolId {
    // resolve the module query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return symbol_id;
    };

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    if let Some(canonical) = symbol.canonical_symbol
        && canonical != symbol_id
    {
        drop(symbols);
        drop(module);
        return get_canonical_symbol(session, canonical);
    }

    // return the original symbol id
    symbol_id
}

/// Resolve a symbol name string when possible.
pub(crate) fn resolve_symbol_name(
    session: &Session,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // read the symbol name string
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol
        .name()
        .map(|name_id| session.strings.get(name_id).to_string())
}

/// Resolve the local alias text for explicit import aliases.
pub(crate) fn resolve_local_import_alias_name(
    session: &Session,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve query context for the symbol module
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // resolve the symbol declaration and support declaration/import forms
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let declaration = symbol.primary_declaration?;
    if declaration.local_id.ty == NodeType::Declaration {
        let declaration_id: dir::LocalNodeId<Declaration> = declaration.local_id.try_into().ok()?;
        let dir_tree = ctx.tree();
        let declaration = dir_tree.get::<Declaration>(declaration_id);
        let Declaration::ImportAlias { descriptor, .. } = declaration else {
            return None;
        };

        let name_id = descriptor.name?.string();
        return Some(session.strings.get(name_id).to_string());
    }

    if declaration.local_id.ty != NodeType::DependencyItem {
        return None;
    }
    drop(symbols);

    // resolve the local import binding name
    let item_id: dir::LocalNodeId<DependencyItem> = declaration.local_id.try_into().ok()?;
    let dir_tree = ctx.tree();
    let local_name_id = match dir_tree.get::<DependencyItem>(item_id) {
        // default imports: use the local binding name (for import foo from ...)
        DependencyItem::Remote {
            mode, name, alias, ..
        }
        | DependencyItem::UnresolvedRemote {
            mode, name, alias, ..
        } => {
            if *mode == DependencyMode::Default {
                name.as_ref().map(|name| name.string()).or(*alias)
            } else {
                *alias
            }
        }
        // local dependency items are not import aliases
        DependencyItem::Local { mode, alias, .. } => {
            if *mode == DependencyMode::Default {
                None
            } else {
                *alias
            }
        }
        _ => None,
    }?;

    Some(session.strings.get(local_name_id).to_string())
}

/// Check whether a symbol is a local import alias for a canonical target.
pub(crate) fn is_dependency_alias_for_target(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
    canonical_target: dir::GlobalSymbolId,
) -> bool {
    // resolve the symbol's primary declaration
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let Some(declaration) = symbol.primary_declaration else {
        return false;
    };
    drop(symbols);

    // bail out when the declaration is not a dependency item
    if declaration.local_id.ty != NodeType::DependencyItem {
        return false;
    }

    // resolve the dependency item node
    let Ok(item_id): Result<dir::LocalNodeId<DependencyItem>, _> = declaration.local_id.try_into()
    else {
        return false;
    };

    // check for an alias that targets the canonical symbol
    let dir_tree = ctx.tree();
    let item = dir_tree.get::<DependencyItem>(item_id);
    let (alias, target_symbol, mode) = match item {
        DependencyItem::Local {
            alias,
            target_symbol,
            mode,
            ..
        }
        | DependencyItem::Remote {
            alias,
            target_symbol,
            mode,
            ..
        } => (alias, target_symbol, mode),
        _ => return false,
    };

    // require an explicit alias
    if alias.is_none() {
        return false;
    }

    // allow default imports to be renamed with their targets
    if *mode == DependencyMode::Default {
        return false;
    }

    // compare canonical targets
    let target_canonical = get_canonical_symbol(session, *target_symbol);
    target_canonical == canonical_target
}

/// Resolve a symbol name id, loading the owning module when needed.
pub(crate) fn resolve_symbol_name_id(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<StringId> {
    // prefer the current query context when the symbol is local
    if symbol_id.module_id == ctx.module_id {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        return symbol.name();
    }

    // fall back to the symbol's module context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let target_ctx = session.query_context(&module)?;
    let symbols = target_ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol.name()
}

/// Resolve a static member name from a dynamic key.
pub(crate) fn member_key_name(session: &Session, key: &DynamicKey) -> Option<String> {
    // resolve name and numeric keys into strings
    match key {
        DynamicKey::Name(name_id) | DynamicKey::Number(name_id) => {
            Some(session.strings.get(*name_id).to_string())
        }
        _ => None,
    }
}

/// Check whether a member field is the synthetic `function` keyword placeholder.
pub(crate) fn is_synthetic_function_keyword_field(
    member: &Member,
    name: &str,
    range: Span,
) -> bool {
    // only skip field members that match the exact `function` token length
    if !matches!(member, Member::Field { .. }) || name != "function" {
        return false;
    }

    let full_len = range.end.saturating_sub(range.start);
    full_len == 8
}

/// Resolve a symbol span using the provided declaration span strategy.
fn get_symbol_span_with(
    session: &Session,
    symbol_id: GlobalSymbolId,
    span_for_declaration: impl Fn(&ModuleAst, &ModuleDir, LocalNodeIdAny) -> Option<Span> + Copy,
    prefer_precise_member_name: bool,
) -> Option<Span> {
    // resolve the module and query context for this symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // read the symbol and follow the canonical chain
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let canonical_id = symbol.canonical_symbol.unwrap_or(symbol_id);

    // recurse when the canonical symbol lives in another module
    if canonical_id.module_id != symbol_id.module_id {
        drop(symbols);
        drop(module);
        return get_symbol_span_with(
            session,
            canonical_id,
            span_for_declaration,
            prefer_precise_member_name,
        );
    }

    // resolve the primary declaration for the canonical symbol
    let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
    let declaration = canonical_symbol.primary_declaration;
    let target_symbol = canonical_symbol.target_symbol;

    drop(symbols);

    // extract the desired span from the declaration node
    if let Some(declaration) = declaration {
        if prefer_precise_member_name
            && let Some(span) =
                precise_member_name_span_for_declaration(session, &ctx, declaration.local_id)
        {
            return Some(span);
        }

        return span_for_declaration(ctx.ast, ctx.dir, declaration.local_id);
    }

    // follow target symbols when the canonical symbol is an alias
    if let Some(target) = target_symbol {
        drop(module);
        return get_symbol_span_with(
            session,
            target,
            span_for_declaration,
            prefer_precise_member_name,
        );
    }

    // fall back to default or export assignment spans when present
    fallback_export_span(&ctx, canonical_id, span_for_declaration)
}

/// Get the definition span of a symbol.
///
/// Returns the span of the symbol's primary declaration identifier.
pub fn get_symbol_definition_span(session: &Session, symbol_id: GlobalSymbolId) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_dir_node_main_span, true)
}

/// Get the local definition span of a symbol without canonical expansion.
pub(crate) fn get_symbol_local_definition_span(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    // resolve the module and query context for this symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // resolve the symbol declaration in its local module
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let declaration = symbol.primary_declaration;
    drop(symbols);

    // prefer the main declaration span and then full declaration span
    if let Some(declaration) = declaration {
        if let Some(span) =
            precise_member_name_span_for_declaration(session, &ctx, declaration.local_id)
        {
            return Some(span);
        }

        return get_dir_node_main_span(ctx.ast, ctx.dir, declaration.local_id)
            .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, declaration.local_id));
    }

    // fall back to local export assignment style spans
    fallback_export_span(&ctx, symbol_id, get_dir_node_main_span)
}

/// Resolve a definition span when the symbol is a type symbol.
pub(crate) fn type_definition_span_for_symbol(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // check whether the symbol participates in the type namespace
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    if !matches_symbol_space_filter(symbol.ty, symbol.space, Some(SymbolSpace::Type)) {
        return None;
    }

    drop(symbols);
    drop(module);

    // resolve the definition span for the type symbol
    get_symbol_definition_span(session, symbol_id)
}

/// Get the full declaration span of a symbol.
pub fn get_symbol_declaration_span(session: &Session, symbol_id: GlobalSymbolId) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_dir_node_span, false)
}

/// Resolve the precise token span for a member declaration name.
fn precise_member_name_span_for_declaration(
    session: &Session,
    ctx: &QueryContext<'_>,
    declaration: LocalNodeIdAny,
) -> Option<Span> {
    if declaration.ty != NodeType::Member {
        return None;
    }

    let member_id = declaration.try_into().ok()?;
    let dir_tree = ctx.tree();
    let member = dir_tree.get::<Member>(member_id);
    let expected_name = match member {
        Member::Type { name, .. } | Member::ComptimeConst { name, .. } => {
            session.strings.get(*name).to_string()
        }
        Member::Field { key, .. } | Member::Method { key, .. } => {
            member_key_name(session, key.as_ref()?)?
        }
        _ => return None,
    };

    let declaration_span = get_dir_node_span(ctx.ast, ctx.dir, declaration)?;
    let source_file = session.files.get(ctx.file_id);
    for token in &ctx.ast.tokens {
        if token.span.file != ctx.file_id {
            continue;
        }

        if token.span.start < declaration_span.start || token.span.end > declaration_span.end {
            continue;
        }

        if token.token.ty != ast::TokenType::Identifier {
            continue;
        }

        if source_file.span_str(token.span) == expected_name {
            return Some(token.span);
        }
    }

    None
}

/// Resolve fallback spans for default and export-assignment symbols.
fn fallback_export_span(
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
    span_for_declaration: impl Fn(&ModuleAst, &ModuleDir, LocalNodeIdAny) -> Option<Span> + Copy,
) -> Option<Span> {
    // handle export assignment symbols
    let is_export_assignment = symbol_id.local_id == ctx.dir.export_assignment_symbol;
    if is_export_assignment {
        let item_id = *ctx.dir.export_assignment.read();
        if let Some(item_id) = item_id {
            return span_for_declaration(ctx.ast, ctx.dir, item_id.into());
        }
    }

    // skip when the symbol is not the default export
    if symbol_id.local_id != ctx.dir.default_symbol {
        return None;
    }

    // scan dependency items for default exports
    let dir_tree = ctx.tree();
    for item_id in dir_tree.iter_node_ids_of_type::<DependencyItem>() {
        let DependencyItem::Value { mode, .. } = dir_tree.get(item_id) else {
            continue;
        };

        if *mode != DependencyMode::Default {
            continue;
        }

        return span_for_declaration(ctx.ast, ctx.dir, item_id.into());
    }

    None
}
