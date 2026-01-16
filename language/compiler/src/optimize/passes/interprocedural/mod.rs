mod dead_arg_eliminate;
mod dead_function_eliminate;
mod function_attrs;
mod global_dead_code_eliminate;
mod global_opt;
mod inline;
mod ip_constant_prop;

pub use dead_arg_eliminate::*;
pub use dead_function_eliminate::*;
pub use function_attrs::*;
pub use global_dead_code_eliminate::*;
pub use global_opt::*;
pub use inline::*;
pub use ip_constant_prop::*;
