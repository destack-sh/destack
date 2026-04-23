#![cfg(test)]

use std::sync::Arc;

use super::Allocator;
use crate::HeapOptions;

/// Create one allocator for one explicit heap options set.
pub(crate) fn test_allocator(options: &HeapOptions) -> Arc<Allocator> {
    Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
            .expect("checked heap options should build one allocator"),
    )
}
