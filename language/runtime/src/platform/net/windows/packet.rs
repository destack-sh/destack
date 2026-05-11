use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use destack_workspace::PlatformWindowsPacketBackend;
use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, IN_ADDR, IN_ADDR_0, INVALID_SOCKET, IPPROTO_IP, RCVALL_OFF, RCVALL_ON, SIO_RCVALL,
    SO_RCVTIMEO, SOCK_RAW, SOCKADDR, SOCKADDR_IN, SOCKET, SOCKET_ERROR, SOL_SOCKET, WSAETIMEDOUT,
    WSAGetLastError, WSAIoctl, bind, closesocket, recv, send, setsockopt, socket,
};

use super::util::{ensure_winsock, net_error_with_code, socket_descriptor};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::core::select_packet_backend_for_open;
use crate::platform::net::*;
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind, SocketHandle};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::BindingCallContext;

/// Maximum packet buffer length used by the Windows packet backend.
const WINDOWS_PACKET_MAX_LENGTH: usize = 65_535;
/// Number of cBPF scratch memory words.
const BPF_MEMORY_WORDS: usize = 16;
/// cBPF instruction class mask.
const BPF_CLASS_MASK: u16 = 0x07;
/// cBPF instruction mode mask.
const BPF_MODE_MASK: u16 = 0xe0;
/// cBPF instruction size mask.
const BPF_SIZE_MASK: u16 = 0x18;
/// cBPF arithmetic and jump operation mask.
const BPF_OPERATION_MASK: u16 = 0xf0;
/// cBPF source selector bit.
const BPF_SOURCE_X: u16 = 0x08;
/// cBPF return value selector mask.
const BPF_RETURN_MASK: u16 = 0x18;
/// cBPF miscellaneous operation mask.
const BPF_MISC_MASK: u16 = 0xf8;
/// cBPF class: load A.
const BPF_CLASS_LD: u16 = 0x00;
/// cBPF class: load X.
const BPF_CLASS_LDX: u16 = 0x01;
/// cBPF class: store A.
const BPF_CLASS_ST: u16 = 0x02;
/// cBPF class: store X.
const BPF_CLASS_STX: u16 = 0x03;
/// cBPF class: ALU.
const BPF_CLASS_ALU: u16 = 0x04;
/// cBPF class: jump.
const BPF_CLASS_JMP: u16 = 0x05;
/// cBPF class: return.
const BPF_CLASS_RET: u16 = 0x06;
/// cBPF class: misc.
const BPF_CLASS_MISC: u16 = 0x07;
/// cBPF load mode: immediate.
const BPF_MODE_IMMEDIATE: u16 = 0x00;
/// cBPF load mode: absolute packet offset.
const BPF_MODE_ABSOLUTE: u16 = 0x20;
/// cBPF load mode: X + packet offset.
const BPF_MODE_INDIRECT: u16 = 0x40;
/// cBPF load mode: scratch memory.
const BPF_MODE_MEMORY: u16 = 0x60;
/// cBPF load mode: packet length.
const BPF_MODE_LENGTH: u16 = 0x80;
/// cBPF load mode: nibble shift helper.
const BPF_MODE_MSH: u16 = 0xa0;
/// cBPF load size: 32-bit word.
const BPF_SIZE_WORD: u16 = 0x00;
/// cBPF load size: 16-bit half.
const BPF_SIZE_HALF: u16 = 0x08;
/// cBPF load size: 8-bit byte.
const BPF_SIZE_BYTE: u16 = 0x10;
/// cBPF ALU and jump op: add.
const BPF_OPERATION_ADD: u16 = 0x00;
/// cBPF ALU and jump op: subtract.
const BPF_OPERATION_SUB: u16 = 0x10;
/// cBPF ALU and jump op: multiply.
const BPF_OPERATION_MUL: u16 = 0x20;
/// cBPF ALU and jump op: divide.
const BPF_OPERATION_DIV: u16 = 0x30;
/// cBPF ALU and jump op: bitwise or.
const BPF_OPERATION_OR: u16 = 0x40;
/// cBPF ALU and jump op: bitwise and.
const BPF_OPERATION_AND: u16 = 0x50;
/// cBPF ALU and jump op: left shift.
const BPF_OPERATION_LSH: u16 = 0x60;
/// cBPF ALU and jump op: right shift.
const BPF_OPERATION_RSH: u16 = 0x70;
/// cBPF ALU and jump op: negate.
const BPF_OPERATION_NEG: u16 = 0x80;
/// cBPF ALU and jump op: modulo.
const BPF_OPERATION_MOD: u16 = 0x90;
/// cBPF ALU and jump op: xor.
const BPF_OPERATION_XOR: u16 = 0xa0;
/// cBPF jump op: unconditional jump.
const BPF_OPERATION_JA: u16 = 0x00;
/// cBPF jump op: equals.
const BPF_OPERATION_JEQ: u16 = 0x10;
/// cBPF jump op: greater-than.
const BPF_OPERATION_JGT: u16 = 0x20;
/// cBPF jump op: greater-or-equal.
const BPF_OPERATION_JGE: u16 = 0x30;
/// cBPF jump op: bit-test.
const BPF_OPERATION_JSET: u16 = 0x40;
/// cBPF return mode: return constant K.
const BPF_RETURN_K: u16 = 0x00;
/// cBPF return mode: return accumulator A.
const BPF_RETURN_A: u16 = 0x10;
/// cBPF misc op: transfer A to X.
const BPF_MISC_TAX: u16 = 0x00;
/// cBPF misc op: transfer X to A.
const BPF_MISC_TXA: u16 = 0x80;

