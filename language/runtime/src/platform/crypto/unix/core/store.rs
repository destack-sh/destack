use std::path::PathBuf;

use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::{
    SnapshotConfig, load_host_key_snapshot_bytes as load_snapshot_bytes, not_supported,
    store_host_key_snapshot_bytes as store_snapshot_bytes,
};

/// Host callback signature for resolving one snapshot path for one lane.
pub(crate) type SnapshotPathResolver = fn(&BindingCallContext, CryptoStoreKind) -> Option<PathBuf>;

/// Host callback signature for collecting system certificates.
pub(crate) type SystemCertificateCollector = fn(&BindingCallContext) -> Vec<X509>;

/// Host callback signature for detecting system certificate source availability.
pub(crate) type SystemCertificateSourceAvailability = fn(&BindingCallContext) -> bool;

/// Return whether one host snapshot backend is available for one lane.
pub(crate) fn host_store_persistence_backend_is_available_with_resolver(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    resolve_snapshot_path: SnapshotPathResolver,
) -> bool {
    let snapshot_path = resolve_snapshot_path(context, kind);

    super::host_store_persistence_backend_is_available(snapshot_path)
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available_with_resolver(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    has_system_certificate_source: SystemCertificateSourceAvailability,
    resolve_snapshot_path: SnapshotPathResolver,
) -> bool {
    match kind {
        CryptoStoreKind::System => has_system_certificate_source(context),
        CryptoStoreKind::User => resolve_snapshot_path(context, CryptoStoreKind::User).is_some(),
        CryptoStoreKind::Machine => {
            has_system_certificate_source(context)
                || resolve_snapshot_path(context, CryptoStoreKind::Machine).is_some()
        }
        CryptoStoreKind::Provider | CryptoStoreKind::Ephemeral => false,
    }
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates_with_collector(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    collect_system_certificates: SystemCertificateCollector,
    operation: &'static str,
) -> RuntimeResult<Vec<X509>> {
    match kind {
        CryptoStoreKind::System => Ok(collect_system_certificates(context)),
        CryptoStoreKind::User => Ok(Vec::new()),
        CryptoStoreKind::Machine => Ok(collect_system_certificates(context)),
        CryptoStoreKind::Provider => Err(not_supported(operation)),
        CryptoStoreKind::Ephemeral => Ok(Vec::new()),
    }
}

/// Load one backend host-key snapshot payload for one store lane.
pub(crate) fn load_host_key_snapshot_bytes_with_resolver(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    config: SnapshotConfig,
    resolve_snapshot_path: SnapshotPathResolver,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    let snapshot_path = resolve_snapshot_path(context, kind);

    load_snapshot_bytes(snapshot_path, config, operation)
}

/// Store one backend host-key snapshot payload for one store lane.
pub(crate) fn store_host_key_snapshot_bytes_with_resolver(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    config: SnapshotConfig,
    resolve_snapshot_path: SnapshotPathResolver,
    operation: &'static str,
) -> RuntimeResult<()> {
    let snapshot_path = resolve_snapshot_path(context, kind);

    store_snapshot_bytes(snapshot_path, kind, snapshot_bytes, config, operation)
}
