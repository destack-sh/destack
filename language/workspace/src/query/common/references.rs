use std::collections::HashSet;

use destack_dir::{
    self as dir, DependencyItem, DependencyKind, DependencyMode, Expression, GlobalSymbolId,
    NodeType, SymbolSpace,
};
use destack_source::{FileId, ModuleId, NodeSpanType, Span};

use super::resolve::resolve_type_symbol_from_imports;
use super::span::{get_dir_node_main_span, get_dir_node_span};
use super::symbol::{
    find_symbol_at_offset, get_canonical_symbol, get_member_access_name_span,
    is_dependency_alias_for_target, resolve_member_access_symbol,
};
use super::{QueryContext, is_identifier_continue, visible_symbols, visible_symbols_full};
use crate::Session;

/// Options for collecting symbol references.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceCollectionOptions<'a> {
    /// Whether to include direct expression references.
    pub include_expressions: bool,
    /// Whether to include member access references.
    pub include_members: bool,
    /// Whether to include dependency item references.
    pub include_dependencies: bool,
    /// Whether to include namespace member fallbacks.
    pub include_namespace_members: bool,
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
            include_namespace_members: false,
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

    // compute namespace import aliases when namespace fallbacks are enabled
    let namespace_aliases = if options.include_namespace_members {
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

    // collect interface type parameter spans when expression references are missing
    if let Some(target_name) = options.target_name {
        let interface_spans = collect_interface_type_parameter_spans(
            session,
            ctx,
            canonical_id,
            target_name,
            options,
        );
        spans.extend(interface_spans);
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
                let target_symbol = resolve_expression_target_symbol(
                    session,
                    ctx,
                    &dir_tree,
                    expression_id,
                    expression,
                    canonical_id,
                    options,
                );

                // skip member expressions when member references are enabled
                if options.include_members && matches!(expression, Expression::Member { .. }) {
                    return None;
                }

                // skip dependency aliases when requested
                let is_alias = if options.skip_dependency_aliases {
                    target_symbol
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
                let target_symbol = target_symbol?;
                if symbol_resolves_to_canonical(session, target_symbol, canonical_id) {
                    return Some(expression_id);
                }

                // fall back to matching visible type parameters by name
                let target_name = options.target_name?;
                if !symbol_is_type_parameter(session, canonical_id) {
                    return None;
                }
                if !symbol_visible_in_scope(ctx, &dir_tree, expression_id, canonical_id) {
                    return None;
                }
                let full_span = get_dir_node_span(ctx.ast, ctx.dir, expression_id.into())?;
                find_name_span_in_span(session, full_span, target_name, true)?;

                Some(expression_id)
            })
            .collect()
    };

    // resolve spans for the matching expressions
    let mut spans = Vec::new();
    for expression_id in matching_expression_ids {
        let span = resolve_expression_reference_span(session, ctx, expression_id, options);
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

    // capture member receiver spans when the receiver matches the target symbol
    if options.include_members {
        let dir_tree = ctx.tree();
        for (_expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            let Expression::Member { left, .. } = expression else {
                continue;
            };
            let left_expr = dir_tree.get::<Expression>(*left);
            let target_symbol = left_expr.target_symbol().or_else(|| {
                let span = get_dir_node_main_span(ctx.ast, ctx.dir, (*left).into())
                    .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, (*left).into()))?;
                let symbol_at = find_symbol_at_offset(session, ctx.file_id, span.start)?;
                Some(symbol_at.symbol_id)
            });
            let Some(target_symbol) = target_symbol else {
                continue;
            };
            if !symbol_resolves_to_canonical(session, target_symbol, canonical_id) {
                continue;
            }

            let span = resolve_receiver_reference_span(session, ctx, *left, options)
                .or_else(|| get_dir_node_main_span(ctx.ast, ctx.dir, (*left).into()))
                .or_else(|| get_dir_node_span(ctx.ast, ctx.dir, (*left).into()));
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

        // capture receiver spans for qualified path references
        if let Some(target_name) = options.target_name {
            for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
                let path = match expression {
                    Expression::LocalReference { path, .. }
                    | Expression::ModuleReference { path, .. }
                    | Expression::GlobalReference { path, .. } => path,
                    _ => continue,
                };
                if path.segments.len() < 2 {
                    continue;
                }

                if path
                    .first_segment()
                    .is_some_and(|id| session.strings.get(id) != target_name)
                {
                    continue;
                }

                let full_span = get_dir_node_span(ctx.ast, ctx.dir, expression_id.into());
                let Some(full_span) = full_span else {
                    continue;
                };
                let receiver_span = find_name_span_in_span(session, full_span, target_name, false);
                let Some(receiver_span) = receiver_span else {
                    continue;
                };

                let is_visible_target = if canonical_id.module_id == ctx.module_id {
                    let (scope_id, mark) = dir_tree.get_scope(expression_id);
                    let symbols = ctx.symbols();
                    let symbol = symbols.get_symbol(canonical_id.local_id);
                    visible_symbols(&symbols, scope_id, mark, Some(symbol.space))
                        .any(|visible| visible.id == canonical_id.local_id)
                } else {
                    let symbol_at =
                        find_symbol_at_offset(session, receiver_span.file, receiver_span.start);
                    let Some(symbol_at) = symbol_at else {
                        continue;
                    };
                    symbol_resolves_to_canonical(session, symbol_at.symbol_id, canonical_id)
                };

                if !is_visible_target {
                    continue;
                }

                if let Some(limit_file) = options.limit_to_file
                    && receiver_span.file != limit_file
                {
                    continue;
                }

                spans.push(receiver_span);
            }
        }
    }

    // return the matching spans
    spans
}

