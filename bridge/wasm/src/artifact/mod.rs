#[path = "dependency.generated.rs"]
mod dependency;
#[path = "key.generated.rs"]
mod key;
#[path = "record.generated.rs"]
mod record;
#[path = "sidecar.generated.rs"]
mod sidecar;
#[path = "version.generated.rs"]
mod version;

pub use dependency::*;
pub use key::*;
pub use record::*;
pub use sidecar::*;
pub use version::*;
