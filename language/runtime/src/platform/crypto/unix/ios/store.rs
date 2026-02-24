use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::constants::STORE_OPEN_OPERATION;
use super::core::not_supported;

/// Return whether one host lane has a writable persistent-key backend.
pub(crate) fn host_store_persistence_backend_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (context, kind);

    false
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (context, kind);

    false
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> RuntimeResult<Vec<X509>> {
    let _ = context;

    match kind {
        CryptoStoreKind::Provider => Err(not_supported(STORE_OPEN_OPERATION)),
        _ => Ok(Vec::new()),
    }
}

/// Load one backend host-key snapshot payload for one store lane.
pub(crate) fn load_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    let _ = (context, kind, operation);

    Ok(None)
}

/// Store one backend host-key snapshot payload for one store lane.
pub(crate) fn store_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (context, kind, snapshot_bytes);

    Err(not_supported(operation))
}
