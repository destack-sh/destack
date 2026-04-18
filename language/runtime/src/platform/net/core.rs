use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
#[cfg(unix)]
use crate::platform::fs::{OsPath, core as core_fs};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::net::PACKET_BACKEND_CAP_TIMESTAMP;
use crate::platform::net::{
    AcceptFlags, PacketBackend, PacketBackendCapabilityFlags, PacketBackendDescriptor,
};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::platform::net::{
    PACKET_BACKEND_CAP_CAPTURE, PACKET_BACKEND_CAP_FILTER, PACKET_BACKEND_CAP_SEND,
};
#[cfg(target_os = "linux")]
use crate::platform::net::{PACKET_BACKEND_CAP_FANOUT, PACKET_BACKEND_CAP_RING};
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::net::{PacketBackendSelectionPolicy, PacketCaptureOptions};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::BindingCallContext;
#[cfg(windows)]
use destack_workspace::PlatformWindowsPacketBackend;

/// Normalized accept-flag bit for nonblocking sockets.
pub(crate) const ACCEPT_FLAG_NONBLOCK: u32 = 1 << 0;
/// Normalized accept-flag bit for close-on-exec or non-inheritable sockets.
pub(crate) const ACCEPT_FLAG_CLOEXEC: u32 = 1 << 1;
/// Bitmask of all supported normalized accept flags.
const ACCEPT_FLAG_SUPPORTED_BITS: u32 = ACCEPT_FLAG_NONBLOCK | ACCEPT_FLAG_CLOEXEC;

/// Decoded accept-flag behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AcceptBehavior {
    /// Whether the accepted socket should be nonblocking.
    pub(crate) nonblocking: bool,
    /// Whether the accepted socket should be close-on-exec or non-inheritable.
    pub(crate) cloexec: bool,
}

/// Decode one normalized accept-flag bitset.
pub(crate) fn decode_accept_flags(flags: AcceptFlags) -> RuntimeResult<AcceptBehavior> {
    let unsupported_flags = flags.0 & !ACCEPT_FLAG_SUPPORTED_BITS;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported accept flags: {unsupported_flags:#x}"),
        ))
        .boxed());
    }

    Ok(AcceptBehavior {
        nonblocking: (flags.0 & ACCEPT_FLAG_NONBLOCK) != 0,
        cloexec: (flags.0 & ACCEPT_FLAG_CLOEXEC) != 0,
    })
}

/// Resolve a socket or listener handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    binding: &BindingCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = binding
        .worker()
        .resources
        .with_entry(id, |entry| {
            if entry.kind != kind {
                return None;
            }
            Some(with_entry(entry))
        })
        .flatten();

    match resolved {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(error),
        None => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            format!("unknown {label} handle"),
        ))
        .boxed()),
    }
}

/// Resolve an `OsPath` into byte path data on unix targets.
#[cfg(unix)]
pub(crate) fn unix_path_bytes(path: OsPath, label: &str) -> RuntimeResult<Vec<u8>> {
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            let bytes = unsafe { path_bytes.bytes.0.as_slice()? };
            Ok(bytes.to_vec())
        }
        OsPath::OsPathUtf16(path_utf16) => {
            core_fs::utf16_path_to_utf8_bytes(path_utf16.utf16, label)
        }
    }
}

/// Return the effective host packet backend for this runtime.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(super) fn current_packet_backend(binding: &BindingCallContext) -> Option<PacketBackend> {
    #[cfg(target_os = "linux")]
    {
        let _ = binding;
        Some(PacketBackend::AfPacket)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = binding;
        Some(PacketBackend::Bpf)
    }

    #[cfg(target_os = "windows")]
    {
        match binding.worker().options.platform.windows.net_packet_backend {
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
