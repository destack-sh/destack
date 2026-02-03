use super::Program;

mod async_state_machine;
mod call_direct_deep;
mod call_direct_shallow;
mod call_indirect_table;
mod call_many_args;
mod call_many_args_16;
mod call_mutual;
mod call_mutual_tail;
mod call_pipeline_chain;
mod call_recursive_tail;
mod call_tail_hash;
mod call_vtable_dispatch;
mod callback_pipeline;
mod callgraph_fanout;
mod recursive_tree_sum;
mod request_router;
mod service_chain;
mod task_pipeline;
mod yield_batch_filter;
mod yield_buffer_scan;
mod yield_loop;
mod yield_nested_call;
mod yield_stream_join;
mod yield_task_queue;

pub use async_state_machine::ASYNC_STATE_MACHINE;
pub use call_direct_deep::CALL_DIRECT_DEEP;
pub use call_direct_shallow::CALL_DIRECT_SHALLOW;
pub use call_indirect_table::CALL_INDIRECT_TABLE;
pub use call_many_args::CALL_MANY_ARGS;
pub use call_many_args_16::CALL_MANY_ARGS_16;
pub use call_mutual::CALL_MUTUAL;
pub use call_mutual_tail::CALL_MUTUAL_TAIL;
pub use call_pipeline_chain::CALL_PIPELINE_CHAIN;
pub use call_recursive_tail::CALL_RECURSIVE_TAIL;
pub use call_tail_hash::CALL_TAIL_HASH;
pub use call_vtable_dispatch::CALL_VTABLE_DISPATCH;
pub use callback_pipeline::CALLBACK_PIPELINE;
pub use callgraph_fanout::CALLGRAPH_FANOUT;
pub use recursive_tree_sum::RECURSIVE_TREE_SUM;
pub use request_router::REQUEST_ROUTER;
pub use service_chain::SERVICE_CHAIN;
pub use task_pipeline::TASK_PIPELINE;
pub use yield_batch_filter::YIELD_BATCH_FILTER;
pub use yield_buffer_scan::YIELD_BUFFER_SCAN;
pub use yield_loop::YIELD_LOOP;
pub use yield_nested_call::YIELD_NESTED_CALL;
pub use yield_stream_join::YIELD_STREAM_JOIN;
pub use yield_task_queue::YIELD_TASK_QUEUE;

/// All benchmark programs in this category.
pub const ALL: &[&Program] = &[
    &CALLGRAPH_FANOUT,
    &CALLBACK_PIPELINE,
    &SERVICE_CHAIN,
    &CALL_DIRECT_SHALLOW,
    &CALL_DIRECT_DEEP,
    &CALL_MANY_ARGS,
    &CALL_MANY_ARGS_16,
    &CALL_PIPELINE_CHAIN,
    &TASK_PIPELINE,
    &REQUEST_ROUTER,
    &CALL_VTABLE_DISPATCH,
    &CALL_INDIRECT_TABLE,
    &CALL_RECURSIVE_TAIL,
    &RECURSIVE_TREE_SUM,
    &CALL_TAIL_HASH,
    &CALL_MUTUAL,
    &CALL_MUTUAL_TAIL,
    &ASYNC_STATE_MACHINE,
    &YIELD_BUFFER_SCAN,
    &YIELD_BATCH_FILTER,
    &YIELD_TASK_QUEUE,
    &YIELD_STREAM_JOIN,
    &YIELD_LOOP,
    &YIELD_NESTED_CALL,
];
