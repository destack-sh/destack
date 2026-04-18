use openssl::x509::X509;

use crate::diagnostic::RuntimeResult;
use crate::host::HostStatus;
use crate::host::os::android::abi::crypto::ffi::{
    destack_host_android_crypto_delete_certificate, destack_host_android_crypto_import_certificate,
    destack_host_android_crypto_supports_certificate_write,
};
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::CryptoStoreKind;
use crate::platform::crypto::host::unix::core as unix_core;
use crate::runtime::BindingCallContext;

use super::core::{
    configured_system_certificate_directories, configured_system_certificate_files,
    host_session_id, host_status_result, host_store_kind, invalid_data,
};

/// Return whether one host store lane supports certificate write operations.
pub(crate) fn host_store_supports_certificate_write(
    binding: &BindingCallContext,
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
    let runtime_id = binding.worker().runtime_id.0;
    let Ok(encoded_kind) = host_store_kind(kind, "destack.crypto.store.probeCapability") else {
        return false;
    };
    let status =
        unsafe { destack_host_android_crypto_supports_certificate_write(runtime_id, encoded_kind) };

    status == HostStatus::Ok.code()
}

/// Import one certificate into one host store lane.
pub(crate) fn host_store_import_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve runtime id
    let runtime_id = host_session_id(binding, operation)?;

    // encode host arguments
    let encoded_kind = host_store_kind(kind, operation)?;
    let certificate_der = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_der = NativeSlice {
        data: certificate_der.as_ptr() as *mut u8,
        len: certificate_der.len() as u32,
    };

    // route host entrypoint
    let status = unsafe {
        destack_host_android_crypto_import_certificate(runtime_id, encoded_kind, certificate_der)
    };
    host_status_result(status, operation, "import_certificate")
}

/// Delete one certificate from one host store lane.
pub(crate) fn host_store_delete_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve runtime id
    let runtime_id = host_session_id(binding, operation)?;

    // encode host arguments
    let encoded_kind = host_store_kind(kind, operation)?;
    let certificate_der = certificate
        .to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let certificate_der = NativeSlice {
        data: certificate_der.as_ptr() as *mut u8,
        len: certificate_der.len() as u32,
    };

    // route host entrypoint and treat missing-certificate delete as success
    let status = unsafe {
        destack_host_android_crypto_delete_certificate(runtime_id, encoded_kind, certificate_der)
    };
    if status == HostStatus::NotFound.code() {
        return Ok(());
    }

    host_status_result(status, operation, "delete_certificate")
}

/// Collect certificates from configured Android system trust-bundle locations.
pub(super) fn collect_system_certificates(binding: &BindingCallContext) -> Vec<X509> {
    let system_certificate_files = configured_system_certificate_files(binding);
    let system_certificate_directories = configured_system_certificate_directories(binding);

    unix_core::collect_system_certificates(
        &system_certificate_files,
        &system_certificate_directories,
    )
}

/// Return whether one Android host certificate source path exists.
pub(super) fn has_system_certificate_source(binding: &BindingCallContext) -> bool {
    let system_certificate_files = configured_system_certificate_files(binding);
    let system_certificate_directories = configured_system_certificate_directories(binding);

    unix_core::has_system_certificate_source(
        &system_certificate_files,
        &system_certificate_directories,
    )
}
