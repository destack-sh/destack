use destack_base::ImmutableStringPool;
use destack_mir as mir;
use serde::{Deserialize, Serialize};

use crate::isolate::GlobalStorage;
use crate::options::IsolateOptions;
use crate::snapshot::{InterpreterImage, StringInternerImage};

/// Immutable isolate image for one isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolateImage {
    /// The MIR tree executed by this isolate.
    pub tree: mir::NodeTree,
    /// The immutable string pool for this isolate.
    pub strings: ImmutableStringPool,
    /// The isolate configuration options.
    pub options: IsolateOptions,
    /// The isolate id captured in this image.
    pub isolate_id: u64,
    /// The captured string interner state.
    pub string_interner: StringInternerImage,
    /// The captured global storage.
    pub globals: GlobalStorage,
    /// The captured interpreter state.
    pub interpreter: InterpreterImage,
}

/// Serialized isolate snapshot for one isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolateSnapshot {
    /// The captured isolate image.
    pub image: IsolateImage,
}
