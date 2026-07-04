use std::collections::HashSet;

use destack_dir as dir;
use destack_dir::HeritageKind;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::navigation::{NavigationRelation, NavigationTarget};
use crate::{ModuleQueryContext, Position, ProgramQueryContext};

/// Request goto implementation at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoImplementationRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for goto implementation queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct GotoImplementationResponse {
    /// Implementation targets.
    pub targets: Vec<NavigationTarget>,
}

impl ModuleQueryContext<'_> {
    /// Return whether a symbol can have implementation targets.
    fn symbol_can_have_implementations(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        let symbol_module = self.module_context(symbol_id.module_id);

        let symbols = symbol_module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol.kind == dir::SymbolKind::Interface || symbol.kind == dir::SymbolKind::Class
    }

    /// Find implementations of the symbol at the given position.
    ///
    /// For interfaces: finds implementing structs/classes.
    /// For classes: finds subclasses.
    pub fn goto_implementation(
        &self,
        program: &ProgramQueryContext<'_>,
        offset: u32,
    ) -> Vec<NavigationTarget> {
        let Some(target_symbol_id) = self.implementation_target_symbol_at_offset(offset) else {
            return Vec::new();
        };

        // collect canonical targets reachable from the cursor symbol
        let target_symbols = self.collect_target_symbols(target_symbol_id);

        // select an implementable symbol from the target set
        let canonical_id = target_symbols
            .iter()
            .copied()
            .find(|symbol_id| self.symbol_can_have_implementations(*symbol_id));

        let Some(canonical_id) = canonical_id else {
            return Vec::new();
        };

        // resolve the target symbol type information
        let target_module = self.module_context(canonical_id.module_id);

        // resolve the target symbol metadata
        let (is_interface, is_class) = {
            let symbols = target_module.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);
            (
                symbol.kind == dir::SymbolKind::Interface,
                symbol.kind == dir::SymbolKind::Class,
            )
        };

        // bail out for symbols that cannot be implemented
        if !is_interface && !is_class {
            return Vec::new();
        }

        // initialize the result targets
        let mut targets = Vec::new();

        // match cached direct edges against the target symbol set
        for target_symbol in target_symbols {
            let entries = program.base_heritage(target_symbol);

            for entry in entries {
                let matches = if is_interface {
                    entry.kind == HeritageKind::Implements
                } else {
                    entry.kind == HeritageKind::Extends
                };

                if matches {
                    let derived_module = self.module_context(entry.derived.module_id);
                    let Some(span) = derived_module.symbol_definition_span(entry.derived) else {
                        continue;
                    };
                    let target = NavigationTarget::span(
                        &derived_module,
                        span,
                        NavigationRelation::Implementation,
                    )
                    .with_symbol(entry.derived);
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

    /// Resolve the symbol that should drive an implementation query at one offset.
    fn implementation_target_symbol_at_offset(&self, offset: u32) -> Option<dir::GlobalSymbolId> {
        // prefer the symbol directly under the cursor
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return self.resolve_type_symbol_at_offset(offset);
        };
        if self.symbol_can_have_implementations(symbol_at.symbol_id) {
            return Some(symbol_at.symbol_id);
        }

        // prefer type symbols when the cursor is on a type annotation
        self.resolve_type_symbol_from_node(symbol_at.node_id)
            .or_else(|| self.resolve_type_symbol_from_expression_node(symbol_at.node_id))
            .or_else(|| self.resolve_type_symbol_at_offset(offset))
            .or(Some(symbol_at.symbol_id))
    }

    /// Resolve a nominal type symbol from a node's checked type.
    fn resolve_type_symbol_from_node(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let global_node_id = node_id.into_global(self.module_id());
        let types = self.types();
        let type_id = types.get_node_type_id(global_node_id)?;

        self.read_global_type(type_id, |ty, _| ty.symbol())
    }

    /// Resolve a type symbol from an expression node when available.
    fn resolve_type_symbol_from_expression_node(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        if node_id.ty != dir::NodeType::Expression {
            return None;
        }

        let Ok(expr_id) = node_id.try_into_typed() else {
            return None;
        };

        self.resolve_nominal_symbol_from_type_expression(expr_id)
    }

    /// Resolve a type symbol at the given offset when the cursor is on a type annotation.
    fn resolve_type_symbol_at_offset(&self, offset: u32) -> Option<dir::GlobalSymbolId> {
        // scan expression nodes to find a type reference under the cursor
        let view = self.view();
        for (expression_id, _expression) in view.iter_nodes_of_type::<dir::Expression>() {
            let span = self.get_main_span(view, expression_id.into());

            if offset < span.start || offset > span.end {
                continue;
            }

            if let Some(symbol_id) = self.resolve_nominal_symbol_from_type_expression(expression_id)
            {
                return Some(symbol_id);
            }
        }

        None
    }

    /// Collect canonical symbols reachable from a target.
    fn collect_target_symbols(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> HashSet<dir::GlobalSymbolId> {
        // seed the search with the initial symbol
        let mut pending = vec![symbol_id];
        let mut visited = HashSet::new();

        // walk canonical and dependency chains
        while let Some(current) = pending.pop() {
            // canonicalize the current symbol
            let canonical_id = self.canonical_symbol(current);
            if !visited.insert(canonical_id) {
                continue;
            }

            // resolve the module for the canonical symbol
            let canonical_module = self.module_context(canonical_id.module_id);

            // resolve the next target symbol from symbol metadata or dependency items
            let symbols = canonical_module.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);

            let target_symbol = symbol.declaration.and_then(|declaration| {
                if declaration.local_id.ty != dir::NodeType::DependencyItem {
                    return None;
                }

                let item_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "dependency declaration has incompatible node id: {:?}",
                        declaration.local_id
                    )
                });
                canonical_module.dependency_symbol_target(item_id)
            });

            // continue walking when a dependency target exists
            if let Some(target_symbol) = target_symbol {
                pending.push(target_symbol);
            }
        }

        // return the collected canonical targets
        visited
    }
}