/// Resolve the target symbol for an expression reference, including import fallbacks.
fn resolve_expression_target_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    expression: &Expression,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
) -> Option<GlobalSymbolId> {
    // prefer the resolved target symbol on the expression
    if let Some(target_symbol) = expression.target_symbol() {
        return Some(target_symbol);
    }

    // fall back to resolving unresolved paths through imports when possible
    let Expression::UnresolvedPath { path, .. } = expression else {
        return None;
    };
    let name_id = path.last_segment()?;
    if let Some(target_name) = options.target_name
        && session.strings.get(name_id) != target_name
    {
        return None;
    }

    if let Some(symbol_id) = resolve_type_symbol_from_imports(session, ctx, name_id) {
        return Some(symbol_id);
    }

    if canonical_id.module_id != ctx.module_id {
        return None;
    }

    let (scope_id, mark) = dir_tree.get_scope(expression_id);
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);
    let is_visible = visible_symbols(&symbols, scope_id, mark, Some(symbol.space))
        .any(|visible| visible.id == canonical_id.local_id);
    is_visible.then_some(canonical_id)
}

/// Check whether a symbol resolves to the requested canonical id.
fn symbol_resolves_to_canonical(
    session: &Session,
    symbol_id: GlobalSymbolId,
    canonical_id: GlobalSymbolId,
) -> bool {
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
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
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
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    options: ReferenceCollectionOptions<'_>,
) -> Option<Span> {
    // resolve the full expression span for fallback lookup
    let full_span = get_dir_node_span(ctx.ast, ctx.dir, expression_id.into())?;

    // prefer a precise name span when a target name is available
    if let Some(target_name) = options.target_name {
        if let Some(span) = find_name_span_in_span(session, full_span, target_name, true) {
            return Some(span);
        }

        // avoid guessing for rename-style queries
        if options.skip_dependency_aliases {
            return None;
        }
    }

    // fall back to the main span when available
    get_dir_node_main_span(ctx.ast, ctx.dir, expression_id.into()).or(Some(full_span))
}

/// Resolve a receiver span inside a member access expression.
fn resolve_receiver_reference_span(
    session: &Session,
    ctx: &QueryContext<'_>,
    left_expression_id: dir::LocalNodeId<Expression>,
    options: ReferenceCollectionOptions<'_>,
) -> Option<Span> {
    // require a target name to find a receiver span within the expression text
    let target_name = options.target_name?;
    let span = get_dir_node_span(ctx.ast, ctx.dir, left_expression_id.into())?;
    find_name_span_in_span(session, span, target_name, true)
}

/// Find a name span inside a larger span.
fn find_name_span_in_span(
    session: &Session,
    span: Span,
    target_name: &str,
    prefer_last: bool,
) -> Option<Span> {
    let source_file = session.files.get(span.file);
    let text = source_file.span_str(span);
    let pos = find_name_offset(text, target_name, prefer_last)?;
    let start = span.start + pos as u32;
    let end = start + target_name.len() as u32;
    Some(Span::new(span.file, start, end))
}

/// Collect all matching name spans inside a larger span.
fn collect_name_spans_in_span(session: &Session, span: Span, target_name: &str) -> Vec<Span> {
    let source_file = session.files.get(span.file);
    let text = source_file.span_str(span);
    let mut spans = Vec::new();

    for (pos, _) in text.match_indices(target_name) {
        if !is_identifier_boundary(text, pos, pos + target_name.len()) {
            continue;
        }
        let start = span.start + pos as u32;
        let end = start + target_name.len() as u32;
        spans.push(Span::new(span.file, start, end));
    }

    spans
}

