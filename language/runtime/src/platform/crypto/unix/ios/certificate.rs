use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::CryptoStoreKind;
use crate::runtime::BindingCallContext;

use super::core::not_supported;

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
