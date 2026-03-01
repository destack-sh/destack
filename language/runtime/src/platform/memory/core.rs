use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

/// Validate one output pointer argument.
pub(crate) fn ensure_out<T>(out: *mut T, field: &'static str) -> RuntimeResult<()> {
    // reject null out pointers explicitly
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(field)).boxed());
    }

    Ok(())
}

/// Build one invalid-argument runtime error.
pub(crate) fn invalid_argument(
    field: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed()
}

/// Build one unsupported-operation runtime error.
pub(crate) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Build one unsupported-flags runtime error.
pub(crate) fn unsupported_flags(field: &'static str, flags: u32) -> Box<RuntimeError> {
    invalid_argument(field, format!("unsupported flag bits: 0x{flags:x}"))
}

/// Convert one u64 length into one nonzero host usize.
pub(crate) fn nonzero_length(length: u64, field: &'static str) -> RuntimeResult<usize> {
    // reject zero-length ranges
    if length == 0 {
        return Err(invalid_argument(field, "length must be greater than zero"));
    }

    // reject host-width overflow
    usize::try_from(length).map_err(|_| invalid_argument(field, "length exceeds host usize range"))
}

/// Convert one u64 address into one nonzero host usize.
pub(crate) fn nonzero_address(address: u64, field: &'static str) -> RuntimeResult<usize> {
    // reject null addresses
    if address == 0 {
        return Err(invalid_argument(field, "address must be non-zero"));
    }

    // reject host-width overflow
    usize::try_from(address)
        .map_err(|_| invalid_argument(field, "address exceeds host pointer width"))
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
    let address = usize::try_from(address)
        .map_err(|_| invalid_argument(field, "address hint exceeds host pointer width"))?;

    Ok(Some(address))
}

/// Convert one host usize into one checked u64.
pub(crate) fn usize_to_u64(value: usize, field: &'static str) -> RuntimeResult<u64> {
    u64::try_from(value).map_err(|_| invalid_argument(field, "value exceeds u64 range"))
}

/// Validate one page-aligned value.
pub(crate) fn require_page_alignment(
    value: usize,
    page_size: usize,
    field: &'static str,
) -> RuntimeResult<()> {
    // validate page-size alignment contract
    if !value.is_multiple_of(page_size) {
        return Err(invalid_argument(
            field,
            format!("{field} must be aligned to page size"),
        ));
    }

    Ok(())
}