/// Find the byte offset of an identifier name in a string.
fn find_name_offset(text: &str, target_name: &str, prefer_last: bool) -> Option<usize> {
    // scan for matching identifier spans
    let mut match_pos = None;
    for (pos, _) in text.match_indices(target_name) {
        if !is_identifier_boundary(text, pos, pos + target_name.len()) {
            continue;
        }

        match_pos = Some(pos);
        if !prefer_last {
            break;
        }
    }

    match_pos
}

/// Check that a match aligns with identifier boundaries.
fn is_identifier_boundary(text: &str, start: usize, end: usize) -> bool {
    // ensure the match is not part of a larger identifier
    let bytes = text.as_bytes();
    if start > 0 {
        let prev = bytes[start - 1] as char;
        if is_identifier_continue(prev) {
            return false;
        }
    }
    if end < bytes.len() {
        let next = bytes[end] as char;
        if is_identifier_continue(next) {
            return false;
        }
    }
    true
}

/// Check whether a symbol represents a static type parameter.
fn symbol_is_type_parameter(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return false;
    };

    // ensure the symbol is declared by a parameter node in type space
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let Some(declaration) = symbol.primary_declaration else {
        return false;
    };
    if declaration.local_id.ty != NodeType::Parameter {
        return false;
    }

    matches!(symbol.space, SymbolSpace::Type | SymbolSpace::TypeValue)
}

/// Check whether a symbol is visible at an expression's scope.
fn symbol_visible_in_scope(
    ctx: &QueryContext<'_>,
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
    symbol_id: GlobalSymbolId,
) -> bool {
    if symbol_id.module_id != ctx.module_id {
        return false;
    }

    let (scope_id, mark) = dir_tree.get_scope(expression_id);
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    if visible_symbols(&symbols, scope_id, mark, Some(symbol.space))
        .any(|visible| visible.id == symbol_id.local_id)
    {
        return true;
    }

    visible_symbols_full(&symbols, scope_id, Some(symbol.space))
        .any(|visible| visible.id == symbol_id.local_id)
}

/// Collect reference spans for type parameters owned by interfaces.
fn collect_interface_type_parameter_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    target_name: &str,
    options: ReferenceCollectionOptions<'_>,
) -> Vec<Span> {
    if canonical_id.module_id != ctx.module_id {
        return Vec::new();
    }
    if !symbol_is_type_parameter(session, canonical_id) {
        return Vec::new();
    }

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(canonical_id.local_id);
    let Some(declaration) = symbol.primary_declaration else {
        return Vec::new();
    };
    if declaration.local_id.ty != NodeType::Parameter {
        return Vec::new();
    }
    drop(symbols);

    let dir_tree = ctx.tree();
    let mut current = declaration.local_id.id;
    while let Some(parent) = dir_tree.get_parent(current) {
        if parent.ty == NodeType::Declaration {
            let Ok(declaration_id) = parent.try_into() else {
                break;
            };
            let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
            if !matches!(declaration, dir::Declaration::Interface { .. }) {
                break;
            }

            let Some(span) = get_dir_node_span(ctx.ast, ctx.dir, parent) else {
                break;
            };
            let mut spans = collect_name_spans_in_span(session, span, target_name);
            if let Some(limit_file) = options.limit_to_file {
                spans.retain(|span| span.file == limit_file);
            }
            return spans;
        }

        current = parent.id;
    }

    Vec::new()
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

    // scan member expressions for matches and namespace fallbacks
    let dir_tree = ctx.tree();
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Member { left, name, .. } = expression else {
            continue;
        };

        // resolve the member symbol through normal resolution first
        if let Some(member_symbol) =
            resolve_member_access_symbol(session, ctx, expression_id, *left, *name)
        {
            if !symbol_resolves_to_canonical(session, member_symbol, canonical_id) {
                continue;
            }

            let member_name = session.strings.get(*name).to_string();
            let Some(span) = get_member_access_name_span(session, ctx, expression_id, &member_name)
            else {
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

        // apply namespace import fallback when enabled
        if !options.include_namespace_members {
            continue;
        }

        // require a target name for namespace member matching
        let Some(target_name) = options.target_name else {
            continue;
        };

        // resolve the receiver symbol and require it to be a namespace alias
        let left_expr = dir_tree.get::<Expression>(*left);
        let Some(receiver_symbol) = left_expr.target_symbol() else {
            continue;
        };
        if !namespace_aliases.contains(&receiver_symbol) {
            continue;
        }

        // require the member name to match the requested target name
        let member_name = session.strings.get(*name).to_string();
        if member_name != target_name {
            continue;
        }

        let Some(span) = get_member_access_name_span(session, ctx, expression_id, &member_name)
        else {
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
        .or_else(|| get_dir_node_main_span(ctx.ast, ctx.dir, item_id.into()));

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
        return get_dir_node_main_span(ctx.ast, ctx.dir, item_id.into());
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
