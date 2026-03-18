use destack_dir as dir;
use std::collections::HashSet;

use destack_dir::{
    Declaration, DependencyItem, Expression, GlobalSymbolId, LocalNodeIdAny, NodeType, SymbolSpace,
    SymbolType,
};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::common::{
    find_symbol_at_offset, get_canonical_symbol, get_dir_node_main_span,
    get_symbol_definition_span, resolve_nominal_symbol_from_type_expression,
    resolve_type_symbol_from_dependency_symbol, resolve_type_symbol_from_module,
    sort_and_dedup_spans,
};
use destack_workspace::Session;

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
    let mut target_symbols = collect_target_symbols(session, target_symbol_id);

    // select an implementable symbol from the target set
    let mut canonical_id = target_symbols
        .iter()
        .copied()
        .find(|symbol_id| symbol_is_implementable(session, *symbol_id));

    // fall back to type definition resolution when needed
    if canonical_id.is_none()
        && let Some((fallback_symbols, fallback_id)) =
            fallback_target_symbols_from_type_definition(session, file, offset)
    {
        target_symbols = fallback_symbols;
        canonical_id = Some(fallback_id);
    }

    let Some(canonical_id) = canonical_id else {
        return Some(ImplementationResult::empty());
    };

    // resolve the target symbol type information
    let target_module = session.modules.get(canonical_id.module_id);
    let target_module = target_module.as_ref();
    let Some(ctx) = crate::query_context(session, &target_module) else {
        return Some(ImplementationResult::empty());
    };

    // resolve the target symbol metadata
    let (is_interface, is_class) = {
        let symbols = ctx.symbols();
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

    // search all modules for types that implement or extend this symbol
    for module in session.modules.iter() {
        // resolve query context for each module
        let module = module.as_ref();
        let Some(ctx) = crate::query_context(session, &module) else {
            continue;
        };

        // check all lineages in this module
        {
            let types = ctx.types();
            for (symbol_id, lineage) in types.iter_lineages() {
                // check if this type implements or extends the target symbol
                let matches = if is_interface {
                    lineage.implements.iter().any(|symbol| {
                        let candidates = collect_target_symbols(session, *symbol);
                        candidates
                            .iter()
                            .any(|candidate| target_symbols.contains(candidate))
                    })
                } else {
                    lineage
                        .extends
                        .map(|symbol| {
                            let candidates = collect_target_symbols(session, symbol);
                            candidates
                                .iter()
                                .any(|candidate| target_symbols.contains(candidate))
                        })
                        .unwrap_or(false)
                };

                if matches {
                    // get the span of the implementing type's declaration
                    if let Some(span) = get_symbol_definition_span(session, symbol_id) {
                        locations.push(span);
                    }
                }
            }
        }

        // scan syntactic heritage when lineages are incomplete
        let dir_tree = ctx.tree();
        for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
            // extract declaration heritage and symbol
            let (descriptor, heritage) = match declaration {
                Declaration::Struct {
                    descriptor,
                    heritage,
                    ..
                }
                | Declaration::Class {
                    descriptor,
                    heritage,
                    ..
                } => (descriptor, heritage),
                _ => continue,
            };

            // select the relevant heritage clause
            let related_types = if is_interface {
                heritage.implements_types.as_ref()
            } else {
                heritage.extends_types.as_ref()
            };
            let Some(related_types) = related_types else {
                continue;
            };

            // resolve heritage types to nominal symbols
            let mut matches = false;
            for type_expr_id in related_types {
                let mut matches_target = false;
                let mut symbol_id =
                    resolve_nominal_symbol_from_type_expression(session, &ctx, *type_expr_id);

                // try to match the nominally resolved symbol first
                if let Some(symbol_id) = symbol_id {
                    let candidates = collect_target_symbols(session, symbol_id);
                    if candidates
                        .iter()
                        .any(|candidate| target_symbols.contains(candidate))
                    {
                        matches_target = true;
                    }
                }

                // fall back to goto_type_definition when nominal resolution misses
                if !matches_target {
                    let span = get_dir_node_main_span(&ctx.ast, &ctx.dir, (*type_expr_id).into());
                    if let Some(span) = span
                        && let Some(result) =
                            super::goto_type_definition(session, span.file, span.start)
                        && let Some(location) = result.locations.first()
                        && let Some(def_symbol) =
                            find_symbol_at_offset(session, location.file, location.start)
                    {
                        symbol_id = Some(def_symbol.symbol_id);
                    }

                    if let Some(symbol_id) = symbol_id {
                        let candidates = collect_target_symbols(session, symbol_id);
                        if candidates
                            .iter()
                            .any(|candidate| target_symbols.contains(candidate))
                        {
                            matches_target = true;
                        }
                    }
                }

                // record when a related type matches the target
                if matches_target {
                    matches = true;
                    break;
                }
            }

            // skip unrelated declarations
            if !matches {
                continue;
            }

            // record the implementing declaration span
            let symbol_id = GlobalSymbolId {
                module_id: ctx.module_id,
                local_id: descriptor.symbol,
            };
            if let Some(span) = get_symbol_definition_span(session, symbol_id) {
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
    crate::with_query_context_for_file(session, file, |ctx| {
        // scan expression nodes to find a type reference under the cursor
        let dir_tree = ctx.tree();
        for (expression_id, _expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            let Some(span) = get_dir_node_main_span(&ctx.ast, &ctx.dir, expression_id.into())
            else {
                continue;
            };

            if offset < span.start || offset > span.end {
                continue;
            }

            if let Some(symbol_id) =
                resolve_nominal_symbol_from_type_expression(session, &ctx, expression_id)
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
    crate::with_query_context_for_file(session, file, |ctx| {
        let global_node_id = node_id.into_global(ctx.module_id);
        let types = ctx.types();
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

    crate::with_query_context_for_file(session, file, |ctx| {
        resolve_nominal_symbol_from_type_expression(session, &ctx, expr_id)
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
        let Some(ctx) = crate::query_context(session, &module) else {
            continue;
        };

        // resolve the next target symbol from symbol metadata or dependency items
        let symbols = ctx.symbols();
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

                let dir_tree = ctx.tree();
                let item = dir_tree.get::<DependencyItem>(item_id);
                target_symbol = item.target_symbol();
            }
        }

        // resolve dependency items to their target symbols when possible
        if target_symbol.is_none()
            && let Some(name_id) = symbol.name()
            && let Some(resolved_symbol) =
                resolve_type_symbol_from_dependency_symbol(session, &ctx, canonical_id, name_id)
        {
            target_symbol = Some(resolved_symbol);
        }

        // fall back to export table resolution when no target is recorded
        if target_symbol.is_none()
            && let Some(name_id) = symbol.name()
        {
            let exports = &ctx.resolved.exported_symbols;
            let dir_tree = ctx.tree();
            for ((space, key), export) in exports.iter() {
                let dir::StaticKey::Name(export_name) = *key else {
                    continue;
                };

                if export_name != name_id {
                    continue;
                }

                if !matches!(*space, SymbolSpace::Type | SymbolSpace::TypeValue) {
                    continue;
                }

                if let Some(target) = export.target.resolved() {
                    target_symbol = Some(target);
                    break;
                }

                if let Some(item_id) = export.item {
                    let item = dir_tree.get::<DependencyItem>(item_id);
                    if let Some(target) = item.target_symbol() {
                        target_symbol = Some(target);
                        break;
                    }
                }
            }
        }

        if target_symbol.is_none()
            && let Some(name_id) = symbol.name()
            && let Some(resolved_symbol) = resolve_type_symbol_from_module(
                session,
                ctx.module_id,
                name_id,
                &mut HashSet::new(),
            )
        {
            target_symbol = Some(resolved_symbol);
        }

        // continue walking when a dependency target exists
        if let Some(target_symbol) = target_symbol {
            pending.push(target_symbol);
        }
    }

    // return the collected canonical targets
    visited
}

/// Resolve target symbols by consulting goto_type_definition results.
fn fallback_target_symbols_from_type_definition(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<(HashSet<GlobalSymbolId>, GlobalSymbolId)> {
    // resolve the type definition location
    let result = super::goto_type_definition(session, file, offset)?;
    let location = result.locations.first()?;

    // resolve the symbol at the type definition
    let symbol_at = find_symbol_at_offset(session, location.file, location.start)?;
    let target_symbols = collect_target_symbols(session, symbol_at.symbol_id);

    // select an implementable target from the resolved set
    let canonical_id = target_symbols
        .iter()
        .copied()
        .find(|symbol_id| symbol_is_implementable(session, *symbol_id))?;

    Some((target_symbols, canonical_id))
}

/// Check whether a symbol is an interface or class.
fn symbol_is_implementable(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let Some(ctx) = crate::query_context(session, &module) else {
        return false;
    };

    // read the symbol type
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // return whether the symbol is implementable
    symbol.ty == SymbolType::Interface || symbol.ty == SymbolType::Class
}
