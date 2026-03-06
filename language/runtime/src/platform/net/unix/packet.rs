#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "macos")]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::OsPath;
use crate::platform::net::{core as core_net, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::{BindingCallContext, NativeSlice};

use std::ffi::{CStr, CString};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

/// One sockaddr storage length in bytes.
#[cfg(target_os = "linux")]
const SOCKADDR_LL_LENGTH: libc::socklen_t =
    std::mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;
/// One packet control-plane timestamp in nanoseconds per second.
#[cfg(any(target_os = "linux", target_os = "macos"))]
const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;
/// One microsecond in nanoseconds.
#[cfg(target_os = "macos")]
const NANOSECONDS_PER_MICROSECOND: u64 = 1_000;
/// One BPF packet-alignment width in bytes.
#[cfg(target_os = "macos")]
const MACOS_BPF_ALIGNMENT: usize = std::mem::size_of::<u32>();
/// One fallback BPF read buffer size in bytes.
#[cfg(target_os = "macos")]
const MACOS_BPF_BUFFER_FALLBACK: u32 = 4096;
/// One maximum scanned `/dev/bpfN` index.
#[cfg(target_os = "macos")]
const MACOS_BPF_DEVICE_MAX_INDEX: u32 = 256;
/// One classic BPF opcode for return statements.
#[cfg(target_os = "macos")]
const MACOS_BPF_OPCODE_RETURN: u16 = 0x06;
/// One classic BPF opcode mode for immediate constants.
#[cfg(target_os = "macos")]
const MACOS_BPF_OPCODE_IMMEDIATE: u16 = 0x00;
/// One classic BPF immediate return value for accept-all.
#[cfg(target_os = "macos")]
const MACOS_BPF_ACCEPT_ALL: u32 = u32::MAX;

/// One classic BPF instruction row used by macOS BPF ioctls.
#[cfg(target_os = "macos")]
#[repr(C)]
struct MacosBpfInstruction {
    /// BPF opcode.
    code: u16,
    /// Jump target on true.
    jt: u8,
    /// Jump target on false.
    jf: u8,
    /// Immediate operand value.
    k: u32,
}

/// One classic BPF filter program descriptor.
#[cfg(target_os = "macos")]
#[repr(C)]
struct MacosBpfProgram {
    /// Number of instructions in the program.
    instruction_count: libc::c_uint,
    /// Pointer to contiguous instruction rows.
    instructions: *mut MacosBpfInstruction,
}

/// One packet capture statistics row from BIOCGSTATS.
#[cfg(target_os = "macos")]
#[repr(C)]
struct MacosBpfStats {
    /// Number of packets received by the backend.
    received_packets: libc::c_uint,
    /// Number of packets dropped by the backend.
    dropped_packets: libc::c_uint,
}

/// Return one packet binding not-supported error.
fn packet_not_supported(operation: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Return one ioWouldBlock packet error.
#[cfg(target_os = "macos")]
fn packet_would_block(operation: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        None,
        None,
        format!("{operation}: packet read timed out"),
    ))
    .boxed())
}

/// Convert one packet fanout mode into the host fanout mode constant.
#[cfg(target_os = "linux")]
fn packet_fanout_mode(mode: PacketFanoutMode) -> u32 {
    match mode {
        PacketFanoutMode::Hash => libc::PACKET_FANOUT_HASH,
        PacketFanoutMode::LoadBalance => libc::PACKET_FANOUT_LB,
        PacketFanoutMode::Cpu => libc::PACKET_FANOUT_CPU,
        PacketFanoutMode::RoundRobin => libc::PACKET_FANOUT_RND,
        PacketFanoutMode::Rollover => libc::PACKET_FANOUT_ROLLOVER,
        PacketFanoutMode::QueueMap => libc::PACKET_FANOUT_QM,
    }
}

/// Read one monotonic host timestamp as nanoseconds.
#[cfg(target_os = "linux")]
fn monotonic_timestamp_nanoseconds() -> RuntimeResult<u64> {
    // query one monotonic host clock timestamp
    let mut timestamp = std::mem::MaybeUninit::<libc::timespec>::uninit();
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, timestamp.as_mut_ptr()) };
    if rc != 0 {
        return Err(core_platform::net_error("clock_gettime"));
    }

    // convert timespec fields into nanoseconds
    let timestamp = unsafe { timestamp.assume_init() };
    let seconds = u64::try_from(timestamp.tv_sec).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "timestamp",
            "clock seconds out of range",
        ))
        .boxed()
    })?;
    let nanoseconds = u64::try_from(timestamp.tv_nsec).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "timestamp",
            "clock nanoseconds out of range",
        ))
        .boxed()
    })?;

    Ok(seconds
        .saturating_mul(NANOSECONDS_PER_SECOND)
        .saturating_add(nanoseconds))
}

