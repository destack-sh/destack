use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_READWRITE,
    VirtualAlloc, VirtualFree, VirtualProtect, VirtualQuery,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::memory::{
    MemoryProtection, MemoryRemapFlags, ProtectedMemoryRange, core as memory_core,
};
use crate::runtime::BindingCallContext;

use super::core::{REMAP_OPERATION, decode_remap_flags, page_size, windows_protection};

unsafe extern "system" {
    fn FlushInstructionCache(
        process: isize,
        base_address: *const core::ffi::c_void,
        size: usize,
    ) -> i32;
}

/// Return whether one windows protection mode allows reads.
fn protection_allows_read(protection: u32) -> bool {
    // fail when guard-page behavior can trap reads
    if protection & PAGE_GUARD != 0 {
        return false;
    }

    // compare the base protection without modifier bits
    let base_protection = protection & 0xff;

    matches!(
        base_protection,
        windows_sys::Win32::System::Memory::PAGE_READONLY
            | windows_sys::Win32::System::Memory::PAGE_READWRITE
            | windows_sys::Win32::System::Memory::PAGE_WRITECOPY
            | windows_sys::Win32::System::Memory::PAGE_EXECUTE_READ
            | windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE
            | windows_sys::Win32::System::Memory::PAGE_EXECUTE_WRITECOPY
    )
}

/// Change memory protection for one range.
pub(crate) unsafe fn destack_memory_protect(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
    protection: MemoryProtection,
) -> RuntimeResult<()> {
    // validate range and protection mode
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // decode portable protection mask
    let protection = windows_protection(protection)?;

    // apply protection to the requested range
    let mut old_protection = 0u32;
    let status = unsafe {
        VirtualProtect(
            address as *mut core::ffi::c_void,
            length,
            protection,
            &mut old_protection,
        )
    };
    if status == 0 {
        return Err(core_platform::io_error("VirtualProtect"));
    }

    Ok(())
}

/// Resize one mapped range.
pub(crate) unsafe fn destack_memory_remap(
    binding: &BindingCallContext,
    out: *mut ProtectedMemoryRange,
    address: u64,
    oldlength: u64,
    newlength: u64,
    flags: MemoryRemapFlags,
) -> RuntimeResult<()> {
    // validate output pointer and remap parameters
    core_platform::ensure_out(out, "out")?;
    let page_size = page_size()?;
    let old_address = memory_core::nonzero_address(address, "address")?;
    let old_length = memory_core::nonzero_length(oldlength, "oldLength")?;
    let new_length = memory_core::nonzero_length(newlength, "newLength")?;

    memory_core::require_page_alignment(old_address, page_size, "address")?;
    memory_core::require_page_alignment(old_length, page_size, "oldLength")?;
    memory_core::require_page_alignment(new_length, page_size, "newLength")?;

    // fast path identical lengths
    if old_length == new_length {
        unsafe {
            out.write(ProtectedMemoryRange {
                address,
                length: newlength,
            });
        }

        return Ok(());
    }

    // require move permission for windows remap strategy
    let may_move = decode_remap_flags(flags.0)?;
    if !may_move {
        return Err(core_platform::not_supported(REMAP_OPERATION));
    }

    // query source mapping metadata
    let mut memory_info = unsafe { std::mem::zeroed::<MEMORY_BASIC_INFORMATION>() };
    let queried = unsafe {
        VirtualQuery(
            old_address as *const core::ffi::c_void,
            &mut memory_info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if queried == 0 {
        return Err(core_platform::io_error("VirtualQuery"));
    }

    // validate that the source range is committed and fully covered
    if memory_info.State != MEM_COMMIT {
        return Err(core_platform::not_supported(REMAP_OPERATION));
    }

    let region_start = memory_info.BaseAddress as usize;
    let region_end = region_start
        .checked_add(memory_info.RegionSize)
        .ok_or_else(|| core_platform::invalid_argument("oldLength", "source region overflowed"))?;
    let old_end = old_address
        .checked_add(old_length)
        .ok_or_else(|| core_platform::invalid_argument("oldLength", "source range overflowed"))?;
    if old_address < region_start || old_end > region_end {
        return Err(core_platform::invalid_argument(
            "oldLength",
            "source range extends beyond one committed region",
        ));
    }

    // fail loud when existing protection cannot be copied safely
    if !protection_allows_read(memory_info.Protect) {
        return Err(core_platform::not_supported(REMAP_OPERATION));
    }

    // allocate destination range with the same protection policy
    let target_protection = if memory_info.Protect == 0 {
        PAGE_READWRITE
    } else {
        memory_info.Protect
    };
    let new_pointer = unsafe {
        VirtualAlloc(
            std::ptr::null_mut(),
            new_length,
            MEM_RESERVE | MEM_COMMIT,
            target_protection,
        )
    };
    if new_pointer.is_null() {
        return Err(core_platform::io_error("VirtualAlloc"));
    }

    // copy source bytes into the new range
    let bytes_to_copy = old_length.min(new_length);
    if bytes_to_copy != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                old_address as *const u8,
                new_pointer as *mut u8,
                bytes_to_copy,
            );
        }
    }

    // release the original mapping after successful copy
    let released = unsafe { VirtualFree(old_address as *mut core::ffi::c_void, 0, MEM_RELEASE) };
    if released == 0 {
        unsafe {
            VirtualFree(new_pointer, 0, MEM_RELEASE);
        }
        return Err(core_platform::io_error("VirtualFree"));
    }

    // return remapped range metadata
    let remapped_address = core_platform::usize_to_u64(new_pointer as usize, "out.address")?;
    let remapped_length = core_platform::usize_to_u64(new_length, "out.length")?;
    unsafe {
        out.write(ProtectedMemoryRange {
            address: remapped_address,
            length: remapped_length,
        });
    }

    Ok(())
}

/// Flush instruction cache for one range.
pub(crate) unsafe fn destack_memory_flush_instruction_cache(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    // validate range before cache flush
    let page_size = page_size()?;
    let address = memory_core::nonzero_address(address, "address")?;
    let length = memory_core::nonzero_length(length, "length")?;
    memory_core::require_page_alignment(address, page_size, "address")?;
    memory_core::require_page_alignment(length, page_size, "length")?;

    // flush process instruction cache for the range
    let status = unsafe {
        FlushInstructionCache(
            GetCurrentProcess(),
            address as *const core::ffi::c_void,
            length,
        )
    };
    if status == 0 {
        return Err(core_platform::io_error("FlushInstructionCache"));
    }

    Ok(())
}
