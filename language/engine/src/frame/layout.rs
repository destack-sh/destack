use crate::{ControlTransfer, FunctionId, LocalId, ResumePointId, TypeId, ValueId};

/// The identifier for one frame layout table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FrameLayoutId(pub u32);

/// The identifier for one byte region inside a frame layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FrameRegionId(pub u32);

/// One byte region inside one frame.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameRegion {
    /// The region identifier.
    pub id: FrameRegionId,
    /// The byte offset from the frame base.
    pub offset: u32,
    /// The byte width reserved for this region.
    pub byte_len: u32,
    /// The required byte alignment.
    pub alignment: u16,
    /// Whether this region stores one machine word directly.
    pub is_word: bool,
    /// The value type stored in this region.
    pub ty: TypeId,
}

/// The byte layout of one frame.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameLayout {
    /// The layout identifier.
    pub id: FrameLayoutId,
    /// The owning function.
    pub function: FunctionId,
    /// The byte regions for SSA values.
    pub values: Vec<FrameRegion>,
    /// The byte regions for mutable locals.
    pub locals: Vec<FrameRegion>,
    /// The callable environment region when present.
    pub environment: Option<FrameRegion>,
    /// The fixed frame byte width.
    pub byte_len: u32,
}

impl FrameLayout {
    /// Return one frame region by id.
    pub fn region(&self, id: FrameRegionId) -> Option<&FrameRegion> {
        let index = id.0 as usize;
        let value_count = self.values.len();
        if index < value_count {
            return self.values.get(index);
        }

        let local_index = index.checked_sub(value_count)?;
        let local_count = self.locals.len();
        if local_index < local_count {
            return self.locals.get(local_index);
        }

        if local_index == local_count {
            return self.environment.as_ref();
        }

        None
    }

    /// Return the frame region id for one SSA value.
    pub fn value_region_id(&self, value: ValueId) -> Option<FrameRegionId> {
        self.value(value).map(|region| region.id)
    }

    /// Return the frame region id for one local.
    pub fn local_region_id(&self, local: LocalId) -> Option<FrameRegionId> {
        self.local(local).map(|region| region.id)
    }

    /// Return the value region at the given SSA value index.
    pub fn value(&self, value: ValueId) -> Option<&FrameRegion> {
        self.values.get(value.0 as usize)
    }

    /// Return the value region at the given SSA value index.
    pub fn value_index(&self, value: u32) -> Option<&FrameRegion> {
        self.value(ValueId(value))
    }

    /// Return the local region at the given local index.
    pub fn local(&self, local: LocalId) -> Option<&FrameRegion> {
        self.locals.get(local.0 as usize)
    }

    /// Return the local region at the given local index.
    pub fn local_index(&self, local: u32) -> Option<&FrameRegion> {
        self.local(LocalId(local))
    }

    /// Return the SSA value addressed by one region id.
    pub fn value_for_region(&self, id: FrameRegionId) -> Option<ValueId> {
        if id.0 < self.values.len() as u32 {
            Some(ValueId(id.0))
        } else {
            None
        }
    }

    /// Return the local addressed by one region id.
    pub fn local_for_region(&self, id: FrameRegionId) -> Option<LocalId> {
        let value_count = self.values.len() as u32;
        let local_index = id.0.checked_sub(value_count)?;
        if local_index < self.locals.len() as u32 {
            Some(LocalId(local_index))
        } else {
            None
        }
    }

    /// Return whether one region id addresses the callable environment.
    pub fn is_environment_region(&self, id: FrameRegionId) -> bool {
        let environment_index = self.values.len() + self.locals.len();

        self.environment.is_some() && id.0 as usize == environment_index
    }

    /// Return the frame region count.
    pub fn region_len(&self) -> usize {
        self.values.len() + self.locals.len() + usize::from(self.environment.is_some())
    }

    /// Return all frame region ids in layout order.
    pub fn region_ids(&self) -> impl Iterator<Item = FrameRegionId> {
        (0..self.region_len()).map(|index| FrameRegionId(index as u32))
    }
}

/// One durable logical frame image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FrameImage {
    /// The frame layout used by this frame.
    pub frame_layout: FrameLayoutId,
    /// The current resume point for this frame.
    pub resume_point: ResumePointId,
    /// The pending transfer owned by this frame when another frame is active.
    pub transfer: Option<ControlTransfer>,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}
