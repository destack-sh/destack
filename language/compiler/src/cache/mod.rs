mod context;
mod disk;
mod handle;
mod hasher;
mod memory;
mod options;
mod registry;
mod signature;

pub use context::CacheContext;
pub use destack_workspace::{
    DEFAULT_CACHE_DIR, DEFAULT_COMPILER_CACHE_NAMESPACE, DEFAULT_GLOBAL_CACHE_DIR,
};
pub use handle::CacheHandle;
pub use options::CacheOptions;
pub use registry::{CacheKey, CacheRegistry};

#[cfg(test)]
mod tests;