/// One classic BPF instruction row.
#[derive(Clone, Copy)]
#[repr(C)]
struct ClassicBpfInstruction {
    /// BPF opcode.
    code: u16,
    /// Jump offset when condition is true.
    jt: u8,
    /// Jump offset when condition is false.
    jf: u8,
    /// Immediate constant operand.
    k: u32,
}

/// Runtime packet-socket metadata for one Windows raw socket endpoint.
#[derive(Clone)]
struct WindowsPacketState {
    /// Bound interface index reported in capture records.
    interface_index: u32,
    /// Effective snap length used for receive truncation.
    snap_length: usize,
    /// Total packets observed by this endpoint.
    received_packets: u64,
    /// Total packets truncated to caller buffers or snap length.
    dropped_packets: u64,
    /// Total interface-drop packets when available from host APIs.
    interface_dropped_packets: u64,
    /// Optional classic-BPF filter program applied in receive path.
    filter_program: Option<Arc<[ClassicBpfInstruction]>>,
}

/// Finalizer for packet sockets that also clears packet state rows.
#[derive(Debug)]
struct WindowsPacketFinalizer {
    /// Raw WinSock socket descriptor.
    socket: SOCKET,
}

impl WindowsPacketFinalizer {
    /// Build one packet-socket finalizer for one raw socket.
    fn new(socket: SOCKET) -> Self {
        Self { socket }
    }
}

impl ResourceFinalizer for WindowsPacketFinalizer {
    /// Close one packet socket and clear its packet-state row.
    fn finalize(self: Box<Self>, resource_id: ResourceId) {
        // close the packet socket descriptor
        unsafe {
            closesocket(self.socket);
        }

        // clear packet metadata for this resource id
        PACKET_SOCKET_STATES.lock().remove(&resource_id);
    }
}

/// Packet-state table for Windows packet sockets.
static PACKET_SOCKET_STATES: LazyLock<Mutex<HashMap<ResourceId, WindowsPacketState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Return configured packet backend mode for Windows packet lanes.
fn windows_packet_backend_mode(binding: &BindingCallContext) -> PlatformWindowsPacketBackend {
    binding.worker().options.platform.windows.net_packet_backend
}

/// Return one `notSupported` error for unsupported Windows packet lanes.
fn windows_packet_not_supported(operation: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Require that the Windows packet backend is enabled in runtime options.
fn require_windows_packet_backend(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<()> {
    match windows_packet_backend_mode(binding) {
        PlatformWindowsPacketBackend::RawSocket => Ok(()),
        PlatformWindowsPacketBackend::HostBackend => windows_packet_not_supported(operation),
        PlatformWindowsPacketBackend::Disabled => windows_packet_not_supported(operation),
    }
}

/// Return one `ioWouldBlock` timeout error for packet receive.
fn packet_receive_timeout_error(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        None,
        None,
        None,
        format!("{operation}: packet receive timed out"),
    ))
    .boxed()
}

/// Return one default IPv4 bind address for packet sockets.
fn any_ipv4_bind_address() -> SOCKADDR_IN {
    SOCKADDR_IN {
        sin_family: AF_INET,
        sin_port: 0,
        sin_addr: IN_ADDR {
            S_un: IN_ADDR_0 { S_addr: 0 },
        },
        sin_zero: [0; 8],
    }
}

