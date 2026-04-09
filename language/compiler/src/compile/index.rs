use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::ModuleGraph;
use destack_source::ProfileId;

use crate::analyze::interface::graph::InterfaceComponentGraphIndex;
use crate::resolve::module::globals::{GlobalSymbolTable, GlobalSymbolTableCacheKey};

/// Ephemeral compiler indices derived from authoritative artifacts.
#[derive(Debug, Default)]
pub(crate) struct CompilerIndex {
    /// Cached interface component graph indices keyed by the current graph snapshot.
    pub interface_component_graph_indices:
        DashMap<ProfileId, (Arc<ModuleGraph>, Arc<InterfaceComponentGraphIndex>)>,
    /// Cached global symbol tables keyed by roots and current graph snapshot.
    pub global_symbol_tables:
        DashMap<GlobalSymbolTableCacheKey, (Arc<ModuleGraph>, Arc<GlobalSymbolTable>)>,
}
