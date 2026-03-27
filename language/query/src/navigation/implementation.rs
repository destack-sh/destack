use std::collections::HashSet;

use destack_dir::{
    DependencyItem, Expression, GlobalSymbolId, LocalNodeIdAny, NodeType, SymbolType,
};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::ast::{get_node_tree_main_span, sort_and_dedup_spans};
use crate::core::{SessionQueryIndexExt, query_context, with_query_context_for_file};
use crate::dir::{
    find_symbol_at_offset, get_canonical_symbol, get_symbol_definition_span,
    resolve_nominal_symbol_from_type_expression,
};
use destack_workspace::{NominalRelationKind, Session};

/// Result of a goto implementation query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImplementationResult {
    /// Implementation locations.
    pub locations: Vec<Span>,
}

impl ImplementationResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            locations: Vec::new(),
        }
    }

    /// Whether any implementations were found.
    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }
}

/// Request goto implementation at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoImplementationRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto implementation queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoImplementationResponse {
    /// Implementation locations, if any.
    pub result: Option<ImplementationResult>,
}

/// Find implementations of the symbol at the given position.
///
/// For interfaces: finds implementing structs/classes.
/// For abstract methods: finds concrete implementations.
/// For classes: finds subclasses.
pub fn goto_implementation(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<ImplementationResult> {
    // find the symbol at the cursor position
    let symbol_at = find_symbol_at_offset(session, file, offset);

    // prefer type symbols when the cursor is on a type annotation
    let target_symbol_id = if let Some(symbol_at) = symbol_at {
        let mut symbol_id = symbol_at.symbol_id;
        if !symbol_is_implementable(session, symbol_id) {
            let node_type_symbol = resolve_type_symbol_from_node(session, file, symbol_at.node_id);
            let expression_type_symbol =
                resolve_type_symbol_from_expression_node(session, file, symbol_at.node_id);
            let offset_type_symbol = resolve_type_symbol_at_offset(session, file, offset);

            if let Some(type_symbol_id) = node_type_symbol {
                symbol_id = type_symbol_id;
            } else if let Some(type_symbol_id) = expression_type_symbol {
                symbol_id = type_symbol_id;
            } else if let Some(type_symbol_id) = offset_type_symbol {
                symbol_id = type_symbol_id;
            }
        }

        symbol_id
    } else if let Some(type_symbol_id) = resolve_type_symbol_at_offset(session, file, offset) {
        type_symbol_id
    } else {
        return Some(ImplementationResult::empty());
    };

    // collect canonical targets reachable from the cursor symbol
    let target_symbols = collect_target_symbols(session, target_symbol_id);

    // select an implementable symbol from the target set
    let canonical_id = target_symbols
        .iter()
        .copied()
        .find(|symbol_id| symbol_is_implementable(session, *symbol_id));

    let Some(canonical_id) = canonical_id else {
        return Some(ImplementationResult::empty());
    };

    // resolve the target symbol type information
    let target_module = session.modules.get(canonical_id.module_id);
    let target_module = target_module.as_ref();
    let Some(ctx) = query_context(session, target_module) else {
        return Some(ImplementationResult::empty());
    };

    // resolve the target symbol metadata
    let (is_interface, is_class) = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        (
            symbol.ty == SymbolType::Interface,
            symbol.ty == SymbolType::Class,
        )
    };

    // bail out for symbols that cannot be implemented
    if !is_interface && !is_class {
        return Some(ImplementationResult::empty());
    }

    // initialize the result spans
    let mut locations = Vec::new();

    // match cached direct edges against the target symbol set
    for target_symbol in target_symbols {
        let entries = session.nominal_index_entries_for_target(target_symbol);

        for entry in entries {
            let matches = if is_interface {
                entry.relation == NominalRelationKind::Implements
            } else {
                entry.relation == NominalRelationKind::Extends
            };

            if matches && let Some(span) = get_symbol_definition_span(session, entry.source_symbol)
            {
                locations.push(span);
            }
        }
    }

    // normalize spans for stable ordering and deduplication
    sort_and_dedup_spans(&mut locations);

    Some(ImplementationResult { locations })
}

/// Resolve a type symbol at the given offset when the cursor is on a type annotation.
fn resolve_type_symbol_at_offset(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<GlobalSymbolId> {
    with_query_context_for_file(session, file, |ctx| {
        // scan expression nodes to find a type reference under the cursor
        let dir_tree = ctx.dir().tree();
        for (expression_id, _expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            let span = get_node_tree_main_span(ctx.ast(), ctx.dir().tree(), expression_id.into());

            if offset < span.start || offset > span.end {
                continue;
            }

            if let Some(symbol_id) =
                resolve_nominal_symbol_from_type_expression(session, ctx.dir(), expression_id)
            {
                return Some(symbol_id);
            }
        }

        None
    })
    .unwrap_or(None)
}

/// Resolve a nominal type symbol from a node's declared or inferred type.
fn resolve_type_symbol_from_node(
    session: &Session,
    file: FileId,
    node_id: LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    with_query_context_for_file(session, file, |ctx| {
        let global_node_id = node_id.into_global(ctx.module_id());
        let types = ctx.dir().types();
        let type_id = types.get_declared_or_inferred_type_id(global_node_id)?;
        let ty = types.get_type(type_id);
        ty.symbol()
    })
    .unwrap_or(None)
}

/// Resolve a type symbol from an expression node when available.
fn resolve_type_symbol_from_expression_node(
    session: &Session,
    file: FileId,
    node_id: LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    if node_id.ty != NodeType::Expression {
        return None;
    }

    let Ok(expr_id) = node_id.try_into_typed() else {
        return None;
    };

    with_query_context_for_file(session, file, |ctx| {
        resolve_nominal_symbol_from_type_expression(session, ctx.dir(), expr_id)
    })
    .unwrap_or(None)
}

/// Collect canonical symbols reachable from a query target.
fn collect_target_symbols(session: &Session, symbol_id: GlobalSymbolId) -> HashSet<GlobalSymbolId> {
    // seed the search with the initial symbol
    let mut pending = vec![symbol_id];
    let mut visited = HashSet::new();

    // walk canonical and dependency chains
    while let Some(current) = pending.pop() {
        // canonicalize the current symbol
        let canonical_id = get_canonical_symbol(session, current);
        if !visited.insert(canonical_id) {
            continue;
        }

        // resolve the module and query context for the canonical symbol
        let module = session.modules.get(canonical_id.module_id);
        let module = module.as_ref();
        let Some(ctx) = query_context(session, module) else {
            continue;
        };

        // resolve the next target symbol from symbol metadata or dependency items
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);

        let mut target_symbol = symbol.target_symbol;
        if target_symbol.is_none() {
            let Some(primary_decl) = symbol.primary_declaration else {
                continue;
            };

            if primary_decl.local_id.ty == NodeType::DependencyItem {
                let Ok(item_id) = primary_decl.local_id.try_into() else {
                    continue;
                };

                let dir_tree = ctx.dir().tree();
                let item = dir_tree.get::<DependencyItem>(item_id);
                target_symbol = item.target_symbol();
            }
        }

        // continue walking when a dependency target exists
        if let Some(target_symbol) = target_symbol {
            pending.push(target_symbol);
        }
    }

    // return the collected canonical targets
    visited
}

/// Check whether a symbol is an interface or class.
fn symbol_is_implementable(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let Some(ctx) = query_context(session, module) else {
        return false;
    };

    // read the symbol type
    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // return whether the symbol is implementable
    symbol.ty == SymbolType::Interface || symbol.ty == SymbolType::Class
}
