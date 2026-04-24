use serde::{Deserialize, Serialize};

use destack_core::ImmutableStringPool;
use destack_mir as mir;

use crate::isolate::GlobalStorage;
use crate::options::IsolateOptions;
use crate::snapshot::InterpreterImage;
use destack_engine::IsolateId;

/// Immutable isolate image for one isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolateImage {
    /// The MIR tree used to rebuild the isolate module.
    pub tree: mir::NodeTree,
    /// The string pool used to rebuild the isolate module.
    pub strings: ImmutableStringPool,
    /// The isolate configuration options.
    pub options: IsolateOptions,
    /// The isolate id captured in this image.
    pub isolate_id: IsolateId,
    /// The captured global storage.
    pub globals: GlobalStorage,
    /// The captured interpreter state.
    pub interpreter: InterpreterImage,
}
