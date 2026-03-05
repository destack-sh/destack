use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::core::{configured_system_certificate_directories, configured_system_certificate_files};

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (binding, kind);

    false
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (binding, kind, certificate);

    Err(core_platform::not_supported(operation))
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (binding, kind, certificate);

    Err(core_platform::not_supported(operation))
}

/// Collect certificates from configured Unix system trust-bundle locations.
pub(super) fn collect_system_certificates(binding: &BindingCallContext) -> Vec<X509> {
    let system_certificate_files = configured_system_certificate_files(binding);
    let system_certificate_directories = configured_system_certificate_directories(binding);

    unix_core::collect_system_certificates(
        &system_certificate_files,
        &system_certificate_directories,
    )
}

/// Return whether one Unix host certificate source path exists.
pub(super) fn has_system_certificate_source(binding: &BindingCallContext) -> bool {
    let system_certificate_files = configured_system_certificate_files(binding);
    let system_certificate_directories = configured_system_certificate_directories(binding);

    unix_core::has_system_certificate_source(
        &system_certificate_files,
        &system_certificate_directories,
    )
}
