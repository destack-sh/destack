use std::collections::HashSet;

use destack_core::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyMode, DynamicKey, EnumField, Expression, GlobalSymbolId,
    LocalNodeIdAny, LocalSymbolId, Member, NodeType, Parameter, Pattern, PatternField, SymbolSpace,
    SymbolType,
};
use destack_source::{FileId, ModuleId, NodeSpanType, Span};
use {destack_ast as ast, destack_dir as dir};

use super::expression::{expression_is_type_position, resolve_expression_symbol};
use super::namespace::{resolve_namespace_receiver_symbol, resolve_path_segment_symbol};
use super::nominal::resolve_member_access_symbol;
use crate::ast::{
    get_module_by_file_id, get_node_tree_main_span, get_node_tree_span, token_at_offset,
    token_span_at_offset, try_span_for_dir_node,
};
use crate::core::{
    AstQuery, DirQuery, QueryContext, SessionQueryIndexExt, with_ast_and_resolved_for_module,
};
use destack_artifact::DirResolved;
use destack_workspace::{Session, SymbolIndexEntry, SymbolIndexKind};

/// Result of finding a symbol at an offset.
#[derive(Debug, Clone)]
pub(crate) struct SymbolAtOffset {
    /// The symbol that was referenced.
    pub symbol_id: GlobalSymbolId,
    /// The DIR node that contains the reference.
    pub node_id: LocalNodeIdAny,
    /// The span of the reference.
    pub span: Span,
}

/// Check whether a symbol type participates in the type namespace.
pub(crate) fn is_type_symbol(symbol_type: SymbolType) -> bool {
    matches!(
        symbol_type,
        SymbolType::Class
            | SymbolType::Struct
            | SymbolType::Interface
            | SymbolType::Enum
            | SymbolType::TypeAlias
            | SymbolType::Newtype
    )
}

/// Check whether a symbol matches a requested symbol space filter.
pub(crate) fn matches_symbol_space_filter(
    symbol_type: SymbolType,
    symbol_space: SymbolSpace,
    filter: Option<SymbolSpace>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    match filter {
        SymbolSpace::Type => match symbol_space {
            SymbolSpace::Type => true,
            SymbolSpace::TypeValue => true,
            _ => is_type_symbol(symbol_type),
        },
        SymbolSpace::Value => {
            symbol_space == SymbolSpace::Value || symbol_space == SymbolSpace::TypeValue
        }
        SymbolSpace::TypeValue => symbol_space == SymbolSpace::TypeValue,
        SymbolSpace::Label => symbol_space == SymbolSpace::Label,
    }
}

/// Check whether a symbol matches an explicit import-clause space filter.
pub(crate) fn matches_import_clause_space_filter(
    symbol_type: SymbolType,
    symbol_space: SymbolSpace,
    filter: Option<SymbolSpace>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    match filter {
        SymbolSpace::Type => symbol_space == SymbolSpace::Type || is_type_symbol(symbol_type),
        _ => matches_symbol_space_filter(symbol_type, symbol_space, Some(filter)),
    }
}

/// Build a global symbol id from a module and local symbol id.
pub(crate) fn global_symbol(module_id: ModuleId, local_id: LocalSymbolId) -> GlobalSymbolId {
    GlobalSymbolId {
        module_id,
        local_id,
    }
}

/// Resolve a typed global symbol id from one module-local symbol id.
pub fn resolve_global_symbol_id(
    session: &Session,
    module_id: ModuleId,
    local_symbol_id: u32,
) -> Option<GlobalSymbolId> {
    let module = session.modules.get(module_id);
    let module = module.as_ref();
    let ctx = crate::core::query_context(session, module)?;
    let symbol_entry = ctx.dir().symbols().get_symbol_by_id(local_symbol_id);

    Some(GlobalSymbolId {
        module_id,
        local_id: LocalSymbolId::new_typed(local_symbol_id, symbol_entry.ty),
    })
}

/// Resolve the semantic target symbol used by semantic navigation queries.
pub(crate) fn semantic_target_symbol_at_offset(
    session: &Session,
    file_id: FileId,
    offset: u32,
    symbol_at: &SymbolAtOffset,
) -> Option<GlobalSymbolId> {
    if symbol_at.node_id.ty != NodeType::Expression {
        return Some(symbol_at.symbol_id);
    }

    let expression_id = symbol_at.node_id.try_into().ok()?;
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.as_ref();
    let ctx = crate::core::query_context(session, module)?;
    let ast = ctx.ast();
    let dir = ctx.dir();

    // use the enclosing member target when the cursor is on a namespace receiver
    if let Some(target_symbol) =
        member_access_target_symbol_at_offset(session, ast, dir, expression_id, offset)
        && target_symbol != symbol_at.symbol_id
    {
        return Some(target_symbol);
    }

    Some(symbol_at.symbol_id)
}

