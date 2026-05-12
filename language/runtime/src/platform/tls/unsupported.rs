#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
use crate::platform::tls::abi_generated as bindings;

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::tls::{
    TlsContextOptions, TlsHandshakeStatus, TlsHostnameVerificationMode, TlsRole,
    TlsSessionResumptionMode, TlsSessionResumptionState, TlsVersion,
};

/// Close one tls context object.
pub(crate) unsafe fn destack_tls_context_close(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.context.close")).boxed())
}

/// Open one tls context object.
pub(crate) unsafe fn destack_tls_context_open(
    _binding: &BindingCallContext,
    out: *mut resource::TlsContextHandle,
    options: TlsContextOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.context.open")).boxed())
}

/// Set allowed tls cipher suites for one context.
pub(crate) unsafe fn destack_tls_context_set_cipher_suites(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    suites: NativeStringSlice,
) -> RuntimeResult<()> {
    let _ = (handle, suites);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setCipherSuites",
    ))
    .boxed())
}

/// Set allowed tls key exchange groups for one context.
pub(crate) unsafe fn destack_tls_context_set_groups(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    groups: NativeStringSlice,
) -> RuntimeResult<()> {
    let _ = (handle, groups);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setGroups",
    ))
    .boxed())
}

/// Set hostname verification mode for one context.
pub(crate) unsafe fn destack_tls_context_set_hostname_verification_mode(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    mode: TlsHostnameVerificationMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setHostnameVerificationMode",
    ))
    .boxed())
}

/// Set one local certificate chain and private key on a tls context.
pub(crate) unsafe fn destack_tls_context_set_identity_pem(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    certificatechainpem: NativeSlice<u8>,
    privatekeypem: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, certificatechainpem, privatekeypem);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setIdentityPem",
    ))
    .boxed())
}

/// Set keylog emission for one context.
pub(crate) unsafe fn destack_tls_context_set_session_resumption(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    mode: TlsSessionResumptionMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setSessionResumption",
    ))
    .boxed())
}

/// Set allowed tls signature algorithms for one context.
pub(crate) unsafe fn destack_tls_context_set_signature_algorithms(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    algorithms: NativeStringSlice,
) -> RuntimeResult<()> {
    let _ = (handle, algorithms);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setSignatureAlgorithms",
    ))
    .boxed())
}

/// Set trust anchors on a tls context from one PEM bundle.
pub(crate) unsafe fn destack_tls_context_set_trust_anchors_pem(
    _binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    trustanchorspem: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, trustanchorspem);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setTrustAnchorsPem",
    ))
    .boxed())
}

/// Close one tls session object.
pub(crate) unsafe fn destack_tls_session_close(
    _binding: &BindingCallContext,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.close")).boxed())
}

/// Export keying material bytes for one tls session.
pub(crate) unsafe fn destack_tls_session_export_keying_material(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::TlsSessionHandle,
    label: NativeStringRef,
    argument_context: NativeSlice<u8>,
    outputlength: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, label, argument_context, outputlength);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.exportKeyingMaterial",
    ))
    .boxed())
}

/// Advance one tls handshake state machine.
pub(crate) unsafe fn destack_tls_session_handshake(
    _binding: &BindingCallContext,
    out: *mut TlsHandshakeStatus,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.handshake",
    ))
    .boxed())
}

/// Return negotiated alpn protocol bytes.
pub(crate) unsafe fn destack_tls_session_negotiated_alpn(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.negotiatedAlpn",
    ))
    .boxed())
}

/// Open one tls session over one connected socket.
pub(crate) unsafe fn destack_tls_session_open(
    _binding: &BindingCallContext,
    out: *mut resource::TlsSessionHandle,
    argument_context: resource::TlsContextHandle,
    socket: resource::SocketHandle,
    servername: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, argument_context, socket, servername);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.open")).boxed())
}

/// Return the peer certificate chain bytes in pem encoding.
pub(crate) unsafe fn destack_tls_session_peer_certificates_pem(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.peerCertificatesPem",
    ))
    .boxed())
}

/// Read decrypted application bytes from one tls session.
pub(crate) unsafe fn destack_tls_session_read(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TlsSessionHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.read")).boxed())
}

/// Return whether one session resumed from cached state or ticket.
pub(crate) unsafe fn destack_tls_session_resumption_state(
    _binding: &BindingCallContext,
    out: *mut TlsSessionResumptionState,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.resumptionState",
    ))
    .boxed())
}

/// Shutdown one tls session.
pub(crate) unsafe fn destack_tls_session_shutdown(
    _binding: &BindingCallContext,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.shutdown")).boxed())
}

/// Write plaintext application bytes to one tls session.
pub(crate) unsafe fn destack_tls_session_write(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TlsSessionHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.write")).boxed())
}
