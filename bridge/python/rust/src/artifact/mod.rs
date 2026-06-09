#[path = "key.generated.rs"]
mod key;
mod registry;
#[path = "sidecar.generated.rs"]
mod sidecar;
#[path = "version.generated.rs"]
mod version;

pub use key::*;
pub(crate) use registry::register;
pub use sidecar::*;
pub use version::*;