/// Round one BPF payload length to the next kernel alignment boundary.
#[cfg(target_os = "macos")]
fn macos_bpf_word_align(length: usize) -> usize {
    (length + (MACOS_BPF_ALIGNMENT - 1)) & !(MACOS_BPF_ALIGNMENT - 1)
}

/// Open one macOS BPF descriptor from `/dev/bpf` or `/dev/bpfN`.
#[cfg(target_os = "macos")]
fn open_macos_bpf_descriptor() -> RuntimeResult<RawFd> {
    // try one cloning BPF device path first
    let clone_device = CString::new("/dev/bpf").map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "device",
            "invalid BPF device path",
        ))
        .boxed()
    })?;
    let descriptor = unsafe { libc::open(clone_device.as_ptr(), libc::O_RDWR) };
    if descriptor >= 0 {
        return Ok(descriptor);
    }

    // scan legacy per-device BPF paths
    let mut last_errno = core_platform::get_errno();
    for index in 0..MACOS_BPF_DEVICE_MAX_INDEX {
        let path = format!("/dev/bpf{index}");
        let device = CString::new(path).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "device",
                "invalid BPF device path",
            ))
            .boxed()
        })?;
        let descriptor = unsafe { libc::open(device.as_ptr(), libc::O_RDWR) };
        if descriptor >= 0 {
            return Ok(descriptor);
        }

        let errno = core_platform::get_errno();
        if errno == libc::EBUSY {
            last_errno = errno;
            continue;
        }
        if errno == libc::ENOENT {
            break;
        }

        last_errno = errno;
    }

    // report the captured open failure
    core_platform::set_errno(last_errno);
    Err(core_platform::net_error("open(/dev/bpf)"))
}

/// Resolve one interface name for one interface index.
#[cfg(target_os = "macos")]
fn macos_interface_name(interface_index: u32) -> RuntimeResult<CString> {
    // query one interface name from the kernel index table
    let mut name = [0 as libc::c_char; libc::IF_NAMESIZE];
    let pointer = unsafe { libc::if_indextoname(interface_index, name.as_mut_ptr()) };
    if pointer.is_null() {
        return Err(core_platform::net_error("if_indextoname"));
    }

    Ok(unsafe { CStr::from_ptr(name.as_ptr()) }.to_owned())
}

/// Bind one BPF descriptor to one network interface.
#[cfg(target_os = "macos")]
fn bind_macos_bpf_interface(descriptor: RawFd, interface_name: &CStr) -> RuntimeResult<()> {
    // copy one interface name into one ifreq payload
    let mut request = unsafe { std::mem::zeroed::<libc::ifreq>() };
    let name_bytes = interface_name.to_bytes_with_nul();
    if name_bytes.len() > request.ifr_name.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.interfaceIndex",
            "interface name exceeds ifreq host limit",
        ))
        .boxed());
    }
    for (destination, value) in request.ifr_name.iter_mut().zip(name_bytes.iter().copied()) {
        *destination = value as libc::c_char;
    }

    // apply one BIOCSETIF interface binding
    let rc = unsafe { libc::ioctl(descriptor, libc::BIOCSETIF as _, &request) };
    if rc != 0 {
        return Err(core_platform::net_error("ioctl(BIOCSETIF)"));
    }

    Ok(())
}

/// Read the bound interface index for one BPF descriptor.
#[cfg(target_os = "macos")]
fn macos_bpf_interface_index(descriptor: RawFd) -> u32 {
    // query one interface row from BIOCGETIF
    let mut request = unsafe { std::mem::zeroed::<libc::ifreq>() };
    let rc = unsafe { libc::ioctl(descriptor, libc::BIOCGETIF as _, &mut request) };
    if rc != 0 {
        return 0;
    }

    // map one interface name to one index
    let name = request.ifr_name.as_ptr();
    if name.is_null() {
        return 0;
    }

    unsafe { libc::if_nametoindex(name) }
}

/// Configure one BPF descriptor timeout window.
#[cfg(target_os = "macos")]
fn configure_macos_bpf_timeout(descriptor: RawFd, timeout_ms: i32) -> RuntimeResult<()> {
    // convert timeout milliseconds into one timeval
    let seconds = timeout_ms / 1000;
    let microseconds = (timeout_ms % 1000) * 1000;
    let timeout = libc::timeval {
        tv_sec: seconds as libc::time_t,
        tv_usec: microseconds as libc::suseconds_t,
    };

    // apply one BIOCSRTIMEOUT timeout policy
    let rc = unsafe { libc::ioctl(descriptor, libc::BIOCSRTIMEOUT as _, &timeout) };
    if rc != 0 {
        return Err(core_platform::net_error("ioctl(BIOCSRTIMEOUT)"));
    }

    Ok(())
}

