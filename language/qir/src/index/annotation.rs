use destack_dir::GlobalNodeIdAny;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

/// Searchable annotation and decorator index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationIndex {
    /// The annotation entries in stable order.
    entries: Vec<AnnotationEntry>,
}

impl AnnotationIndex {
    /// Create an annotation index from entries.
    pub fn new(entries: Vec<AnnotationEntry>) -> Self {
        let mut index = Self { entries };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.entries
            .sort_by(|left, right| annotation_entry_key(left).cmp(&annotation_entry_key(right)));
        self.entries.dedup();
    }

    /// Return entries matching one optional annotation name.
    pub fn search(&self, name: Option<&str>) -> Vec<AnnotationEntry> {
        self.entries
            .iter()
            .filter(|entry| name.is_none_or(|name| entry.name.as_deref() == Some(name)))
            .cloned()
            .collect()
    }

    /// Return all indexed annotation entries.
    pub fn entries(&self) -> &[AnnotationEntry] {
        &self.entries
    }
}

/// Searchable annotation or decorator entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationEntry {
    /// The annotation name when syntactically known.
    pub name: Option<String>,
    /// The module that owns the annotation.
    pub module_id: ModuleId,
    /// The decorator node.
    pub decorator_id: GlobalNodeIdAny,
    /// The annotated target node.
    pub target_id: GlobalNodeIdAny,
}

/// Return the stable ordering key for one annotation entry.
fn annotation_entry_key(entry: &AnnotationEntry) -> (Option<&str>, ModuleId, u32, u32) {
    (
        entry.name.as_deref(),
        entry.module_id,
        entry.target_id.local_id.id,
        entry.decorator_id.local_id.id,
    )
}
