mod eliminate_dead_stores;
mod eliminate_partial_redundant_loads;
mod eliminate_partial_redundant_stores;
mod eliminate_redundant_memory;
mod forward_stored_values;
mod promote_memory_to_registers;
mod sink_stores;
mod split_aggregates;

pub use eliminate_dead_stores::*;
pub use eliminate_partial_redundant_loads::*;
pub use eliminate_partial_redundant_stores::*;
pub use eliminate_redundant_memory::*;
pub use forward_stored_values::*;
pub use promote_memory_to_registers::*;
pub use sink_stores::*;
pub use split_aggregates::*;