/// Configure one BPF descriptor read buffer length.
#[cfg(target_os = "macos")]
fn configure_macos_bpf_buffer_length(descriptor: RawFd, snap_length: u32) -> RuntimeResult<()> {
    // choose one target kernel BPF buffer length
    let target_length = snap_length.max(MACOS_BPF_BUFFER_FALLBACK);
    let mut length = libc::c_uint::try_from(target_length).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "options.snapLength",
            "snap length exceeds host BPF buffer range",
        ))
        .boxed()
    })?;

    // request one BPF kernel buffer length
    let rc = unsafe { libc::ioctl(descriptor, libc::BIOCSBLEN as _, &mut length) };
    if rc != 0 {
        return Err(core_platform::net_error("ioctl(BIOCSBLEN)"));
    }

    Ok(())
}

/// Configure one classic BPF filter program on one descriptor.
#[cfg(target_os = "macos")]
fn set_macos_bpf_filter(
    descriptor: RawFd,
    instructions: &mut [MacosBpfInstruction],
    operation: &'static str,
) -> RuntimeResult<()> {
    // map instructions into one BIOCSETF program descriptor
    let mut program = MacosBpfProgram {
        instruction_count: libc::c_uint::try_from(instructions.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "filterProgram",
                "filter program instruction count exceeds host limit",
            ))
            .boxed()
        })?,
        instructions: instructions.as_mut_ptr(),
    };

    // apply one filter program on the BPF descriptor
    let rc = unsafe { libc::ioctl(descriptor, libc::BIOCSETF as _, &mut program) };
    if rc != 0 {
        return Err(core_platform::net_error(operation));
    }

    Ok(())
}

/// Read one kernel BPF buffer length for one descriptor.
#[cfg(target_os = "macos")]
fn macos_bpf_buffer_length(descriptor: RawFd) -> RuntimeResult<usize> {
    // read one configured BPF buffer length
    let mut length = 0 as libc::c_uint;
    let rc = unsafe { libc::ioctl(descriptor, libc::BIOCGBLEN as _, &mut length) };
    if rc != 0 {
        return Err(core_platform::net_error("ioctl(BIOCGBLEN)"));
    }
    if length == 0 {
        return Err(RuntimeError::from(PlatformError::io(
            "ioctl(BIOCGBLEN): host reported zero packet buffer length",
        ))
        .boxed());
    }

    Ok(length as usize)
}

