use destack_mir as mir;

use crate::{ControlTransfer, ResumePointId};

/// The identifier for one frame layout table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FrameLayoutId(pub u32);

/// One byte region in one function activation record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameRegion {
    /// The byte offset from the frame base.
    pub offset: u32,
    /// The byte width reserved for this region.
    pub byte_len: u32,
    /// The required byte alignment.
    pub alignment: u16,
    /// Whether this region stores a VM word directly.
    pub is_word: bool,
    /// The MIR type stored in this slot.
    pub ty: mir::LocalNodeId<mir::Type>,
}

/// The byte layout of one function activation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameLayout {
    /// The layout identifier.
    pub id: FrameLayoutId,
    /// The owning MIR function.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The byte regions for SSA values.
    pub values: Vec<FrameRegion>,
    /// The byte regions for mutable locals.
    pub locals: Vec<FrameRegion>,
    /// The callable environment region when present.
    pub environment: Option<FrameRegion>,
    /// The fixed activation record byte width.
    pub byte_len: u32,
}

impl FrameLayout {
    /// Return the value region at the given SSA value index.
    pub fn value(&self, value: mir::Value) -> Option<&FrameRegion> {
        self.values.get(value.0 as usize)
    }

    /// Return the local region at the given local index.
    pub fn local(&self, local: mir::LocalNodeId<mir::Local>) -> Option<&FrameRegion> {
        self.locals.get(local.id as usize)
    }

    /// Return a region by materialization index.
    pub fn materialized_region(&self, index: u32) -> Option<&FrameRegion> {
        let value_count = self.values.len() as u32;
        if index < value_count {
            return self.values.get(index as usize);
        }

        let local_index = index.checked_sub(value_count)?;
        let local_count = self.locals.len() as u32;
        if local_index < local_count {
            return self.locals.get(local_index as usize);
        }

        if local_index == local_count {
            return self.environment.as_ref();
        }

        None
    }

    /// Return whether the materialization index addresses an SSA value.
    pub fn materialized_value(&self, index: u32) -> Option<mir::Value> {
        if index < self.values.len() as u32 {
            Some(mir::Value::new(index))
        } else {
            None
        }
    }

    /// Return whether the materialization index addresses a local.
    pub fn materialized_local(&self, index: u32) -> Option<mir::LocalNodeId<mir::Local>> {
        let value_count = self.values.len() as u32;
        let local_index = index.checked_sub(value_count)?;
        if local_index < self.locals.len() as u32 {
            Some(mir::LocalNodeId::new(local_index))
        } else {
            None
        }
    }

    /// Return whether the materialization index addresses the environment.
    pub fn is_materialized_environment(&self, index: u32) -> bool {
        let environment_index = self.values.len() + self.locals.len();

        self.environment.is_some() && index as usize == environment_index
    }

    /// Return the materialized region count.
    pub fn materialized_len(&self) -> usize {
        self.values.len() + self.locals.len() + usize::from(self.environment.is_some())
    }
}

/// One durable logical frame image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameImage {
    /// The frame layout used by this activation.
    pub frame_layout: FrameLayoutId,
    /// The current resume point for this activation.
    pub resume_point: ResumePointId,
    /// The pending transfer owned by this frame when another frame is active.
    pub transfer: Option<ControlTransfer>,
    /// The captured activation record bytes.
    pub bytes: Vec<u8>,
}
