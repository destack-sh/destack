use destack_vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{
    allocate_vm_read_buffer as allocate_read_buffer, bytes_to_vm as bytes_slice_to_vm, call_out,
    store_bytes_from_vm as bytes_slice_from_vm, store_string_from_vm as string_ref_from_vm,
    store_string_slice_from_vm as string_slice_from_vm,
    store_vm_byte_slices as bytes_slices_from_vm, write_vm_read_buffer as write_read_buffer,
};
use crate::platform::tls::{
    TlsContextOptions, TlsContextOptionsVm, TlsHandshakeStatus, TlsHostnameVerificationMode,
    TlsSessionResumptionMode, TlsSessionResumptionState, host as host_tls,
};
use crate::platform::{VmSlice, resource};
use crate::runtime::BindingCallContext;

/// Close one tls context object.
///
/// Release one backend-backed tls context and associated runtime resources.
/// Existing sessions created from this context keep their current runtime state.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls context teardown semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `tls.context`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_context_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_context_close(binding, handle) }
}

/// Open one tls context object.
///
/// Create one rustls-backed tls context with explicit role and version bounds.
/// Cipher suite, group, signature, and resumption policy are runtime-owned rather than delegated to host provider state.
///
/// # Platform
/// Unix and Windows.
/// Uses the runtime tls engine over host sockets.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.context`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_context_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    options: TlsContextOptionsVm,
) -> RuntimeResult<resource::TlsContextHandle> {
    // decode one VM options payload
    let alpn_protocols =
        bytes_slices_from_vm(binding, context, options.alpn_protocols, "alpn_protocols")?;
    let options = TlsContextOptions {
        role: options.role,
        min_version: options.min_version,
        max_version: options.max_version,
        verify_peer: options.verify_peer,
        alpn_protocols,
    };

    call_out(|out| unsafe { host_tls::destack_tls_context_open(binding, out, options) })
}

/// Set allowed tls cipher suites for one context.
///
/// Apply one ordered list of cipher-suite names to one context policy.
/// Names are matched case-insensitively after normalizing spaces and hyphens to underscores against rustls cipher-suite names.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls policy filtering.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.context`, `tls.policy`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_cipher_suites(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    suites: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    // decode one VM string slice
    let suites = string_slice_from_vm(binding, context, suites)?;

    unsafe { host_tls::destack_tls_context_set_cipher_suites(binding, handle, suites) }
}

/// Set allowed tls key exchange groups for one context.
///
/// Apply one ordered list of key exchange groups to one context policy.
/// Names are matched case-insensitively after normalizing spaces and hyphens to underscores against rustls key-exchange group names.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls policy filtering.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.context`, `tls.policy`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_groups(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    groups: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    // decode one VM string slice
    let groups = string_slice_from_vm(binding, context, groups)?;

    unsafe { host_tls::destack_tls_context_set_groups(binding, handle, groups) }
}

/// Set hostname verification mode for one context.
///
/// Configure hostname verification behavior for sessions created by this context.
/// Verification defaults match strict hostname checks unless explicitly overridden.
/// This setting applies to client contexts, and server contexts ignore it.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls verification policy.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.context`, `tls.hostname.verify`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_hostname_verification_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    mode: TlsHostnameVerificationMode,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_context_set_hostname_verification_mode(binding, handle, mode) }
}

/// Set one local certificate chain and private key on a tls context.
///
/// Install one PEM-encoded certificate chain and one PEM-encoded private key for local endpoint authentication.
/// PEM parsing is strict and malformed inputs fail immediately.
///
/// # Platform
/// Unix and Windows.
/// Uses provider identity import APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.identity.use`, `tls.identity.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_identity_pem(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    certificatechainpem: VmSlice<u8>,
    privatekeypem: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode one certificate chain payload
    let certificatechainpem = bytes_slice_from_vm(binding, context, certificatechainpem)?;

    // decode one private key payload
    let privatekeypem = bytes_slice_from_vm(binding, context, privatekeypem)?;

    unsafe {
        host_tls::destack_tls_context_set_identity_pem(
            binding,
            handle,
            certificatechainpem,
            privatekeypem,
        )
    }
}

/// Set session resumption policy for one context.
///
/// Configure whether sessions use stateful cache, stateless tickets, or both.
/// Cache size, lifetime, and ticket semantics follow bounded runtime policy.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls resumption controls.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.context`, `tls.resumption`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_session_resumption(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    mode: TlsSessionResumptionMode,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_context_set_session_resumption(binding, handle, mode) }
}

/// Set allowed tls signature algorithms for one context.
///
/// Apply one ordered list of signature algorithms to one context policy.
/// Names are matched case-insensitively after normalizing spaces and hyphens to underscores against rustls signature-scheme names.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls policy filtering.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.context`, `tls.policy`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_signature_algorithms(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    algorithms: VmSlice<destack_vm::StringHandle>,
) -> RuntimeResult<()> {
    // decode one VM string slice
    let algorithms = string_slice_from_vm(binding, context, algorithms)?;

    unsafe { host_tls::destack_tls_context_set_signature_algorithms(binding, handle, algorithms) }
}

/// Set trust anchors on a tls context from one PEM bundle.
///
/// Install one PEM-encoded trust-anchor bundle used for peer certificate validation.
/// Bundle parsing is strict and malformed trust anchors fail immediately.
///
/// # Platform
/// Unix and Windows.
/// Uses provider trust-store APIs or runtime trust bundle loading.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.trust.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_trust_anchors_pem(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsContextHandle,
    trustanchorspem: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode one PEM trust-anchor payload
    let trustanchorspem = bytes_slice_from_vm(binding, context, trustanchorspem)?;

    unsafe { host_tls::destack_tls_context_set_trust_anchors_pem(binding, handle, trustanchorspem) }
}

