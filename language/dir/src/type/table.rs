use std::collections::HashMap;

use crate::{Arena, LocalNodeId, LocalSymbolId, ModuleId, Type};

/// A TypeTable is a side table for a node. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct TypeTable {
    /// The module id of the type table.
    pub module_id: ModuleId,

    // meta arenas
    pub(crate) types: Arena<Type>,

    // meta side data
    /// The declared type by symbol id. Index is the local symbol id.
    pub(crate) declared_type_by_symbol_id: HashMap<LocalSymbolId, LocalNodeId<Type>>,
    /// The inferred type by symbol id. Index is the local symbol id.
    pub(crate) inferred_type_by_symbol_id: HashMap<LocalSymbolId, LocalNodeId<Type>>,
}

impl TypeTable {
    /// Create a new TypeTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            types: Arena::new(),
            declared_type_by_symbol_id: HashMap::new(),
            inferred_type_by_symbol_id: HashMap::new(),
        }
    }
}
