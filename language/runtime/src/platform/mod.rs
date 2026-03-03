/// Native platform ABI for bindings.
pub mod abi;
/// Audio bindings.
pub mod audio;
/// Platform context and configuration.
pub mod context;
/// Cross-domain platform helpers.
pub(crate) mod core;
/// Cryptography bindings.
pub mod crypto;
/// Debug bindings.
pub mod debug;
/// Device bindings.
pub mod device;
/// Platform diagnostics.
pub mod diagnostic;
/// Display bindings.
pub mod display;
/// Error bindings.
pub mod error;
/// FFI bindings.
pub mod ffi;
/// Filesystem bindings.
pub mod fs;
/// Generated platform binding lists.
mod generated;
/// GPU bindings.
pub mod gpu;
/// Input bindings.
pub mod input;
/// I/O event and completion bindings.
pub mod io;
/// IPC bindings.
pub mod ipc;
/// Memory bindings.
pub mod memory;
/// Network bindings.
pub mod net;
/// OS bindings.
pub mod os;
/// Completion-based I/O abstraction.
pub mod proactor;
/// Process bindings.
pub mod process;
/// Randomness bindings.
pub mod random;
/// External resource table and finalizers.
pub mod resource;
/// Security bindings.
pub mod security;
/// Runtime-owned platform module state.
pub(crate) mod state;
/// Thread bindings.
pub mod thread;
/// Time bindings.
pub mod time;
/// TLS bindings.
pub mod tls;
/// TTY bindings.
pub mod tty;

pub use abi::{NativeArray, RuntimeStatus, VmAggregateCodec, VmArray, VmSlice, VmValueCodec};
pub use context::PlatformContext;
pub use diagnostic::{PlatformError, PlatformErrorCode, PlatformResult};
pub use generated::{PLATFORM_NATIVE_BINDINGS, PLATFORM_VM_BINDINGS};
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
