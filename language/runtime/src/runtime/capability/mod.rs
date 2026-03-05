#[path = "capability.generated.rs"]
mod capability_generated;
mod id;
mod profile;
mod set;

pub use capability_generated::PlatformCapability;
pub use id::PlatformCapabilityId;
pub use profile::*;
pub use set::PlatformCapabilitySet;
