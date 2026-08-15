use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalNodeIdAny, LocalSymbolId};

/// One symbol's uses inside a checked module.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BindingUse(u8);

impl BindingUse {
    /// The symbol is read after initialization.
    pub const READ: Self = Self(1 << 0);
    /// The symbol is written after initialization.
    pub const WRITTEN: Self = Self(1 << 1);
    /// The symbol is used through a closure.
    pub const CAPTURED: Self = Self(1 << 2);
    /// The binding storage requires mutable or exclusive access.
    pub const MUTABLE: Self = Self(1 << 3);

    /// Return whether every bit of `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Return whether no use is recorded.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOrAssign for BindingUse {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Flow conclusions for one checked DIR module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FlowSegment {
    /// The module id of the flow segment.
    pub module_id: ModuleId,
    /// Nodes flow proves unreachable, sorted.
    unreachable: Vec<LocalNodeIdAny>,
    /// Nodes flow proves never return, sorted.
    diverging: Vec<LocalNodeIdAny>,
    /// Uses of each module symbol, sorted by symbol.
    uses: Vec<(LocalSymbolId, BindingUse)>,
    /// Uses of the foreign symbols this module touches, sorted by symbol.
    foreign_uses: Vec<(GlobalSymbolId, BindingUse)>,
    /// The node each module symbol use occurred at, sorted by node.
    occurrences: Vec<BindingOccurrence>,
}

/// One symbol use at its occurrence node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BindingOccurrence {
    /// The occurrence node.
    pub node: LocalNodeIdAny,
    /// The used symbol.
    pub symbol: LocalSymbolId,
    /// The recorded uses.
    pub uses: BindingUse,
}

impl FlowSegment {
    /// Create an empty flow segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            unreachable: Vec::new(),
            diverging: Vec::new(),
            uses: Vec::new(),
            foreign_uses: Vec::new(),
            occurrences: Vec::new(),
        }
    }

    /// Create a new empty segment after an existing flow segment.
    pub fn from_base(base: &Self) -> Self {
        Self::new(base.module_id)
    }

    /// Mark one node unreachable.
    pub fn mark_unreachable(&mut self, node: LocalNodeIdAny) {
        if let Err(index) = self.unreachable.binary_search(&node) {
            self.unreachable.insert(index, node);
        }
    }

    /// Mark one node diverging.
    pub fn mark_diverging(&mut self, node: LocalNodeIdAny) {
        if let Err(index) = self.diverging.binary_search(&node) {
            self.diverging.insert(index, node);
        }
    }

    /// Record one use of a module symbol.
    pub fn record_use(&mut self, symbol: LocalSymbolId, binding_use: BindingUse) {
        match self.uses.binary_search_by_key(&symbol, |(key, _)| *key) {
            Ok(index) => self.uses[index].1 |= binding_use,
            Err(index) => self.uses.insert(index, (symbol, binding_use)),
        }
    }

    /// Record one use of a foreign symbol.
    pub fn record_foreign_use(&mut self, symbol: GlobalSymbolId, binding_use: BindingUse) {
        match self
            .foreign_uses
            .binary_search_by_key(&symbol, |(key, _)| *key)
        {
            Ok(index) => self.foreign_uses[index].1 |= binding_use,
            Err(index) => self.foreign_uses.insert(index, (symbol, binding_use)),
        }
    }

    /// Record one symbol use at its occurrence node.
    pub fn record_occurrence(
        &mut self,
        node: LocalNodeIdAny,
        symbol: LocalSymbolId,
        binding_use: BindingUse,
    ) {
        let key = (node, symbol);
        match self
            .occurrences
            .binary_search_by_key(&key, |occurrence| (occurrence.node, occurrence.symbol))
        {
            Ok(index) => self.occurrences[index].uses |= binding_use,
            Err(index) => self.occurrences.insert(
                index,
                BindingOccurrence {
                    node,
                    symbol,
                    uses: binding_use,
                },
            ),
        }
    }

    /// Iterate the recorded symbol uses by occurrence node.
    pub fn occurrences(&self) -> impl Iterator<Item = BindingOccurrence> + '_ {
        self.occurrences.iter().copied()
    }

    /// Return whether flow proves one node unreachable.
    pub fn is_unreachable(&self, node: LocalNodeIdAny) -> bool {
        self.unreachable.binary_search(&node).is_ok()
    }

    /// Return whether flow proves one node never returns.
    pub fn is_diverging(&self, node: LocalNodeIdAny) -> bool {
        self.diverging.binary_search(&node).is_ok()
    }

    /// Return the recorded uses of one module symbol.
    pub fn use_of(&self, symbol: LocalSymbolId) -> BindingUse {
        self.uses
            .binary_search_by_key(&symbol, |(key, _)| *key)
            .map(|index| self.uses[index].1)
            .unwrap_or_default()
    }

    /// Iterate the recorded module symbol uses.
    pub fn uses(&self) -> impl Iterator<Item = (LocalSymbolId, BindingUse)> + '_ {
        self.uses.iter().copied()
    }

    /// Iterate the recorded foreign symbol uses.
    pub fn foreign_uses(&self) -> impl Iterator<Item = (GlobalSymbolId, BindingUse)> + '_ {
        self.foreign_uses.iter().copied()
    }

    /// Iterate the nodes flow proves unreachable.
    pub fn unreachable_nodes(&self) -> impl Iterator<Item = LocalNodeIdAny> + '_ {
        self.unreachable.iter().copied()
    }

    /// Iterate the nodes flow proves never return.
    pub fn diverging_nodes(&self) -> impl Iterator<Item = LocalNodeIdAny> + '_ {
        self.diverging.iter().copied()
    }
}
