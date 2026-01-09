mod dead_store_eliminate;
mod load_store_forward;
mod mem2reg;
mod sroa;

pub use dead_store_eliminate::*;
pub use load_store_forward::*;
pub use mem2reg::*;
pub use sroa::*;
