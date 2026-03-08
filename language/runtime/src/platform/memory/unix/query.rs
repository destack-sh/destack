#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io::ErrorKind;

#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::PlatformError;
use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

use super::core::page_size;

/// Read the host allocation granularity.
pub(crate) unsafe fn destack_memory_allocation_granularity(
    _binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate output pointer and query page size
    core_platform::ensure_out(out, "out")?;
    let page_size = page_size()?;
    let page_size = core_platform::usize_to_u64(page_size, "out")?;

    // write page size result
    unsafe {
        out.write(page_size);
    }

    Ok(())
}

/// Read the host huge-page allocation size when available.
pub(crate) unsafe fn destack_memory_huge_page_size(
    _binding: &BindingCallContext,
    out: *mut Option<u64>,
) -> RuntimeResult<()> {
    // validate output pointer for optional huge-page size
    core_platform::ensure_out(out, "out")?;

    #[cfg(target_os = "linux")]
    {
        // query and return linux huge-page size metadata
        let huge_page_size = linux_huge_page_size()?;
        unsafe {
            out.write(huge_page_size);
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        // report no huge-page size on non-linux unix backends
        unsafe {
            out.write(None);
        }

        Ok(())
    }
}

/// Read the host virtual-memory page size.
pub(crate) unsafe fn destack_memory_page_size(
    _binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    // validate output pointer and query page size
    core_platform::ensure_out(out, "out")?;
    let page_size = page_size()?;
    let page_size = core_platform::usize_to_u64(page_size, "out")?;

    // write page size result
    unsafe {
        out.write(page_size);
    }

    Ok(())
}

/// Parse huge-page size from linux `/proc/meminfo`.
#[cfg(target_os = "linux")]
fn linux_huge_page_size() -> RuntimeResult<Option<u64>> {
    // read procfs memory metadata
    let contents = match fs::read_to_string("/proc/meminfo") {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => {
            return Err(RuntimeError::from(PlatformError::io(error.to_string())).boxed());
        }
    };

    // find the huge page size line
    for line in contents.lines() {
        if !line.starts_with("Hugepagesize:") {
            continue;
        }

        // decode numeric value and unit
        let mut parts = line.split_whitespace();
        let _label = parts.next();
        let value = parts
            .next()
            .ok_or_else(|| core_platform::invalid_argument("out", "invalid Hugepagesize format"))?;
        let unit = parts
            .next()
            .ok_or_else(|| core_platform::invalid_argument("out", "missing Hugepagesize unit"))?;

        let value = value
            .parse::<u64>()
            .map_err(|_| core_platform::invalid_argument("out", "invalid Hugepagesize value"))?;
        let bytes = match unit {
            "kB" => value.checked_mul(1024),
            "mB" | "MB" => value.checked_mul(1024 * 1024),
            "gB" | "GB" => value.checked_mul(1024 * 1024 * 1024),
            _ => None,
        }
        .ok_or_else(|| core_platform::invalid_argument("out", "Hugepagesize value overflow"))?;

        return Ok(Some(bytes));
    }

    // no huge page size entry was reported
    Ok(None)
}