/// Resolve one interface index to one IPv4 bind address.
fn interface_ipv4_bind_address(interface_index: u32) -> RuntimeResult<SOCKADDR_IN> {
    // query adapter rows with one growable address buffer
    let mut size = 15_000u32;
    let mut buffer = vec![0u8; size as usize];
    let status = loop {
        let status = unsafe {
            GetAdaptersAddresses(
                AF_INET as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                &mut size,
            )
        };

        if status == ERROR_BUFFER_OVERFLOW {
            buffer.resize(size as usize, 0u8);
            continue;
        }

        break status;
    };
    if status != ERROR_SUCCESS {
        return Err(net_error_with_code("GetAdaptersAddresses", status as i32));
    }

    // locate one adapter row that matches the requested index
    let mut adapter = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
    while !adapter.is_null() {
        let row = unsafe { &*adapter };
        let mut index = unsafe { row.Anonymous1.Anonymous.IfIndex };
        if index == 0 {
            index = row.Ipv6IfIndex;
        }

        if index == interface_index {
            // locate one IPv4 unicast address for the adapter row
            let mut unicast = row.FirstUnicastAddress;
            while !unicast.is_null() {
                let address_row = unsafe { &*unicast };
                let socket_address = address_row.Address;

                if socket_address.lpSockaddr.is_null() || socket_address.iSockaddrLength <= 0 {
                    unicast = address_row.Next;
                    continue;
                }

                if socket_address.iSockaddrLength as usize >= std::mem::size_of::<SOCKADDR_IN>() {
                    let address = unsafe { &*(socket_address.lpSockaddr as *const SOCKADDR_IN) };
                    if address.sin_family as i32 == AF_INET as i32 {
                        let mut bind_address = any_ipv4_bind_address();
                        bind_address.sin_addr.S_un.S_addr = unsafe { address.sin_addr.S_un.S_addr };
                        return Ok(bind_address);
                    }
                }

                unicast = address_row.Next;
            }

            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.interfaceIndex",
                "interface has no IPv4 unicast address",
            ))
            .boxed());
        }

        adapter = row.Next;
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "options.interfaceIndex",
        "interface index not found",
    ))
    .boxed())
}

/// Configure receive timeout for one packet socket when timeout is finite.
fn configure_packet_receive_timeout(socket: SOCKET, timeout_ms: i32) -> RuntimeResult<()> {
    if timeout_ms < 0 {
        return Ok(());
    }

    let timeout = timeout_ms;
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_RCVTIMEO,
            &timeout as *const _ as *const u8,
            std::mem::size_of::<i32>() as i32,
        )
    };
    if rc != 0 {
        return Err(net_error_with_code("setsockopt(SO_RCVTIMEO)", unsafe {
            WSAGetLastError()
        }));
    }

    Ok(())
}

/// Configure packet promiscuous capture mode for one raw socket.
fn configure_packet_promiscuous_mode(socket: SOCKET, promiscuous: bool) -> RuntimeResult<()> {
    // choose the RCVALL mode from the requested capture mode
    let mut mode = if promiscuous { RCVALL_ON } else { RCVALL_OFF };
    let mut bytes_returned = 0u32;
    let rc = unsafe {
        WSAIoctl(
            socket,
            SIO_RCVALL,
            &mut mode as *mut _ as *mut _,
            std::mem::size_of_val(&mode) as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
            None,
        )
    };
    if rc != 0 {
        return Err(net_error_with_code("WSAIoctl(SIO_RCVALL)", unsafe {
            WSAGetLastError()
        }));
    }

    Ok(())
}

/// Resolve one packet socket descriptor and packet metadata row.
fn packet_socket_metadata(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<(SOCKET, WindowsPacketState)> {
    // resolve one socket descriptor from the resource table
    let socket = socket_descriptor(binding, handle)?;

    // resolve one packet metadata row for packet-only lanes
    let state = PACKET_SOCKET_STATES
        .lock()
        .get(&handle.0)
        .cloned()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "socket handle is not one packet endpoint",
            ))
            .boxed()
        })?;

    Ok((socket, state))
}

/// Read one packet byte from one absolute offset.
fn read_packet_u8(packet: &[u8], offset: usize) -> Option<u8> {
    packet.get(offset).copied()
}

/// Read one packet half-word in network byte order.
fn read_packet_u16(packet: &[u8], offset: usize) -> Option<u16> {
    let bytes = packet.get(offset..offset.saturating_add(2))?;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

/// Read one packet word in network byte order.
fn read_packet_u32(packet: &[u8], offset: usize) -> Option<u32> {
    let bytes = packet.get(offset..offset.saturating_add(4))?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Decode one classic-BPF byte payload into instruction rows.
fn decode_filter_program(bytes: &[u8]) -> RuntimeResult<Vec<ClassicBpfInstruction>> {
    let instruction_size = std::mem::size_of::<ClassicBpfInstruction>();
    if bytes.is_empty() || !bytes.len().is_multiple_of(instruction_size) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "filterProgram",
            "filter program length must be a non-zero multiple of bpf instruction size",
        ))
        .boxed());
    }

    let instruction_count = bytes.len() / instruction_size;
    if instruction_count > usize::from(u16::MAX) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "filterProgram",
            "filter program instruction count exceeds host limit",
        ))
        .boxed());
    }

    let mut instructions = Vec::with_capacity(instruction_count);
    for row in 0..instruction_count {
        let start = row * instruction_size;
        let end = start + instruction_size;
        let instruction = unsafe {
            let pointer = bytes[start..end].as_ptr() as *const ClassicBpfInstruction;
            std::ptr::read_unaligned(pointer)
        };
        instructions.push(instruction);
    }

    Ok(instructions)
}

