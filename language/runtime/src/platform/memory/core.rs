use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;

/// Convert one u64 length into one nonzero host usize.
pub(crate) fn nonzero_length(length: u64, field: &'static str) -> RuntimeResult<usize> {
    // reject zero-length ranges
    if length == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "length must be greater than zero",
        ));
    }

    // reject host-width overflow
    core_platform::u64_to_usize(length, field)
}

/// Convert one u64 address into one nonzero host usize.
pub(crate) fn nonzero_address(address: u64, field: &'static str) -> RuntimeResult<usize> {
    // reject null addresses
    if address == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "address must be non-zero",
        ));
    }

    // reject host-width overflow
    core_platform::u64_to_usize_with_message(address, field, "address exceeds host pointer width")
}

/// Convert one optional u64 address hint into one host pointer width value.
pub(crate) fn optional_address_hint(
    address: u64,
    field: &'static str,
) -> RuntimeResult<Option<usize>> {
    // treat zero as "host chooses address"
    if address == 0 {
        return Ok(None);
    }

    // validate hint fits host pointer width
    let address = core_platform::u64_to_usize_with_message(
        address,
        field,
        "address hint exceeds host pointer width",
    )?;

    Ok(Some(address))
}

/// Validate one page-aligned value.
pub(crate) fn require_page_alignment(
    value: usize,
    page_size: usize,
    field: &'static str,
) -> RuntimeResult<()> {
    // validate page-size alignment contract
    if !value.is_multiple_of(page_size) {
        return Err(core_platform::invalid_argument(
            field,
            format!("{field} must be aligned to page size"),
        ));
    }

    Ok(())
}