/// Open a packet capture or inject endpoint.
///
/// Opens one host packet endpoint for packet capture and injection.
/// Frame shape and metadata are backend specific.
/// Host privilege checks and backend-specific limits are enforced by the kernel or driver.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET on Linux and `/dev/bpf` packet devices on macOS.
/// Returns `notSupported` on Unix targets without a packet backend.
/// Uses one configured host packet backend on Windows.
/// Current Windows backend uses raw IPv4 sockets with `SIO_RCVALL`, payloads are IP packets rather than Ethernet frames.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_open(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    // reject unix targets without one packet backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, out, options);
        packet_not_supported("destack.net.packetOpen")
    }

    #[cfg(target_os = "linux")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // validate capture options
        if options.interface_index == 0 && options.promiscuous {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.interfaceIndex",
                "interface index is required for promiscuous mode",
            ))
            .boxed());
        }

        // open one packet socket for all ethernet protocols
        let protocol = (libc::ETH_P_ALL as u16).to_be() as i32;
        let fd = unsafe { libc::socket(libc::AF_PACKET, libc::SOCK_RAW, protocol) };
        if fd < 0 {
            return Err(core_platform::net_error("socket"));
        }

        // bind packet capture to one interface when requested
        if options.interface_index != 0 {
            let mut address = unsafe { std::mem::zeroed::<libc::sockaddr_ll>() };
            address.sll_family = libc::AF_PACKET as libc::c_ushort;
            address.sll_protocol = (libc::ETH_P_ALL as u16).to_be();
            address.sll_ifindex = i32::try_from(options.interface_index).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "options.interfaceIndex",
                    "interface index is out of range",
                ))
                .boxed()
            })?;

            let rc = unsafe {
                libc::bind(
                    fd,
                    &address as *const _ as *const libc::sockaddr,
                    SOCKADDR_LL_LENGTH,
                )
            };
            if rc != 0 {
                let _ = unsafe { libc::close(fd) };
                return Err(core_platform::net_error("bind"));
            }
        }

        // enable promiscuous membership when requested
        if options.promiscuous {
            let mut membership = unsafe { std::mem::zeroed::<libc::packet_mreq>() };
            membership.mr_ifindex = i32::try_from(options.interface_index).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "options.interfaceIndex",
                    "interface index is out of range",
                ))
                .boxed()
            })?;
            membership.mr_type = libc::PACKET_MR_PROMISC as libc::c_ushort;

            let rc = unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_PACKET,
                    libc::PACKET_ADD_MEMBERSHIP,
                    &membership as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::packet_mreq>() as libc::socklen_t,
                )
            };
            if rc != 0 {
                let _ = unsafe { libc::close(fd) };
                return Err(core_platform::net_error(
                    "setsockopt(PACKET_ADD_MEMBERSHIP)",
                ));
            }
        }

        // configure one receive timeout when provided
        if options.timeout_ms >= 0 {
            let seconds = options.timeout_ms / 1000;
            let microseconds = (options.timeout_ms % 1000) * 1000;
            let timeout = libc::timeval {
                tv_sec: seconds as libc::time_t,
                tv_usec: microseconds as libc::suseconds_t,
            };
            let rc = unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_RCVTIMEO,
                    &timeout as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                )
            };
            if rc != 0 {
                let _ = unsafe { libc::close(fd) };
                return Err(core_platform::net_error("setsockopt(SO_RCVTIMEO)"));
            }
        }

        // register one packet socket resource
        let entry = ResourceEntry::new(ResourceKind::Socket)
            .with_socket(fd)
            .with_finalizer(SocketFinalizer { fd });
        let resource_id =
            binding
                .agent()
                .resources
                .insert(binding.world(), entry, Some(binding.engine()));
        unsafe {
            *out = SocketHandle(resource_id);
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // require one explicit capture interface for BPF backends
        if options.interface_index == 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.interfaceIndex",
                "interface index is required for bpf packet capture",
            ))
            .boxed());
        }

        // open one BPF descriptor
        let descriptor = open_macos_bpf_descriptor()?;

        // configure the BPF kernel buffer for capture frames
        if let Err(error) = configure_macos_bpf_buffer_length(descriptor, options.snap_length) {
            let _ = unsafe { libc::close(descriptor) };
            return Err(error);
        }

        // enable immediate packet delivery semantics
        let immediate = 1 as libc::c_uint;
        let immediate_rc = unsafe { libc::ioctl(descriptor, libc::BIOCIMMEDIATE as _, &immediate) };
        if immediate_rc != 0 {
            let error = core_platform::net_error("ioctl(BIOCIMMEDIATE)");
            let _ = unsafe { libc::close(descriptor) };
            return Err(error);
        }

        // configure one BPF read timeout when requested
        if options.timeout_ms >= 0 {
            let timeout_result = configure_macos_bpf_timeout(descriptor, options.timeout_ms);
            if let Err(error) = timeout_result {
                let _ = unsafe { libc::close(descriptor) };
                return Err(error);
            }
        }

        // bind one interface index to the BPF descriptor
        let interface_name = macos_interface_name(options.interface_index)?;
        if let Err(error) = bind_macos_bpf_interface(descriptor, &interface_name) {
            let _ = unsafe { libc::close(descriptor) };
            return Err(error);
        }

        // enable promiscuous mode when requested
        if options.promiscuous {
            let promiscuous_rc = unsafe { libc::ioctl(descriptor, libc::BIOCPROMISC as _, 0) };
            if promiscuous_rc != 0 {
                let error = core_platform::net_error("ioctl(BIOCPROMISC)");
                let _ = unsafe { libc::close(descriptor) };
                return Err(error);
            }
        }

        // flush one stale packet queue snapshot
        let flush_rc = unsafe { libc::ioctl(descriptor, libc::BIOCFLUSH as _, 0) };
        if flush_rc != 0 {
            let error = core_platform::net_error("ioctl(BIOCFLUSH)");
            let _ = unsafe { libc::close(descriptor) };
            return Err(error);
        }

        // register one packet socket resource
        let entry = ResourceEntry::new(ResourceKind::Socket)
            .with_socket(descriptor)
            .with_finalizer(SocketFinalizer { fd: descriptor });
        let resource_id =
            binding
                .agent()
                .resources
                .insert(binding.world(), entry, Some(binding.engine()));
        unsafe {
            *out = SocketHandle(resource_id);
        }

        Ok(())
    }
}

