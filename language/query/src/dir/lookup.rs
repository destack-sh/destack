use destack_core::StringId;
use destack_dir::{
    Declaration, DependencyItem, Expression, GlobalSymbolId, LocalNodeIdAny, Parameter, Pattern,
    PatternField,
};
use destack_source::{FileId, NodeSpanType, Span};
use std::str::FromStr;
use {destack_ast as ast, destack_dir as dir};

use super::expression::{expression_is_type_position, resolve_expression_symbol};
use super::namespace::{resolve_namespace_receiver_symbol, resolve_path_segment_symbol};
use super::nominal::resolve_member_access_symbol;
use super::symbol::global_symbol;
use crate::ast::{
    get_module_by_file_id, get_node_tree_main_span, get_node_tree_span, token_at_offset,
    token_span_at_offset,
};
use crate::core::{AstQuery, DirQuery, query_context};
use destack_workspace::{Repository, Revision};

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

/// Resolve the semantic target symbol used by semantic navigation queries.
pub(crate) fn semantic_target_symbol_at_offset(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
    symbol_at: &SymbolAtOffset,
) -> Option<GlobalSymbolId> {
    if symbol_at.node_id.ty != dir::NodeType::Expression {
        return Some(symbol_at.symbol_id);
    }

    let expression_id = symbol_at.node_id.try_into().ok()?;
    let module = get_module_by_file_id(repository, revision, file_id)?;
    let ctx = query_context(repository, revision, module.id)?;
    let ast = ctx.ast();
    let dir = ctx.dir();

    // use the enclosing member target when the cursor is on a namespace receiver
    if let Some(target_symbol) =
        member_access_target_symbol_at_offset(ast, dir, expression_id, offset)
        && target_symbol != symbol_at.symbol_id
    {
        return Some(target_symbol);
    }

    Some(symbol_at.symbol_id)
}

