use napi_derive::napi;

/// Return the crate version.
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
