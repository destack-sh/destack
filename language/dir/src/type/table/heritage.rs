use destack_source::AdaptImage;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, Lineage, LocalLineageId};

use super::TypeTable;

/// Heritage ownership for nominal relationships.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct HeritageTable {
    /// The next lineage id to allocate.
    pub(crate) next_lineage_id: u32,
    /// The lineages.
    pub(crate) lineages: crate::Arena<Lineage>,
    /// The lineage by symbol id (for type declarations: their resolved heritage).
    pub(crate) lineage_by_symbol_id: IndexMap<GlobalSymbolId, LocalLineageId>,
}

impl HeritageTable {
    /// Create an empty heritage table.
    pub fn new() -> Self {
        Self {
            next_lineage_id: 0,
            lineages: crate::Arena::new(),
            lineage_by_symbol_id: IndexMap::new(),
        }
    }
}

impl Default for HeritageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeTable {
    /// Insert a new lineage.
    pub fn insert_lineage(&mut self, lineage: Lineage) -> LocalLineageId {
        let lineage_id = LocalLineageId::new(self.heritage.next_lineage_id);
        self.heritage.next_lineage_id += 1;
        self.heritage.lineages.allocate(lineage);
        lineage_id
    }

    /// Get a lineage by its id.
    pub fn get_lineage(&self, lineage_id: LocalLineageId) -> &Lineage {
        self.heritage.lineages.get(lineage_id.0)
    }

    /// Get a mutable lineage by its id.
    pub fn get_lineage_mut(&mut self, lineage_id: LocalLineageId) -> &mut Lineage {
        self.heritage.lineages.get_mut(lineage_id.0)
    }

    /// Set the lineage for a symbol (type declaration).
    pub fn set_lineage_for_symbol(
        &mut self,
        symbol_id: GlobalSymbolId,
        lineage_id: LocalLineageId,
    ) {
        self.heritage
            .lineage_by_symbol_id
            .insert(symbol_id, lineage_id);
    }

    /// Get the lineage id for a symbol.
    pub fn get_lineage_id_for_symbol(&self, symbol_id: GlobalSymbolId) -> Option<LocalLineageId> {
        self.heritage.lineage_by_symbol_id.get(&symbol_id).copied()
    }

    /// Get the lineage for a symbol directly.
    pub fn get_lineage_for_symbol(&self, symbol_id: GlobalSymbolId) -> Option<&Lineage> {
        self.heritage
            .lineage_by_symbol_id
            .get(&symbol_id)
            .map(|id| self.heritage.lineages.get(id.0))
    }

    /// Iterate over all lineages with their associated symbol ids.
    pub fn iter_lineages(&self) -> impl Iterator<Item = (GlobalSymbolId, &Lineage)> {
        self.heritage
            .lineage_by_symbol_id
            .iter()
            .map(|(symbol_id, lineage_id)| (*symbol_id, self.heritage.lineages.get(lineage_id.0)))
    }
}
