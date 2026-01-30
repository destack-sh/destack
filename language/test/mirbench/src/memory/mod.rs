use super::Program;

mod aggregate_update;
mod alias_escape;
mod alloc_array_large;
mod alloc_array_small;
mod alloc_burst;
mod alloc_single;
mod alloc_struct_large;
mod arena_copy;
mod array_map_reduce;
mod array_random_access;
mod array_walk;
mod borrow_reborrow;
mod buffer_compact;
mod managed_vec_compact;
mod field_access;
mod global_counter;
mod graph_traversal;
mod header_map_merge;
mod hash_table_probe;
mod histogram;
mod invariant_load;
mod linked_walk;
mod load_store;
mod local_accumulate;
mod lru_cache;
mod page_checksum;
mod memory_ssa_weave;
mod move_drop_diamond;
mod nested_array_sum;
mod object_pool;
mod object_graph_mark;
mod raw_alloc_drop;
mod raw_alloc_free;
mod ring_buffer;
mod string_intern;
mod small_map;
mod symbol_table;
mod soa_vs_aos_update;
mod stack_scratch;
mod struct_shuffle;

pub use aggregate_update::AGGREGATE_UPDATE;
pub use alias_escape::ALIAS_ESCAPE;
pub use alloc_array_large::ALLOC_ARRAY_LARGE;
pub use alloc_array_small::ALLOC_ARRAY_SMALL;
pub use alloc_burst::ALLOC_BURST;
pub use alloc_single::ALLOC_SINGLE;
pub use alloc_struct_large::ALLOC_STRUCT_LARGE;
pub use arena_copy::ARENA_COPY;
pub use array_map_reduce::ARRAY_MAP_REDUCE;
pub use array_random_access::ARRAY_RANDOM_ACCESS;
pub use array_walk::ARRAY_WALK;
pub use borrow_reborrow::BORROW_REBORROW;
pub use buffer_compact::BUFFER_COMPACT;
pub use managed_vec_compact::MANAGED_VEC_COMPACT;
pub use field_access::FIELD_ACCESS;
pub use global_counter::GLOBAL_COUNTER;
pub use graph_traversal::GRAPH_TRAVERSAL;
pub use header_map_merge::HEADER_MAP_MERGE;
pub use hash_table_probe::HASH_TABLE_PROBE;
pub use histogram::HISTOGRAM;
pub use invariant_load::INVARIANT_LOAD;
pub use linked_walk::LINKED_WALK;
pub use load_store::LOAD_STORE;
pub use local_accumulate::LOCAL_ACCUMULATE;
pub use lru_cache::LRU_CACHE;
pub use page_checksum::PAGE_CHECKSUM;
pub use memory_ssa_weave::MEMORY_SSA_WEAVE;
pub use move_drop_diamond::MOVE_DROP_DIAMOND;
pub use nested_array_sum::NESTED_ARRAY_SUM;
pub use object_pool::OBJECT_POOL;
pub use object_graph_mark::OBJECT_GRAPH_MARK;
pub use raw_alloc_drop::RAW_ALLOC_DROP;
pub use raw_alloc_free::RAW_ALLOC_FREE;
pub use ring_buffer::RING_BUFFER;
pub use string_intern::STRING_INTERN;
pub use small_map::SMALL_MAP;
pub use symbol_table::SYMBOL_TABLE;
pub use soa_vs_aos_update::SOA_VS_AOS_UPDATE;
pub use stack_scratch::STACK_SCRATCH;
pub use struct_shuffle::STRUCT_SHUFFLE;

/// All benchmark programs in this category.
pub const ALL: &[&Program] = &[
    &ALIAS_ESCAPE,
    &ALLOC_SINGLE,
    &ALLOC_BURST,
    &ALLOC_ARRAY_SMALL,
    &ALLOC_ARRAY_LARGE,
    &ALLOC_STRUCT_LARGE,
    &AGGREGATE_UPDATE,
    &STRUCT_SHUFFLE,
    &BORROW_REBORROW,
    &MOVE_DROP_DIAMOND,
    &BUFFER_COMPACT,
    &MANAGED_VEC_COMPACT,
    &RAW_ALLOC_FREE,
    &RAW_ALLOC_DROP,
    &RING_BUFFER,
    &STRING_INTERN,
    &SMALL_MAP,
    &LOAD_STORE,
    &LOCAL_ACCUMULATE,
    &LRU_CACHE,
    &HISTOGRAM,
    &PAGE_CHECKSUM,
    &SYMBOL_TABLE,
    &MEMORY_SSA_WEAVE,
    &HEADER_MAP_MERGE,
    &INVARIANT_LOAD,
    &STACK_SCRATCH,
    &GLOBAL_COUNTER,
    &ARRAY_MAP_REDUCE,
    &LINKED_WALK,
    &FIELD_ACCESS,
    &ARRAY_WALK,
    &ARRAY_RANDOM_ACCESS,
    &NESTED_ARRAY_SUM,
    &HASH_TABLE_PROBE,
    &OBJECT_POOL,
    &OBJECT_GRAPH_MARK,
    &ARENA_COPY,
    &SOA_VS_AOS_UPDATE,
    &GRAPH_TRAVERSAL,
];
