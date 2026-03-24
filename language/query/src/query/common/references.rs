use std::collections::HashSet;

use destack_ast as ast;
use destack_dir::{
    self as dir, DependencyItem, DependencyKind, DependencyMode, Expression, GlobalSymbolId,
    NodeType,
};
use destack_source::{FileId, ModuleId, NodeSpanType, Span};

use super::QueryContext;
use super::resolve::{
    resolve_expression_symbol, resolve_namespace_path_receiver_symbol,
    resolve_namespace_receiver_symbol,
};
use super::span::{get_dir_node_main_span, get_dir_node_span};
use super::symbol::{
    get_canonical_symbol, get_member_access_name_span, get_namespace_path_receiver_span,
    is_dependency_alias_for_target, resolve_member_access_symbol,
};
use destack_workspace::Session;

/// Options for collecting symbol references.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceCollectionOptions<'a> {
    /// Whether to include direct expression references.
    pub include_expressions: bool,
    /// Whether to include member access references.
    pub include_members: bool,
    /// Whether to include dependency item references.
    pub include_dependencies: bool,
    /// Whether to include namespace receiver references.
    pub include_namespace_receivers: bool,
    /// Whether to skip dependency aliases that target the symbol.
    pub skip_dependency_aliases: bool,
    /// Whether to prefer name spans inside dependency items.
    pub use_dependency_name_spans: bool,
    /// An optional target name for name span resolution.
    pub target_name: Option<&'a str>,
    /// An optional file filter for collected spans.
    pub limit_to_file: Option<FileId>,
}

impl<'a> ReferenceCollectionOptions<'a> {
    /// Create options that collect all reference types.
    pub fn all() -> Self {
        Self {
            include_expressions: true,
            include_members: true,
            include_dependencies: true,
            include_namespace_receivers: false,
            skip_dependency_aliases: false,
            use_dependency_name_spans: false,
            target_name: None,
            limit_to_file: None,
        }
    }
}

/// Collect symbol references within a query context.
pub fn collect_symbol_references_in_context(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // compute namespace import aliases when namespace receiver matching is enabled
    let namespace_aliases = if options.include_namespace_receivers {
        namespace_import_aliases_for_module(ctx, canonical_id.module_id)
    } else {
        Vec::new()
    };

    // collect direct expression references
    if options.include_expressions {
        let expression_spans =
            collect_expression_reference_spans(session, ctx, canonical_id, options);
        spans.extend(expression_spans);
    }

    // collect member access references
    if options.include_members {
        let member_spans =
            collect_member_reference_spans(session, ctx, canonical_id, options, &namespace_aliases);
        spans.extend(member_spans);
    }

    // collect dependency item references
    if options.include_dependencies {
        let dependency_spans =
            collect_dependency_reference_spans(session, ctx, canonical_id, options);
        spans.extend(dependency_spans);
    }

    // return the collected spans
    spans
}

