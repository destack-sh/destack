use std::collections::{HashMap, HashSet};

use destack_source::{FileId, ModuleId, ProfileId, Span};
use serde::{Deserialize, Serialize};

/// One typed provenance record identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProvenanceId(
    /// The raw index into the provenance table.
    u32,
);

impl ProvenanceId {
    /// Create a provenance id from one raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the raw index as one vector index.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// One typed AST node key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AstNodeKey {
    /// The file containing the AST node.
    pub file_id: FileId,
    /// The raw AST node id within the file.
    pub node_id: u32,
}

/// One typed DIR node key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DirNodeKey {
    /// The source module when it is known.
    pub module_id: Option<ModuleId>,
    /// The raw DIR node id within the source module.
    pub node_id: u32,
    /// The semantic profile when it is known.
    pub profile_id: Option<ProfileId>,
}

impl DirNodeKey {
    /// Create one DIR node key from one local source node id.
    pub fn local(node_id: u32) -> Self {
        Self {
            module_id: None,
            node_id,
            profile_id: None,
        }
    }
}

/// The primary diagnostic or display anchor for one MIR provenance record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceAnchor {
    /// One primary AST node anchor.
    Ast(AstNodeKey),
    /// One primary DIR node anchor.
    Dir(DirNodeKey),
    /// One primary MIR origin anchor.
    Mir(ProvenanceId),
    /// One primary text anchor.
    Text(FileId),
    /// One synthetic anchor with no direct upstream source.
    Synthetic,
}

/// One contributing provenance key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceKey {
    /// One contributing AST node.
    Ast(AstNodeKey),
    /// One contributing DIR node.
    Dir(DirNodeKey),
    /// One contributing MIR origin record.
    Mir(ProvenanceId),
    /// One contributing text source.
    Text(FileId),
}

/// The reason one MIR provenance record was created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceReason {
    /// The node was parsed directly from MIR text.
    Parsed,
    /// The node was lowered from upstream IR.
    Lowered,
    /// The node was synthesized by merging multiple sources.
    Merged,
    /// The node was synthesized by inlining.
    Inlined,
    /// The node was synthesized by canonicalization.
    Canonicalized,
    /// The node was synthesized by optimization.
    Optimized,
}

/// One typed MIR provenance record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// The primary diagnostic or display anchor.
    pub anchor: ProvenanceAnchor,
    /// The primary source span when one exists.
    pub span: Option<Span>,
    /// Additional contributing provenance keys.
    pub contributors: Vec<ProvenanceKey>,
    /// The transform or lowering reason when there is one.
    pub reason: Option<ProvenanceReason>,
}

impl ProvenanceRecord {
    /// Return the first direct DIR source id when one exists.
    pub fn primary_dir_source_id(&self) -> Option<u32> {
        match self.anchor {
            ProvenanceAnchor::Dir(key) => Some(key.node_id),
            _ => self.contributors.iter().find_map(|input| match input {
                ProvenanceKey::Dir(key) => Some(key.node_id),
                _ => None,
            }),
        }
    }
}

/// Provenance and source-tracking metadata for MIR nodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provenance {
    /// Maps MIR node id to one typed provenance record.
    pub provenance_by_node_id: Vec<Option<ProvenanceId>>,
    /// Canonical typed provenance records.
    pub record_by_id: Vec<ProvenanceRecord>,
    /// Reverse index from AST node to provenance records.
    pub record_by_ast: HashMap<AstNodeKey, Vec<ProvenanceId>>,
    /// Reverse index from DIR node to provenance records.
    pub record_by_dir: HashMap<DirNodeKey, Vec<ProvenanceId>>,
}

impl Provenance {
    /// Create a new empty provenance table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create one typed provenance record.
    pub fn create(
        &mut self,
        anchor: ProvenanceAnchor,
        span: Option<Span>,
        contributors: Vec<ProvenanceKey>,
        reason: Option<ProvenanceReason>,
    ) -> ProvenanceId {
        let provenance_id = ProvenanceId::new(self.record_by_id.len() as u32);
        let record = ProvenanceRecord {
            anchor,
            span,
            contributors,
            reason,
        };

        self.index_record(provenance_id, &record);
        self.record_by_id.push(record);
        provenance_id
    }

    /// Return true when one provenance id exists.
    pub fn contains(&self, provenance_id: ProvenanceId) -> bool {
        provenance_id.index() < self.record_by_id.len()
    }

    /// Return one provenance record by id.
    pub fn record(&self, provenance_id: ProvenanceId) -> &ProvenanceRecord {
        &self.record_by_id[provenance_id.index()]
    }

    /// Return the primary source span for one provenance record when present.
    pub fn span(&self, provenance_id: ProvenanceId) -> Option<Span> {
        self.record(provenance_id).span
    }

