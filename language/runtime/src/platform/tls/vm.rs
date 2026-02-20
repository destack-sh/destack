use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tls::{
    TlsContextOptions, TlsContextOptionsVm, TlsHandshakeStatus, TlsHostnameVerificationMode,
    TlsSessionResumptionMode, TlsSessionResumptionState, host as host_tls,
};
use crate::platform::{NativeSlice, NativeStringRef, NativeStringSlice, VmSlice, resource};
use crate::runtime::BindingCallContext;

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate one output slot and invoke one host call
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    call(value.as_mut_ptr())?;

    Ok(unsafe { value.assume_init() })
}

fn string_ref_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    // resolve one VM string handle
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    // store one call-scoped native string reference
    Ok(runtime.store_string(value.as_str()))
}

fn string_slice_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<vm::StringHandle>,
) -> RuntimeResult<NativeStringSlice> {
    // decode one VM string handle slice
    let values = values.read_values(context)?;
    let mut native_values = Vec::with_capacity(values.len());
    for value in values {
        native_values.push(string_ref_from_vm(runtime, context, value)?);
    }

    // store one native string slice for host calls
    Ok(runtime.store_string_slice(native_values))
}

fn bytes_slices_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<NativeSlice<NativeSlice<u8>>> {
    // decode one VM nested byte-slice payload
    let values = values.raw_values(context)?;
    let mut native_values = Vec::with_capacity(values.len());
    for value in values {
        let value = VmSlice::<u8>::from_value(context, value, "alpn_protocols", "Slice<uint8>")?;
        let value = value.read_bytes(context)?;
        native_values.push(runtime.store_slice(value));
    }

    // store one native nested byte-slice payload
    Ok(runtime.store_slice(native_values))
}

fn bytes_slice_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    // copy one VM byte slice into native call storage
    let values = values.read_bytes(context)?;

    Ok(runtime.store_slice(values))
}

fn bytes_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    // read one native byte slice
    let values = unsafe { values.as_slice()? };

    // encode one VM byte slice
    VmSlice::from_values(context, values)
}

fn allocate_read_buffer(runtime: &BindingCallContext, buffer: VmSlice<u8>) -> NativeSlice<u8> {
    // allocate one native read buffer with matching length
    let length = buffer.len as usize;

    runtime.store_slice(vec![0u8; length])
}

fn write_read_buffer(
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // copy one native read buffer back into VM memory
    let bytes = unsafe { native.as_slice()? };

    buffer.write_bytes(context, bytes)
}

fn context_options_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: TlsContextOptionsVm,
) -> RuntimeResult<TlsContextOptions> {
    // decode one ALPN protocol list
    let alpn_protocols = bytes_slices_from_vm(runtime, context, options.alpn_protocols)?;

    Ok(TlsContextOptions {
        role: options.role,
        min_version: options.min_version,
        max_version: options.max_version,
        verify_peer: options.verify_peer,
        alpn_protocols,
    })
}

/// Close one tls context object.
///
/// Release one backend-backed tls context and associated host resources.
/// Existing sessions created from this context remain backend-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses provider-specific context teardown semantics.
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_context_close(runtime, handle) }
}

/// Open one tls context object.
///
/// Create one backend-backed tls context with explicit role and version bounds.
/// Cipher suite policy and backend defaults follow host tls backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider context APIs.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: TlsContextOptionsVm,
) -> RuntimeResult<resource::TlsContextHandle> {
    // decode one VM options payload
    let options = context_options_from_vm(runtime, context, options)?;

    call_out(|out| unsafe { host_tls::destack_tls_context_open(runtime, out, options) })
}

/// Set allowed tls cipher suites for one context.
///
/// Apply one ordered list of cipher-suite names to one context policy.
/// Name parsing and provider-specific filtering follow backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses SSL_CTX_set_ciphersuites style APIs in OpenSSL or BoringSSL and equivalent provider policy APIs in Schannel or SecureTransport.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    suites: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    // decode one VM string slice
    let suites = string_slice_from_vm(runtime, context, suites)?;

    unsafe { host_tls::destack_tls_context_set_cipher_suites(runtime, handle, suites) }
}

