#[path = "file.generated.rs"]
mod file;
#[path = "format.generated.rs"]
mod format;
#[path = "lint.generated.rs"]
mod lint;
#[path = "module.generated.rs"]
mod module;
mod registry;
mod source;

pub use file::*;
pub use format::*;
pub use lint::*;
pub use module::*;
pub(crate) use registry::register;
pub use source::*;
