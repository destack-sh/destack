use std::sync::Arc;

use crate::{Arena, HeapOptions};

/// Create one shared arena for one explicit heap options set.
pub(crate) fn test_arena(options: &HeapOptions) -> Arc<Arena> {
    Arc::new(
        Arena::try_new(options.page_bytes, options.arena_segment_bytes)
            .expect("checked heap options should build one arena"),
    )
}
