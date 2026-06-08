use napi_derive::napi;

/// The static backend marker for Node API clients.
pub const BACKEND: &str = "napi";

/// Shared bridge metadata exposed to Node API clients.
#[derive(Debug)]
#[napi]
pub struct Bridge;

#[napi]
impl Bridge {
    /// Create a bridge metadata handle.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Return the backend marker.
    #[napi]
    pub fn backend(&self) -> &'static str {
        BACKEND
    }

    /// Return the crate version.
    #[napi]
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}

/// Return the crate version.
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
