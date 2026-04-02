use destack_core::ImmutableStringPool;
use destack_mir as mir;
use serde::{Deserialize, Serialize};
use std::mem::size_of;

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

impl IsolateImage {
    /// Return the owned bytes for this durable isolate image.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.tree.owned_bytes() - size_of::<mir::NodeTree>();
        owned_bytes += self.strings.owned_bytes() - size_of::<ImmutableStringPool>();
        owned_bytes += self.string_interner.owned_bytes() - size_of::<StringInternerImage>();
        owned_bytes += self.globals.owned_bytes() - size_of::<GlobalStorage>();
        owned_bytes += self.interpreter.owned_bytes() - size_of::<InterpreterImage>();

        owned_bytes
    }
}

/// Serialized isolate snapshot for one isolate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolateSnapshot {
    /// The captured isolate image.
    pub image: IsolateImage,
}
