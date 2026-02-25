use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::core::{
    configured_system_certificate_directories, configured_system_certificate_files, not_supported,
};

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (context, kind);

    false
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (context, kind, certificate);

    Err(not_supported(operation))
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (context, kind, certificate);

    Err(not_supported(operation))
}

/// Collect certificates from configured Android system trust-bundle locations.
pub(super) fn collect_system_certificates(context: &BindingCallContext) -> Vec<X509> {
    let system_certificate_files = configured_system_certificate_files(context);
    let system_certificate_directories = configured_system_certificate_directories(context);

    unix_core::collect_system_certificates(
        &system_certificate_files,
        &system_certificate_directories,
    )
}

/// Return whether one Android host certificate source path exists.
pub(super) fn has_system_certificate_source(context: &BindingCallContext) -> bool {
    let system_certificate_files = configured_system_certificate_files(context);
    let system_certificate_directories = configured_system_certificate_directories(context);

    unix_core::has_system_certificate_source(
        &system_certificate_files,
        &system_certificate_directories,
    )
}