/// Validate one decoded classic-BPF instruction sequence.
fn validate_filter_program(instructions: &[ClassicBpfInstruction]) -> RuntimeResult<()> {
    if instructions.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "filterProgram",
            "filter program must contain at least one instruction",
        ))
        .boxed());
    }

    for (program_counter, instruction) in instructions.iter().enumerate() {
        let class = instruction.code & BPF_CLASS_MASK;
        match class {
            BPF_CLASS_LD | BPF_CLASS_LDX => {
                let mode = instruction.code & BPF_MODE_MASK;
                if mode == BPF_MODE_MEMORY && (instruction.k as usize) >= BPF_MEMORY_WORDS {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "filterProgram",
                        "bpf memory load index exceeds scratch space",
                    ))
                    .boxed());
                }

                if class == BPF_CLASS_LDX
                    && mode == BPF_MODE_MSH
                    && (instruction.code & BPF_SIZE_MASK) != BPF_SIZE_BYTE
                {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "filterProgram",
                        "bpf ldx msh mode requires byte size",
                    ))
                    .boxed());
                }
            }
            BPF_CLASS_ST | BPF_CLASS_STX => {
                if (instruction.k as usize) >= BPF_MEMORY_WORDS {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "filterProgram",
                        "bpf memory store index exceeds scratch space",
                    ))
                    .boxed());
                }
            }
            BPF_CLASS_ALU => {
                let operation = instruction.code & BPF_OPERATION_MASK;
                if (operation == BPF_OPERATION_DIV || operation == BPF_OPERATION_MOD)
                    && (instruction.code & BPF_SOURCE_X) == 0
                    && instruction.k == 0
                {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "filterProgram",
                        "bpf division by zero is invalid",
                    ))
                    .boxed());
                }
            }
            BPF_CLASS_JMP => {
                let operation = instruction.code & BPF_OPERATION_MASK;
                if operation == BPF_OPERATION_JA {
                    let target = program_counter
                        .saturating_add(1)
                        .saturating_add(instruction.k as usize);
                    if target >= instructions.len() {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "filterProgram",
                            "bpf jump target is out of bounds",
                        ))
                        .boxed());
                    }
                    continue;
                }

                let true_target = program_counter
                    .saturating_add(1)
                    .saturating_add(usize::from(instruction.jt));
                let false_target = program_counter
                    .saturating_add(1)
                    .saturating_add(usize::from(instruction.jf));
                if true_target >= instructions.len() || false_target >= instructions.len() {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "filterProgram",
                        "bpf conditional jump target is out of bounds",
                    ))
                    .boxed());
                }
            }
            BPF_CLASS_RET => {}
            BPF_CLASS_MISC => {}
            _ => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "filterProgram",
                    "bpf instruction class is invalid",
                ))
                .boxed());
            }
        }
    }

    let last = instructions[instructions.len() - 1];
    if (last.code & BPF_CLASS_MASK) != BPF_CLASS_RET {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "filterProgram",
            "bpf program must end with one return instruction",
        ))
        .boxed());
    }

    Ok(())
}

