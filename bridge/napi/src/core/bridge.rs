use napi_derive::napi;

/// Return the crate version.
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Convert one bridge error into one NAPI error.
pub(crate) fn to_error(error: impl ToString) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
