#[cfg(unix)]
#[path = "unix/mod.rs"]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(not(any(unix, windows)))]
#[path = "unsupported.rs"]
mod unsupported;
#[cfg(not(any(unix, windows)))]
pub(crate) use unsupported::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::net::{PacketBackend, PacketBackendCapabilityFlags, PacketBackendDescriptor};
use crate::runtime::{BindingCallContext, NativeSlice};

/// List host packet backends.
pub(crate) unsafe fn destack_net_packet_backend_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<PacketBackendDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let mut descriptors = Vec::new();

    #[cfg(target_os = "linux")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::AfPacket,
            name: context.store_string("af_packet"),
            available: true,
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    #[cfg(target_os = "macos")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Bpf,
            name: context.store_string("bpf"),
            available: true,
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    #[cfg(target_os = "windows")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::WinRawSocket,
            name: context.store_string("win_raw_socket"),
            available: true,
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Null,
            name: context.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Null,
            name: context.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    unsafe {
        *out = context.store_slice(descriptors);
    }

    Ok(())
}