/// Receive one packet from a packet endpoint.
///
/// Reads one packet record into the provided payload buffer and returns packet metadata.
/// Truncation is reported explicitly when the payload buffer is smaller than the captured frame.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET packet reads on Linux and BPF packet reads on macOS.
/// Returns `notSupported` on Unix targets without a packet backend.
/// Uses one configured host packet backend on Windows.
/// Current Windows backend reads raw IPv4 packets from `SOCK_RAW` capture lanes.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_receive(
    binding: &BindingCallContext,
    out: *mut PacketCaptureRecord,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // reject unix targets without one packet backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, out, handle, payload);
        packet_not_supported("destack.net.packetReceive")
    }

    #[cfg(target_os = "linux")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve the packet descriptor and payload buffer
        let fd = socket_descriptor(binding, handle)?;
        let buffer = unsafe { payload.as_mut_slice()? };

        // read one packet and capture source interface metadata
        let mut source = unsafe { std::mem::zeroed::<libc::sockaddr_ll>() };
        let mut source_length = SOCKADDR_LL_LENGTH;
        let bytes = unsafe {
            libc::recvfrom(
                fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
                libc::MSG_TRUNC,
                &mut source as *mut _ as *mut libc::sockaddr,
                &mut source_length,
            )
        };
        if bytes < 0 {
            return Err(core_platform::net_error("recvfrom"));
        }

        // map packet receive metadata into runtime output
        let total_bytes = u64::try_from(bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "bytes",
                "received byte count is out of range",
            ))
            .boxed()
        })?;
        let written_bytes = total_bytes.min(buffer.len() as u64);
        let interface_index = u32::try_from(source.sll_ifindex).unwrap_or(0);
        let timestamp_ns = monotonic_timestamp_nanoseconds()?;
        let truncated = total_bytes > written_bytes;

        unsafe {
            *out = PacketCaptureRecord {
                bytes: written_bytes,
                interface_index,
                timestamp_ns,
                truncated,
            };
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve the packet descriptor and payload buffer
        let descriptor = socket_descriptor(binding, handle)?;
        let payload = unsafe { payload.as_mut_slice()? };

        // read one full BPF packet buffer from the descriptor
        let buffer_length = macos_bpf_buffer_length(descriptor)?;
        let mut buffer = vec![0u8; buffer_length];
        let read = unsafe {
            libc::read(
                descriptor,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
            )
        };
        if read < 0 {
            return Err(core_platform::net_error("read"));
        }
        if read == 0 {
            return packet_would_block("destack.net.packetReceive");
        }
        let read = usize::try_from(read).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "bytes",
                "packet byte count is out of range",
            ))
            .boxed()
        })?;

        // decode the first BPF packet frame from the returned buffer
        let mut offset = 0usize;
        while offset + std::mem::size_of::<libc::bpf_hdr>() <= read {
            let header = unsafe {
                let pointer = buffer.as_ptr().add(offset) as *const libc::bpf_hdr;
                std::ptr::read_unaligned(pointer)
            };
            let header_length = usize::from(header.bh_hdrlen);
            if header_length < std::mem::size_of::<libc::bpf_hdr>() {
                break;
            }

            let packet_offset = offset.saturating_add(header_length);
            let captured_length = usize::try_from(header.bh_caplen).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "bytes",
                    "captured packet length is out of range",
                ))
                .boxed()
            })?;
            let packet_end = packet_offset.saturating_add(captured_length);
            if packet_offset > read || packet_end > read {
                break;
            }
            if captured_length == 0 {
                let frame_length = header_length;
                let step = macos_bpf_word_align(frame_length);
                if step == 0 {
                    break;
                }

                offset = offset.saturating_add(step);
                continue;
            }

            // copy one captured packet payload into the caller buffer
            let original_length = usize::try_from(header.bh_datalen).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "bytes",
                    "packet length is out of range",
                ))
                .boxed()
            })?;
            let written = captured_length.min(payload.len());
            payload[..written].copy_from_slice(&buffer[packet_offset..packet_offset + written]);

            // map one BPF timestamp into nanoseconds
            let seconds = u64::try_from(header.bh_tstamp.tv_sec).unwrap_or(0);
            let microseconds = u64::try_from(header.bh_tstamp.tv_usec).unwrap_or(0);
            let timestamp_ns = seconds
                .saturating_mul(NANOSECONDS_PER_SECOND)
                .saturating_add(microseconds.saturating_mul(NANOSECONDS_PER_MICROSECOND));
            let truncated = original_length > written || captured_length > written;
            let interface_index = macos_bpf_interface_index(descriptor);

            unsafe {
                *out = PacketCaptureRecord {
                    bytes: written as u64,
                    interface_index,
                    timestamp_ns,
                    truncated,
                };
            }

            return Ok(());
        }

        Err(RuntimeError::from(PlatformError::io(
            "read(BPF): no valid packet frame in capture buffer",
        ))
        .boxed())
    }
}

