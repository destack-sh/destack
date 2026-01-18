mod dead_store_eliminate;
mod load_pre;
mod load_store_forward;
mod mem2reg;
mod mem_cse;
mod sroa;
mod store_pre;
mod store_sink;

pub use dead_store_eliminate::*;
pub use load_pre::*;
pub use load_store_forward::*;
pub use mem_cse::*;
pub use mem2reg::*;
pub use sroa::*;
pub use store_pre::*;
pub use store_sink::*;
