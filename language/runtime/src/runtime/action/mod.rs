#[path = "action.generated.rs"]
mod action_generated;
mod id;
mod profile;
mod set;

pub use action_generated::HostAction;
pub use id::HostActionId;
pub use profile::*;
pub use set::HostActionSet;