    /// Record the primary source span for one provenance record.
    pub fn set_span(&mut self, provenance_id: ProvenanceId, span: Span) {
        let record = &mut self.record_by_id[provenance_id.index()];
        record.span = Some(span);

        if let ProvenanceAnchor::Text(file_id) = &mut record.anchor {
            *file_id = span.file;
        }
    }

    /// Create one direct DIR-local provenance record.
    pub fn direct_dir_local(&mut self, node_id: u32) -> ProvenanceId {
        let key = DirNodeKey::local(node_id);

        self.create(
            ProvenanceAnchor::Dir(key),
            None,
            vec![ProvenanceKey::Dir(key)],
            Some(ProvenanceReason::Lowered),
        )
    }

    /// Create one direct MIR text provenance record.
    pub fn text(&mut self, span: Span) -> ProvenanceId {
        self.create(
            ProvenanceAnchor::Text(span.file),
            Some(span),
            vec![ProvenanceKey::Text(span.file)],
            Some(ProvenanceReason::Parsed),
        )
    }

    /// Create one synthetic provenance record.
    pub fn synthetic(
        &mut self,
        reason: Option<ProvenanceReason>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        let contributors = parents
            .into_iter()
            .map(ProvenanceKey::Mir)
            .collect::<Vec<_>>();
        let anchor = anchor_from_contributors(&contributors).unwrap_or(ProvenanceAnchor::Synthetic);

        self.create(anchor, None, contributors, reason)
    }

    /// Create one derived provenance record from DIR sources and MIR parents.
    pub fn derived(
        &mut self,
        reason: Option<ProvenanceReason>,
        origins: Vec<u32>,
        parents: Vec<ProvenanceId>,
    ) -> ProvenanceId {
        let mut contributors = origins
            .into_iter()
            .map(DirNodeKey::local)
            .map(ProvenanceKey::Dir)
            .collect::<Vec<_>>();
        contributors.extend(parents.into_iter().map(ProvenanceKey::Mir));

        let anchor = anchor_from_contributors(&contributors).unwrap_or(ProvenanceAnchor::Synthetic);

        self.create(anchor, None, contributors, reason)
    }

    /// Create one merged provenance record.
    pub fn merged(&mut self, origins: Vec<u32>, parents: Vec<ProvenanceId>) -> ProvenanceId {
        self.derived(Some(ProvenanceReason::Merged), origins, parents)
    }

    /// Create one inlined provenance record.
    pub fn inlined(&mut self, origins: Vec<u32>, parents: Vec<ProvenanceId>) -> ProvenanceId {
        self.derived(Some(ProvenanceReason::Inlined), origins, parents)
    }

    /// Create one optimized provenance record.
    pub fn optimized(&mut self, origins: Vec<u32>, parents: Vec<ProvenanceId>) -> ProvenanceId {
        self.derived(Some(ProvenanceReason::Optimized), origins, parents)
    }

    /// Index one new record in the reverse provenance tables.
    fn index_record(&mut self, provenance_id: ProvenanceId, record: &ProvenanceRecord) {
        let mut seen_ast = HashSet::new();
        let mut seen_dir = HashSet::new();

        if let ProvenanceAnchor::Ast(key) = record.anchor
            && seen_ast.insert(key)
        {
            self.record_by_ast
                .entry(key)
                .or_default()
                .push(provenance_id);
        }

        if let ProvenanceAnchor::Dir(key) = record.anchor
            && seen_dir.insert(key)
        {
            self.record_by_dir
                .entry(key)
                .or_default()
                .push(provenance_id);
        }

        for input in &record.contributors {
            match *input {
                ProvenanceKey::Ast(key) => {
                    if seen_ast.insert(key) {
                        self.record_by_ast
                            .entry(key)
                            .or_default()
                            .push(provenance_id);
                    }
                }
                ProvenanceKey::Dir(key) => {
                    if seen_dir.insert(key) {
                        self.record_by_dir
                            .entry(key)
                            .or_default()
                            .push(provenance_id);
                    }
                }
                ProvenanceKey::Mir(_) | ProvenanceKey::Text(_) => {}
            }
        }
    }
}

/// Choose one primary anchor from one contributor list when possible.
fn anchor_from_contributors(inputs: &[ProvenanceKey]) -> Option<ProvenanceAnchor> {
    inputs.first().copied().map(|input| match input {
        ProvenanceKey::Ast(key) => ProvenanceAnchor::Ast(key),
        ProvenanceKey::Dir(key) => ProvenanceAnchor::Dir(key),
        ProvenanceKey::Mir(provenance_id) => ProvenanceAnchor::Mir(provenance_id),
        ProvenanceKey::Text(file_id) => ProvenanceAnchor::Text(file_id),
    })
}
