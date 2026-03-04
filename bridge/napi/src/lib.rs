use napi_derive::napi;

/// Return the crate version.
#[napi]
pub fn version() -> &'static str {
    destack_bridge_core::version()
}
