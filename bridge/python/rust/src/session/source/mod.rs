#[path = "file.generated.rs"]
mod file;
mod registry;
#[path = "source.generated.rs"]
mod source;
#[path = "update.generated.rs"]
mod update;

pub use file::*;
pub(crate) use registry::register;
pub use source::*;
pub use update::*;
