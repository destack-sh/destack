use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::host::{HOST_STATUS_NOT_FOUND, android_host_crypto_callbacks_snapshot};
use crate::platform::NativeSlice;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::core::{
    callback_runtime_id, configured_system_certificate_directories,
    configured_system_certificate_files, host_status_result, host_store_kind, invalid_data,
    not_supported,
};

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // writable certificate callbacks target host lanes only
    if !matches!(
        kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) {
        return false;
    }

    // require one callback runtime id and both certificate callbacks
    let Some(runtime_id) = context.host().callback_runtime_id() else {
        return false;
    };
    let Some(callbacks) = android_host_crypto_callbacks_snapshot(runtime_id) else {
        return false;
    };

    callbacks.import_certificate.is_some() && callbacks.delete_certificate.is_some()
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve runtime id and callback entrypoint
    let runtime_id = callback_runtime_id(context, operation)?;
    let Some(callbacks) = android_host_crypto_callbacks_snapshot(runtime_id) else {
        return Err(not_supported(operation));
    };
    let Some(import_callback) = callbacks.import_certificate else {
        return Err(not_supported(operation));
    };

    // encode callback arguments
    let encoded_kind = host_store_kind(kind, operation)?;
    let certificate_der = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_der = NativeSlice {
        data: certificate_der.as_ptr() as *mut u8,
        len: certificate_der.len() as u32,
    };

    // route host callback
    let status = unsafe { import_callback(runtime_id, encoded_kind, certificate_der) };
    host_status_result(status, operation, "import_certificate")
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve runtime id and callback entrypoint
    let runtime_id = callback_runtime_id(context, operation)?;
    let Some(callbacks) = android_host_crypto_callbacks_snapshot(runtime_id) else {
        return Err(not_supported(operation));
    };
    let Some(delete_callback) = callbacks.delete_certificate else {
        return Err(not_supported(operation));
    };

    // encode callback arguments
    let encoded_kind = host_store_kind(kind, operation)?;
    let certificate_der = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_der = NativeSlice {
        data: certificate_der.as_ptr() as *mut u8,
        len: certificate_der.len() as u32,
    };

    // route host callback and treat missing-certificate delete as success
    let status = unsafe { delete_callback(runtime_id, encoded_kind, certificate_der) };
    if status == HOST_STATUS_NOT_FOUND {
        return Ok(());
    }

    host_status_result(status, operation, "delete_certificate")
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
