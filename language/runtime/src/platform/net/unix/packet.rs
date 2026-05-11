#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::core::*;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::net::core::select_packet_backend_for_open;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeSlice;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::*;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::resource::{ResourceFinalizer, ResourceId};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::collections::HashMap;

#[cfg(target_os = "macos")]
use std::ffi::{CStr, CString};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::io::RawFd;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::sync::LazyLock;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use parking_lot::Mutex;

/// One sockaddr storage length in bytes.
#[cfg(target_os = "linux")]
const SOCKADDR_LL_LENGTH: libc::socklen_t =
    std::mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;
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

/// Runtime packet metadata for one Unix packet endpoint.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Clone, Copy)]
struct UnixPacketState {
    /// Timestamp mode for packet capture records.
    timestamp_mode: PacketTimestampMode,
}

/// Finalizer for packet endpoints that also clears packet metadata rows.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[derive(Debug)]
struct UnixPacketFinalizer {
    /// Raw Unix packet descriptor.
    fd: RawFd,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl ResourceFinalizer for UnixPacketFinalizer {
    /// Close the packet descriptor and clear packet metadata for the resource.
    fn finalize(self: Box<Self>, resource_id: ResourceId) {
        // close the descriptor first
        unsafe {
            libc::close(self.fd);
        }

        // clear the packet metadata row
        PACKET_SOCKET_STATES.lock().remove(&resource_id);
    }
}

/// Packet-state table for Unix packet endpoints.
#[cfg(any(target_os = "linux", target_os = "macos"))]
static PACKET_SOCKET_STATES: LazyLock<Mutex<HashMap<ResourceId, UnixPacketState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Resolve one packet-state row for one packet endpoint handle.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn packet_socket_state(handle: SocketHandle) -> RuntimeResult<UnixPacketState> {
    PACKET_SOCKET_STATES
        .lock()
        .get(&handle.0)
        .copied()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "socket handle is not one packet endpoint",
            ))
            .boxed()
        })
}

/// Update one packet-state row in place.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn update_packet_socket_state(
    handle: SocketHandle,
    update: impl FnOnce(&mut UnixPacketState),
) -> RuntimeResult<()> {
    let mut states = PACKET_SOCKET_STATES.lock();
    let state = states.get_mut(&handle.0).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "socket handle is not one packet endpoint",
        ))
        .boxed()
    })?;
    update(state);

    Ok(())
}

/// Convert one positive second count into nanoseconds.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn seconds_to_nanos(seconds: i128, operation: &'static str) -> RuntimeResult<u64> {
    if seconds < 0 {
        return Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "packet timestamp reported negative seconds",
        ));
    }

    let seconds = u128::try_from(seconds).map_err(|_| {
        core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "packet timestamp seconds are out of range",
        )
    })?;
    let nanos = seconds.checked_mul(1_000_000_000u128).ok_or_else(|| {
        core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "packet timestamp seconds overflow nanoseconds",
        )
    })?;

    Ok(nanos.min(u64::MAX as u128) as u64)
}

/// Convert one host `timespec` packet timestamp into nanoseconds.
#[cfg(target_os = "linux")]
fn timespec_to_nanos(spec: libc::timespec, operation: &'static str) -> RuntimeResult<u64> {
    if spec.tv_nsec < 0 || spec.tv_nsec >= 1_000_000_000 {
        return Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "packet timestamp reported invalid nanoseconds",
        ));
    }

    let seconds_nanos = seconds_to_nanos(spec.tv_sec as i128, operation)?;
    let nanos = u64::try_from(spec.tv_nsec).map_err(|_| {
        core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "packet timestamp nanoseconds are out of range",
        )
    })?;

    Ok(seconds_nanos.saturating_add(nanos))
}

/// Convert one host `timeval` packet timestamp into nanoseconds.
#[cfg(target_os = "macos")]
fn timeval_to_nanos(
    seconds: i128,
    microseconds: i128,
    operation: &'static str,
) -> RuntimeResult<u64> {
    if microseconds < 0 || microseconds >= 1_000_000 {
        return Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "packet timestamp reported invalid microseconds",
        ));
    }

    let seconds_nanos = seconds_to_nanos(seconds, operation)?;
    let microseconds = u64::try_from(microseconds).map_err(|_| {
        core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "packet timestamp microseconds are out of range",
        )
    })?;
    let nanos = microseconds.saturating_mul(1_000);

    Ok(seconds_nanos.saturating_add(nanos))
}

