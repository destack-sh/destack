/// Native platform ABI for bindings.
pub mod abi;
/// Platform bindings for runtime integration.
pub mod bindings;
/// Console bindings.
pub mod console;
/// Platform context and configuration.
pub mod context;
/// Cross-domain platform helpers.
pub(crate) mod core;
/// Platform diagnostics.
pub mod diagnostic;
/// Error bindings.
pub mod error;
/// Filesystem bindings.
pub mod fs;
/// Generated platform binding lists.
mod generated;
/// I/O event and completion bindings.
pub mod io;
/// Network bindings.
pub mod net;
/// Platform event polling abstraction.
pub mod poller;
/// Completion-based I/O abstraction.
pub mod proactor;
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
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, RuntimeStatus, VmArray, VmSlice,
    VmValueCodec,
};
pub use bindings::{BindingPolicy, BindingRegistry, ExecutionMode, VmBindingSet};
pub use context::PlatformContext;
pub use diagnostic::{PlatformError, PlatformErrorCode, PlatformResult};
pub use generated::{PLATFORM_NATIVE_BINDINGS, PLATFORM_VM_BINDINGS};
#[cfg(target_os = "linux")]
pub use poller::IoUringPoller;
#[cfg(unix)]
pub use poller::UnixPoller;
#[cfg(windows)]
pub use poller::WindowsPoller;
pub use poller::{
    PlatformEvent, PlatformEventFlags, PlatformEventMask, PlatformEventSource, PlatformHandle,
    PlatformInterest, PlatformPoller, PlatformPollerFlags, PollerToken,
};
#[cfg(target_os = "linux")]
pub use proactor::IoUringProactor;
#[cfg(windows)]
pub use proactor::IocpProactor;
#[cfg(all(unix, not(target_os = "linux")))]
pub use proactor::UnixProactor;
pub use proactor::{
    Proactor, ProactorAddress, ProactorAddressStorage, ProactorBuffer, ProactorBufferVec,
    ProactorCompletion, ProactorCompletionData, ProactorOp, ProactorOpKind, ProactorRequest,
    ProactorShutdown,
};
pub use resource::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind, ResourceTable};
