mod ast;
mod dir;
mod environment;
mod graph;
mod hasher;
mod image;
mod mir;
mod output;
mod store;
#[cfg(test)]
mod tests;
mod workspace;

pub(crate) use hasher::CacheHasher;

/// The compiler version embedded in persisted cache headers.
pub(crate) const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return the compiler version embedded in persisted cache headers.
pub(crate) fn compiler_version() -> String {
    COMPILER_VERSION.to_string()
}
