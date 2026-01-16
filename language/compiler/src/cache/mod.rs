mod context;
mod disk;
mod handle;
mod hash;
mod memory;
mod options;
mod registry;
mod signature;

pub use context::{
    CacheContext, DEFAULT_CACHE_DIR, DEFAULT_CACHE_NAMESPACE, DEFAULT_GLOBAL_CACHE_DIR,
};
pub use handle::CacheHandle;
pub use options::CacheOptions;
pub use registry::{CacheKey, CacheRegistry};

#[cfg(test)]
mod tests;