/// Send one packet through a packet endpoint.
///
/// Writes one raw packet frame from the provided payload buffer.
/// Partial sends are reported through the returned byte count.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET packet writes on Linux and BPF packet writes on macOS.
/// Returns `notSupported` on Unix targets without a packet backend.
/// Uses one configured host packet backend on Windows.
/// Current Windows backend sends raw IPv4 packets through `SOCK_RAW`.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_send(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // reject unix targets without one packet backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, out, handle, payload);
        packet_not_supported("destack.net.packetSend")
    }

    #[cfg(target_os = "linux")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve descriptor and payload bytes
        let fd = socket_descriptor(binding, handle)?;
        let bytes = unsafe { payload.as_slice()? };

        // write one packet frame
        let sent = unsafe {
            libc::send(
                fd,
                bytes.as_ptr() as *const libc::c_void,
                bytes.len(),
                libc::MSG_NOSIGNAL,
            )
        };
        if sent < 0 {
            return Err(core_platform::net_error("send"));
        }

        // write the transmitted byte count
        let sent = u64::try_from(sent).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "bytes",
                "sent byte count is out of range",
            ))
            .boxed()
        })?;
        unsafe {
            *out = sent;
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve descriptor and payload bytes
        let descriptor = socket_descriptor(binding, handle)?;
        let bytes = unsafe { payload.as_slice()? };

        // write one packet frame to the BPF descriptor
        let written = unsafe {
            libc::write(
                descriptor,
                bytes.as_ptr() as *const libc::c_void,
                bytes.len(),
            )
        };
        if written < 0 {
            return Err(core_platform::net_error("write"));
        }

        // write the transmitted byte count
        let written = u64::try_from(written).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "bytes",
                "sent byte count is out of range",
            ))
            .boxed()
        })?;
        unsafe {
            *out = written;
        }

        Ok(())
    }
}

/// Configure packet timestamp mode for a socket or packet endpoint.
///
/// Updates timestamping mode for packet metadata capture on supported backends.
/// Unsupported timestamp modes return notSupported.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_TIMESTAMP families on Linux and BPF timestamp lanes on macOS.
/// Returns `notSupported` on Unix targets without timestamp-capable packet backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_timestamp_mode(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    // reject unix targets without one packet backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, handle, mode);
        packet_not_supported("destack.net.packetSetTimestampMode")
    }

    #[cfg(target_os = "linux")]
    {
        // map runtime timestamp mode to SO_TIMESTAMPNS support
        let enable_value: libc::c_int = match mode {
            PacketTimestampMode::Disabled => 0,
            PacketTimestampMode::Software => 1,
            PacketTimestampMode::Hardware => {
                return packet_not_supported("destack.net.packetSetTimestampMode");
            }
        };

        // apply timestamp option on the packet socket
        let fd = socket_descriptor(binding, handle)?;
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_TIMESTAMPNS,
                &enable_value as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("setsockopt(SO_TIMESTAMPNS)"));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate descriptor ownership
        let _ = socket_descriptor(binding, handle)?;

        // support software timestamps and reject unsupported modes
        if mode == PacketTimestampMode::Software {
            return Ok(());
        }

        packet_not_supported("destack.net.packetSetTimestampMode")
    }
}

/// Clear packet fanout from a packet endpoint.
///
/// Remove this endpoint from any active fanout group.
/// Group teardown behavior and packet redistribution follow host kernel semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_FANOUT reset on Linux and returns `notSupported` where fanout groups are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_clear_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // reject non-linux unix targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        packet_not_supported("destack.net.packetClearFanout")
    }

    #[cfg(target_os = "linux")]
    {
        // clear fanout by resetting PACKET_FANOUT to zero
        let fd = socket_descriptor(binding, handle)?;
        let value: u32 = 0;
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_FANOUT,
                &value as *const _ as *const libc::c_void,
                std::mem::size_of::<u32>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("setsockopt(PACKET_FANOUT)"));
        }

        Ok(())
    }
}

/// Clear the active packet filter program.
///
/// Removes any backend packet filter from the raw endpoint.
/// Filter teardown semantics are host defined.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_DETACH_FILTER on Linux and BIOCSETF reset on macOS.
/// Returns `notSupported` on Unix targets without packet-filter backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_clear_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // reject unix targets without one packet backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, handle);
        packet_not_supported("destack.net.packetClearFilter")
    }

    #[cfg(target_os = "linux")]
    {
        // detach the current classic BPF filter
        let fd = socket_descriptor(binding, handle)?;
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_DETACH_FILTER,
                std::ptr::null(),
                0,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("setsockopt(SO_DETACH_FILTER)"));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // resolve one packet descriptor
        let descriptor = socket_descriptor(binding, handle)?;

        // install one allow-all fallback filter to clear restrictive programs
        let mut instructions = [MacosBpfInstruction {
            code: MACOS_BPF_OPCODE_RETURN | MACOS_BPF_OPCODE_IMMEDIATE,
            jt: 0,
            jf: 0,
            k: MACOS_BPF_ACCEPT_ALL,
        }];
        set_macos_bpf_filter(descriptor, &mut instructions, "ioctl(BIOCSETF)")
    }
}