/// Close one tls session object.
///
/// Release one session object and runtime tls state.
/// Socket ownership remains with the caller and is not implicitly closed by this operation.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls session teardown.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `tls.session`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_session_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_session_close(binding, handle) }
}

/// Export keying material bytes for one tls session.
///
/// Derive exporter keying material for one label and one explicit context value.
/// Empty context bytes remain distinct input and are not treated as absence.
/// Exporter derivation follows RFC 5705 and RFC 8446 runtime tls rules.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls exporter support.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.session`, `tls.exporter`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_session_export_keying_material(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
    label: destack_vm::StringHandle,
    argument_context: VmSlice<u8>,
    outputlength: u32,
) -> RuntimeResult<VmSlice<u8>> {
    // decode one label string
    let label = string_ref_from_vm(binding, context, label)?;

    // decode one exporter context payload
    let argument_context = bytes_slice_from_vm(binding, context, argument_context)?;

    // execute one host exporter call
    let bytes = call_out(|out| unsafe {
        host_tls::destack_tls_session_export_keying_material(
            binding,
            out,
            handle,
            label,
            argument_context,
            outputlength,
        )
    })?;

    bytes_slice_to_vm(context, bytes)
}

/// Advance one tls handshake state machine.
///
/// Drive one handshake step for one session and return readiness requirements for continuation.
/// Handshake transitions follow rustls state-machine semantics for the selected protocol version.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls handshake step APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.handshake`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_session_handshake(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<TlsHandshakeStatus> {
    call_out(|out| unsafe { host_tls::destack_tls_session_handshake(binding, out, handle) })
}

/// Return negotiated alpn protocol bytes.
///
/// Read one negotiated application protocol value selected during handshake.
/// Empty bytes indicate no protocol was negotiated by the peer and provider.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls negotiated-protocol state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.session`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_session_negotiated_alpn(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<VmSlice<u8>> {
    // read one native ALPN byte slice
    let value = call_out(|out| unsafe {
        host_tls::destack_tls_session_negotiated_alpn(binding, out, handle)
    })?;

    bytes_slice_to_vm(context, value)
}

/// Open one tls session over one connected socket.
///
/// Bind one tls session object to one connected socket using one tls context.
/// Transport ownership remains with the caller, and tls uses the socket for encrypted record I/O.
/// `serverName` is required for client contexts and ignored for server contexts.
///
/// # Platform
/// Unix and Windows.
/// Uses the runtime tls engine over host sockets.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.session`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_session_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    argument_context: resource::TlsContextHandle,
    socket: resource::SocketHandle,
    servername: destack_vm::StringHandle,
) -> RuntimeResult<resource::TlsSessionHandle> {
    // decode one server name
    let servername = string_ref_from_vm(binding, context, servername)?;

    call_out(|out| unsafe {
        host_tls::destack_tls_session_open(binding, out, argument_context, socket, servername)
    })
}

/// Return the peer certificate chain bytes in pem encoding.
///
/// Read one peer certificate chain as normalized PEM bytes for verification and inspection.
/// Chain ordering and included intermediates follow peer presentation order.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls peer-certificate state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_session_peer_certificates_pem(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<VmSlice<u8>> {
    // read one native PEM certificate chain payload
    let value = call_out(|out| unsafe {
        host_tls::destack_tls_session_peer_certificates_pem(binding, out, handle)
    })?;

    bytes_slice_to_vm(context, value)
}

/// Read decrypted application bytes from one tls session.
///
/// Read plaintext bytes into one caller-provided buffer after record decryption.
/// Decrypt and read semantics follow runtime buffering behavior and transport readiness.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls record processing over host sockets.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.session`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_session_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // allocate one native read buffer
    let native_buffer = allocate_read_buffer(binding, buffer);

    // execute one host read call
    let bytes_read = call_out(|out| unsafe {
        host_tls::destack_tls_session_read(binding, out, handle, native_buffer)
    })?;

    // copy bytes back into VM memory
    write_read_buffer(context, buffer, native_buffer)?;

    Ok(bytes_read)
}

/// Return whether one session resumed from cached state or ticket.
///
/// Report resumption state as observed by the runtime after handshake completion.
/// State semantics follow runtime cache and ticket policy behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls session-state inspection.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.session`, `tls.resumption`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_session_resumption_state(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<TlsSessionResumptionState> {
    call_out(|out| unsafe { host_tls::destack_tls_session_resumption_state(binding, out, handle) })
}

/// Shutdown one tls session.
///
/// Emit closure alerts and transition one session to closed state.
/// Half-close behavior and alert sequencing follow rustls close-notify semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls shutdown state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.session`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tls_session_shutdown(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_session_shutdown(binding, handle) }
}

/// Write plaintext application bytes to one tls session.
///
/// Encrypt and write plaintext bytes from one caller-provided buffer into tls records.
/// Record emission and flush behavior follow runtime buffering and transport readiness.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime tls record processing over host sockets.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.session`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_session_write(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::TlsSessionHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // decode one VM write buffer
    let buffer = bytes_slice_from_vm(binding, context, buffer)?;

    call_out(|out| unsafe { host_tls::destack_tls_session_write(binding, out, handle, buffer) })
}