/// Resolve the member target for a receiver position inside one member access.
fn member_access_target_symbol_at_offset(
    _session: &Session,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    offset: u32,
) -> Option<GlobalSymbolId> {
    let dir_tree = dir.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    // prefer the current member expression when the cursor is on its receiver
    if let Expression::Member { .. } = expression
        && let Some(name_span) = get_member_access_name_span(ast, dir, expression_id)
        && offset < name_span.start
    {
        return resolve_member_access_symbol(dir, expression_id);
    }

    // otherwise lift the current expression into its enclosing member receiver slot
    let parent = dir_tree.get_parent(expression_id.id)?;
    if parent.ty != NodeType::Expression {
        return None;
    }

    let parent_expression_id = parent.try_into().ok()?;
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    let Expression::Member { left, .. } = parent_expression else {
        return None;
    };
    if *left != expression_id {
        return None;
    }

    let name_span = get_member_access_name_span(ast, dir, parent_expression_id)?;
    if offset >= name_span.start {
        return None;
    }

    resolve_member_access_symbol(dir, parent_expression_id)
}

/// Resolve the local binding symbol used by declaration style queries.
pub(crate) fn binding_symbol_at_offset(
    session: &Session,
    file_id: FileId,
    offset: u32,
    symbol_at: &SymbolAtOffset,
) -> Option<GlobalSymbolId> {
    if symbol_at.node_id.ty != NodeType::Expression {
        return None;
    }

    let expression_id = symbol_at.node_id.try_into().ok()?;
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.as_ref();
    let ctx = crate::core::query_context(session, module)?;
    let ast = ctx.ast();
    let dir = ctx.dir();

    // preserve plain path segments as their local binding symbols
    if let Some(result) = path_segment_symbol_at_offset(session, ast, dir, expression_id, offset) {
        return Some(result.symbol_id);
    }

    // preserve namespace member receivers as their local binding symbols
    if let Expression::Member { left, .. } = dir.tree().get::<Expression>(expression_id)
        && let Some(name_span) = get_member_access_name_span(ast, dir, expression_id)
        && offset < name_span.start
    {
        return resolve_namespace_receiver_symbol(dir, *left);
    }

    resolve_expression_symbol(dir, expression_id)
}

