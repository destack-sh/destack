mod constant_fold;
mod dead_code_eliminate;
mod instruction_combine;
mod simplify_cfg;

pub use constant_fold::*;
pub use dead_code_eliminate::*;
pub use instruction_combine::*;
pub use simplify_cfg::*;
