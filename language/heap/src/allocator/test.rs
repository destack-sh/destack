#![cfg(test)]

use std::sync::Arc;

use super::Allocator;
use crate::{HeapOptions, SharedHeapOptions};

/// Create one allocator for one explicit heap options set.
pub(crate) fn test_allocator(options: &HeapOptions) -> Arc<Allocator> {
    Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("checked heap options should build one allocator"),
    )
}

/// Create one allocator for one explicit shared heap options set.
pub(crate) fn test_shared_allocator(options: &SharedHeapOptions) -> Arc<Allocator> {
    Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
            .expect("checked shared heap options should build one allocator"),
    )
}
