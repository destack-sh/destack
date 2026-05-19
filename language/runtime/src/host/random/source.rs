use std::sync::Arc;

use crate::diagnostic::RuntimeResult;

/// Source of host entropy and secure randomness.
pub(crate) trait HostRandomSource: std::fmt::Debug + Send + Sync {
    /// Fill bytes from one host entropy source.
    fn fill_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()>;

    /// Try to fill bytes from one host entropy source without blocking.
    fn try_fill_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        self.fill_bytes(buffer)
    }

    /// Return one random u64 from one host entropy source.
    fn next_u64(&self) -> RuntimeResult<u64> {
        // read one u64 worth of host entropy
        let mut bytes = [0u8; 8];
        self.fill_bytes(&mut bytes)?;

        Ok(u64::from_le_bytes(bytes))
    }
}

/// Host-backed entropy source using host dispatch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SystemHostRandomSource;

impl HostRandomSource for SystemHostRandomSource {
    /// Fill bytes from one host entropy source.
    fn fill_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        host_fill_bytes(buffer)
    }

    /// Try to fill bytes from one host entropy source without blocking.
    fn try_fill_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        host_try_fill_bytes(buffer)
    }
}

/// Host randomness state used by runtime random services.
#[derive(Debug, Clone)]
pub(crate) struct HostRandom {
    /// Entropy source for host randomness.
    source: Arc<dyn HostRandomSource>,
}

impl HostRandom {
    /// Create one host randomness source.
    pub(crate) fn new() -> Self {
        Self::with_source(Arc::new(SystemHostRandomSource))
    }

    /// Create one host randomness source from one explicit source.
    pub(crate) fn with_source(source: Arc<dyn HostRandomSource>) -> Self {
        Self { source }
    }

    /// Fill bytes from host entropy.
    pub(crate) fn fill_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        self.source.fill_bytes(buffer)
    }

    /// Try to fill bytes from host entropy without blocking.
    pub(crate) fn try_fill_bytes(&self, buffer: &mut [u8]) -> RuntimeResult<()> {
        self.source.try_fill_bytes(buffer)
    }

    /// Return one host random u64.
    pub(crate) fn next_u64(&self) -> RuntimeResult<u64> {
        self.source.next_u64()
    }
}

impl Default for HostRandom {
    /// Create one default host randomness source.
    fn default() -> Self {
        Self::new()
    }
}

/// Fill bytes from the selected host entropy backend.
fn host_fill_bytes(buffer: &mut [u8]) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        super::unix::host_fill_bytes(buffer)
    }

    #[cfg(windows)]
    {
        super::windows::host_fill_bytes(buffer)
    }

    #[cfg(not(any(unix, windows)))]
    {
        super::unsupported::host_fill_bytes(buffer)
    }
}

/// Try to fill bytes from the selected host entropy backend without blocking.
fn host_try_fill_bytes(buffer: &mut [u8]) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        super::unix::host_try_fill_bytes(buffer)
    }

    #[cfg(windows)]
    {
        super::windows::host_try_fill_bytes(buffer)
    }

    #[cfg(not(any(unix, windows)))]
    {
        super::unsupported::host_try_fill_bytes(buffer)
    }
}
