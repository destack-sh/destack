use std::sync::{Arc, OnceLock};

#[cfg(any(unix, windows))]
use super::core::CryptoStoreRuntimeState;

/// Runtime-owned crypto module state.
#[derive(Default)]
pub struct PlatformCryptoState {
    /// Runtime-owned crypto store lane state.
    #[cfg(any(unix, windows))]
    crypto_store_runtime_state: OnceLock<Arc<CryptoStoreRuntimeState>>,
}

impl std::fmt::Debug for PlatformCryptoState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformCryptoState")
            .finish_non_exhaustive()
    }
}

impl PlatformCryptoState {
    /// Return runtime-owned crypto store lane state.
    #[cfg(any(unix, windows))]
    pub(crate) fn crypto_store_runtime_state(
        &self,
        initialize: impl FnOnce() -> CryptoStoreRuntimeState,
    ) -> Arc<CryptoStoreRuntimeState> {
        Arc::clone(
            self.crypto_store_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
