use std::fmt;
use std::sync::Arc;

/// Loaded native library handle.
#[derive(Debug, Clone)]
pub struct LibraryHandle {
    /// Owner that releases the platform handle on drop.
    owner: Arc<dyn fmt::Debug + Send + Sync>,
}

impl LibraryHandle {
    /// Create one loaded native library handle.
    pub fn new(owner: impl fmt::Debug + Send + Sync + 'static) -> Self {
        Self {
            owner: Arc::new(owner),
        }
    }

    /// Borrow the owner that keeps this handle alive.
    pub fn owner(&self) -> &(dyn fmt::Debug + Send + Sync + '_) {
        self.owner.as_ref()
    }
}