/// Evaluate one classic-BPF program against one packet payload.
fn evaluate_filter_program(instructions: &[ClassicBpfInstruction], packet: &[u8]) -> u32 {
    let mut accumulator = 0u32;
    let mut index = 0u32;
    let mut memory = [0u32; BPF_MEMORY_WORDS];
    let mut program_counter = 0usize;
    while program_counter < instructions.len() {
        let instruction = instructions[program_counter];
        let class = instruction.code & BPF_CLASS_MASK;
        match class {
            BPF_CLASS_LD => {
                let mode = instruction.code & BPF_MODE_MASK;
                let size = instruction.code & BPF_SIZE_MASK;
                match mode {
                    BPF_MODE_IMMEDIATE => {
                        accumulator = instruction.k;
                    }
                    BPF_MODE_LENGTH => {
                        accumulator = packet.len() as u32;
                    }
                    BPF_MODE_MEMORY => {
                        let slot = instruction.k as usize;
                        if slot >= BPF_MEMORY_WORDS {
                            return 0;
                        }
                        accumulator = memory[slot];
                    }
                    BPF_MODE_ABSOLUTE => {
                        let offset = instruction.k as usize;
                        accumulator = match size {
                            BPF_SIZE_WORD => read_packet_u32(packet, offset).unwrap_or(0),
                            BPF_SIZE_HALF => read_packet_u16(packet, offset).unwrap_or(0) as u32,
                            BPF_SIZE_BYTE => read_packet_u8(packet, offset).unwrap_or(0) as u32,
                            _ => return 0,
                        };
                    }
                    BPF_MODE_INDIRECT => {
                        let offset = index.saturating_add(instruction.k) as usize;
                        accumulator = match size {
                            BPF_SIZE_WORD => read_packet_u32(packet, offset).unwrap_or(0),
                            BPF_SIZE_HALF => read_packet_u16(packet, offset).unwrap_or(0) as u32,
                            BPF_SIZE_BYTE => read_packet_u8(packet, offset).unwrap_or(0) as u32,
                            _ => return 0,
                        };
                    }
                    _ => return 0,
                }

                program_counter = program_counter.saturating_add(1);
            }
            BPF_CLASS_LDX => {
                let mode = instruction.code & BPF_MODE_MASK;
                let size = instruction.code & BPF_SIZE_MASK;
                match mode {
                    BPF_MODE_IMMEDIATE => {
                        index = instruction.k;
                    }
                    BPF_MODE_LENGTH => {
                        index = packet.len() as u32;
                    }
                    BPF_MODE_MEMORY => {
                        let slot = instruction.k as usize;
                        if slot >= BPF_MEMORY_WORDS {
                            return 0;
                        }
                        index = memory[slot];
                    }
                    BPF_MODE_MSH => {
                        if size != BPF_SIZE_BYTE {
                            return 0;
                        }
                        let offset = instruction.k as usize;
                        let value = read_packet_u8(packet, offset).unwrap_or(0);
                        index = u32::from(value & 0x0f).saturating_mul(4);
                    }
                    _ => return 0,
                }

                program_counter = program_counter.saturating_add(1);
            }
            BPF_CLASS_ST => {
                let slot = instruction.k as usize;
                if slot >= BPF_MEMORY_WORDS {
                    return 0;
                }
                memory[slot] = accumulator;
                program_counter = program_counter.saturating_add(1);
            }
            BPF_CLASS_STX => {
                let slot = instruction.k as usize;
                if slot >= BPF_MEMORY_WORDS {
                    return 0;
                }
                memory[slot] = index;
                program_counter = program_counter.saturating_add(1);
            }
            BPF_CLASS_ALU => {
                let source = if (instruction.code & BPF_SOURCE_X) != 0 {
                    index
                } else {
                    instruction.k
                };
                let operation = instruction.code & BPF_OPERATION_MASK;
                accumulator = match operation {
                    BPF_OPERATION_ADD => accumulator.wrapping_add(source),
                    BPF_OPERATION_SUB => accumulator.wrapping_sub(source),
                    BPF_OPERATION_MUL => accumulator.wrapping_mul(source),
                    BPF_OPERATION_DIV => {
                        if source == 0 {
                            return 0;
                        }
                        accumulator / source
                    }
                    BPF_OPERATION_OR => accumulator | source,
                    BPF_OPERATION_AND => accumulator & source,
                    BPF_OPERATION_LSH => accumulator.wrapping_shl(source & 31),
                    BPF_OPERATION_RSH => accumulator.wrapping_shr(source & 31),
                    BPF_OPERATION_NEG => (!accumulator).wrapping_add(1),
                    BPF_OPERATION_MOD => {
                        if source == 0 {
                            return 0;
                        }
                        accumulator % source
                    }
                    BPF_OPERATION_XOR => accumulator ^ source,
                    _ => return 0,
                };

                program_counter = program_counter.saturating_add(1);
            }
            BPF_CLASS_JMP => {
                let source = if (instruction.code & BPF_SOURCE_X) != 0 {
                    index
                } else {
                    instruction.k
                };
                let operation = instruction.code & BPF_OPERATION_MASK;
                if operation == BPF_OPERATION_JA {
                    let target = program_counter
                        .saturating_add(1)
                        .saturating_add(instruction.k as usize);
                    if target >= instructions.len() {
                        return 0;
                    }
                    program_counter = target;
                    continue;
                }

                let condition = match operation {
                    BPF_OPERATION_JEQ => accumulator == source,
                    BPF_OPERATION_JGT => accumulator > source,
                    BPF_OPERATION_JGE => accumulator >= source,
                    BPF_OPERATION_JSET => (accumulator & source) != 0,
                    _ => return 0,
                };
                let delta = if condition {
                    usize::from(instruction.jt)
                } else {
                    usize::from(instruction.jf)
                };
                let target = program_counter.saturating_add(1).saturating_add(delta);
                if target >= instructions.len() {
                    return 0;
                }
                program_counter = target;
            }
            BPF_CLASS_RET => {
                let return_mode = instruction.code & BPF_RETURN_MASK;
                if return_mode == BPF_RETURN_A {
                    return accumulator;
                }
                if return_mode == BPF_RETURN_K {
                    return instruction.k;
                }

                return 0;
            }
            BPF_CLASS_MISC => {
                let operation = instruction.code & BPF_MISC_MASK;
                if operation == BPF_MISC_TAX {
                    index = accumulator;
                    program_counter = program_counter.saturating_add(1);
                    continue;
                }
                if operation == BPF_MISC_TXA {
                    accumulator = index;
                    program_counter = program_counter.saturating_add(1);
                    continue;
                }

                return 0;
            }
            _ => return 0,
        }
    }

    0
}

