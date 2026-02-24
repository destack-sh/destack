use std::fs;

use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::certificate::{collect_system_certificates, has_system_certificate_source};
use super::constants::STORE_OPEN_OPERATION;
use super::core::{keystore_path, not_supported, permission_denied};

pub(crate) fn host_store_persistence_backend_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    keystore_path(context, kind).is_some()
}

/// Return whether one host store lane is currently available.
pub(crate) fn host_store_lane_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    match kind {
        CryptoStoreKind::System => has_system_certificate_source(context),
        CryptoStoreKind::User => keystore_path(context, CryptoStoreKind::User).is_some(),
        CryptoStoreKind::Machine => has_system_certificate_source(context),
        CryptoStoreKind::Provider | CryptoStoreKind::Ephemeral => false,
    }
}

/// Open one host store lane and return certificate snapshots.
pub(crate) fn open_host_store_certificates(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> RuntimeResult<Vec<X509>> {
    match kind {
        CryptoStoreKind::System => Ok(collect_system_certificates(context)),
        CryptoStoreKind::User => Ok(Vec::new()),
        CryptoStoreKind::Machine => Ok(collect_system_certificates(context)),
        CryptoStoreKind::Provider => Err(not_supported(STORE_OPEN_OPERATION)),
        CryptoStoreKind::Ephemeral => Ok(Vec::new()),
    }
}

/// Load one backend host-key snapshot payload for one store lane.
pub(crate) fn load_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    let Some(path) = keystore_path(context, kind) else {
        return Ok(None);
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }

            return Err(permission_denied(
                operation,
                format!("failed to read unix host keystore snapshot {path:?}: {error}"),
            ));
        }
    };

    Ok(Some(bytes))
}

/// Store one backend host-key snapshot payload for one store lane.
pub(crate) fn store_host_key_snapshot_bytes(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(path) = keystore_path(context, kind) else {
        return Err(not_supported(operation));
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to create unix host keystore path {parent:?}: {error}"),
            )
        })?;
    }

    fs::write(&path, snapshot_bytes).map_err(|error| {
        permission_denied(
            operation,
            format!("failed to write unix host keystore snapshot {path:?}: {error}"),
        )
    })?;

    // harden local snapshot file mode
    #[cfg(target_family = "unix")]
    if matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        use std::os::unix::fs::PermissionsExt;

        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, permissions).map_err(|error| {
            permission_denied(
                operation,
                format!("failed to set unix host keystore permissions for {path:?}: {error}"),
            )
        })?;
    }

    Ok(())
}