/// Collect direct expression reference spans.
fn collect_expression_reference_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching expression ids first to avoid borrow issues
    let matching_expression_ids: Vec<_> = {
        let dir_tree = ctx.tree();
        dir_tree
            .iter_nodes_of_type::<Expression>()
            .filter_map(|(expression_id, expression)| {
                let resolved_target_symbol =
                    resolve_expression_target_symbol(ctx, expression_id, expression);

                // skip member expressions when member references are enabled
                if options.include_members && matches!(expression, Expression::Member { .. }) {
                    return None;
                }
                // skip dependency aliases when requested
                let is_alias = if options.skip_dependency_aliases {
                    resolved_target_symbol
                        .map(|target_symbol| {
                            is_dependency_alias_for_target(
                                session,
                                ctx,
                                target_symbol,
                                canonical_id,
                            )
                        })
                        .unwrap_or(false)
                } else {
                    false
                };
                if is_alias {
                    return None;
                }

                // check for a canonical target match
                if let Some(target_symbol) = resolved_target_symbol {
                    if symbol_resolves_to_canonical(session, target_symbol, canonical_id) {
                        return Some(expression_id);
                    }

                    return None;
                }

                None
            })
            .collect()
    };

    // resolve spans for the matching expressions
    let dir_tree = ctx.tree();
    let mut spans = Vec::new();
    for expression_id in matching_expression_ids {
        let span = resolve_expression_reference_span(ctx, dir_tree, expression_id);
        let Some(span) = span else {
            continue;
        };

        // filter out spans outside the requested file
        if options
            .limit_to_file
            .is_some_and(|limit_file| span.file != limit_file)
        {
            continue;
        }

        spans.push(span);
    }

    // capture namespace path receivers for plain multi segment path expressions
    for (expression_id, _expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Some(receiver_symbol) = resolve_namespace_path_receiver_symbol(ctx, expression_id)
        else {
            continue;
        };
        if !symbol_resolves_to_canonical(session, receiver_symbol, canonical_id) {
            continue;
        }

        let Some(span) = get_namespace_path_receiver_span(session, ctx, expression_id) else {
            continue;
        };

        if options
            .limit_to_file
            .is_some_and(|limit_file| span.file != limit_file)
        {
            continue;
        }

        spans.push(span);
    }

    // capture member receiver spans when the receiver matches the target symbol
    if options.include_members {
        let dir_tree = ctx.tree();
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            let Expression::Member { left, .. } = expression else {
                continue;
            };
            if !member_receiver_matches_target(
                session,
                ctx,
                dir_tree,
                *left,
                canonical_id,
                options.target_name,
            ) {
                continue;
            }

            let span = resolve_member_receiver_reference_span(ctx, expression_id, *left);
            let Some(span) = span else {
                continue;
            };

            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
        }
    }

    // return the matching spans
    spans
}

/// Check whether a member receiver expression matches the canonical symbol target.
fn member_receiver_matches_target(
    session: &Session,
    ctx: &QueryContext<'_>,
    _dir_tree: &dir::NodeTree,
    receiver_expression_id: dir::LocalNodeId<Expression>,
    canonical_id: GlobalSymbolId,
    _target_name: Option<&str>,
) -> bool {
    let resolved_symbol = resolve_namespace_receiver_symbol(ctx, receiver_expression_id);
    let Some(target_symbol) = resolved_symbol else {
        return false;
    };

    symbol_resolves_to_canonical(session, target_symbol, canonical_id)
}

/// Resolve the target symbol for an expression reference.
fn resolve_expression_target_symbol(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    expression: &Expression,
) -> Option<GlobalSymbolId> {
    if expression_is_member_receiver_expression(ctx.tree(), expression_id) {
        return resolve_namespace_receiver_symbol(ctx, expression_id);
    }

    if expression.target_symbol().is_some() {
        return expression.target_symbol();
    }

    resolve_expression_symbol(ctx, expression_id)
}

/// Check whether an expression is used as the receiver of a member access.
fn expression_is_member_receiver_expression(
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
) -> bool {
    let Some(parent) = dir_tree.get_parent(expression_id.id) else {
        return false;
    };
    if parent.ty != NodeType::Expression {
        return false;
    }

    let Ok(parent_expression_id) = parent.try_into() else {
        return false;
    };
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    matches!(
        parent_expression,
        Expression::Member { left, .. } if *left == expression_id
    )
}