/// Resolve one Linux packet timestamp from `recvmsg` control data.
#[cfg(target_os = "linux")]
fn linux_packet_timestamp_ns(
    message: &libc::msghdr,
    timestamp_mode: PacketTimestampMode,
) -> RuntimeResult<(PacketTimestampClock, u64)> {
    // suppress packet timestamps when the mode is disabled
    if timestamp_mode == PacketTimestampMode::Disabled {
        return Ok((PacketTimestampClock::None, 0));
    }

    // scan the ancillary payload for the kernel software timestamp
    let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(message) };
    while !cmsg.is_null() {
        let header = unsafe { &*cmsg };
        if header.cmsg_level == libc::SOL_SOCKET && header.cmsg_type == libc::SCM_TIMESTAMPNS {
            let timestamp = unsafe { *(libc::CMSG_DATA(cmsg) as *const libc::timespec) };
            let timestamp_wall_ns = timespec_to_nanos(timestamp, "destack.net.packetReceive")?;

            return Ok((PacketTimestampClock::Wall, timestamp_wall_ns));
        }

        cmsg = unsafe { libc::CMSG_NXTHDR(message, cmsg) };
    }

    Err(core_platform::io_operation_error(
        "destack.net.packetReceive",
        Some(PlatformErrorCode::IoInvalidData),
        "recvmsg did not include the requested packet timestamp",
    ))
}

/// Resolve one captured macOS BPF packet timestamp.
#[cfg(target_os = "macos")]
fn macos_packet_timestamp_ns(
    header: &libc::bpf_hdr,
    timestamp_mode: PacketTimestampMode,
) -> RuntimeResult<(PacketTimestampClock, u64)> {
    // suppress packet timestamps when the mode is disabled
    if timestamp_mode == PacketTimestampMode::Disabled {
        return Ok((PacketTimestampClock::None, 0));
    }

    let timestamp_wall_ns = timeval_to_nanos(
        header.bh_tstamp.tv_sec as i128,
        header.bh_tstamp.tv_usec as i128,
        "destack.net.packetReceive",
    )?;

    Ok((PacketTimestampClock::Wall, timestamp_wall_ns))
}

/// Configure Linux packet timestamping for one packet endpoint.
#[cfg(target_os = "linux")]
fn configure_linux_packet_timestamp_mode(
    fd: RawFd,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    // map runtime timestamp mode to Linux software timestamp support
    let enable_value: libc::c_int = match mode {
        PacketTimestampMode::Disabled => 0,
        PacketTimestampMode::Software => 1,
        PacketTimestampMode::Hardware => {
            return packet_not_supported("destack.net.packetSetTimestampMode");
        }
    };

    // apply the socket option directly to the packet descriptor
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
        // validate requested backend selection before opening resources
        let _backend = select_packet_backend_for_open(binding, options, "destack.net.packetOpen")?;

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

        // enable software timestamps by default so packet capture records stay populated
        if let Err(error) = configure_linux_packet_timestamp_mode(fd, PacketTimestampMode::Software)
        {
            let _ = unsafe { libc::close(fd) };
            return Err(error);
        }

        // register one packet socket resource
        let entry = ResourceEntry::new(ResourceKind::Socket)
            .with_socket(fd)
            .with_finalizer(UnixPacketFinalizer { fd });
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));
        PACKET_SOCKET_STATES.lock().insert(
            resource_id,
            UnixPacketState {
                timestamp_mode: PacketTimestampMode::Software,
            },
        );
        unsafe {
            *out = SocketHandle(resource_id);
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate requested backend selection before opening resources
        let _backend = select_packet_backend_for_open(binding, options, "destack.net.packetOpen")?;

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
            .with_finalizer(UnixPacketFinalizer { fd: descriptor });
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));
        PACKET_SOCKET_STATES.lock().insert(
            resource_id,
            UnixPacketState {
                timestamp_mode: PacketTimestampMode::Software,
            },
        );
        unsafe {
            *out = SocketHandle(resource_id);
        }

        Ok(())
    }
}