/// Update one packet metadata row in place.
fn update_packet_socket_state(
    handle: SocketHandle,
    update: impl FnOnce(&mut WindowsPacketState),
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

/// Record one packet delivered from the host backend.
fn record_received_packet(state: &mut WindowsPacketState) {
    state.received_packets = state.received_packets.saturating_add(1);
}

/// Open a packet capture or inject endpoint.
pub(crate) unsafe fn destack_net_packet_open(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    // validate requested backend selection before opening resources
    let _backend = select_packet_backend_for_open(binding, options, "destack.net.packetOpen")?;

    // require backend enablement for Windows packet lanes
    require_windows_packet_backend(binding, "destack.net.packetOpen")?;

    // validate output pointer before creating resources
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate promiscuous-mode interface requirements
    if options.promiscuous && options.interface_index == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.interfaceIndex",
            "interface index is required for promiscuous mode",
        ))
        .boxed());
    }

    // initialize winsock and create one raw IPv4 packet socket
    ensure_winsock()?;
    let socket = unsafe { socket(AF_INET as i32, SOCK_RAW, IPPROTO_IP) };
    if socket == INVALID_SOCKET {
        return Err(net_error_with_code("socket(SOCK_RAW)", unsafe {
            WSAGetLastError()
        }));
    }

    // bind the socket to one requested interface or to INADDR_ANY
    let bind_address = if options.interface_index == 0 {
        any_ipv4_bind_address()
    } else {
        interface_ipv4_bind_address(options.interface_index)?
    };
    let bind_rc = unsafe {
        bind(
            socket,
            &bind_address as *const _ as *const SOCKADDR,
            std::mem::size_of::<SOCKADDR_IN>() as i32,
        )
    };
    if bind_rc != 0 {
        let _ = unsafe { closesocket(socket) };
        return Err(net_error_with_code("bind", unsafe { WSAGetLastError() }));
    }

    // configure one receive timeout window when requested
    if let Err(error) = configure_packet_receive_timeout(socket, options.timeout_ms) {
        let _ = unsafe { closesocket(socket) };
        return Err(error);
    }

    // configure promiscuous capture mode when requested
    if let Err(error) = configure_packet_promiscuous_mode(socket, options.promiscuous) {
        let _ = unsafe { closesocket(socket) };
        return Err(error);
    }

    // register one packet socket resource and one packet metadata row
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket as _)
        .with_finalizer(WindowsPacketFinalizer::new(socket));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    let snap_length = usize::try_from(options.snap_length).unwrap_or(WINDOWS_PACKET_MAX_LENGTH);
    let snap_length = snap_length.clamp(1, WINDOWS_PACKET_MAX_LENGTH);
    PACKET_SOCKET_STATES.lock().insert(
        resource_id,
        WindowsPacketState {
            interface_index: options.interface_index,
            snap_length,
            received_packets: 0,
            dropped_packets: 0,
            interface_dropped_packets: 0,
            filter_program: None,
        },
    );

    // return the registered packet socket handle
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}