/// Find the symbol referenced at a given offset.
pub(crate) fn find_symbol_at_offset(
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
    let module = module.as_ref();
    let ctx = crate::core::query_context(session, module)?;
    let ast = ctx.ast();
    let dir = ctx.dir();

    // find AST nodes at the offset
    let enclosing = ast.tree().source_map.get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    // sort by length (smallest first) to get most specific node
    let mut enclosing = enclosing;

    // prefer the most specific span when multiple nodes tie
    enclosing.sort_by_key(|span| (span.length, -(span.idx as i64)));

    // skip doc and comment tokens before resolving symbols
    let mut is_comment_token = |token: &ast::TokenSpan| {
        if token.span.file != ast.file_id() {
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

    if ast.tokens().iter().any(&mut is_comment_token)
        || ast.side_tokens().iter().any(&mut is_comment_token)
    {
        return None;
    }

    // check if we're in a doc/comment first (these should not return symbols)
    for enclosing_span in &enclosing {
        let node_type = ast.tree().get_node_type(enclosing_span.idx);
        if matches!(node_type, ast::NodeType::Doc | ast::NodeType::Comment) {
            return None;
        }
    }

    let dir_tree = dir.tree();

    // check static parameters first to avoid capturing the enclosing declaration
    if let Some(result) = static_parameter_symbol_at_offset(session, ast, dir, offset) {
        return Some(result);
    }

    // scan member access expressions first to lock onto the member name span
    for (expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expr else {
            continue;
        };
        let Some(name) = *name else {
            continue;
        };

        // resolve member symbols when the cursor is on the member name
        if let Some(result) = member_symbol_at_offset(
            session,
            ast,
            dir,
            expr_id,
            expr_id.into(),
            *left,
            name,
            offset,
        ) {
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
                    let Some(name) = *name else {
                        continue;
                    };
                    let member_name = session.strings.get(name);
                    let is_member_name_token = token_at_offset(session, ast.file_id(), offset)
                        .as_deref()
                        .is_some_and(|token| member_name == token);

                    if !is_member_name_token {
                        skip_expression_target_symbol = true;
                    }

                    let result = member_symbol_at_offset(
                        session,
                        ast,
                        dir,
                        expr_id,
                        dir_node_id,
                        *left,
                        name,
                        offset,
                    );
                    if let Some(result) = result {
                        return Some(result);
                    }

                    // use the recorded member target when the cursor is on the member name
                    if is_member_name_token && let Some(target_symbol) = expr.target_symbol() {
                        let span = get_member_access_name_span(ast, dir, expr_id)
                            .or_else(|| {
                                token_span_at_offset(session, ast.file_id(), offset)
                                    .map(|token| token.span)
                            })
                            .unwrap_or_else(|| {
                                Span::new(
                                    ast.file_id(),
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
                    if let Some(name_span) = get_member_access_name_span(ast, dir, expr_id)
                        && offset < name_span.start
                    {
                        skip_expression_target_symbol = true;

                        if let Some(receiver_symbol) = resolve_namespace_receiver_symbol(dir, *left)
                        {
                            let member_span = get_node_tree_span(ast, dir.tree(), dir_node_id);
                            let receiver_end = name_span.start.saturating_sub(1);
                            let receiver_span = (member_span.start <= receiver_end).then_some(
                                Span::new(member_span.file, member_span.start, receiver_end),
                            );

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
                    let left_span = get_node_tree_span(ast, dir.tree(), (*left).into());
                    if offset >= left_span.start
                        && offset <= left_span.end
                        && let Some(target_symbol) = resolve_namespace_receiver_symbol(dir, *left)
                    {
                        return Some(SymbolAtOffset {
                            symbol_id: target_symbol,
                            node_id: (*left).into(),
                            span: left_span,
                        });
                    }
                }

                // resolve plain path segments before falling back to full path targets
                if let Some(result) =
                    path_segment_symbol_at_offset(session, ast, dir, expr_id, offset)
                {
                    return Some(result);
                }

                if !skip_expression_target_symbol
                    && let Some(target_symbol) = resolve_expression_symbol(dir, expr_id)
                {
                    let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);
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
                    let symbol_id = global_symbol(dir.module_id(), local_symbol);
                    let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);
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
                    let symbol_id = global_symbol(dir.module_id(), local_symbol);
                    let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);

                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span,
                    });
                }

                if let Some(symbol_at) =
                    pattern_field_symbol_at_offset(session, ast, dir, dir_tree, field_id, offset)
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
                let symbol_id = global_symbol(dir.module_id(), local_symbol);
                let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);
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
                let symbol_id = global_symbol(dir.module_id(), local_symbol);
                let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);
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
                let symbol_id = global_symbol(dir.module_id(), field.symbol);
                let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);
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
                let symbol_id = global_symbol(dir.module_id(), local_symbol);
                let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);
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
                if let Some(alias_side_span) = ast
                    .tree()
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
                    .map(|span| Span::new(ast.file_id(), span.start, span.end))
                    && offset_matches_symbol_span(offset, alias_side_span)
                    && let Some(symbol_id) = item
                        .symbol()
                        .map(|symbol_id| global_symbol(dir.module_id(), symbol_id))
                {
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span: alias_side_span,
                    });
                }

                // prefer imported name spans as target symbol references in aliased imports
                if let Some(name_side_span) = ast
                    .tree()
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Type)
                    .map(|span| Span::new(ast.file_id(), span.start, span.end))
                    && offset_matches_symbol_span(offset, name_side_span)
                    && let Some(symbol_id) = item.target_symbol()
                {
                    return Some(SymbolAtOffset {
                        symbol_id,
                        node_id: dir_node_id,
                        span: name_side_span,
                    });
                }

                let Some(symbol_id) = item
                    .symbol()
                    .map(|symbol_id| global_symbol(dir.module_id(), symbol_id))
                else {
                    continue;
                };

                let span = get_node_tree_main_span(ast, dir.tree(), dir_node_id);
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

    // scan type position expressions directly when source to dir mapping is absent
    if let Some(result) = type_expression_symbol_at_offset(session, ast, dir, offset) {
        return Some(result);
    }

    None
}

/// Resolve one plain path segment symbol when the cursor is on that segment.
fn path_segment_symbol_at_offset(
    _session: &Session,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let segment_count = path_segment_count(dir.tree().get::<Expression>(expression_id))?;

    // choose the exact path segment under the cursor
    for segment_index in 0..segment_count {
        let segment_index =
            u16::try_from(segment_index).expect("path segment offset index overflow");

        let span = get_path_segment_span(ast, dir, expression_id, segment_index)?;
        if !offset_matches_symbol_span(offset, span) {
            continue;
        }

        let symbol_id = resolve_path_segment_symbol(dir, expression_id, segment_index)?;
        return Some(SymbolAtOffset {
            symbol_id,
            node_id: expression_id.into(),
            span,
        });
    }

    None
}