/// Receive one packet from a packet endpoint.
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

        // resolve the packet descriptor, endpoint state, and payload buffer
        let fd = socket_descriptor(binding, handle)?;
        let state = packet_socket_state(handle)?;
        let buffer = unsafe { payload.as_mut_slice()? };

        // build one recvmsg payload with source-address and optional timestamp control
        let mut source = unsafe { std::mem::zeroed::<libc::sockaddr_ll>() };
        let mut iovec = libc::iovec {
            iov_base: buffer.as_mut_ptr() as *mut libc::c_void,
            iov_len: buffer.len(),
        };
        let mut control = if state.timestamp_mode == PacketTimestampMode::Software {
            vec![
                0u8;
                unsafe { libc::CMSG_SPACE(std::mem::size_of::<libc::timespec>() as u32) } as usize
            ]
        } else {
            Vec::new()
        };
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_name = &mut source as *mut _ as *mut libc::c_void;
        message.msg_namelen = SOCKADDR_LL_LENGTH;
        message.msg_iov = &mut iovec;
        message.msg_iovlen = 1;
        if !control.is_empty() {
            message.msg_control = control.as_mut_ptr() as *mut libc::c_void;
            message.msg_controllen = control.len();
        }

        // receive one packet and preserve kernel-provided timestamp control data
        let bytes = unsafe { libc::recvmsg(fd, &mut message, libc::MSG_TRUNC) };
        if bytes < 0 {
            return Err(core_platform::net_error("recvmsg"));
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
        let (timestamp_clock, timestamp_ns) =
            linux_packet_timestamp_ns(&message, state.timestamp_mode)?;
        let truncated = total_bytes > written_bytes || (message.msg_flags & libc::MSG_TRUNC) != 0;

        unsafe {
            *out = PacketCaptureRecord {
                bytes: written_bytes,
                interface_index,
                timestamp_clock,
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

        // resolve the packet descriptor, endpoint state, and payload buffer
        let descriptor = socket_descriptor(binding, handle)?;
        let state = packet_socket_state(handle)?;
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

            // convert the BPF packet timestamp into the shared runtime monotonic domain
            let (timestamp_clock, timestamp_ns) =
                macos_packet_timestamp_ns(&header, state.timestamp_mode)?;
            let truncated = original_length > written || captured_length > written;
            let interface_index = macos_bpf_interface_index(descriptor);

            unsafe {
                *out = PacketCaptureRecord {
                    bytes: written as u64,
                    interface_index,
                    timestamp_clock,
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
        // apply Linux packet timestamp mode and persist it in endpoint state
        let fd = socket_descriptor(binding, handle)?;
        let _ = packet_socket_state(handle)?;
        configure_linux_packet_timestamp_mode(fd, mode)?;
        update_packet_socket_state(handle, |state| {
            state.timestamp_mode = mode;
        })?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate descriptor ownership and persist the requested software-timestamp mode
        let _ = socket_descriptor(binding, handle)?;
        let _ = packet_socket_state(handle)?;
        if mode == PacketTimestampMode::Hardware {
            return packet_not_supported("destack.net.packetSetTimestampMode");
        }

        update_packet_socket_state(handle, |state| {
            state.timestamp_mode = mode;
        })?;

        Ok(())
    }
}

/// Clear packet fanout from a packet endpoint.
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

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;

    /// Convert one valid timespec packet timestamp into nanoseconds.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_timespec_to_nanos_converts_valid_packet_timestamp() {
        let spec = libc::timespec {
            tv_sec: 12,
            tv_nsec: 345,
        };

        let nanos = timespec_to_nanos(spec, "destack.net.packetReceive")
            .expect("timespec packet timestamp should convert");

        assert_eq!(nanos, 12_000_000_345);
    }

    /// Return zero packet timestamp output when Linux timestamp mode is disabled.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_packet_timestamp_ns_returns_zero_when_disabled() {
        let message: libc::msghdr = unsafe { std::mem::zeroed() };

        let (timestamp_clock, timestamp_ns) =
            linux_packet_timestamp_ns(&message, PacketTimestampMode::Disabled)
                .expect("disabled Linux packet timestamp mode should succeed");

        assert_eq!(timestamp_clock, PacketTimestampClock::None);
        assert_eq!(timestamp_ns, 0);
    }

    /// Convert one valid timeval packet timestamp into nanoseconds.
    #[cfg(target_os = "macos")]
    #[test]
    fn test_timeval_to_nanos_converts_valid_packet_timestamp() {
        let nanos = timeval_to_nanos(12, 345, "destack.net.packetReceive")
            .expect("timeval packet timestamp should convert");

        assert_eq!(nanos, 12_000_345_000);
    }

    /// Return zero packet timestamp output when macOS timestamp mode is disabled.
    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_packet_timestamp_ns_returns_zero_when_disabled() {
        let header: libc::bpf_hdr = unsafe { std::mem::zeroed() };

        let (timestamp_clock, timestamp_ns) =
            macos_packet_timestamp_ns(&header, PacketTimestampMode::Disabled)
                .expect("disabled macOS packet timestamp mode should succeed");

        assert_eq!(timestamp_clock, PacketTimestampClock::None);
        assert_eq!(timestamp_ns, 0);
    }
}