/// Receive one packet from a packet endpoint.
pub(crate) unsafe fn destack_net_packet_receive(
    binding: &BindingCallContext,
    out: *mut PacketCaptureRecord,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // require backend enablement for Windows packet lanes
    require_windows_packet_backend(binding, "destack.net.packetReceive")?;

    // validate output pointer before receiving data
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one packet socket and one packet metadata row
    let (socket, state) = packet_socket_metadata(binding, handle)?;
    let payload = unsafe { payload.as_mut_slice()? };

    // allocate one bounded receive buffer from snap-length configuration
    let receive_length = state.snap_length.clamp(1, WINDOWS_PACKET_MAX_LENGTH);
    let mut receive_buffer = vec![0u8; receive_length];

    // read one packet payload and apply the optional filter
    let bytes = loop {
        let bytes = unsafe {
            recv(
                socket,
                receive_buffer.as_mut_ptr(),
                receive_buffer.len() as i32,
                0,
            )
        };
        if bytes == SOCKET_ERROR {
            let code = unsafe { WSAGetLastError() };
            if code == WSAETIMEDOUT {
                return Err(packet_receive_timeout_error("destack.net.packetReceive"));
            }

            return Err(net_error_with_code("recv", code));
        }

        let bytes = usize::try_from(bytes).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "bytes",
                "received byte count is out of range",
            ))
            .boxed()
        })?;

        // count one backend-delivered packet before runtime filtering
        update_packet_socket_state(handle, record_received_packet)?;

        let packet = &receive_buffer[..bytes];
        let accepted = match state.filter_program.as_ref() {
            Some(filter_program) => evaluate_filter_program(filter_program, packet) > 0,
            None => true,
        };
        if accepted {
            break bytes;
        }
    };

    // copy packet bytes into the caller payload buffer
    let written = bytes.min(payload.len());
    payload[..written].copy_from_slice(&receive_buffer[..written]);

    // compute packet metadata fields for the output record
    let truncated = bytes > written;
    let record = PacketCaptureRecord {
        bytes: written as u64,
        interface_index: state.interface_index,
        timestamp_clock: PacketTimestampClock::None,
        timestamp_ns: 0,
        truncated,
    };

    // write one packet capture record to the output pointer
    unsafe {
        *out = record;
    }

    Ok(())
}

/// Send one packet through a packet endpoint.
pub(crate) unsafe fn destack_net_packet_send(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // require backend enablement for Windows packet lanes
    require_windows_packet_backend(binding, "destack.net.packetSend")?;

    // validate output pointer before sending bytes
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one packet socket descriptor and payload bytes
    let (socket, _) = packet_socket_metadata(binding, handle)?;
    let payload = unsafe { payload.as_slice()? };

    // validate payload length against WinSock send argument limits
    if payload.len() > i32::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "payload",
            "packet payload exceeds host send limit",
        ))
        .boxed());
    }

    // send one packet payload through the raw socket endpoint
    let sent = unsafe { send(socket, payload.as_ptr(), payload.len() as i32, 0) };
    if sent == SOCKET_ERROR {
        return Err(net_error_with_code("send", unsafe { WSAGetLastError() }));
    }

    // write the number of payload bytes sent
    unsafe {
        *out = sent as u64;
    }

    Ok(())
}

/// Configure packet timestamp mode for a socket or packet endpoint.
pub(crate) unsafe fn destack_net_packet_set_timestamp_mode(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    // require backend enablement for Windows packet lanes
    require_windows_packet_backend(binding, "destack.net.packetSetTimestampMode")?;

    let _ = (handle, mode);

    // reject packet timestamping on raw WinSock packet backends
    windows_packet_not_supported("destack.net.packetSetTimestampMode")
}

/// Clear packet fanout from a packet endpoint.
pub(crate) unsafe fn destack_net_packet_clear_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    windows_packet_not_supported("destack.net.packetClearFanout")
}

/// Clear the active packet filter program.
pub(crate) unsafe fn destack_net_packet_clear_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // require backend enablement for Windows packet lanes
    require_windows_packet_backend(binding, "destack.net.packetClearFilter")?;

    // ensure one packet endpoint exists and clear the active filter
    let _ = packet_socket_metadata(binding, handle)?;
    update_packet_socket_state(handle, |state| {
        state.filter_program = None;
    })?;

    Ok(())
}

/// Clear packet rx and tx ring configuration.
pub(crate) unsafe fn destack_net_packet_clear_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    windows_packet_not_supported("destack.net.packetClearRing")
}

/// Set packet fanout on a packet endpoint.
pub(crate) unsafe fn destack_net_packet_set_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketFanoutOptions,
) -> RuntimeResult<()> {
    let _ = (binding, handle, options);
    windows_packet_not_supported("destack.net.packetSetFanout")
}

/// Attach one packet filter program to a raw endpoint.
pub(crate) unsafe fn destack_net_packet_set_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
    filterprogram: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // require backend enablement for Windows packet lanes
    require_windows_packet_backend(binding, "destack.net.packetSetFilter")?;

    // ensure one packet endpoint exists before decoding the filter
    let _ = packet_socket_metadata(binding, handle)?;
    let bytes = unsafe { filterprogram.as_slice()? };

    // decode and validate one classic-BPF filter payload
    let instructions = decode_filter_program(bytes)?;
    validate_filter_program(&instructions)?;
    let filter_program = Arc::<[ClassicBpfInstruction]>::from(instructions.into_boxed_slice());

    // install the filter program on the packet endpoint
    update_packet_socket_state(handle, |state| {
        state.filter_program = Some(filter_program);
    })?;

    Ok(())
}

/// Configure one packet rx ring for zero-copy capture.
pub(crate) unsafe fn destack_net_packet_set_rx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    let _ = (binding, handle, options);
    windows_packet_not_supported("destack.net.packetSetRxRing")
}

