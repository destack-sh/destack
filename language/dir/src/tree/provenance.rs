use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One DIR provenance record identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProvenanceId(pub u32);

impl ProvenanceId {
    /// Create one provenance id from one raw index.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the raw index for this provenance id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The reason one DIR provenance record was created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProvenanceReason {
    /// The node was synthesized during binding.
    Bound,
    /// The node was transformed during declaration normalization.
    Normalized,
    /// The node was transformed during elaborate.
    Elaborated,
    /// The node was synthesized from static evaluation.
    Evaluated,
    /// The node was reified during elaborate.
    Reified,
}

/// One DIR provenance record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// The primary source id for this DIR node.
    pub source_id: u32,
    /// The parent DIR provenance record when this node was derived from another DIR node.
    pub parent: Option<ProvenanceId>,
    /// The transform reason when there is one.
    pub reason: Option<ProvenanceReason>,
}

/// Provenance metadata for one DIR tree.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvenanceMetadata {
    /// The provenance id for each DIR node id.
    pub provenance_by_node_id: Vec<ProvenanceId>,
    /// Canonical provenance records.
    pub record_by_id: Vec<ProvenanceRecord>,
    /// Reverse index from source id to provenance records.
    pub record_by_source_id: BTreeMap<u32, Vec<ProvenanceId>>,
}

impl ProvenanceMetadata {
    /// Create one direct source-backed provenance record.
    pub fn create_source(&mut self, source_id: u32) -> ProvenanceId {
        self.create(source_id, None, None)
    }

    /// Create one derived provenance record from one parent provenance id.
    pub fn create_derived(
        &mut self,
        source_id: u32,
        parent: ProvenanceId,
        reason: Option<ProvenanceReason>,
    ) -> ProvenanceId {
        self.create(source_id, Some(parent), reason)
    }

    /// Return one provenance record by id.
    pub fn record(&self, provenance_id: ProvenanceId) -> ProvenanceRecord {
        self.record_by_id[provenance_id.index()]
    }

    /// Return the primary source id for one provenance record.
    pub fn source_id(&self, provenance_id: ProvenanceId) -> u32 {
        self.record(provenance_id).source_id
    }

    /// Create one provenance record.
    fn create(
        &mut self,
        source_id: u32,
        parent: Option<ProvenanceId>,
        reason: Option<ProvenanceReason>,
    ) -> ProvenanceId {
        let provenance_id = ProvenanceId::new(self.record_by_id.len() as u32);
        let record = ProvenanceRecord {
            source_id,
            parent,
            reason,
        };

        self.record_by_id.push(record);
        self.record_by_source_id
            .entry(source_id)
            .or_default()
            .push(provenance_id);

        provenance_id
    }
}