/// Set allowed tls key exchange groups for one context.
///
/// Apply one ordered list of key exchange groups to one context policy.
/// Group parsing and provider-specific filtering follow backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses SSL_CTX_set1_groups_list style APIs in OpenSSL or BoringSSL and equivalent provider policy APIs in Schannel or SecureTransport.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    groups: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    // decode one VM string slice
    let groups = string_slice_from_vm(runtime, context, groups)?;

    unsafe { host_tls::destack_tls_context_set_groups(runtime, handle, groups) }
}

/// Set hostname verification mode for one context.
///
/// Configure hostname verification behavior for sessions created by this context.
/// Verification defaults match strict hostname checks unless explicitly overridden.
///
/// # Platform
/// Unix and Windows.
/// Uses X509_VERIFY_PARAM_set_hostflags style APIs in OpenSSL or BoringSSL and equivalent provider verification controls in Schannel or SecureTransport.
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    mode: TlsHostnameVerificationMode,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_context_set_hostname_verification_mode(runtime, handle, mode) }
}

/// Set one local certificate chain and private key on a tls context.
///
/// Install one PEM-encoded certificate chain and one PEM-encoded private key for local endpoint authentication.
/// Key parsing and supported key formats follow host provider behavior.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    certificatechainpem: VmSlice<u8>,
    privatekeypem: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode one certificate chain payload
    let certificatechainpem = bytes_slice_from_vm(runtime, context, certificatechainpem)?;

    // decode one private key payload
    let privatekeypem = bytes_slice_from_vm(runtime, context, privatekeypem)?;

    unsafe {
        host_tls::destack_tls_context_set_identity_pem(
            runtime,
            handle,
            certificatechainpem,
            privatekeypem,
        )
    }
}

/// Set keylog emission for one context.
///
/// Enable or disable NSS keylog line emission for sessions from this context.
/// Output destination routing is controlled by runtime telemetry or debug sinks.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider keylog callback APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tls.context`, `tls.keylog`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_tls_context_set_keylog_enabled(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_context_set_keylog_enabled(runtime, handle, enabled) }
}

/// Set session resumption policy for one context.
///
/// Configure whether sessions use stateful cache, stateless tickets, or both.
/// Cache size, lifetime, and ticket semantics follow backend policy.
///
/// # Platform
/// Unix and Windows.
/// Uses SSL_CTX_set_session_cache_mode and SSL_CTX_set_options style APIs in OpenSSL or BoringSSL and equivalent provider controls in Schannel or SecureTransport.
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    mode: TlsSessionResumptionMode,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_context_set_session_resumption(runtime, handle, mode) }
}

/// Set allowed tls signature algorithms for one context.
///
/// Apply one ordered list of signature algorithms to one context policy.
/// Algorithm parsing and provider-specific filtering follow backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses SSL_CTX_set1_sigalgs_list style APIs in OpenSSL or BoringSSL and equivalent provider policy APIs in Schannel or SecureTransport.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    algorithms: VmSlice<vm::StringHandle>,
) -> RuntimeResult<()> {
    // decode one VM string slice
    let algorithms = string_slice_from_vm(runtime, context, algorithms)?;

    unsafe { host_tls::destack_tls_context_set_signature_algorithms(runtime, handle, algorithms) }
}

/// Set trust anchors on a tls context from one PEM bundle.
///
/// Install one PEM-encoded trust-anchor bundle used for peer certificate validation.
/// Bundle parse rules and chain-building behavior follow host provider semantics.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsContextHandle,
    trustanchorspem: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode one PEM trust-anchor payload
    let trustanchorspem = bytes_slice_from_vm(runtime, context, trustanchorspem)?;

    unsafe { host_tls::destack_tls_context_set_trust_anchors_pem(runtime, handle, trustanchorspem) }
}

/// Close one tls session object.
///
/// Release one session object and provider-specific state.
/// Socket ownership remains with the caller and is not implicitly closed by this operation.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider session teardown APIs.
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_session_close(runtime, handle) }
}

