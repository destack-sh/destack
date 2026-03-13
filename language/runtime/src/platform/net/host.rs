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

#[cfg(windows)]
use destack_workspace::PlatformWindowsPacketBackend;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::net::PACKET_BACKEND_CAP_TIMESTAMP;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::platform::net::{
    PACKET_BACKEND_CAP_CAPTURE, PACKET_BACKEND_CAP_FILTER, PACKET_BACKEND_CAP_SEND,
};
#[cfg(target_os = "linux")]
use crate::platform::net::{PACKET_BACKEND_CAP_FANOUT, PACKET_BACKEND_CAP_RING};
use crate::platform::net::{PacketBackend, PacketBackendCapabilityFlags, PacketBackendDescriptor};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::net::{PacketBackendSelectionPolicy, PacketCaptureOptions};
use crate::runtime::{BindingCallContext, NativeSlice};

/// Return the effective host packet backend for this runtime.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) fn current_packet_backend(binding: &BindingCallContext) -> Option<PacketBackend> {
    #[cfg(target_os = "linux")]
    {
        let _ = binding;
        return Some(PacketBackend::AfPacket);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = binding;
        Some(PacketBackend::Bpf)
    }

    #[cfg(target_os = "windows")]
    {
        match binding.agent().options.platform.windows.net_packet_backend {
            PlatformWindowsPacketBackend::RawSocket => Some(PacketBackend::WinRawSocket),
            PlatformWindowsPacketBackend::Disabled | PlatformWindowsPacketBackend::HostBackend => {
                None
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = binding;
        None
    }
}

/// Return whether one packet backend is currently available on this runtime.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) fn packet_backend_available(
    binding: &BindingCallContext,
    backend: PacketBackend,
) -> bool {
    current_packet_backend(binding) == Some(backend)
}

/// Validate shared packet-capture options before backend dispatch.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
fn validate_packet_capture_options(options: PacketCaptureOptions) -> RuntimeResult<()> {
    // require one non-zero snap length
    if options.snap_length == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.snapLength",
            "snap length must be greater than zero",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one packet-open backend request against the active host backend.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) fn select_packet_backend_for_open(
    binding: &BindingCallContext,
    options: PacketCaptureOptions,
    operation: &'static str,
) -> RuntimeResult<PacketBackend> {
    validate_packet_capture_options(options)?;

    let fallback_backend = current_packet_backend(binding)
        .ok_or_else(|| RuntimeError::from(PlatformError::not_supported(operation)).boxed())?;

    if options.backend == PacketBackend::Auto || options.backend == fallback_backend {
        return Ok(fallback_backend);
    }

    if options.backend_policy == PacketBackendSelectionPolicy::AllowFallback {
        return Ok(fallback_backend);
    }

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// List host packet backends.
pub(crate) unsafe fn destack_net_packet_backend_list(
    binding: &BindingCallContext,
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
            name: binding.store_string("af_packet"),
            available: packet_backend_available(binding, PacketBackend::AfPacket),
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(
                PACKET_BACKEND_CAP_CAPTURE.0
                    | PACKET_BACKEND_CAP_SEND.0
                    | PACKET_BACKEND_CAP_TIMESTAMP.0
                    | PACKET_BACKEND_CAP_FILTER.0
                    | PACKET_BACKEND_CAP_FANOUT.0
                    | PACKET_BACKEND_CAP_RING.0,
            ),
        });
    }

    #[cfg(target_os = "macos")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Bpf,
            name: binding.store_string("bpf"),
            available: packet_backend_available(binding, PacketBackend::Bpf),
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(
                PACKET_BACKEND_CAP_CAPTURE.0
                    | PACKET_BACKEND_CAP_SEND.0
                    | PACKET_BACKEND_CAP_TIMESTAMP.0
                    | PACKET_BACKEND_CAP_FILTER.0,
            ),
        });
    }

    #[cfg(target_os = "windows")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::WinRawSocket,
            name: binding.store_string("win_raw_socket"),
            available: packet_backend_available(binding, PacketBackend::WinRawSocket),
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(
                PACKET_BACKEND_CAP_CAPTURE.0
                    | PACKET_BACKEND_CAP_SEND.0
                    | PACKET_BACKEND_CAP_FILTER.0,
            ),
        });
    }

    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Null,
            name: binding.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Null,
            name: binding.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    unsafe {
        *out = binding.store_slice(descriptors);
    }

    Ok(())
}
