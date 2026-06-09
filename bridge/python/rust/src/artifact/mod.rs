#[path = "dependency.generated.rs"]
mod dependency;
#[path = "key.generated.rs"]
mod key;
#[path = "record.generated.rs"]
mod record;
mod registry;
#[path = "sidecar.generated.rs"]
mod sidecar;
#[path = "version.generated.rs"]
mod version;

pub use dependency::*;
pub use key::*;
pub use record::*;
pub(crate) use registry::register;
pub use sidecar::*;
pub use version::*;