/// Clear packet rx and tx ring configuration.
///
/// Disable ring-backed packet queues and return to syscall-based send and receive.
/// Pending ring buffers are released according to host packet socket semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_RX_RING and PACKET_TX_RING reset on Linux and returns `notSupported` elsewhere.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_clear_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // reject non-linux unix targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle);
        packet_not_supported("destack.net.packetClearRing")
    }

    #[cfg(target_os = "linux")]
    {
        // reset both packet ring sockets to zero-sized requests
        let fd = socket_descriptor(binding, handle)?;
        let empty = libc::tpacket_req {
            tp_block_size: 0,
            tp_block_nr: 0,
            tp_frame_size: 0,
            tp_frame_nr: 0,
        };

        let rx_rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_RX_RING,
                &empty as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::tpacket_req>() as libc::socklen_t,
            )
        };
        if rx_rc != 0 {
            return Err(core_platform::net_error("setsockopt(PACKET_RX_RING)"));
        }

        let tx_rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_TX_RING,
                &empty as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::tpacket_req>() as libc::socklen_t,
            )
        };
        if tx_rc != 0 {
            return Err(core_platform::net_error("setsockopt(PACKET_TX_RING)"));
        }

        Ok(())
    }
}

/// Set packet fanout on a packet endpoint.
///
/// Attach this endpoint to one kernel packet fanout group with the provided mode.
/// Fanout group behavior and mode-specific flags follow host packet socket semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_FANOUT on Linux and returns `notSupported` where fanout groups are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketFanoutOptions,
) -> RuntimeResult<()> {
    // reject non-linux unix targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, options);
        packet_not_supported("destack.net.packetSetFanout")
    }

    #[cfg(target_os = "linux")]
    {
        // encode fanout options into one host fanout value
        let fd = socket_descriptor(binding, handle)?;
        let mode = packet_fanout_mode(options.mode);
        let fanout_value = u32::from(options.group_id)
            | ((mode & 0xffff) << 16)
            | ((u32::from(options.flags) & 0xffff) << 16);

        // apply PACKET_FANOUT configuration
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_FANOUT,
                &fanout_value as *const _ as *const libc::c_void,
                std::mem::size_of::<u32>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("setsockopt(PACKET_FANOUT)"));
        }

        Ok(())
    }
}

/// Attach one packet filter program to a raw endpoint.
///
/// Installs one backend packet filter program for capture path filtering.
/// Filter verification and accepted instruction sets are host defined.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_ATTACH_FILTER on Linux and BIOCSETF on macOS.
/// Returns `notSupported` on Unix targets without packet-filter backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
    filterprogram: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // reject unix targets without one packet backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, handle, filterprogram);
        packet_not_supported("destack.net.packetSetFilter")
    }

    #[cfg(target_os = "linux")]
    {
        // resolve descriptor and filter bytes
        let fd = socket_descriptor(binding, handle)?;
        let bytes = unsafe { filterprogram.as_slice()? };
        let instruction_size = std::mem::size_of::<libc::sock_filter>();
        if bytes.is_empty() || (bytes.len() % instruction_size) != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "filterProgram",
                "filter program length must be a non-zero multiple of sock_filter size",
            ))
            .boxed());
        }

        // map bytes into classic BPF filter instructions
        let instruction_count = bytes.len() / instruction_size;
        if instruction_count > usize::from(u16::MAX) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "filterProgram",
                "filter program instruction count exceeds host limit",
            ))
            .boxed());
        }
        let instructions = unsafe {
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const libc::sock_filter,
                instruction_count,
            )
        };
        let mut program = libc::sock_fprog {
            len: instruction_count as libc::c_ushort,
            filter: instructions.as_ptr() as *mut libc::sock_filter,
        };

        // attach the filter program to the socket
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_ATTACH_FILTER,
                &mut program as *mut _ as *const libc::c_void,
                std::mem::size_of::<libc::sock_fprog>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("setsockopt(SO_ATTACH_FILTER)"));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // resolve descriptor and filter bytes
        let descriptor = socket_descriptor(binding, handle)?;
        let bytes = unsafe { filterprogram.as_slice()? };
        let instruction_size = std::mem::size_of::<MacosBpfInstruction>();
        if bytes.is_empty() || (bytes.len() % instruction_size) != 0 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "filterProgram",
                "filter program length must be a non-zero multiple of bpf_insn size",
            ))
            .boxed());
        }

        // map bytes into one mutable instruction vector
        let instruction_count = bytes.len() / instruction_size;
        let mut instructions = Vec::with_capacity(instruction_count);
        for row in 0..instruction_count {
            let start = row * instruction_size;
            let end = start + instruction_size;
            let instruction = unsafe {
                let pointer = bytes[start..end].as_ptr() as *const MacosBpfInstruction;
                std::ptr::read_unaligned(pointer)
            };
            instructions.push(instruction);
        }

        // install the requested BPF filter program
        set_macos_bpf_filter(descriptor, instructions.as_mut_slice(), "ioctl(BIOCSETF)")
    }
}