/// Check whether a symbol resolves to the requested canonical id.
fn symbol_resolves_to_canonical(
    session: &Session,
    symbol_id: GlobalSymbolId,
    canonical_id: GlobalSymbolId,
) -> bool {
    // allow exact symbol identity matches before canonical expansion
    if symbol_id == canonical_id {
        return true;
    }

    // prefer the canonical symbol chain when it matches
    if get_canonical_symbol(session, symbol_id) == canonical_id {
        return true;
    }

    // fall back to following target symbol chains for alias symbols
    let mut current_symbol = symbol_id;
    let mut visited = HashSet::new();

    // walk target symbols until we resolve or cycle
    while visited.insert(current_symbol) {
        // load the module context for the current symbol
        let module = session.modules.get(current_symbol.module_id);
        let module = module.as_ref();
        let Some(ctx) = crate::query_context(session, module) else {
            return false;
        };

        // read the next target symbol
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(current_symbol.local_id);
        let Some(target_symbol) = symbol.target_symbol else {
            return false;
        };

        // stop when the target resolves to the requested canonical id
        if get_canonical_symbol(session, target_symbol) == canonical_id {
            return true;
        }

        // continue walking the target chain
        current_symbol = target_symbol;
    }

    false
}
/// Resolve the best span for a reference expression.
fn resolve_expression_reference_span(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let span = get_dir_node_main_span(
        ctx.ast_context(),
        ctx.dir_analyzed_context(),
        expression_id.into(),
    )
    .or_else(|| {
        get_dir_node_span(
            ctx.ast_context(),
            ctx.dir_analyzed_context(),
            expression_id.into(),
        )
    })?;

    let Some(parent) = dir_tree.get_parent(expression_id.id) else {
        return Some(span);
    };
    if parent.ty != NodeType::Expression {
        return Some(span);
    }

    let Ok(parent_expression_id) = parent.try_into() else {
        return Some(span);
    };
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    let Expression::Member { left, .. } = parent_expression else {
        return Some(span);
    };
    if *left != expression_id {
        return Some(span);
    }

    let Some(member_name_span) = get_member_access_name_span(ctx, parent_expression_id) else {
        return Some(span);
    };
    let receiver_end = member_name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if span.file == member_name_span.file && span.start < receiver_end && span.end >= receiver_end {
        return Some(Span::new(span.file, span.start, receiver_end));
    }

    // derive receiver spans when direct mapping points at member names
    if let Some(member_span) = get_dir_node_span(
        ctx.ast_context(),
        ctx.dir_analyzed_context(),
        parent_expression_id.into(),
    ) && member_span.file == member_name_span.file
        && member_span.start < receiver_end
    {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(span)
}

/// Resolve the best span for a member receiver expression.
fn resolve_member_receiver_reference_span(
    ctx: &QueryContext<'_>,
    member_expression_id: dir::LocalNodeId<Expression>,
    receiver_expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let receiver_expression_id = {
        let dir_tree = ctx.tree();
        let mut receiver_expression_id = receiver_expression_id;
        loop {
            let receiver_expression = dir_tree.get::<Expression>(receiver_expression_id);
            let Expression::Parenthesized { expression } = receiver_expression else {
                break;
            };
            receiver_expression_id = *expression;
        }
        receiver_expression_id
    };

    let receiver_span = ast_expression_span_for_dir_expression(ctx, receiver_expression_id)
        .or_else(|| {
            get_dir_node_main_span(
                ctx.ast_context(),
                ctx.dir_analyzed_context(),
                receiver_expression_id.into(),
            )
        })
        .or_else(|| {
            get_dir_node_span(
                ctx.ast_context(),
                ctx.dir_analyzed_context(),
                receiver_expression_id.into(),
            )
        })?;

    let Some(member_name_span) = get_member_access_name_span(ctx, member_expression_id) else {
        return Some(receiver_span);
    };
    let receiver_end = member_name_span.start.saturating_sub(1);

    // clamp spans that include `.member` to only the receiver expression
    if receiver_span.file == member_name_span.file
        && receiver_span.start < receiver_end
        && receiver_span.end >= receiver_end
    {
        return Some(Span::new(
            receiver_span.file,
            receiver_span.start,
            receiver_end,
        ));
    }

    // derive receiver spans from member expression spans when needed
    if let Some(member_span) = get_dir_node_span(
        ctx.ast_context(),
        ctx.dir_analyzed_context(),
        member_expression_id.into(),
    ) && member_span.file == member_name_span.file
        && member_span.start < receiver_end
    {
        return Some(Span::new(member_span.file, member_span.start, receiver_end));
    }

    Some(receiver_span)
}

/// Resolve the main AST span for a DIR expression source id.
fn ast_expression_span_for_dir_expression(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<Span> {
    let source_id = {
        let dir_tree = ctx.tree();
        dir_tree.get_source(expression_id.id)
    };

    let expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
    ast_expression_main_span_without_parentheses(ctx, expression_id)
}

/// Resolve an AST expression main span, unwrapping parenthesized expressions.
fn ast_expression_main_span_without_parentheses(
    ctx: &QueryContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Span> {
    let mut expression_id = expression_id;
    loop {
        let expression = ctx.ast_context().tree().get(expression_id);
        let ast::Expression::Parenthesized { expression } = expression else {
            break;
        };
        expression_id = *expression;
    }

    let expression = ctx.ast_context().tree().get(expression_id);
    if let ast::Expression::Path { .. } = expression {
        let expression_span = ctx.ast_context().tree().source_map.get(expression_id.id);
        if let Some(identifier_span) = last_identifier_span_in_expression(ctx, expression_span) {
            return Some(identifier_span);
        }
    }

    let span = ctx
        .ast
        .tree
        .source_map
        .get_main(expression_id.id)
        .unwrap_or_else(|| ctx.ast_context().tree().source_map.get(expression_id.id));
    Some(Span::new(ctx.file_id, span.start, span.end))
}

/// Resolve the last identifier token span inside an expression span.
fn last_identifier_span_in_expression(
    ctx: &QueryContext<'_>,
    expression_span: Span,
) -> Option<Span> {
    let ast = ctx.ast_context();

    let mut last_identifier_span = None;
    for token in ast.tokens() {
        if token.span.file != ctx.file_id {
            continue;
        }
        if token.token.ty != ast::TokenType::Identifier {
            continue;
        }
        if token.span.start < expression_span.start || token.span.end > expression_span.end {
            continue;
        }

        last_identifier_span = Some(token.span);
    }

    last_identifier_span
}

/// Collect member access reference spans.
fn collect_member_reference_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
    namespace_aliases: &[GlobalSymbolId],
) -> Vec<Span> {
    // initialize the collected span list
    let mut spans = Vec::new();

    // scan member expressions for matches and namespace receivers
    let dir_tree = ctx.tree();
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expression else {
            continue;
        };
        let Some(name) = *name else {
            continue;
        };

        // resolve the member symbol through normal resolution first
        if let Some(member_symbol) =
            resolve_member_access_symbol(session, ctx, expression_id, *left, name)
        {
            if !symbol_resolves_to_canonical(session, member_symbol, canonical_id) {
                continue;
            }

            let Some(span) = get_member_access_name_span(ctx, expression_id) else {
                continue;
            };

            // filter out spans outside the requested file
            if let Some(limit_file) = options.limit_to_file
                && span.file != limit_file
            {
                continue;
            }

            spans.push(span);
            continue;
        }

        // apply namespace import receiver matching when enabled
        if !options.include_namespace_receivers {
            continue;
        }

        // require a target name for namespace member matching
        let Some(target_name) = options.target_name else {
            continue;
        };

        // resolve the receiver symbol and require it to be a namespace alias
        let Some(receiver_symbol) = resolve_namespace_receiver_symbol(ctx, *left) else {
            continue;
        };
        if !namespace_aliases.contains(&receiver_symbol) {
            continue;
        }

        // require the member name to match the requested target name
        let member_name = session.strings.get(name).to_string();
        if member_name != target_name {
            continue;
        }

        let Some(span) = get_member_access_name_span(ctx, expression_id) else {
            continue;
        };

        // filter out spans outside the requested file
        if let Some(limit_file) = options.limit_to_file
            && span.file != limit_file
        {
            continue;
        }

        spans.push(span);
    }

    // return the collected spans
    spans
}

/// Collect dependency item reference spans.
fn collect_dependency_reference_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    // collect matching dependency item ids first to avoid borrow issues
    let matching_dependency_ids: Vec<_> = {
        let dir_tree = ctx.tree();
        dir_tree
            .iter_nodes_of_type::<DependencyItem>()
            .filter_map(|(item_id, item)| {
                let target = item.target_symbol()?;
                symbol_resolves_to_canonical(session, target, canonical_id).then_some(item_id)
            })
            .collect()
    };

    // resolve spans for the matching dependency items
    let mut spans = Vec::new();
    for item_id in matching_dependency_ids {
        let span = if options.use_dependency_name_spans {
            dependency_item_name_span(session, ctx, item_id, options.target_name)
        } else {
            None
        }
        .or_else(|| {
            get_dir_node_main_span(
                ctx.ast_context(),
                ctx.dir_analyzed_context(),
                item_id.into(),
            )
        });

        let Some(span) = span else {
            continue;
        };

        // filter out spans outside the requested file
        if let Some(limit_file) = options.limit_to_file
            && span.file != limit_file
        {
            continue;
        }

        spans.push(span);
    }

    // return the collected spans
    spans
}

