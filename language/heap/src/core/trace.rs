use destack_mir::{TraceId, TraceMap};

/// Heap trace maps indexed by TraceId.
#[derive(Debug, Clone, Copy)]
pub struct TraceView<'a> {
    /// Decoded trace maps indexed by TraceId.
    maps: &'a [TraceMap],
}

impl<'a> TraceView<'a> {
    /// Create one trace map view.
    pub const fn new(maps: &'a [TraceMap]) -> Self {
        Self { maps }
    }

    /// Borrow one decoded trace map.
    pub fn trace(self, id: TraceId) -> Option<&'a TraceMap> {
        self.maps.get(id.index())
    }

    /// Borrow all decoded trace maps.
    pub const fn maps(self) -> &'a [TraceMap] {
        self.maps
    }
}