/// Configure one packet tx ring for zero-copy transmit.
pub(crate) unsafe fn destack_net_packet_set_tx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    let _ = (binding, handle, options);
    windows_packet_not_supported("destack.net.packetSetTxRing")
}

/// Read packet capture statistics from one endpoint.
pub(crate) unsafe fn destack_net_packet_stats(
    binding: &BindingCallContext,
    out: *mut PacketCaptureStats,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // require backend enablement for Windows packet lanes
    require_windows_packet_backend(binding, "destack.net.packetStats")?;

    // validate output pointer before loading stats
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one packet endpoint state row
    let (_, state) = packet_socket_metadata(binding, handle)?;

    // write one packet-stats snapshot to the output pointer
    unsafe {
        *out = PacketCaptureStats {
            received_packets: state.received_packets,
            dropped_packets: state.dropped_packets,
            interface_dropped_packets: state.interface_dropped_packets,
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::platform::PlatformErrorCode;
    use crate::platform::net::host::windows::packet::{
        BPF_CLASS_JMP, BPF_CLASS_LD, BPF_CLASS_RET, BPF_MODE_ABSOLUTE, BPF_OPERATION_JA,
        BPF_OPERATION_JEQ, BPF_RETURN_K, BPF_SIZE_HALF, ClassicBpfInstruction, WindowsPacketState,
        decode_filter_program, evaluate_filter_program, record_received_packet,
        validate_filter_program,
    };

    /// Return one encoded classic-BPF byte payload.
    fn encode_program(instructions: &[ClassicBpfInstruction]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of_val(instructions));
        for instruction in instructions {
            let row = unsafe {
                std::slice::from_raw_parts(
                    instruction as *const _ as *const u8,
                    std::mem::size_of::<ClassicBpfInstruction>(),
                )
            };
            bytes.extend_from_slice(row);
        }

        bytes
    }

    /// Validate and evaluate one simple ether-type equality filter.
    #[test]
    fn test_decode_validate_evaluate_filter_program() {
        let instructions = [
            ClassicBpfInstruction {
                code: BPF_CLASS_LD | BPF_MODE_ABSOLUTE | BPF_SIZE_HALF,
                jt: 0,
                jf: 0,
                k: 12,
            },
            ClassicBpfInstruction {
                code: BPF_CLASS_JMP | BPF_OPERATION_JEQ,
                jt: 0,
                jf: 1,
                k: 0x0800,
            },
            ClassicBpfInstruction {
                code: BPF_CLASS_RET | BPF_RETURN_K,
                jt: 0,
                jf: 0,
                k: u32::MAX,
            },
            ClassicBpfInstruction {
                code: BPF_CLASS_RET | BPF_RETURN_K,
                jt: 0,
                jf: 0,
                k: 0,
            },
        ];
        let bytes = encode_program(&instructions);
        let decoded = decode_filter_program(&bytes).expect("decode should succeed");
        validate_filter_program(&decoded).expect("validation should succeed");

        let mut ipv4_packet = vec![0u8; 64];
        ipv4_packet[12] = 0x08;
        ipv4_packet[13] = 0x00;
        assert!(evaluate_filter_program(&decoded, &ipv4_packet) > 0);

        let mut arp_packet = vec![0u8; 64];
        arp_packet[12] = 0x08;
        arp_packet[13] = 0x06;
        assert_eq!(evaluate_filter_program(&decoded, &arp_packet), 0);
    }

    /// Reject one invalid jump target that escapes the filter program.
    #[test]
    fn test_validate_filter_program_rejects_out_of_bounds_jump() {
        let instructions = [ClassicBpfInstruction {
            code: BPF_CLASS_JMP | BPF_OPERATION_JA,
            jt: 0,
            jf: 0,
            k: 99,
        }];

        let error = validate_filter_program(&instructions)
            .expect_err("validation should reject out-of-range jump");
        let platform_error = error
            .platform_error()
            .expect("platform error should be present");
        assert_eq!(platform_error.code, PlatformErrorCode::InvalidArgumentValue);
    }

    /// Count host-delivered packets without mutating backend-drop counters.
    #[test]
    fn test_record_received_packet_keeps_drop_counters_stable() {
        let mut state = WindowsPacketState {
            interface_index: 7,
            snap_length: 4096,
            received_packets: 0,
            dropped_packets: 3,
            interface_dropped_packets: 5,
            filter_program: None,
        };

        record_received_packet(&mut state);
        record_received_packet(&mut state);

        assert_eq!(state.received_packets, 2);
        assert_eq!(state.dropped_packets, 3);
        assert_eq!(state.interface_dropped_packets, 5);
    }
}
