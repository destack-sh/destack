use std::collections::HashSet;

use destack_dir::{Expression, GlobalSymbolId, LocalNodeIdAny, NodeType, SymbolForm};
use serde::{Deserialize, Serialize};

use crate::core::{
    ModuleQueryContext, NominalRelation, QueryPosition, WorkspaceQueryContext,
    nominal_relations_for_target,
};
use crate::dir::{
    dependency_symbol_target, find_symbol_at_offset, resolve_nominal_symbol_from_type_expression,
    symbol_definition_span,
};
use crate::navigation::{NavigationRelation, NavigationTarget};
use crate::source::get_node_tree_main_span;

/// Request goto implementation at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoImplementationRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for goto implementation queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoImplementationResponse {
    /// Implementation targets.
    pub targets: Vec<NavigationTarget>,
}

/// Find implementations of the symbol at the given position.
///
/// For interfaces: finds implementing structs/classes.
/// For abstract methods: finds concrete implementations.
/// For classes: finds subclasses.
pub fn goto_implementation(
    ctx: &ModuleQueryContext<'_>,
    workspace: &WorkspaceQueryContext<'_>,
    offset: u32,
) -> Vec<NavigationTarget> {
    // find the symbol at the cursor position
    let symbol_at = find_symbol_at_offset(ctx, offset);

    // prefer type symbols when the cursor is on a type annotation
    let target_symbol_id = if let Some(symbol_at) = symbol_at {
        let mut symbol_id = symbol_at.symbol_id;
        if !symbol_is_implementable(ctx, symbol_id) {
            let node_type_symbol = resolve_type_symbol_from_node(ctx, symbol_at.node_id);
            let expression_type_symbol =
                resolve_type_symbol_from_expression_node(ctx, symbol_at.node_id);
            let offset_type_symbol = resolve_type_symbol_at_offset(ctx, offset);

            if let Some(type_symbol_id) = node_type_symbol {
                symbol_id = type_symbol_id;
            } else if let Some(type_symbol_id) = expression_type_symbol {
                symbol_id = type_symbol_id;
            } else if let Some(type_symbol_id) = offset_type_symbol {
                symbol_id = type_symbol_id;
            }
        }

        symbol_id
    } else if let Some(type_symbol_id) = resolve_type_symbol_at_offset(ctx, offset) {
        type_symbol_id
    } else {
        return Vec::new();
    };

    // collect canonical targets reachable from the cursor symbol
    let target_symbols = collect_target_symbols(ctx, target_symbol_id);

    // select an implementable symbol from the target set
    let canonical_id = target_symbols
        .iter()
        .copied()
        .find(|symbol_id| symbol_is_implementable(ctx, *symbol_id));

    let Some(canonical_id) = canonical_id else {
        return Vec::new();
    };

    // resolve the target symbol type information
    let Some(target_ctx) = ctx.module_context(canonical_id.module_id) else {
        return Vec::new();
    };

    // resolve the target symbol metadata
    let (is_interface, is_class) = {
        let symbols = target_ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        (
            symbol.form == SymbolForm::Interface,
            symbol.form == SymbolForm::Class,
        )
    };

    // bail out for symbols that cannot be implemented
    if !is_interface && !is_class {
        return Vec::new();
    }

    // initialize the result spans
    let mut targets = Vec::new();

    // match cached direct edges against the target symbol set
    for target_symbol in target_symbols {
        let entries = nominal_relations_for_target(workspace, target_symbol);

        for entry in entries {
            let matches = if is_interface {
                entry.relation == NominalRelation::Implements
            } else {
                entry.relation == NominalRelation::Extends
            };

            if matches
                && let Some(source_ctx) = ctx.module_context(entry.source_symbol.module_id)
                && let Some(span) = symbol_definition_span(&source_ctx, entry.source_symbol)
            {
                let target =
                    NavigationTarget::span(&source_ctx, span, NavigationRelation::Implementation)
                        .with_symbol(entry.source_symbol);
                targets.push(target);
            }
        }
    }

    // normalize spans for stable ordering and deduplication
    targets.sort_by_key(|target| {
        (
            target.target.span.file,
            target.target.span.start,
            target.target.span.end,
        )
    });
    targets.dedup();

    targets
}

/// Resolve a type symbol at the given offset when the cursor is on a type annotation.
fn resolve_type_symbol_at_offset(
    ctx: &ModuleQueryContext<'_>,
    offset: u32,
) -> Option<GlobalSymbolId> {
    // scan expression nodes to find a type reference under the cursor
    let dir_tree = ctx.dir().view();
    for (expression_id, _expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let span = get_node_tree_main_span(ctx.dir(), ctx.dir().view(), expression_id.into());

        if offset < span.start || offset > span.end {
            continue;
        }

        if let Some(symbol_id) =
            resolve_nominal_symbol_from_type_expression(ctx.dir(), expression_id)
        {
            return Some(symbol_id);
        }
    }

    None
}

/// Resolve a nominal type symbol from a node's checked type.
fn resolve_type_symbol_from_node(
    ctx: &ModuleQueryContext<'_>,
    node_id: LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    let global_node_id = node_id.into_global(ctx.module_id());
    let types = ctx.dir().types();
    let type_id = types.get_node_type_id(global_node_id)?;
    let ty = types.get_type(type_id);

    ty.symbol()
}

/// Resolve a type symbol from an expression node when available.
fn resolve_type_symbol_from_expression_node(
    ctx: &ModuleQueryContext<'_>,
    node_id: LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    if node_id.ty != NodeType::Expression {
        return None;
    }

    let Ok(expr_id) = node_id.try_into_typed() else {
        return None;
    };

    resolve_nominal_symbol_from_type_expression(ctx.dir(), expr_id)
}

/// Collect canonical symbols reachable from a query target.
fn collect_target_symbols(
    ctx: &ModuleQueryContext<'_>,
    symbol_id: GlobalSymbolId,
) -> HashSet<GlobalSymbolId> {
    // seed the search with the initial symbol
    let mut pending = vec![symbol_id];
    let mut visited = HashSet::new();

    // walk canonical and dependency chains
    while let Some(current) = pending.pop() {
        // canonicalize the current symbol
        let canonical_id = ctx.canonical_symbol(current);
        if !visited.insert(canonical_id) {
            continue;
        }

        // resolve the module and query context for the canonical symbol
        let Some(canonical_ctx) = ctx.module_context(canonical_id.module_id) else {
            continue;
        };

        // resolve the next target symbol from symbol metadata or dependency items
        let symbols = canonical_ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);

        let target_symbol = symbol.declaration.and_then(|declaration| {
            if declaration.local_id.ty != NodeType::DependencyItem {
                return None;
            }

            let item_id = declaration.local_id.try_into().ok()?;
            dependency_symbol_target(canonical_ctx.dir(), item_id)
        });

        // continue walking when a dependency target exists
        if let Some(target_symbol) = target_symbol {
            pending.push(target_symbol);
        }
    }

    // return the collected canonical targets
    visited
}

/// Check whether a symbol is an interface or class.
fn symbol_is_implementable(ctx: &ModuleQueryContext<'_>, symbol_id: GlobalSymbolId) -> bool {
    // resolve the module and query context for the symbol
    let Some(ctx) = ctx.module_context(symbol_id.module_id) else {
        return false;
    };

    // read the symbol type
    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // return whether the symbol is implementable
    symbol.form == SymbolForm::Interface || symbol.form == SymbolForm::Class
}
