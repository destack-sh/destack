#[path = "capability.generated.rs"]
mod capability_generated;
mod id;
mod set;

pub use capability_generated::PlatformCapability;
pub use id::PlatformCapabilityId;
pub use set::PlatformCapabilitySet;
