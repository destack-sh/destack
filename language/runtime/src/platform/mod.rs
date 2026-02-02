/// Native platform ABI for bindings.
pub mod abi;
/// Platform bindings for runtime integration.
pub mod bindings;
/// Console bindings.
pub mod console;
/// Platform context and configuration.
pub mod context;
/// Platform diagnostics.
pub mod diagnostic;
/// Error bindings.
pub mod error;
/// Filesystem bindings.
pub mod fs;
/// Generated platform binding lists.
mod generated;
/// Network bindings.
pub mod net;
/// Platform event polling abstraction.
pub mod poller;
/// Process bindings.
pub mod process;
/// Randomness bindings.
pub mod random;
/// External resource table and finalizers.
pub mod resource;
/// Time bindings.
pub mod time;
/// Timer bindings.
pub mod timer;

pub use abi::{
    PlatformArray, PlatformSlice, PlatformStringRef, PlatformStringSlice, RuntimeStatus, VmArray,
    VmSlice,
};
pub use bindings::{BindingPolicy, BindingRegistry, DeterminismPolicy, ReplayMode, VmBindingSet};
pub use context::PlatformContext;
pub use diagnostic::{PlatformError, PlatformErrorCode, PlatformErrorKind, PlatformResult};
pub use generated::{PLATFORM_NATIVE_BINDINGS, PLATFORM_VM_BINDINGS};
#[cfg(unix)]
pub use poller::UnixPoller;
pub use poller::{
    PlatformEvent, PlatformEventFlags, PlatformEventKind, PlatformHandle, PlatformInterest,
    PlatformPoller, PlatformPollerFlags,
};
pub use resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind, ResourceTable};