/// Resolve a dependency item's name span when a target name is provided.
fn dependency_item_name_span(
    session: &Session,
    ctx: &QueryContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
    target_name: Option<&str>,
) -> Option<Span> {
    // fall back to the main span when no name was provided
    let Some(target_name) = target_name else {
        return get_dir_node_main_span(
            ctx.ast_context(),
            ctx.dir_analyzed_context(),
            item_id.into(),
        );
    };

    // resolve name + alias for matching
    let dir_tree = ctx.tree();
    let item = dir_tree.get::<DependencyItem>(item_id);
    let (name_id, alias_id) = match item {
        DependencyItem::Remote { name, alias, .. }
        | DependencyItem::Local { name, alias, .. }
        | DependencyItem::UnresolvedRemote { name, alias, .. }
        | DependencyItem::UnresolvedLocal { name, alias, .. } => {
            (name.map(|name| name.string()), *alias)
        }
        DependencyItem::Value { .. } => (None, None),
        DependencyItem::Error => return None,
    };

    // resolve the ast node id for span lookup
    let ast_node_id = dir_tree.get_source(item_id.id);

    // match the remote/local item name first
    if let Some(name_id) = name_id
        && session.strings.get(name_id) == target_name
    {
        let span = ctx
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Type)?;
        return Some(Span::new(ctx.file_id, span.start, span.end));
    }

    // fall back to alias when present
    if let Some(alias_id) = alias_id
        && session.strings.get(alias_id) == target_name
    {
        let span = ctx
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)?;
        return Some(Span::new(ctx.file_id, span.start, span.end));
    }

    None
}

/// Collect namespace import aliases that target a module.
fn namespace_import_aliases_for_module(
    ctx: &QueryContext<'_>,
    module_id: ModuleId,
) -> Vec<GlobalSymbolId> {
    // scan dependency items for namespace imports to the target module
    let dir_tree = ctx.tree();
    dir_tree
        .iter_nodes_of_type::<DependencyItem>()
        .filter_map(|(_item_id, item)| {
            let DependencyItem::Remote {
                mode,
                kind,
                symbol,
                target_module,
                ..
            } = item
            else {
                return None;
            };

            // require namespace value imports with a concrete symbol
            if *mode != DependencyMode::Namespace || *kind != DependencyKind::Value {
                return None;
            }

            let local_symbol = symbol.as_ref()?;
            let target_module_id = target_module
                .for_kind(*kind)
                .and_then(|target| target.module_id())?;
            if target_module_id != module_id {
                return None;
            }

            Some(GlobalSymbolId::new(ctx.module_id, *local_symbol))
        })
        .collect()
}
