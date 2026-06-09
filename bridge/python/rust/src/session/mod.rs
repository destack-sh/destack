#[path = "file.generated.rs"]
mod file;
#[path = "module.generated.rs"]
mod module;
mod registry;
mod source;

pub use file::*;
pub use module::*;
pub(crate) use registry::register;
pub use source::*;
