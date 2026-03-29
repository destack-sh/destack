/// Native platform ABI for bindings.
pub mod abi;
#[cfg(not(feature = "generator"))]
/// Accessibility bindings.
pub mod accessibility;
#[cfg(not(feature = "generator"))]
/// Audio bindings.
pub mod audio;
#[cfg(not(feature = "generator"))]
/// Cross-domain platform helpers.
pub mod core;
#[cfg(not(feature = "generator"))]
/// Cryptography bindings.
pub mod crypto;
#[cfg(not(feature = "generator"))]
/// Debug bindings.
pub mod debug;
#[cfg(not(feature = "generator"))]
/// Device bindings.
pub mod device;
/// Platform diagnostics.
pub mod diagnostic;
#[cfg(not(feature = "generator"))]
/// Display bindings.
pub mod display;
#[cfg(not(feature = "generator"))]
/// Error bindings.
pub mod error;
#[cfg(not(feature = "generator"))]
/// FFI bindings.
pub mod ffi;
#[cfg(not(feature = "generator"))]
/// Filesystem bindings.
pub mod fs;
#[cfg(not(feature = "generator"))]
/// Generated platform binding lists.
mod generated;
#[cfg(not(feature = "generator"))]
/// GPU bindings.
pub mod gpu;
#[cfg(not(feature = "generator"))]
/// Input bindings.
pub mod input;
#[cfg(not(feature = "generator"))]
/// I/O event and completion bindings.
pub mod io;
#[cfg(not(feature = "generator"))]
/// IPC bindings.
pub mod ipc;
#[cfg(not(feature = "generator"))]
/// Memory bindings.
pub mod memory;
#[cfg(not(feature = "generator"))]
/// Network bindings.
pub mod net;
#[cfg(not(feature = "generator"))]
/// OS bindings.
pub mod os;
#[cfg(not(feature = "generator"))]
/// Completion-based I/O abstraction.
pub mod proactor;
#[cfg(not(feature = "generator"))]
/// Process bindings.
pub mod process;
#[cfg(not(feature = "generator"))]
/// Randomness bindings.
pub mod random;
#[cfg(not(feature = "generator"))]
/// External resource table and finalizers.
pub mod resource;
#[cfg(not(feature = "generator"))]
/// Low-level runtime control and inspection bindings.
pub mod runtime;
#[cfg(not(feature = "generator"))]
/// Security bindings.
pub mod security;
#[cfg(not(feature = "generator"))]
/// Thread bindings.
pub mod thread;
#[cfg(not(feature = "generator"))]
/// Time bindings.
pub mod time;
#[cfg(not(feature = "generator"))]
/// TLS bindings.
pub mod tls;
#[cfg(not(feature = "generator"))]
/// TTY bindings.
pub mod tty;

pub use abi::{
    NativeArray, RuntimeStatus, VmAggregateCodec, VmArray, VmCollectionElement,
    VmCollectionStorage, VmSlice, VmValueCodec,
};
#[cfg(not(feature = "generator"))]
pub(crate) use core::{NativeAbiCodec, VmAbiCodec};
pub use diagnostic::{PlatformError, PlatformErrorCode, PlatformResult};
#[cfg(not(feature = "generator"))]
pub use generated::{PLATFORM_NATIVE_BINDINGS, PLATFORM_VM_BINDINGS};
#[cfg(all(not(feature = "generator"), target_os = "linux"))]
pub use proactor::IoUringProactor;
#[cfg(all(not(feature = "generator"), windows))]
pub use proactor::IocpProactor;
#[cfg(all(not(feature = "generator"), unix, not(target_os = "linux")))]
pub use proactor::UnixProactor;
#[cfg(not(feature = "generator"))]
pub use proactor::{
    Proactor, ProactorAddress, ProactorAddressStorage, ProactorBuffer, ProactorBufferVec,
    ProactorCompletion, ProactorCompletionData, ProactorOp, ProactorOpKind, ProactorRequest,
    ProactorShutdown,
};
#[cfg(not(feature = "generator"))]
#[allow(unused_imports)]
pub(crate) use resource::{
    ResourceBacking, ResourceCapture, ResourceEntry, ResourceId, ResourceKind, ResourcePortability,
    ResourceTable,
};
