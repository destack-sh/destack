use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Identifier for one provenance record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProvenanceId(u32);

impl ProvenanceId {
    /// Create a provenance id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The broad provenance class for a MIR node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceKind {
    /// The MIR node comes directly from one semantic DIR node.
    Direct,
    /// The MIR node was synthesized without a direct previous-layer node.
    Synthetic,
    /// The MIR node was derived from one existing MIR node.
    Derived,
    /// The MIR node merges multiple semantic or MIR origins.
    Merged,
    /// The MIR node was introduced by inlining.
    Inlined,
    /// The MIR node was created or rewritten by an optimization.
    Optimized,
}

/// The specific reason a MIR node was introduced or rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceReason {
    /// The MIR node was synthesized while lowering a runtime check.
    LoweredCheck,
    /// The MIR node was synthesized while lowering dynamic dispatch.
    LoweredDispatch,
    /// The MIR node was synthesized while lowering closures or function values.
    LoweredClosure,
    /// The MIR node was synthesized while lowering coroutines or async state.
    LoweredCoroutine,
}

/// Provenance for one MIR node.
///
/// Origins are previous-layer DIR node ids.
/// Parents point at earlier MIR provenance records when one MIR transform derives a new node
/// from one or more existing MIR nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The broad provenance class for this node.
    pub kind: ProvenanceKind,
    /// The specific cause for this node, if one is useful.
    pub reason: Option<ProvenanceReason>,
    /// The primary semantic origins in DIR.
    pub origins: Vec<u32>,
    /// Earlier MIR provenance records this node derives from.
    pub parents: Vec<ProvenanceId>,
}

impl Provenance {
    /// Create direct provenance from one DIR origin.
    pub fn direct(origin: u32) -> Self {
        Self {
            kind: ProvenanceKind::Direct,
            reason: None,
            origins: vec![origin],
            parents: Vec::new(),
        }
    }

    /// Return the preferred DIR origin for diagnostics and anchoring.
    pub fn primary_origin(&self) -> Option<u32> {
        self.origins.first().copied()
    }
}

/// Canonical provenance records for MIR nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvenanceTable {
    /// Provenance records indexed by id.
    pub records: Vec<Provenance>,
    /// Optional reverse index from DIR origin to MIR provenance records.
    pub records_by_origin: HashMap<u32, Vec<ProvenanceId>>,
}

impl ProvenanceTable {
    /// Create a new empty provenance table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new provenance record.
    pub fn create(
        &mut self,
        kind: ProvenanceKind,
        reason: Option<ProvenanceReason>,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        let id = ProvenanceId::new(self.records.len() as u32);

        self.records.push(Provenance {
            kind,
            reason,
            origins: origins.clone(),
            parents,
        });

        for origin in origins {
            self.records_by_origin.entry(origin).or_default().push(id);
        }

        id
    }

    /// Create direct provenance from one DIR origin.
    pub fn create_direct(&mut self, origin: u32) -> ProvenanceId {
        self.create(ProvenanceKind::Direct, None, vec![origin], Vec::new())
    }

    /// Get one provenance record by id.
    pub fn record(&self, id: ProvenanceId) -> &Provenance {
        &self.records[id.index()]
    }
}
