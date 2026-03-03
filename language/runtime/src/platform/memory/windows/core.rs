use windows_sys::Win32::System::Memory::{
    PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_NOACCESS, PAGE_READONLY,
    PAGE_READWRITE,
};
use windows_sys::Win32::System::SystemInformation::{GetNativeSystemInfo, SYSTEM_INFO};

use crate::diagnostic::RuntimeResult;
use crate::platform::memory::{MemoryProtection, MemoryReserveFlags};
use crate::platform::{core as core_platform, memory as memory_platform};

/// Operation tag for reserve bindings.
pub(crate) const RESERVE_OPERATION: &str = "destack.memory.map.reserve";
/// Operation tag for numa-bind bindings.
pub(crate) const NUMA_BIND_OPERATION: &str = "destack.memory.map.numaBind";
/// Operation tag for remap bindings.
pub(crate) const REMAP_OPERATION: &str = "destack.memory.protect.remap";
/// Operation tag for huge-page advise bindings.
pub(crate) const HUGE_PAGE_OPERATION: &str = "destack.memory.advise.hugePage";

/// Supported reserve flag mask.
pub(crate) const RESERVE_FLAG_MASK: u32 = memory_platform::MEMORY_RESERVE_TOP_DOWN.0
    | memory_platform::MEMORY_RESERVE_LARGE_PAGES.0
    | memory_platform::MEMORY_RESERVE_NO_RESERVE.0;
/// Reserve-top-down flag bit.
pub(crate) const RESERVE_TOP_DOWN_FLAG: u32 = memory_platform::MEMORY_RESERVE_TOP_DOWN.0;
/// Windows `MEM_TOP_DOWN` bit value.
const WINDOWS_MEM_TOP_DOWN: u32 = 0x0010_0000;
/// Reserve-large-pages flag bit.
pub(crate) const RESERVE_LARGE_PAGES_FLAG: u32 = memory_platform::MEMORY_RESERVE_LARGE_PAGES.0;
/// Reserve-no-reserve flag bit.
pub(crate) const RESERVE_NO_RESERVE_FLAG: u32 = memory_platform::MEMORY_RESERVE_NO_RESERVE.0;

/// Supported remap flag mask.
pub(crate) const REMAP_FLAG_MASK: u32 = memory_platform::MEMORY_REMAP_MAY_MOVE.0;
/// Remap-may-move flag bit.
pub(crate) const REMAP_MAY_MOVE_FLAG: u32 = memory_platform::MEMORY_REMAP_MAY_MOVE.0;
/// Supported protection bit mask.
pub(crate) const PROTECTION_MASK: u32 = memory_platform::MEMORY_PROTECTION_READ.0
    | memory_platform::MEMORY_PROTECTION_WRITE.0
    | memory_platform::MEMORY_PROTECTION_EXECUTE.0;

/// Decode one memory-protection mask into one windows page-protection value.
pub(crate) fn windows_protection(protection: MemoryProtection) -> RuntimeResult<u32> {
    // reject unknown protection bits
    if protection.0 & !PROTECTION_MASK != 0 {
        return Err(core_platform::unsupported_flags("protection", protection.0));
    }

    // decode portable permission bits
    let read = protection.0 & memory_platform::MEMORY_PROTECTION_READ.0 != 0;
    let write = protection.0 & memory_platform::MEMORY_PROTECTION_WRITE.0 != 0;
    let execute = protection.0 & memory_platform::MEMORY_PROTECTION_EXECUTE.0 != 0;

    // map permission tuple to one windows protection mode
    let native = match (read, write, execute) {
        (false, false, false) => PAGE_NOACCESS,
        (true, false, false) => PAGE_READONLY,
        (true, true, false) => PAGE_READWRITE,
        (true, false, true) => PAGE_EXECUTE_READ,
        (true, true, true) => PAGE_EXECUTE_READWRITE,
        (false, false, true) => PAGE_EXECUTE,
        (false, true, false) => PAGE_READWRITE,
        (false, true, true) => PAGE_EXECUTE_READWRITE,
    };

    Ok(native)
}

/// Decode one reserve-flag payload for windows backends.
pub(crate) fn decode_reserve_flags(flags: MemoryReserveFlags) -> RuntimeResult<u32> {
    // reject unknown reserve bits
    if flags.0 & !RESERVE_FLAG_MASK != 0 {
        return Err(core_platform::unsupported_flags("flags", flags.0));
    }

    // reject unsupported reserve modes
    if flags.0 & RESERVE_LARGE_PAGES_FLAG != 0 {
        return Err(core_platform::not_supported(RESERVE_OPERATION));
    }

    if flags.0 & RESERVE_NO_RESERVE_FLAG != 0 {
        return Err(core_platform::not_supported(RESERVE_OPERATION));
    }

    // map supported top-down behavior to windows flags
    let mut native_flags = 0;
    if flags.0 & RESERVE_TOP_DOWN_FLAG != 0 {
        native_flags |= WINDOWS_MEM_TOP_DOWN;
    }

    Ok(native_flags)
}

/// Decode one remap flag payload.
pub(crate) fn decode_remap_flags(flags: u32) -> RuntimeResult<bool> {
    // reject unknown remap bits
    if flags & !REMAP_FLAG_MASK != 0 {
        return Err(core_platform::unsupported_flags("flags", flags));
    }

    Ok(flags & REMAP_MAY_MOVE_FLAG != 0)
}

/// Read one windows system-information payload.
pub(crate) fn system_info() -> SYSTEM_INFO {
    // query host system metadata
    let mut info = unsafe { std::mem::zeroed::<SYSTEM_INFO>() };
    unsafe {
        GetNativeSystemInfo(&mut info);
    }
    info
}

/// Return one windows page size.
pub(crate) fn page_size() -> RuntimeResult<usize> {
    // read page size from host system info
    let info = system_info();
    if info.dwPageSize == 0 {
        return Err(core_platform::invalid_argument(
            "pageSize",
            "GetNativeSystemInfo returned zero page size",
        ));
    }

    Ok(info.dwPageSize as usize)
}

/// Return one windows allocation granularity.
pub(crate) fn allocation_granularity() -> RuntimeResult<usize> {
    // read allocation granularity from host system info
    let info = system_info();
    if info.dwAllocationGranularity == 0 {
        return Err(core_platform::invalid_argument(
            "allocationGranularity",
            "GetNativeSystemInfo returned zero allocation granularity",
        ));
    }

    Ok(info.dwAllocationGranularity as usize)
}
