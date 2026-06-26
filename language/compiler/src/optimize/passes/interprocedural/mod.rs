mod argument_specialize;
mod eliminate_dead_arguments;
mod eliminate_dead_functions;
mod eliminate_global_dead_code;
mod global_opt;
mod inline;
mod ip_constant_prop;
mod ip_dce_cleanup;
mod ip_sccp;

pub use argument_specialize::*;
pub use eliminate_dead_arguments::*;
pub use eliminate_dead_functions::*;
pub use eliminate_global_dead_code::*;
pub use global_opt::*;
pub use inline::*;
pub use ip_constant_prop::*;
pub use ip_dce_cleanup::*;
pub use ip_sccp::*;
