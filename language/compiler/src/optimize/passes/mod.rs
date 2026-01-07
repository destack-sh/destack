mod constant_fold;
mod copy_propagate;
mod dead_code_eliminate;
mod instruction_combine;
mod mem2reg;
mod simplify_cfg;

pub use constant_fold::*;
pub use copy_propagate::*;
pub use dead_code_eliminate::*;
pub use instruction_combine::*;
pub use mem2reg::*;
pub use simplify_cfg::*;
