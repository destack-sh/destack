#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tls::{
    TlsContextOptionsVm, TlsHandshakeStatus, TlsHostnameVerificationMode, TlsSessionResumptionMode,
    TlsSessionResumptionState,
};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Close one tls context object.
pub(crate) fn destack_tls_context_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.context.close")).boxed())
}

/// Open one tls context object.
pub(crate) fn destack_tls_context_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: TlsContextOptionsVm,
) -> RuntimeResult<resource::TlsContextHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.context.open")).boxed())
}

/// Set allowed tls cipher suites for one context.
pub(crate) fn destack_tls_context_set_cipher_suites(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    suites: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (handle, suites);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setCipherSuites",
    ))
    .boxed())
}

/// Set allowed tls key exchange groups for one context.
pub(crate) fn destack_tls_context_set_groups(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    groups: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (handle, groups);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setGroups",
    ))
    .boxed())
}

/// Set hostname verification mode for one context.
pub(crate) fn destack_tls_context_set_hostname_verification_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_tls_context_set_identity_pem(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    certificatechainpem: VmSlice<u8>,
    privatekeypem: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, certificatechainpem, privatekeypem);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setIdentityPem",
    ))
    .boxed())
}

/// Set session resumption policy for one context.
pub(crate) fn destack_tls_context_set_session_resumption(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_tls_context_set_signature_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    algorithms: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    let _ = (handle, algorithms);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setSignatureAlgorithms",
    ))
    .boxed())
}

/// Set trust anchors on a tls context from one PEM bundle.
pub(crate) fn destack_tls_context_set_trust_anchors_pem(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    trustanchorspem: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, trustanchorspem);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.context.setTrustAnchorsPem",
    ))
    .boxed())
}

/// Close one tls session object.
pub(crate) fn destack_tls_session_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.close")).boxed())
}

/// Export keying material bytes for one tls session.
pub(crate) fn destack_tls_session_export_keying_material(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
    label: vm::StringHandle,
    argument_context: VmSlice<u8>,
    outputlength: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, label, argument_context, outputlength);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.exportKeyingMaterial",
    ))
    .boxed())
}

/// Advance one tls handshake state machine.
pub(crate) fn destack_tls_session_handshake(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<TlsHandshakeStatus> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.handshake",
    ))
    .boxed())
}

/// Return negotiated alpn protocol bytes.
pub(crate) fn destack_tls_session_negotiated_alpn(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.negotiatedAlpn",
    ))
    .boxed())
}

/// Open one tls session over one connected socket.
pub(crate) fn destack_tls_session_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    argument_context: resource::TlsContextHandle,
    socket: resource::SocketHandle,
    servername: vm::StringHandle,
) -> RuntimeResult<resource::TlsSessionHandle> {
    let _ = (argument_context, socket, servername);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.open")).boxed())
}

/// Return the peer certificate chain bytes in pem encoding.
pub(crate) fn destack_tls_session_peer_certificates_pem(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.peerCertificatesPem",
    ))
    .boxed())
}

/// Read decrypted application bytes from one tls session.
pub(crate) fn destack_tls_session_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.read")).boxed())
}

/// Return whether one session resumed from cached state or ticket.
pub(crate) fn destack_tls_session_resumption_state(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<TlsSessionResumptionState> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tls.session.resumptionState",
    ))
    .boxed())
}

/// Shutdown one tls session.
pub(crate) fn destack_tls_session_shutdown(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.shutdown")).boxed())
}

/// Write plaintext application bytes to one tls session.
pub(crate) fn destack_tls_session_write(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tls.session.write")).boxed())
}