/// Resolve one symbol from a type-position expression that covers the cursor.
fn type_expression_symbol_at_offset(
    session: &Session,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let dir_tree = dir.tree();
    let token_span = token_span_at_offset(session, ast.file_id(), offset).map(|token| token.span);

    for (expression_id, _expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        if !expression_is_type_position(ast, dir, expression_id) {
            continue;
        }

        let expression_span = get_node_tree_main_span(ast, dir.tree(), expression_id.into());
        if !expression_span.contains(offset) {
            continue;
        }

        let expression = dir_tree.get::<Expression>(expression_id);
        let symbol_id = match expression {
            Expression::Member { .. } => resolve_member_access_symbol(dir, expression_id),
            _ => resolve_expression_symbol(dir, expression_id),
        }?;

        let span = match expression {
            Expression::Member { .. } => get_member_access_name_span(ast, dir, expression_id)
                .or(token_span)
                .unwrap_or(expression_span),
            _ => token_span.unwrap_or(expression_span),
        };

        return Some(SymbolAtOffset {
            symbol_id,
            node_id: expression_id.into(),
            span,
        });
    }

    None
}

/// Resolve one path segment span inside a plain multi segment path expression.
pub(crate) fn get_path_segment_span(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    segment_index: u16,
) -> Option<Span> {
    let expression = dir.tree().get::<Expression>(expression_id);
    let path = match expression {
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => path,
        _ => return None,
    };

    if usize::from(segment_index) >= path.segments.len() {
        return None;
    }

    let source_id = dir.tree().get_source(expression_id.id);
    let span = ast
        .tree()
        .get_side_span_by_id(source_id, NodeSpanType::Segment(segment_index))?;

    Some(Span::new(ast.file_id(), span.start, span.end))
}

/// Return the number of segments in one plain path expression.
fn path_segment_count(expression: &Expression) -> Option<usize> {
    let path = match expression {
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => path,
        _ => return None,
    };

    (path.segments.len() > 1).then_some(path.segments.len())
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
    let module = module.as_ref();
    let ctx = crate::core::query_context(session, module)?;
    let dir_tree = ctx.dir().tree();

    // walk enclosing ast declarations from inner to outer
    let mut enclosing = ctx
        .ast()
        .tree()
        .source_map
        .get_enclosing_spans(offset, offset);
    enclosing.sort_by_key(|span| span.length);

    for enclosing_span in enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
            continue;
        };

        if dir_node_id.ty != NodeType::Declaration {
            continue;
        }

        // only treat modifier positions before the declaration name
        let Some(name_span) = ctx.ast().tree().source_map.get_main(enclosing_span.idx) else {
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
        let symbol_id = global_symbol(ctx.module_id(), declaration.symbol());

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

/// Resolve a static parameter symbol at the given offset.
fn static_parameter_symbol_at_offset(
    session: &Session,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve identifier text at the cursor when available
    let token_name = token_at_offset(session, ast.file_id(), offset);

    // scan declarations with static parameters for a matching name span
    let dir_tree = dir.tree();
    for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
        let Some(parameters) = declaration.static_parameters() else {
            continue;
        };

        for parameter_id in parameters {
            let ast_node_id = dir_tree.get_source(parameter_id.id);
            let main_span = ast
                .tree()
                .get_main_span_by_id(ast_node_id)
                .unwrap_or_else(|| ast.tree().source_map.get_main_or_enclosing(ast_node_id));
            let parameter = dir_tree.get::<Parameter>(*parameter_id);
            let span = Span::new(ast.file_id(), main_span.start, main_span.end);
            if !offset_matches_symbol_span(offset, span) {
                continue;
            }

            if let Some(token_name) = token_name.as_deref()
                && let Some(parameter_name) = static_parameter_name(session, parameter)
                && parameter_name != token_name
            {
                continue;
            }

            let symbol_id = global_symbol(dir.module_id(), parameter.symbol());
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
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } | Parameter::Error { .. } => {
            None
        }
    }
}

