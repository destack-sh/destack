mod ast;
mod dir;
mod environment;
mod graph;
mod hasher;
mod mir;
mod output;
mod store;
#[cfg(test)]
mod tests;
mod workspace;

pub use destack_workspace::{
    DEFAULT_CACHE_DIR, DEFAULT_COMPILER_CACHE_NAMESPACE, DEFAULT_GLOBAL_CACHE_DIR,
};
pub(crate) use hasher::CacheHasher;

/// The compiler version embedded in persisted cache headers.
pub(crate) const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return the compiler version embedded in persisted cache headers.
pub(crate) fn compiler_version() -> String {
    COMPILER_VERSION.to_string()
}
