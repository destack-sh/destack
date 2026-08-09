use destack_repository::Revision;
use destack_serde::Reflect;
use destack_source::Edit;
use serde::{Deserialize, Serialize};

/// One requested source mutation batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SourceUpdate {
    /// Optional expected base revision.
    pub base: Option<Revision>,
    /// Source file edits.
    pub edits: Vec<Edit>,
}
