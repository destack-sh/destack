use destack_dir::{
    self as dir, DependencyItem, DependencyKind, DependencyMode, Expression, GlobalSymbolId,
};
use destack_source::{FileId, ModuleId, Span};

use super::QueryContext;
use super::span::{find_identifier_span_in_span, get_dir_node_main_span, get_dir_node_span};
use super::symbol::{
    get_canonical_symbol, get_member_access_name_span, is_dependency_alias_for_target,
    resolve_member_access_symbol,
};
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
                let target_symbol = expression.target_symbol();

                // skip member expressions when member references are enabled
                if options.include_members && matches!(expression, Expression::Member { .. }) {
                    return None;
                }

                // skip dependency aliases when requested
                if options.skip_dependency_aliases
                    && let Some(target_symbol) = target_symbol
                    && is_dependency_alias_for_target(session, ctx, target_symbol, canonical_id)
                {
                    return None;
                }

                // check for a canonical target match
                let target_symbol = target_symbol?;
                let target_canonical = get_canonical_symbol(session, target_symbol);
                (target_canonical == canonical_id).then_some(expression_id)
            })
            .collect()
    };

    // resolve spans for the matching expressions
    let mut spans = Vec::new();
    for expression_id in matching_expression_ids {
        let Some(span) = get_dir_node_span(ctx.ast, ctx.dir, expression_id.into()) else {
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

    spans
}

/// Collect member access reference spans.
fn collect_member_reference_spans(
    session: &Session,
    ctx: &QueryContext<'_>,
    canonical_id: GlobalSymbolId,
    options: ReferenceCollectionOptions<'_>,
    namespace_aliases: &[GlobalSymbolId],
) -> Vec<Span> {
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
            let target_canonical = get_canonical_symbol(session, member_symbol);
            if target_canonical != canonical_id {
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
                let target_canonical = get_canonical_symbol(session, target);
                (target_canonical == canonical_id).then_some(item_id)
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

    spans
}

/// Resolve a dependency item's name span when a target name is provided.
fn dependency_item_name_span(
    session: &Session,
    ctx: &QueryContext<'_>,
    item_id: dir::LocalNodeId<DependencyItem>,
    target_name: Option<&str>,
) -> Option<Span> {
    // resolve the full dependency item span
    let full_span = get_dir_node_span(ctx.ast, ctx.dir, item_id.into())?;

    // fall back to the full span when no name was provided
    let Some(target_name) = target_name else {
        return Some(full_span);
    };

    find_identifier_span_in_span(session, full_span, target_name)
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
            let target_module_id = target_module.module_id()?;
            if target_module_id != module_id {
                return None;
            }

            Some(GlobalSymbolId::new(ctx.module_id, *local_symbol))
        })
        .collect()
}
