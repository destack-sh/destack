use serde::{Deserialize, Serialize};

use crate::{FrameStateId, TypeId};

/// One frame layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameLayoutId(pub u32);

/// One frame region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRegionId(pub u32);

/// One byte region inside one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameRegion {
    /// The region id.
    pub id: FrameRegionId,
    /// The byte offset from the frame base.
    pub offset: u32,
    /// The region byte length.
    pub byte_len: u32,
    /// The region byte alignment.
    pub alignment: u16,
    /// Whether this region stores one word.
    pub is_word: bool,
    /// The region value type.
    pub ty: TypeId,
}

/// Byte layout for one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameLayout {
    /// The layout id.
    pub id: FrameLayoutId,
    /// Regions in frame order.
    pub regions: Vec<FrameRegion>,
    /// Number of SSA value regions.
    pub value_count: u32,
    /// Number of local regions.
    pub local_count: u32,
    /// Callable environment region.
    pub environment_region: Option<FrameRegionId>,
    /// The frame byte length.
    pub byte_len: u32,
}

impl FrameLayout {
    /// Return one frame region by id.
    pub fn region(&self, id: FrameRegionId) -> Option<&FrameRegion> {
        self.regions.get(id.0 as usize)
    }

    /// Return the frame region id for one SSA value.
    pub fn value_region_id(&self, value: u32) -> Option<FrameRegionId> {
        self.value(value).map(|region| region.id)
    }

    /// Return the frame region id for one local.
    pub fn local_region_id(&self, local: u32) -> Option<FrameRegionId> {
        self.local(local).map(|region| region.id)
    }

    /// Return the value region at one SSA value index.
    pub fn value(&self, value: u32) -> Option<&FrameRegion> {
        if value < self.value_count {
            self.regions.get(value as usize)
        } else {
            None
        }
    }

    /// Return the local region at the given local index.
    pub fn local(&self, local: u32) -> Option<&FrameRegion> {
        if local >= self.local_count {
            return None;
        }

        let index = self.value_count + local;

        self.regions.get(index as usize)
    }

    /// Return the SSA value addressed by one region id.
    pub fn value_for_region(&self, id: FrameRegionId) -> Option<u32> {
        if id.0 < self.value_count {
            Some(id.0)
        } else {
            None
        }
    }

    /// Return the local addressed by one region id.
    pub fn local_for_region(&self, id: FrameRegionId) -> Option<u32> {
        if id.0 < self.value_count {
            return None;
        }

        let local_index = id.0 - self.value_count;
        if local_index < self.local_count {
            Some(local_index)
        } else {
            None
        }
    }

    /// Return whether one region id addresses the callable environment.
    pub fn is_environment_region(&self, id: FrameRegionId) -> bool {
        self.environment_region == Some(id)
    }

    /// Return the callable environment region when present.
    pub fn environment(&self) -> Option<&FrameRegion> {
        self.environment_region.and_then(|id| self.region(id))
    }

    /// Return all value regions.
    pub fn values(&self) -> &[FrameRegion] {
        &self.regions[..self.value_count as usize]
    }

    /// Return all local regions.
    pub fn locals(&self) -> &[FrameRegion] {
        let start = self.value_count as usize;
        let end = start + self.local_count as usize;

        &self.regions[start..end]
    }

    /// Return the frame region count.
    pub fn region_len(&self) -> usize {
        self.regions.len()
    }

    /// Return all frame region ids.
    pub fn region_ids(&self) -> impl Iterator<Item = FrameRegionId> {
        (0..self.region_len()).map(|index| FrameRegionId(index as u32))
    }
}

/// Captured frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// The frame layout.
    pub frame_layout: FrameLayoutId,
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}
