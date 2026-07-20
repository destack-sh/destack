use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::TypeId;

/// One logical local storage slot in a bytecode frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameSlot {
    /// The object-local stored type.
    pub ty: TypeId,
}

const _: () = assert!(size_of::<FrameSlot>() == 4);
