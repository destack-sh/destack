use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::core::CRYPTO_STORE_OPEN_OPERATION;
use crate::platform::crypto::host::unix::core::{self as unix_core, SnapshotConfig};
use crate::runtime::BindingCallContext;

use super::certificate::{collect_system_certificates, has_system_certificate_source};
use super::core::keystore_path;

/// Snapshot codec configuration for POSIX host key-store blobs.
const POSIX_SNAPSHOT_CONFIG: SnapshotConfig = SnapshotConfig {
    store_label: "unix",
    associated_data: b"destack.crypto.posix.snapshot.v1",
};

pub(crate) fn host_store_persistence_backend_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    unix_core::host_store_persistence_backend_is_available_with_resolver(
        context,
        kind,
        keystore_path,
    )
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    unix_core::host_store_lane_is_available_with_resolver(
        context,
        kind,
        has_system_certificate_source,
        keystore_path,
    )
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> RuntimeResult<Vec<X509>> {
    unix_core::open_host_store_certificates_with_collector(
        context,
        kind,
        collect_system_certificates,
        CRYPTO_STORE_OPEN_OPERATION,
    )
}

/// Load one backend host-key snapshot payload for one store lane.
pub(crate) fn load_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    unix_core::load_host_key_snapshot_bytes_with_resolver(
        context,
        kind,
        POSIX_SNAPSHOT_CONFIG,
        keystore_path,
        operation,
    )
}

/// Store one backend host-key snapshot payload for one store lane.
pub(crate) fn store_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    unix_core::store_host_key_snapshot_bytes_with_resolver(
        context,
        kind,
        snapshot_bytes,
        POSIX_SNAPSHOT_CONFIG,
        keystore_path,
        operation,
    )
}
