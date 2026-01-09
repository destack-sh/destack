mod constant_fold;
mod copy_propagate;
mod dead_code_eliminate;
mod gvn;
mod instruction_combine;
mod local_cse;
mod simplify_cfg;
mod sink;

pub use constant_fold::*;
pub use copy_propagate::*;
pub use dead_code_eliminate::*;
pub use gvn::*;
pub use instruction_combine::*;
pub use local_cse::*;
pub use simplify_cfg::*;
pub use sink::*;
