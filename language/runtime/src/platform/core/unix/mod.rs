mod dynamic;
mod error;

#[cfg(target_os = "linux")]
pub(crate) use dynamic::load_dynamic_symbol_named;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use dynamic::{close_dynamic_library, load_dynamic_symbol, open_dynamic_library};
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use error::io_error_with_errno;
pub(crate) use error::{io_error, net_error};
