use std::mem::size_of;

use destack_heap::DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES;

/// The virtual memory range reserved by address-space benchmarks.
pub(crate) const SPACE_SIZE_BYTES: usize = 16 * 1024 * 1024;
/// The allocator page width used by heap benchmarks.
pub(crate) const PAGE_SIZE_BYTES: usize = DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES;
/// The number of small allocations in one wrapper-path sample.
pub(crate) const SMALL_ALLOCATIONS: usize = 1024;
/// The representative small object payload width.
pub(crate) const SMALL_BYTES: usize = 32;
/// The number of large allocations in one large-object sample.
pub(crate) const LARGE_ALLOCATIONS: usize = 64;
/// The representative large object payload width.
pub(crate) const LARGE_BYTES: usize = 1024 * 1024;
/// The native pointer width used by heap references.
pub(crate) const REFERENCE_BYTES: usize = size_of::<usize>();
/// The maximum number of allocations in one size-class matrix sample.
pub(crate) const MATRIX_MAX_ALLOCATIONS: usize = 8 * 1024;
/// The minimum number of allocations in one size-class matrix sample.
pub(crate) const MATRIX_MIN_ALLOCATIONS: usize = 128;
/// The target byte volume in one size-class matrix sample.
pub(crate) const MATRIX_SAMPLE_BYTES: usize = 1024 * 1024;
/// The number of allocations performed by each parallel worker.
pub(crate) const PARALLEL_ALLOCATIONS_PER_WORKER: usize = 4 * 1024;
/// The number of records in one object-graph workload.
pub(crate) const WORKLOAD_OBJECTS: usize = 512;
/// The number of records mutated after one fork.
pub(crate) const WORKLOAD_MUTATIONS: usize = 128;
/// The byte width of one record payload.
pub(crate) const RECORD_BYTES: usize = 32;
/// The byte width of one leaf payload.
pub(crate) const LEAF_BYTES: usize = 24;
/// The allocation sizes sampled by the hot-path matrix.
pub(crate) const ALLOCATION_MATRIX_BYTES: &[usize] =
    &[8, 16, 24, 32, 64, 128, 256, 512, 1024, 4096, 32_768];
/// The worker counts sampled by shared allocation benchmarks.
pub(crate) const PARALLEL_WORKERS: &[usize] = &[1, 2, 4, 8];
/// The materialized page counts sampled by fork benchmarks.
pub(crate) const FORK_MATERIALIZED_PAGES: &[usize] = &[1, 16, 256];
/// The number of live fork ancestors sampled by fork benchmarks.
pub(crate) const FORK_ANCESTOR_COUNTS: &[usize] = &[0, 1, 4, 16];
/// The dirty page-count candidates sampled by fork benchmarks.
pub(crate) const FORK_DIRTY_PAGE_COUNTS: &[usize] = &[0, 1, 4, 16, 256];
/// The reserved heap sizes sampled by large fork benchmarks.
pub(crate) const FORK_LARGE_SPACE_SIZE_BYTES: &[usize] = &[20 * 1024 * 1024, 100 * 1024 * 1024];
/// The active byte counts sampled by large fork benchmarks.
pub(crate) const FORK_LARGE_ACTIVE_BYTES: &[usize] = &[1024 * 1024, 10 * 1024 * 1024];
/// The dirty byte counts sampled by large fork benchmarks.
pub(crate) const FORK_LARGE_DIRTY_BYTES: &[usize] = &[0, 1024 * 1024, 10 * 1024 * 1024];
/// The live ancestor counts sampled by large fork benchmarks.
pub(crate) const FORK_LARGE_ANCESTORS: &[usize] = &[1, 16];
