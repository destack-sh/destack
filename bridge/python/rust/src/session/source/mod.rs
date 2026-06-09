#[path = "file.generated.rs"]
mod file;
mod registry;
#[path = "snapshot.generated.rs"]
mod snapshot;
#[path = "update.generated.rs"]
mod update;

pub use file::*;
pub(crate) use registry::register;
pub use snapshot::*;
pub use update::*;
