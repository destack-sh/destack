use std::sync::Arc;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::SegmentView;

/// Cumulative layouts for one DIR module.
#[derive(Debug, Clone)]
pub struct LayoutTable<'a> {
    /// The module id of the layout table.
    pub module_id: ModuleId,
    /// The ordered layout table segments.
    segments: SegmentView<'a, LayoutSegment>,
}

impl LayoutTable<'static> {
    /// Create a layout table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<LayoutSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a layout table from one segment.
    pub fn from_segment(segment: Arc<LayoutSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> LayoutTable<'a> {
    /// Create a layout table from a segment view.
    pub fn from_view(segments: SegmentView<'a, LayoutSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("layout table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "layout table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a layout table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b LayoutSegment) -> LayoutTable<'b> {
        LayoutTable::from_view(self.segments.with_tail(tail))
    }

    /// Return whether this table has no layouts.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// Layouts added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSegment {
    /// The module id of the layout segment.
    pub module_id: ModuleId,
}

impl LayoutSegment {
    /// Create an empty layout segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self { module_id }
    }

    /// Return whether this segment has no layouts.
    pub fn is_empty(&self) -> bool {
        true
    }
}