/// Resolve a binding symbol inside a pattern field at the cursor.
fn pattern_field_symbol_at_offset(
    session: &Session,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
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
            pattern_symbol_at_offset(session, ast, dir, dir_tree, pattern, offset)
        }
        PatternField::Positional { pattern, .. } => {
            pattern_symbol_at_offset(session, ast, dir, dir_tree, *pattern, offset)
        }
        PatternField::Alias { symbol, .. } => {
            let node_id = field_id.into();
            let span = get_node_tree_main_span(ast, dir.tree(), node_id);
            if !offset_matches_symbol_span(offset, span) {
                return None;
            }

            Some(SymbolAtOffset {
                symbol_id: global_symbol(dir.module_id(), *symbol),
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
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
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
            let span = get_node_tree_main_span(ast, dir.tree(), node_id);
            if offset_matches_symbol_span(offset, span) {
                return Some(SymbolAtOffset {
                    symbol_id: global_symbol(dir.module_id(), *symbol),
                    node_id,
                    span,
                });
            }

            if let Some(inner_pattern) = pattern {
                return pattern_symbol_at_offset(
                    session,
                    ast,
                    dir,
                    dir_tree,
                    *inner_pattern,
                    offset,
                );
            }

            None
        }
        Pattern::Must(inner)
        | Pattern::ReferenceOf { right: inner, .. }
        | Pattern::ValueOf { right: inner, .. } => {
            pattern_symbol_at_offset(session, ast, dir, dir_tree, *inner, offset)
        }
        Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::Array { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                if let Some(symbol_at) =
                    pattern_field_symbol_at_offset(session, ast, dir, dir_tree, *field_id, offset)
                {
                    return Some(symbol_at);
                }
            }

            None
        }
        Pattern::Union { patterns } => {
            for pattern_id in patterns {
                if let Some(symbol_at) =
                    pattern_symbol_at_offset(session, ast, dir, dir_tree, *pattern_id, offset)
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
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expr_id: dir::LocalNodeId<Expression>,
    node_id: LocalNodeIdAny,
    _left: dir::LocalNodeId<Expression>,
    name: StringId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let member_name = session.strings.get(name);
    let source_file = session.files.get(ast.file_id());
    let token_span = token_span_at_offset(session, ast.file_id(), offset).map(|token| token.span);

    // prefer the exact identifier token under the cursor when it matches the member name
    if let Some(token_span) = token_span {
        let token_name = source_file.span_str(token_span);
        if member_name == token_name {
            let symbol_id = resolve_member_access_symbol(dir, expr_id)?;

            return Some(SymbolAtOffset {
                symbol_id,
                node_id,
                span: token_span,
            });
        }
    }

    // otherwise fall back to the computed member name span
    let name_span = get_member_access_name_span(ast, dir, expr_id)?;
    if offset < name_span.start || offset > name_span.end {
        return None;
    }

    let token_name = source_file.span_str(name_span);
    if member_name != token_name {
        return None;
    }

    // resolve the member symbol id
    let symbol_id = resolve_member_access_symbol(dir, expr_id)?;

    // return the resolved member symbol at the cursor
    Some(SymbolAtOffset {
        symbol_id,
        node_id,
        span: name_span,
    })
}

/// Resolve the span for a member access name inside its expression span.
pub(crate) fn get_member_access_name_span(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let expression = dir.tree().get::<Expression>(expression_id);

    // resolve the expression span from DIR or AST maps
    let expression_span = get_node_tree_span(ast, dir.tree(), expression_id.into());

    // prefer the first identifier after the receiver span
    if let Expression::Member { left, .. } = expression {
        let left_span = get_node_tree_span(ast, dir.tree(), (*left).into());

        for token in ast.tokens() {
            if token.span.file != ast.file_id() {
                continue;
            }
            if token.span.start <= left_span.end {
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

            return Some(token.span);
        }
    }

    // prefer the last identifier token in the member expression
    let mut last_identifier = None;
    for token in ast.tokens() {
        if token.span.file != ast.file_id() {
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
    let ast_node_id = dir.tree().get_source(expression_id.id);
    ast.tree().get_main_span_by_id(ast_node_id)
}

/// Get the canonical symbol for a given symbol id.
///
/// Follows the canonical_symbol chain to get the original definition.
/// For imports, this returns the imported symbol.
/// For regular symbols, this returns the same symbol_id.
pub(crate) fn get_canonical_symbol(session: &Session, symbol_id: GlobalSymbolId) -> GlobalSymbolId {
    // resolve the module query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let Some(ctx) = crate::core::query_context(session, module) else {
        return symbol_id;
    };

    let canonical = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.canonical_symbol
    };
    if let Some(canonical) = canonical
        && canonical != symbol_id
    {
        return get_canonical_symbol(session, canonical);
    }

    // return the original symbol id
    symbol_id
}

/// Check whether one symbol still refers to one target for reference queries.
pub(crate) fn symbol_matches_reference_target(
    session: &Session,
    symbol_id: GlobalSymbolId,
    target_symbol_id: GlobalSymbolId,
) -> bool {
    // accept direct identity first
    if symbol_id == target_symbol_id {
        return true;
    }

    // prefer canonical identity when it matches
    if get_canonical_symbol(session, symbol_id) == target_symbol_id {
        return true;
    }

    // follow explicit target edges for nominal imports that keep their own canonical symbol
    let mut current_symbol = symbol_id;
    let mut visited = HashSet::new();
    while visited.insert(current_symbol) {
        let module = session.modules.get(current_symbol.module_id);
        let module = module.as_ref();
        let Some(ctx) = crate::core::query_context(session, module) else {
            return false;
        };

        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(current_symbol.local_id);
        let Some(target_symbol) = symbol.target_symbol else {
            return false;
        };

        if target_symbol == target_symbol_id {
            return true;
        }

        if get_canonical_symbol(session, target_symbol) == target_symbol_id {
            return true;
        }

        current_symbol = target_symbol;
    }

    false
}

/// Resolve a symbol name string when possible.
pub(crate) fn resolve_symbol_name(
    session: &Session,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let ctx = crate::core::query_context(session, module)?;

    // read the symbol name string
    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol
        .name()
        .map(|name_id| session.strings.get(name_id).to_string())
}

/// Build symbol index entries for one module.
pub(crate) fn build_symbol_index_entries_for_module(
    session: &Session,
    module: &destack_workspace::Module,
) -> Vec<SymbolIndexEntry> {
    let Some(ctx) = crate::core::query_context(session, module) else {
        return Vec::new();
    };

    let dir_tree = ctx.dir().tree();
    let mut entries = Vec::new();

    // declarations, members, enum fields
    for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
        let name = super::declaration_display_name(&session.strings, declaration);
        let kind = symbol_index_kind_for_declaration(declaration);
        let container_name =
            super::container_name_for_node(dir_tree, &session.strings, declaration_id.id);

        let Some(range) = symbol_index_range(&ctx, dir_tree, declaration_id.id) else {
            continue;
        };

        entries.push(SymbolIndexEntry {
            name: name.clone(),
            kind,
            module_id: module.id,
            file_id: ctx.file_id(),
            range,
            container_name: container_name.clone(),
        });

        if let Some(member_ids) = declaration.member_ids() {
            for member_id in member_ids {
                let Some(entry) =
                    member_to_symbol_index_entry(session, &ctx, dir_tree, *member_id, &name)
                else {
                    continue;
                };

                entries.push(entry);
            }
        }

        if let dir::Declaration::Enum { fields, .. } = declaration {
            for field_id in fields {
                let Some(entry) =
                    enum_field_to_symbol_index_entry(session, &ctx, dir_tree, *field_id, &name)
                else {
                    continue;
                };

                entries.push(entry);
            }
        }
    }

    entries
}

/// Map one declaration to the symbol index kind.
fn symbol_index_kind_for_declaration(declaration: &dir::Declaration) -> SymbolIndexKind {
    match declaration {
        dir::Declaration::Global { .. } => SymbolIndexKind::Namespace,
        dir::Declaration::Function { .. } => SymbolIndexKind::Function,
        dir::Declaration::Struct { .. } => SymbolIndexKind::Struct,
        dir::Declaration::Class { .. } => SymbolIndexKind::Class,
        dir::Declaration::Interface { .. } => SymbolIndexKind::Interface,
        dir::Declaration::Enum { .. } => SymbolIndexKind::Enum,
        dir::Declaration::Namespace { .. } => SymbolIndexKind::Namespace,
        dir::Declaration::Type { .. } => SymbolIndexKind::TypeParameter,
        dir::Declaration::ImportAlias { .. } => SymbolIndexKind::Variable,
        dir::Declaration::Extension { .. } => SymbolIndexKind::Class,
    }
}

/// Convert one member to one symbol index entry.
fn member_to_symbol_index_entry(
    session: &Session,
    ctx: &QueryContext,
    dir_tree: &dir::NodeTree,
    member_id: dir::LocalNodeId<dir::Member>,
    container_name: &str,
) -> Option<SymbolIndexEntry> {
    // member node
    let member = dir_tree.get::<dir::Member>(member_id);

    // member key and kind
    let key = member.key()?;
    let name = super::member_key_name(session, key)?;
    let kind = symbol_index_kind_for_member(member)?;

    // member range
    let range = symbol_index_range(ctx, dir_tree, member_id.id)?;

    // skip synthetic function keyword fields for methods
    if super::is_synthetic_function_keyword_field(member, &name, range) {
        return None;
    }

    Some(SymbolIndexEntry {
        name,
        kind,
        module_id: ctx.module_id(),
        file_id: ctx.file_id(),
        range,
        container_name: Some(container_name.to_string()),
    })
}

/// Convert one enum field to one symbol index entry.
fn enum_field_to_symbol_index_entry(
    session: &Session,
    ctx: &QueryContext,
    dir_tree: &dir::NodeTree,
    field_id: dir::LocalNodeId<dir::EnumField>,
    container_name: &str,
) -> Option<SymbolIndexEntry> {
    // field node and range
    let field = dir_tree.get::<dir::EnumField>(field_id);
    let range = symbol_index_range(ctx, dir_tree, field_id.id)?;

    Some(SymbolIndexEntry {
        name: session.strings.get(field.name).to_string(),
        kind: SymbolIndexKind::EnumMember,
        module_id: ctx.module_id(),
        file_id: ctx.file_id(),
        range,
        container_name: Some(container_name.to_string()),
    })
}

/// Map one member to the symbol index kind.
fn symbol_index_kind_for_member(member: &dir::Member) -> Option<SymbolIndexKind> {
    match member {
        dir::Member::Type { .. } => Some(SymbolIndexKind::TypeParameter),
        dir::Member::ComptimeConst { .. } => Some(SymbolIndexKind::Constant),
        dir::Member::Field { .. } => Some(SymbolIndexKind::Field),
        dir::Member::Method { .. } => Some(SymbolIndexKind::Method),
        dir::Member::Embed { .. }
        | dir::Member::StaticBlock { .. }
        | dir::Member::ComptimeBlock { .. }
        | dir::Member::Error { .. } => None,
    }
}

/// Resolve one symbol index range without failing the whole query on bad source ids.
fn symbol_index_range(ctx: &QueryContext, dir_tree: &dir::NodeTree, node_id: u32) -> Option<Span> {
    let node_id = dir::LocalNodeIdAny::new(node_id, dir_tree.get_node_type(node_id));
    try_span_for_dir_node(ctx.ast(), dir_tree, node_id)
}

/// Resolve the local alias text for explicit import aliases.
pub(crate) fn resolve_local_import_alias_name(
    session: &Session,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve query context for the symbol module
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let ctx = crate::core::query_context(session, module)?;

    // resolve the symbol declaration and support declaration/import forms
    let declaration = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };
    if declaration.local_id.ty == NodeType::Declaration {
        let declaration_id: dir::LocalNodeId<Declaration> = declaration.local_id.try_into().ok()?;
        let dir_tree = ctx.dir().tree();
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

    // resolve the local import binding name
    let item_id: dir::LocalNodeId<DependencyItem> = declaration.local_id.try_into().ok()?;
    let dir_tree = ctx.dir().tree();
    let local_name_id =
        dependency_item_local_import_alias_name(dir_tree.get::<DependencyItem>(item_id))?;

    Some(session.strings.get(local_name_id).to_string())
}

/// Resolve the local binding name for one default import symbol inside a query context.
fn local_default_import_alias_name_in_context(
    session: &Session,
    dir: DirQuery<'_>,
    local_symbol_id: LocalSymbolId,
) -> Option<String> {
    let declaration = {
        let symbols = dir.symbols();
        let symbol = symbols.get_symbol(local_symbol_id);
        symbol.primary_declaration?
    };

    if declaration.local_id.ty != NodeType::DependencyItem {
        return None;
    }

    let item_id: dir::LocalNodeId<DependencyItem> = declaration.local_id.try_into().ok()?;
    let local_name_id =
        dependency_item_default_import_alias_name(dir.tree().get::<DependencyItem>(item_id))?;

    Some(session.strings.get(local_name_id).to_string())
}

/// Resolve the local binding name for one dependency import alias.
fn dependency_item_local_import_alias_name(
    item: &DependencyItem,
) -> Option<destack_core::StringId> {
    match item {
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
    }
}

/// Resolve the local binding name for one default dependency import alias.
fn dependency_item_default_import_alias_name(
    item: &DependencyItem,
) -> Option<destack_core::StringId> {
    match item {
        DependencyItem::Remote { mode, name, .. }
        | DependencyItem::UnresolvedRemote { mode, name, .. } => {
            if *mode != DependencyMode::Default {
                return None;
            }

            name.as_ref().map(|name| name.string())
        }
        _ => None,
    }
}

/// Collect default import aliases whose imported default export resolves to one symbol.
pub(crate) fn collect_default_import_alias_symbols_for_export(
    session: &Session,
    canonical_id: dir::GlobalSymbolId,
) -> Vec<dir::GlobalSymbolId> {
    let mut symbols = Vec::new();

    for module_id in session.reference_index_modules_for_target(canonical_id) {
        let module = session.modules.get(module_id);
        let module = module.as_ref();
        if !module.is_user() {
            continue;
        }

        let Some(ctx) = crate::core::query_context(session, module) else {
            continue;
        };
        let dir = ctx.dir();

        let symbols_in_module = dir.symbols();
        for symbol_index in 0..symbols_in_module.symbol_count() {
            let local_symbol_id = LocalSymbolId::new(symbol_index);
            let symbol_id = dir::GlobalSymbolId::new(dir.module_id(), local_symbol_id);
            if symbol_id == canonical_id {
                continue;
            }

            let local_alias_name =
                local_default_import_alias_name_in_context(session, dir, local_symbol_id);
            if local_alias_name.is_none() {
                continue;
            }

            if get_canonical_symbol(session, symbol_id) != canonical_id {
                continue;
            }

            symbols.push(symbol_id);
        }
    }

    symbols
}

/// Check whether a symbol is a local import alias for a canonical target.
pub(crate) fn is_dependency_alias_for_target(
    session: &Session,
    dir: DirQuery<'_>,
    symbol_id: dir::GlobalSymbolId,
    canonical_target: dir::GlobalSymbolId,
) -> bool {
    // resolve the symbol's primary declaration
    let declaration = {
        let symbols = dir.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration
    };
    let Some(declaration) = declaration else {
        return false;
    };

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
    let dir_tree = dir.tree();
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
    span_for_declaration: impl Fn(AstQuery<'_>, &dir::NodeTree, LocalNodeIdAny) -> Span + Copy,
) -> Option<Span> {
    with_resolved_symbol_context(session, symbol_id.module_id, |ast, _module_id, resolved| {
        // read the symbol and follow the canonical chain
        let (canonical_id, declaration, target_symbol) = {
            let symbols = &resolved.symbols;
            let symbol = symbols.get_symbol(symbol_id.local_id);
            let canonical_id = symbol.canonical_symbol.unwrap_or(symbol_id);

            // recurse when the canonical symbol lives in another module
            if canonical_id.module_id != symbol_id.module_id {
                (canonical_id, None, None)
            } else {
                // resolve the primary declaration for the canonical symbol
                let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
                (
                    canonical_id,
                    canonical_symbol.primary_declaration,
                    canonical_symbol.target_symbol,
                )
            }
        };
        if canonical_id.module_id != symbol_id.module_id {
            return get_symbol_span_with(session, canonical_id, span_for_declaration);
        }

        // extract the desired span from the declaration node
        if let Some(declaration) = declaration {
            return Some(span_for_declaration(
                ast,
                &resolved.tree,
                declaration.local_id,
            ));
        }

        // follow target symbols when the canonical symbol is an alias
        if let Some(target) = target_symbol {
            return get_symbol_span_with(session, target, span_for_declaration);
        }

        let _ = ast;
        let _ = canonical_id;

        None
    })?
}

/// Execute a closure with AST and resolved DIR for one module.
fn with_resolved_symbol_context<T>(
    session: &Session,
    module_id: ModuleId,
    f: impl FnOnce(AstQuery<'_>, ModuleId, &DirResolved) -> T,
) -> Option<T> {
    let module = session.modules.get(module_id);
    let module = module.as_ref();

    with_ast_and_resolved_for_module(session, module, f)
}

/// Get the definition span of a symbol.
///
/// Returns the span of the symbol's primary declaration identifier.
pub(crate) fn get_symbol_definition_span(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_node_tree_main_span)
}

/// Get the local definition span of a symbol without canonical expansion.
pub(crate) fn get_symbol_local_definition_span(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    with_resolved_symbol_context(session, symbol_id.module_id, |ast, _module_id, resolved| {
        // resolve the symbol declaration in its local module
        let declaration = {
            let symbols = &resolved.symbols;
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.primary_declaration
        };

        // prefer the main declaration span and then full declaration span
        if let Some(declaration) = declaration {
            return Some(get_node_tree_main_span(
                ast,
                &resolved.tree,
                declaration.local_id,
            ));
        }

        let _ = ast;
        let _ = resolved;
        let _ = symbol_id;

        None
    })?
}

/// Resolve a definition span when the symbol is a type symbol.
pub(crate) fn type_definition_span_for_symbol(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    with_resolved_symbol_context(
        session,
        symbol_id.module_id,
        |_ast, _module_id, resolved| {
            if symbol_id.local_id.id >= resolved.symbols.symbol_count() {
                return None;
            }

            // check whether the symbol participates in the type namespace
            let (is_type_symbol, declaration) = {
                let symbols = &resolved.symbols;
                let symbol = symbols.get_symbol(symbol_id.local_id);
                (
                    matches_symbol_space_filter(symbol.ty, symbol.space, Some(SymbolSpace::Type)),
                    symbol.primary_declaration,
                )
            };

            if !is_type_symbol {
                return None;
            }

            // imported type bindings should follow their dependency targets
            if declaration
                .is_some_and(|declaration| declaration.local_id.ty == NodeType::DependencyItem)
            {
                let target_symbol = dependency_item_target_symbol(resolved, symbol_id)?;
                return get_symbol_span_with(session, target_symbol, get_node_tree_main_span);
            }

            // resolve the definition span for the type symbol
            get_symbol_span_with(session, symbol_id, get_node_tree_main_span)
        },
    )?
}

/// Resolve the imported target symbol for a dependency-item binding.
fn dependency_item_target_symbol(
    resolved: &DirResolved,
    symbol_id: GlobalSymbolId,
) -> Option<GlobalSymbolId> {
    // resolve the symbol declaration
    let declaration = {
        let symbols = &resolved.symbols;
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };
    if declaration.local_id.ty != NodeType::DependencyItem {
        return None;
    }

    // resolve the dependency item target symbol
    let item_id = declaration.local_id.try_into().ok()?;
    let resolved_item = resolved.tree.get::<DependencyItem>(item_id);
    resolved_item.target_symbol()
}

/// Get the full declaration span of a symbol.
pub(crate) fn get_symbol_declaration_span(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    get_symbol_span_with(session, symbol_id, get_node_tree_span)
}
