/// Native platform ABI for bindings.
pub mod abi;
/// Accessibility bindings.
pub mod accessibility;
/// Audio bindings.
pub mod audio;
/// Cross-domain platform helpers.
pub mod core;
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
/// Process bindings.
pub mod process;
/// Randomness bindings.
pub mod random;
/// External resource table and finalizers.
pub mod resource;
/// Low-level runtime control and inspection bindings.
pub mod runtime;
/// Security bindings.
pub mod security;
/// Thread bindings.
pub mod thread;
/// Time bindings.
pub mod time;
/// TLS bindings.
pub mod tls;
/// TTY bindings.
pub mod tty;

pub use abi::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, RuntimeStatus, VmAggregateCodec,
    VmArray, VmCollectionElement, VmCollectionStorage, VmSlice, VmValueCodec,
};
pub(crate) use core::{NativeAbiCodec, VmAbiCodec};
pub use diagnostic::{PlatformError, PlatformErrorCode, PlatformResult};
pub use generated::{PLATFORM_NATIVE_BINDINGS, PLATFORM_VM_BINDINGS};
#[allow(unused_imports)]
pub(crate) use resource::{
    ResourceBacking, ResourceCapture, ResourceEntry, ResourceId, ResourceKind, ResourcePortability,
    ResourceTable,
};
