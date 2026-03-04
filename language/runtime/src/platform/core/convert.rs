use std::time::Duration;

use crate::diagnostic::RuntimeResult;

use super::error::invalid_argument;

/// Convert one optional u64 value into one host usize.
pub(crate) fn option_u64_to_usize(value: Option<u64>) -> Option<usize> {
    value.and_then(|value| usize::try_from(value).ok())
}

/// Convert one optional u64 value into one u32.
pub(crate) fn option_u64_to_u32(value: Option<u64>) -> Option<u32> {
    value.and_then(|value| u32::try_from(value).ok())
}

/// Resolve one optional u64 value into one defaulted value with one lower bound.
pub(crate) fn option_u64_or_min(value: Option<u64>, default: u64, min: u64) -> u64 {
    value.unwrap_or(default).max(min)
}

/// Resolve one optional u64 value into one defaulted usize with one lower bound.
#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn option_u64_to_usize_or_min(value: Option<u64>, default: usize, min: usize) -> usize {
    option_u64_to_usize(value).unwrap_or(default).max(min)
}

/// Convert one non-zero u32 value into one host usize.
#[cfg(target_os = "linux")]
pub(crate) fn u32_to_nonzero_usize(field: &'static str, value: u32) -> RuntimeResult<usize> {
    if value == 0 {
        return Err(invalid_argument(field, "value must be greater than zero"));
    }

    Ok(value as usize)
}

/// Resolve one optional u64 nanosecond value into one bounded duration.
pub(crate) fn duration_from_option_ns(value: Option<u64>, default: u64, min: u64) -> Duration {
    Duration::from_nanos(option_u64_or_min(value, default, min))
}

/// Convert one u64 value into one host usize.
pub(crate) fn u64_to_usize(value: u64, field: &str) -> RuntimeResult<usize> {
    u64_to_usize_with_message(value, field, "value exceeds host usize range")
}

/// Convert one u64 value into one host usize using one custom overflow message.
pub(crate) fn u64_to_usize_with_message(
    value: u64,
    field: &str,
    message: &str,
) -> RuntimeResult<usize> {
    usize::try_from(value).map_err(|_| invalid_argument(field, message))
}

/// Convert one usize value into one u64.
pub(crate) fn usize_to_u64(value: usize, field: &str) -> RuntimeResult<u64> {
    u64::try_from(value).map_err(|_| invalid_argument(field, "value exceeds u64 range"))
}
