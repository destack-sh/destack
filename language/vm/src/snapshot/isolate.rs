use serde::{Deserialize, Serialize};

use destack_core::ImmutableStringPool;
use destack_mir as mir;

use crate::options::IsolateOptions;
use crate::snapshot::InterpreterImage;
use destack_engine::EngineId;

/// Immutable isolate image for one isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolateImage {
    /// The MIR tree used to rebuild the isolate program.
    pub tree: mir::NodeTree,
    /// The string pool used to rebuild the isolate program.
    pub strings: ImmutableStringPool,
    /// The isolate configuration options.
    pub options: IsolateOptions,
    /// The isolate id captured in this image.
    pub isolate_id: EngineId,
    /// The captured interpreter state.
    pub interpreter: InterpreterImage,
}
