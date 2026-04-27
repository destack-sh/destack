use dashmap::DashMap;
use destack_artifact::ModuleGraph;

use crate::resolve::module::globals::{GlobalSymbolTable, GlobalSymbolTableCacheKey};

/// Ephemeral compiler indices derived from authoritative artifacts.
#[derive(Debug, Default)]
pub(crate) struct CompilerIndex {
    /// Cached global symbol tables keyed by roots and current graph snapshot.
    pub global_symbol_tables: DashMap<
        GlobalSymbolTableCacheKey,
        (
            std::sync::Arc<ModuleGraph>,
            std::sync::Arc<GlobalSymbolTable>,
        ),
    >,
}
