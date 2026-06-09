mod registry;
#[path = "revision.generated.rs"]
mod revision;

pub(crate) use registry::register;
pub use revision::*;
