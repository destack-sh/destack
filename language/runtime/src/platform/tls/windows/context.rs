use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringSlice};
use crate::platform::tls::{
    TlsContextOptions, TlsHostnameVerificationMode, TlsSessionResumptionMode, core as core_tls,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Close one tls context object.
pub(crate) unsafe fn destack_tls_context_close(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
) -> RuntimeResult<()> {
    core_tls::remove_context_resource(binding, handle)
}

/// Open one tls context object.
pub(crate) unsafe fn destack_tls_context_open(
    binding: &BindingCallContext,
    out: *mut resource::TlsContextHandle,
    options: TlsContextOptions,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // build and store one binding resource
    let policy = core_tls::TlsContextResource::from_options(options)?;
    let handle = core_tls::insert_context_resource(binding, policy);

    // write the resulting handle
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Set allowed tls cipher suites for one context.
pub(crate) unsafe fn destack_tls_context_set_cipher_suites(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    suites: NativeStringSlice,
) -> RuntimeResult<()> {
    // decode and validate suite names
    let suites = core_tls::decode_native_string_slice(suites)?;
    if suites.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "suites",
            "suites must be non-empty",
        ))
        .boxed());
    }

    // store the suite policy
    let policy = core_tls::resolve_context_resource(binding, handle)?;
    let mut policy = policy.lock();
    core_tls::validate_cipher_suites(&policy, &suites)?;
    policy.cipher_suites = Some(suites);
    policy.reset_runtime_state();

    Ok(())
}

/// Set allowed tls key exchange groups for one context.
pub(crate) unsafe fn destack_tls_context_set_groups(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    groups: NativeStringSlice,
) -> RuntimeResult<()> {
    // decode and validate group names
    let groups = core_tls::decode_native_string_slice(groups)?;
    if groups.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "groups",
            "groups must be non-empty",
        ))
        .boxed());
    }

    // store the group policy
    let policy = core_tls::resolve_context_resource(binding, handle)?;
    let mut policy = policy.lock();
    core_tls::validate_groups(&policy, &groups)?;
    policy.groups = Some(groups);
    policy.reset_runtime_state();

    Ok(())
}

/// Set hostname verification mode for one context.
pub(crate) unsafe fn destack_tls_context_set_hostname_verification_mode(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    mode: TlsHostnameVerificationMode,
) -> RuntimeResult<()> {
    // store the hostname verification mode
    let policy = core_tls::resolve_context_resource(binding, handle)?;
    let mut policy = policy.lock();
    policy.hostname_mode = mode;
    policy.reset_runtime_state();

    Ok(())
}

/// Set one local certificate chain and private key on a tls context.
pub(crate) unsafe fn destack_tls_context_set_identity_pem(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    certificatechainpem: NativeSlice<u8>,
    privatekeypem: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // decode the PEM payloads
    let certificate_chain = core_tls::decode_native_bytes(certificatechainpem)?;
    let private_key = core_tls::decode_native_bytes(privatekeypem)?;
    if certificate_chain.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "certificateChainPem",
            "certificateChainPem must be non-empty",
        ))
        .boxed());
    }
    if private_key.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "privateKeyPem",
            "privateKeyPem must be non-empty",
        ))
        .boxed());
    }
    core_tls::validate_identity_pem(&certificate_chain, &private_key)?;

    // store identity material
    let policy = core_tls::resolve_context_resource(binding, handle)?;
    let mut policy = policy.lock();
    policy.identity_chain_pem = Some(certificate_chain);
    policy.identity_key_pem = Some(private_key);
    policy.reset_runtime_state();

    Ok(())
}

/// Set session resumption policy for one context.
pub(crate) unsafe fn destack_tls_context_set_session_resumption(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    mode: TlsSessionResumptionMode,
) -> RuntimeResult<()> {
    // store resumption policy
    let policy = core_tls::resolve_context_resource(binding, handle)?;
    let mut policy = policy.lock();
    policy.resumption_mode = mode;
    policy.reset_runtime_state();

    Ok(())
}

/// Set allowed tls signature algorithms for one context.
pub(crate) unsafe fn destack_tls_context_set_signature_algorithms(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    algorithms: NativeStringSlice,
) -> RuntimeResult<()> {
    // decode and validate algorithm names
    let algorithms = core_tls::decode_native_string_slice(algorithms)?;
    if algorithms.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "algorithms",
            "algorithms must be non-empty",
        ))
        .boxed());
    }
    let algorithms = core_tls::parse_signature_algorithms(&algorithms)?;

    // store signature algorithm policy
    let policy = core_tls::resolve_context_resource(binding, handle)?;
    let mut policy = policy.lock();
    core_tls::validate_signature_algorithms(&policy, &algorithms)?;
    policy.signature_algorithms = Some(algorithms);
    policy.reset_runtime_state();

    Ok(())
}

/// Set trust anchors on a tls context from one PEM bundle.
pub(crate) unsafe fn destack_tls_context_set_trust_anchors_pem(
    binding: &BindingCallContext,
    handle: resource::TlsContextHandle,
    trustanchorspem: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // decode and validate trust anchors
    let trust_anchors = core_tls::decode_native_bytes(trustanchorspem)?;
    if trust_anchors.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "trustAnchorsPem",
            "trustAnchorsPem must be non-empty",
        ))
        .boxed());
    }
    core_tls::validate_trust_anchor_pem(&trust_anchors)?;

    // store trust anchors
    let policy = core_tls::resolve_context_resource(binding, handle)?;
    let mut policy = policy.lock();
    policy.trust_anchors_pem = Some(trust_anchors);
    policy.reset_runtime_state();

    Ok(())
}