/// Resolve the local binding symbol used by declaration style queries.
pub(crate) fn binding_symbol_at_offset(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
    symbol_at: &SymbolAtOffset,
) -> Option<GlobalSymbolId> {
    if symbol_at.node_id.ty != dir::NodeType::Expression {
        return None;
    }

    let expression_id = symbol_at.node_id.try_into().ok()?;
    let module = get_module_by_file_id(repository, revision, file_id)?;
    let ctx = query_context(repository, revision, module.id)?;
    let ast = ctx.ast();
    let dir = ctx.dir();

    // preserve plain path segments as their local binding symbols
    if let Some(result) = path_segment_symbol_at_offset(ast, dir, expression_id, offset) {
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
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    find_symbol_at_offset_impl(repository, revision, file_id, offset)
}

/// Find the symbol for hover at a given offset.
pub(crate) fn find_symbol_for_hover_at_offset(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    find_symbol_at_offset(repository, revision, file_id, offset)
        .or_else(|| declaration_modifier_symbol_at_offset(repository, revision, file_id, offset))
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

/// Resolve the member target for a receiver position inside one member access.
fn member_access_target_symbol_at_offset(
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
    if parent.ty != dir::NodeType::Expression {
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

/// Resolve a symbol from structural AST and DIR mappings.
fn find_symbol_at_offset_impl(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    // resolve the module and query context
    let module = get_module_by_file_id(repository, revision, file_id)?;
    let ctx = query_context(repository, revision, module.id)?;
    let ast = ctx.ast();
    let dir = ctx.dir();

    // find AST nodes at the offset
    let enclosing = ast.tree().source_map.get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    // sort by length: smallest first
    let mut enclosing = enclosing;
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

    let dir_tree = dir.tree();

    // check generic parameters first to avoid capturing the enclosing declaration
    if let Some(result) = generic_parameter_symbol_at_offset(repository, ast, dir, offset) {
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

        if let Some(result) = member_symbol_at_offset(
            repository,
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
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
            continue;
        };

        match dir_node_id.ty {
            dir::NodeType::Expression => {
                let Ok(expr_id) = dir_node_id.try_into() else {
                    continue;
                };
                let expr = dir_tree.get::<Expression>(expr_id);
                let mut skip_expression_target_symbol = false;

                if let Expression::Member { left, name, .. } = expr {
                    let Some(name) = *name else {
                        continue;
                    };
                    let member_name = repository.strings.get(name);
                    let is_member_name_token =
                        token_at_offset(repository, dir.revision(), ast.file_id(), offset)
                            .as_deref()
                            .is_some_and(|token| member_name == token);

                    if !is_member_name_token {
                        skip_expression_target_symbol = true;
                    }

                    let result = member_symbol_at_offset(
                        repository,
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
                                token_span_at_offset(
                                    repository,
                                    dir.revision(),
                                    ast.file_id(),
                                    offset,
                                )
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

                if let Some(result) = path_segment_symbol_at_offset(ast, dir, expr_id, offset) {
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

            dir::NodeType::Pattern => {
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

            dir::NodeType::PatternField => {
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
                    pattern_field_symbol_at_offset(repository, ast, dir, dir_tree, field_id, offset)
                {
                    return Some(symbol_at);
                }
            }

            dir::NodeType::Declaration => {
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

            dir::NodeType::Member => {
                let Ok(member_id): Result<dir::LocalNodeId<dir::Member>, _> =
                    dir_node_id.try_into()
                else {
                    continue;
                };
                let member = dir_tree.get::<dir::Member>(member_id);
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

            dir::NodeType::EnumField => {
                continue;
            }

            dir::NodeType::Parameter => {
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

            dir::NodeType::DependencyItem => {
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
    if let Some(result) = type_expression_symbol_at_offset(repository, ast, dir, offset) {
        return Some(result);
    }

    None
}

/// Resolve one plain path segment symbol when the cursor is on that segment.
fn path_segment_symbol_at_offset(
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let segment_count = path_segment_count(dir.tree().get::<Expression>(expression_id))?;

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
    repository: &Repository,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let dir_tree = dir.tree();
    let token_span = token_span_at_offset(repository, dir.revision(), ast.file_id(), offset)
        .map(|token| token.span);

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
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let token = token_span_at_offset(repository, revision, file_id, offset)?;
    if token.token.ty != ast::TokenType::Identifier {
        return None;
    }

    let source_file = repository.file(revision, file_id).ok().flatten()?;
    let token_text = source_file.span_str(token.span);
    let Ok(keyword) = ast::Keyword::from_str(token_text) else {
        return None;
    };
    if !is_declaration_target_modifier_keyword(keyword) {
        return None;
    }

    let module = get_module_by_file_id(repository, revision, file_id)?;
    let ctx = query_context(repository, revision, module.id)?;
    let dir_tree = ctx.dir().tree();

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

        if dir_node_id.ty != dir::NodeType::Declaration {
            continue;
        }

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

/// Check whether a modifier keyword targets the enclosing declaration symbol.
fn is_declaration_target_modifier_keyword(keyword: ast::Keyword) -> bool {
    matches!(
        keyword,
        ast::Keyword::Export
            | ast::Keyword::Declare
            | ast::Keyword::Abstract
            | ast::Keyword::Async
            | ast::Keyword::Readonly
            | ast::Keyword::Static
    )
}

/// Resolve a generic parameter symbol at the given offset.
fn generic_parameter_symbol_at_offset(
    repository: &Repository,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let token_name = token_at_offset(repository, dir.revision(), ast.file_id(), offset);

    let dir_tree = dir.tree();
    for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
        let Some(parameters) = declaration.generic_parameters() else {
            continue;
        };

        for parameter_id in parameters {
            let ast_node_id = dir_tree.get_source(parameter_id.id);
            let main_span = ast
                .tree()
                .get_main_span_by_id(ast_node_id)
                .unwrap_or_else(|| ast.tree().source_map.get_main_or_enclosing(ast_node_id));
            let parameter = dir_tree.get::<dir::GenericParameter>(*parameter_id);
            let span = Span::new(ast.file_id(), main_span.start, main_span.end);
            if !offset_matches_symbol_span(offset, span) {
                continue;
            }

            if let Some(token_name) = token_name.as_deref()
                && let Some(parameter_name) = generic_parameter_name(repository, parameter)
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

/// Resolve the declared name for a generic parameter when available.
fn generic_parameter_name(
    repository: &Repository,
    parameter: &dir::GenericParameter,
) -> Option<String> {
    match parameter {
        dir::GenericParameter::Type { name, .. } | dir::GenericParameter::Value { name, .. } => {
            Some(repository.strings.get(*name).to_string())
        }
        dir::GenericParameter::Error { .. } => None,
    }
}

/// Resolve a binding symbol inside a pattern field at the cursor.
fn pattern_field_symbol_at_offset(
    repository: &Repository,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    dir_tree: &dir::NodeTree,
    field_id: dir::LocalNodeId<PatternField>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let field = dir_tree.get::<PatternField>(field_id);

    match field {
        PatternField::Named {
            symbol, pattern, ..
        } => {
            if let Some(symbol) = symbol {
                let node_id = field_id.into();
                let span = get_node_tree_main_span(ast, dir.tree(), node_id);
                if offset_matches_symbol_span(offset, span) {
                    return Some(SymbolAtOffset {
                        symbol_id: global_symbol(dir.module_id(), *symbol),
                        node_id,
                        span,
                    });
                }
            }

            let pattern = *pattern.as_ref()?;
            pattern_symbol_at_offset(repository, ast, dir, dir_tree, pattern, offset)
        }
        PatternField::Computed { pattern, .. } => {
            pattern_symbol_at_offset(repository, ast, dir, dir_tree, *pattern, offset)
        }
        PatternField::Spread { pattern, .. } => {
            let pattern = *pattern.as_ref()?;
            pattern_symbol_at_offset(repository, ast, dir, dir_tree, pattern, offset)
        }
        PatternField::Positional { pattern, .. } => {
            pattern_symbol_at_offset(repository, ast, dir, dir_tree, *pattern, offset)
        }
        PatternField::Elision => None,
    }
}

/// Resolve a binding symbol inside a pattern at the cursor.
fn pattern_symbol_at_offset(
    repository: &Repository,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    dir_tree: &dir::NodeTree,
    pattern_id: dir::LocalNodeId<Pattern>,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let pattern = dir_tree.get::<Pattern>(pattern_id);

    match pattern {
        Pattern::Assign { pattern, .. } => {
            pattern_symbol_at_offset(repository, ast, dir, dir_tree, *pattern, offset)
        }
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
                    repository,
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
            pattern_symbol_at_offset(repository, ast, dir, dir_tree, *inner, offset)
        }
        Pattern::TypeExpression { .. } => None,
        Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::Array { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                if let Some(symbol_at) = pattern_field_symbol_at_offset(
                    repository, ast, dir, dir_tree, *field_id, offset,
                ) {
                    return Some(symbol_at);
                }
            }

            None
        }
        Pattern::Union { patterns } => {
            for pattern_id in patterns {
                if let Some(symbol_at) =
                    pattern_symbol_at_offset(repository, ast, dir, dir_tree, *pattern_id, offset)
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
    repository: &Repository,
    ast: AstQuery<'_>,
    dir: DirQuery<'_>,
    expr_id: dir::LocalNodeId<Expression>,
    node_id: LocalNodeIdAny,
    _left: dir::LocalNodeId<Expression>,
    name: StringId,
    offset: u32,
) -> Option<SymbolAtOffset> {
    let member_name = repository.strings.get(name);
    let source_file = repository
        .file(dir.revision(), ast.file_id())
        .ok()
        .flatten()?;
    let token_span = token_span_at_offset(repository, dir.revision(), ast.file_id(), offset)
        .map(|token| token.span);

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

    let name_span = get_member_access_name_span(ast, dir, expr_id)?;
    if offset < name_span.start || offset > name_span.end {
        return None;
    }

    let token_name = source_file.span_str(name_span);
    if member_name != token_name {
        return None;
    }

    let symbol_id = resolve_member_access_symbol(dir, expr_id)?;
    Some(SymbolAtOffset {
        symbol_id,
        node_id,
        span: name_span,
    })
}
