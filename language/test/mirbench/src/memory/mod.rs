use super::Program;

mod aggregate_update;
mod alloc_array_large;
mod alloc_array_small;
mod alloc_burst;
mod alloc_single;
mod alloc_struct_large;
mod arena_copy;
mod array_map_reduce;
mod array_random_access;
mod array_walk;
mod field_access;
mod global_counter;
mod graph_traversal;
mod hash_table_probe;
mod linked_walk;
mod load_store;
mod local_accumulate;
mod nested_array_sum;
mod object_graph_mark;
mod raw_alloc_drop;
mod raw_alloc_free;
mod soa_vs_aos_update;
mod stack_scratch;

pub use aggregate_update::AGGREGATE_UPDATE;
pub use alloc_array_large::ALLOC_ARRAY_LARGE;
pub use alloc_array_small::ALLOC_ARRAY_SMALL;
pub use alloc_burst::ALLOC_BURST;
pub use alloc_single::ALLOC_SINGLE;
pub use alloc_struct_large::ALLOC_STRUCT_LARGE;
pub use arena_copy::ARENA_COPY;
pub use array_map_reduce::ARRAY_MAP_REDUCE;
pub use array_random_access::ARRAY_RANDOM_ACCESS;
pub use array_walk::ARRAY_WALK;
pub use field_access::FIELD_ACCESS;
pub use global_counter::GLOBAL_COUNTER;
pub use graph_traversal::GRAPH_TRAVERSAL;
pub use hash_table_probe::HASH_TABLE_PROBE;
pub use linked_walk::LINKED_WALK;
pub use load_store::LOAD_STORE;
pub use local_accumulate::LOCAL_ACCUMULATE;
pub use nested_array_sum::NESTED_ARRAY_SUM;
pub use object_graph_mark::OBJECT_GRAPH_MARK;
pub use raw_alloc_drop::RAW_ALLOC_DROP;
pub use raw_alloc_free::RAW_ALLOC_FREE;
pub use soa_vs_aos_update::SOA_VS_AOS_UPDATE;
pub use stack_scratch::STACK_SCRATCH;

/// All benchmark programs in this category.
pub const ALL: &[&Program] = &[
    &ALLOC_SINGLE,
    &ALLOC_BURST,
    &ALLOC_ARRAY_SMALL,
    &ALLOC_ARRAY_LARGE,
    &ALLOC_STRUCT_LARGE,
    &AGGREGATE_UPDATE,
    &RAW_ALLOC_FREE,
    &RAW_ALLOC_DROP,
    &LOAD_STORE,
    &LOCAL_ACCUMULATE,
    &STACK_SCRATCH,
    &GLOBAL_COUNTER,
    &ARRAY_MAP_REDUCE,
    &LINKED_WALK,
    &FIELD_ACCESS,
    &ARRAY_WALK,
    &ARRAY_RANDOM_ACCESS,
    &NESTED_ARRAY_SUM,
    &HASH_TABLE_PROBE,
    &OBJECT_GRAPH_MARK,
    &ARENA_COPY,
    &SOA_VS_AOS_UPDATE,
    &GRAPH_TRAVERSAL,
];