/// Configure one packet rx ring for zero-copy capture.
///
/// Configure one receive ring so packet frames are delivered through kernel ring buffers.
/// Ring geometry is validated by the host kernel and may be clamped or rejected.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_RX_RING on Linux and returns `notSupported` where packet rings are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_rx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    // reject non-linux unix targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, options);
        packet_not_supported("destack.net.packetSetRxRing")
    }

    #[cfg(target_os = "linux")]
    {
        // resolve descriptor and build one ring request
        let fd = socket_descriptor(binding, handle)?;
        let request = libc::tpacket_req {
            tp_block_size: options.block_size,
            tp_block_nr: options.block_count,
            tp_frame_size: options.frame_size,
            tp_frame_nr: options.frame_count,
        };

        // apply packet receive ring configuration
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_RX_RING,
                &request as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::tpacket_req>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("setsockopt(PACKET_RX_RING)"));
        }

        Ok(())
    }
}

/// Configure one packet tx ring for zero-copy transmit.
///
/// Configure one transmit ring so packet frames are queued through kernel ring buffers.
/// Ring geometry is validated by the host kernel and may be clamped or rejected.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_TX_RING on Linux and returns `notSupported` where packet rings are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_tx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    // reject non-linux unix targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, options);
        packet_not_supported("destack.net.packetSetTxRing")
    }

    #[cfg(target_os = "linux")]
    {
        // resolve descriptor and build one ring request
        let fd = socket_descriptor(binding, handle)?;
        let request = libc::tpacket_req {
            tp_block_size: options.block_size,
            tp_block_nr: options.block_count,
            tp_frame_size: options.frame_size,
            tp_frame_nr: options.frame_count,
        };

        // apply packet transmit ring configuration
        let rc = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_TX_RING,
                &request as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::tpacket_req>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("setsockopt(PACKET_TX_RING)"));
        }

        Ok(())
    }
}

/// Read packet capture statistics from one endpoint.
///
/// Reads cumulative backend packet counters for the endpoint.
/// Counter units and reset behavior follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses packet socket stats on Linux and BPF stats on macOS.
/// Returns `notSupported` on Unix targets without packet stats backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_stats(
    binding: &BindingCallContext,
    out: *mut PacketCaptureStats,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // reject unix targets without one packet backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, out, handle);
        packet_not_supported("destack.net.packetStats")
    }

    #[cfg(target_os = "linux")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve descriptor and load host packet statistics
        let fd = socket_descriptor(binding, handle)?;
        let mut stats = unsafe { std::mem::zeroed::<libc::tpacket_stats>() };
        let mut stats_length = std::mem::size_of::<libc::tpacket_stats>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_PACKET,
                libc::PACKET_STATISTICS,
                &mut stats as *mut _ as *mut libc::c_void,
                &mut stats_length,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("getsockopt(PACKET_STATISTICS)"));
        }

        // map host counters into the runtime packet stats shape
        unsafe {
            *out = PacketCaptureStats {
                received_packets: u64::from(stats.tp_packets),
                dropped_packets: u64::from(stats.tp_drops),
                interface_dropped_packets: 0,
            };
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // resolve descriptor and load host packet statistics
        let descriptor = socket_descriptor(binding, handle)?;
        let mut stats = unsafe { std::mem::zeroed::<MacosBpfStats>() };
        let rc = unsafe { libc::ioctl(descriptor, libc::BIOCGSTATS as _, &mut stats) };
        if rc != 0 {
            return Err(core_platform::net_error("ioctl(BIOCGSTATS)"));
        }

        // map host counters into the runtime packet stats shape
        unsafe {
            *out = PacketCaptureStats {
                received_packets: u64::from(stats.received_packets),
                dropped_packets: u64::from(stats.dropped_packets),
                interface_dropped_packets: 0,
            };
        }

        Ok(())
    }
}