/// Export keying material bytes for one tls session.
///
/// Derive exporter keying material for one label and optional context value.
/// Exporter derivation follows RFC 5705 and RFC 8446 provider rules.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider exporter APIs.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
    label: vm::StringHandle,
    argument_context: VmSlice<u8>,
    outputlength: u32,
) -> RuntimeResult<VmSlice<u8>> {
    // decode one label string
    let label = string_ref_from_vm(runtime, context, label)?;

    // decode one exporter context payload
    let argument_context = bytes_slice_from_vm(runtime, context, argument_context)?;

    // execute one host exporter call
    let bytes = call_out(|out| unsafe {
        host_tls::destack_tls_session_export_keying_material(
            runtime,
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
/// Handshake transitions follow host provider semantics and selected protocol version.
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<TlsHandshakeStatus> {
    call_out(|out| unsafe { host_tls::destack_tls_session_handshake(runtime, out, handle) })
}

/// Return negotiated alpn protocol bytes.
///
/// Read one negotiated application protocol value selected during handshake.
/// Empty bytes indicate no protocol was negotiated by the peer and provider.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider negotiated-protocol query APIs.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<VmSlice<u8>> {
    // read one native ALPN byte slice
    let value = call_out(|out| unsafe {
        host_tls::destack_tls_session_negotiated_alpn(runtime, out, handle)
    })?;

    bytes_slice_to_vm(context, value)
}

/// Open one tls session over one connected socket.
///
/// Bind one tls session object to one connected socket using one tls context.
/// Transport ownership remains with the caller, and tls uses the socket for encrypted record I/O.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider session APIs over socket transports.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    argument_context: resource::TlsContextHandle,
    socket: resource::SocketHandle,
    servername: vm::StringHandle,
) -> RuntimeResult<resource::TlsSessionHandle> {
    // decode one server name
    let servername = string_ref_from_vm(runtime, context, servername)?;

    call_out(|out| unsafe {
        host_tls::destack_tls_session_open(runtime, out, argument_context, socket, servername)
    })
}

/// Return the peer certificate chain bytes in pem encoding.
///
/// Read one peer certificate chain as normalized PEM bytes for verification and inspection.
/// Chain ordering and included intermediates follow host provider behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider peer-certificate query APIs.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<VmSlice<u8>> {
    // read one native PEM certificate chain payload
    let value = call_out(|out| unsafe {
        host_tls::destack_tls_session_peer_certificates_pem(runtime, out, handle)
    })?;

    bytes_slice_to_vm(context, value)
}

/// Read decrypted application bytes from one tls session.
///
/// Read plaintext bytes into one caller-provided buffer after record decryption.
/// Decrypt and read semantics follow provider buffering behavior and transport readiness.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider read APIs.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // allocate one native read buffer
    let native_buffer = allocate_read_buffer(runtime, buffer);

    // execute one host read call
    let bytes_read = call_out(|out| unsafe {
        host_tls::destack_tls_session_read(runtime, out, handle, native_buffer)
    })?;

    // copy bytes back into VM memory
    write_read_buffer(context, buffer, native_buffer)?;

    Ok(bytes_read)
}

/// Return whether one session resumed from cached state or ticket.
///
/// Report resumption state as observed by the backend after handshake completion.
/// State semantics follow backend cache and ticket policy behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider session-state query APIs.
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<TlsSessionResumptionState> {
    call_out(|out| unsafe { host_tls::destack_tls_session_resumption_state(runtime, out, handle) })
}

/// Shutdown one tls session.
///
/// Emit closure alerts and transition one session to closed state.
/// Half-close behavior and alert sequencing follow host provider semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider shutdown APIs.
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    unsafe { host_tls::destack_tls_session_shutdown(runtime, handle) }
}

/// Write plaintext application bytes to one tls session.
///
/// Encrypt and write plaintext bytes from one caller-provided buffer into tls records.
/// Record emission and flush behavior follow provider buffering and transport readiness.
///
/// # Platform
/// Unix and Windows.
/// Uses host tls provider write APIs.
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TlsSessionHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // decode one VM write buffer
    let buffer = bytes_slice_from_vm(runtime, context, buffer)?;

    call_out(|out| unsafe { host_tls::destack_tls_session_write(runtime, out, handle, buffer) })
}
