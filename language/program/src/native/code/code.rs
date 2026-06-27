use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_mir::TargetLayout;
use destack_source::ContentId;

use crate::native::NATIVE_ABI_VERSION;

use super::{CodeMap, EntryTable, Image, ImportTable};

/// Durable native code produced for one program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Code {
    /// Destack native ABI version required by this code.
    pub abi_version: u32,
    /// Target triple or equivalent target identity.
    pub target: String,
    /// Target ABI layout expected by this code.
    pub target_layout: TargetLayout,
    /// The native image.
    pub image: Image,
    /// Native imports required by this code.
    pub imports: ImportTable,
    /// Native code map for safepoints and deoptimization.
    pub map: CodeMap,
    /// Native entries keyed by program ids.
    pub entries: EntryTable,
}

impl Code {
    /// Create one native code payload.
    pub fn new(
        target: String,
        target_layout: TargetLayout,
        image: Image,
        imports: ImportTable,
        map: CodeMap,
        entries: EntryTable,
    ) -> Self {
        Self {
            abi_version: NATIVE_ABI_VERSION,
            target,
            target_layout,
            image,
            imports,
            map,
            entries,
        }
    }

    /// Return all content ids referenced by this native code.
    pub fn content_ids(&self) -> Vec<ContentId> {
        self.image.content_ids()
    }
}
