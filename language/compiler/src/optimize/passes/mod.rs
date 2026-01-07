mod constant_fold;
mod dead_code_eliminate;
mod simplify_cfg;

pub use constant_fold::*;
pub use dead_code_eliminate::*;
pub use simplify_cfg::*;
