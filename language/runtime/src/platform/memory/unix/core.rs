use crate::diagnostic::RuntimeResult;
use crate::platform::memory::{MemoryProtection, MemoryReserveFlags, core as memory_core};
use crate::platform::{core as core_platform, memory as memory_platform};

/// Operation tag for reserve bindings.
pub(crate) const RESERVE_OPERATION: &str = "destack.memory.map.reserve";
/// Operation tag for numa-bind bindings.
pub(crate) const NUMA_BIND_OPERATION: &str = "destack.memory.map.numaBind";
/// Operation tag for remap bindings.
pub(crate) const REMAP_OPERATION: &str = "destack.memory.protect.remap";
/// Operation tag for huge-page advise bindings.
#[cfg(not(target_os = "linux"))]
pub(crate) const HUGE_PAGE_OPERATION: &str = "destack.memory.advise.hugePage";

/// Supported reserve flag mask.
pub(crate) const RESERVE_FLAG_MASK: u32 = memory_platform::MEMORY_RESERVE_TOP_DOWN.0
    | memory_platform::MEMORY_RESERVE_LARGE_PAGES.0
    | memory_platform::MEMORY_RESERVE_NO_RESERVE.0;
/// Reserve-top-down flag bit.
pub(crate) const RESERVE_TOP_DOWN_FLAG: u32 = memory_platform::MEMORY_RESERVE_TOP_DOWN.0;
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

/// Convert one memory-protection mask into one unix `mprotect` mask.
pub(crate) fn unix_protection(protection: MemoryProtection) -> RuntimeResult<i32> {
    // reject unknown protection bits
    if protection.0 & !PROTECTION_MASK != 0 {
        return Err(core_platform::unsupported_flags("protection", protection.0));
    }

    // map portable bits to native flags
    let mut native = 0;
    if protection.0 & memory_platform::MEMORY_PROTECTION_READ.0 != 0 {
        native |= libc::PROT_READ;
    }
    if protection.0 & memory_platform::MEMORY_PROTECTION_WRITE.0 != 0 {
        native |= libc::PROT_WRITE;
    }
    if protection.0 & memory_platform::MEMORY_PROTECTION_EXECUTE.0 != 0 {
        native |= libc::PROT_EXEC;
    }

    Ok(native)
}

/// Decode and validate one reserve flag payload for unix backends.
pub(crate) fn decode_reserve_flags(flags: MemoryReserveFlags) -> RuntimeResult<i32> {
    // reject unknown reserve bits
    if flags.0 & !RESERVE_FLAG_MASK != 0 {
        return Err(core_platform::unsupported_flags("flags", flags.0));
    }

    // reject top-down request where unix backends do not expose stable semantics
    if flags.0 & RESERVE_TOP_DOWN_FLAG != 0 {
        return Err(core_platform::not_supported(RESERVE_OPERATION));
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    let mut native_flags = libc::MAP_PRIVATE | libc::MAP_ANON;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let native_flags = libc::MAP_PRIVATE | libc::MAP_ANON;
    // apply optional no-reserve behavior where supported
    if flags.0 & RESERVE_NO_RESERVE_FLAG != 0 {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            native_flags |= libc::MAP_NORESERVE;
        }

        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            return Err(core_platform::not_supported(RESERVE_OPERATION));
        }
    }

    // apply huge-page request where supported
    if flags.0 & RESERVE_LARGE_PAGES_FLAG != 0 {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            native_flags |= libc::MAP_HUGETLB;
        }

        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            return Err(core_platform::not_supported(RESERVE_OPERATION));
        }
    }

    Ok(native_flags)
}

/// Decode and validate one remap flag payload.
pub(crate) fn decode_remap_flags(flags: u32) -> RuntimeResult<bool> {
    // reject unknown remap bits
    if flags & !REMAP_FLAG_MASK != 0 {
        return Err(core_platform::unsupported_flags("flags", flags));
    }

    Ok(flags & REMAP_MAY_MOVE_FLAG != 0)
}

/// Read the host page size used by unix memory bindings.
pub(crate) fn page_size() -> RuntimeResult<usize> {
    // query the host page size
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        return Err(core_platform::io_error("sysconf", None));
    }

    // validate representable page size
    usize::try_from(page_size).map_err(|_| {
        core_platform::invalid_argument("pageSize", "page size exceeds host usize range")
    })
}

/// Convert one validated address and length into one host pointer range.
pub(crate) fn validated_range(
    address: u64,
    length: u64,
    page_size: usize,
) -> RuntimeResult<(*mut libc::c_void, usize)> {
    // validate raw address and length values
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;

    // enforce page alignment required by unix memory syscalls
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    Ok((address as *mut libc::c_void, length))
}

/// Convert one optional address hint into one unix pointer.
pub(crate) fn validated_hint(
    address_hint: u64,
    page_size: usize,
) -> RuntimeResult<*mut libc::c_void> {
    // short-circuit when no hint was provided
    let Some(address_hint) = memory_core::optional_address_hint(address_hint, "addressHint")?
    else {
        return Ok(std::ptr::null_mut());
    };

    // enforce alignment for explicit hints
    memory_core::require_page_alignment(address_hint, page_size, "addressHint")?;

    Ok(address_hint as *mut libc::c_void)
}

/// Convert one `mmap` length into one `size_t`.
pub(crate) fn to_size_t(length: usize) -> libc::size_t {
    length as libc::size_t
}

/// Convert one `off_t` from zero offset.
pub(crate) fn zero_offset() -> libc::off_t {
    0
}
