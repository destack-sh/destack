use super::{assert_invalid_argument_value, placeholder_context_handle, with_harness_context};
use crate::platform::tls::{
    TlsHostnameVerificationMode, TlsRole, TlsSessionResumptionMode, TlsVersion,
};

/// Return invalidArgumentValue when closing one unknown TLS context handle.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_close_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_context_close(placeholder_context_handle()),
        )
    });
}

/// Return invalidArgumentValue when setting cipher suites on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_cipher_suites_invalid_argument_value() {
    with_harness_context(|mut context| {
        let suites = context.string_slice_value(&[])?;
        assert_invalid_argument_value(
            context.destack_tls_context_set_cipher_suites(placeholder_context_handle(), suites),
        )
    });
}

/// Return invalidArgumentValue when setting groups on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_groups_invalid_argument_value() {
    with_harness_context(|mut context| {
        let groups = context.string_slice_value(&[])?;
        assert_invalid_argument_value(
            context.destack_tls_context_set_groups(placeholder_context_handle(), groups),
        )
    });
}

/// Return invalidArgumentValue when setting hostname mode on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_hostname_verification_mode_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(context.destack_tls_context_set_hostname_verification_mode(
            placeholder_context_handle(),
            TlsHostnameVerificationMode::Strict,
        ))
    });
}

/// Return invalidArgumentValue when setting identity PEM on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_identity_pem_invalid_argument_value() {
    with_harness_context(|mut context| {
        let certificate_chain = context.bytes_value(&[1u8, 2u8])?;
        let private_key = context.bytes_value(&[3u8, 4u8])?;
        assert_invalid_argument_value(context.destack_tls_context_set_identity_pem(
            placeholder_context_handle(),
            certificate_chain,
            private_key,
        ))
    });
}

/// Return invalidArgumentValue when setting resumption mode on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_session_resumption_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(context.destack_tls_context_set_session_resumption(
            placeholder_context_handle(),
            TlsSessionResumptionMode::Stateful,
        ))
    });
}

/// Return invalidArgumentValue when setting signature algorithms on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_signature_algorithms_invalid_argument_value() {
    with_harness_context(|mut context| {
        let algorithms = context.string_slice_value(&[])?;
        assert_invalid_argument_value(
            context.destack_tls_context_set_signature_algorithms(
                placeholder_context_handle(),
                algorithms,
            ),
        )
    });
}

/// Return invalidArgumentValue when setting trust anchors on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_trust_anchors_pem_invalid_argument_value() {
    with_harness_context(|mut context| {
        let trust_anchors = context.bytes_value(&[1u8, 2u8])?;
        assert_invalid_argument_value(
            context.destack_tls_context_set_trust_anchors_pem(
                placeholder_context_handle(),
                trust_anchors,
            ),
        )
    });
}

/// Reject one invalid certificate and private key bundle when installing identity material.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_identity_pem_rejects_invalid_pem() {
    with_harness_context(|mut context| {
        let options = context.context_options_value(
            TlsRole::Server,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            false,
            &[],
        )?;
        let handle = context.destack_tls_context_open(options)?;

        let certificate_chain = context.bytes_value(b"not a certificate")?;
        let private_key = context.bytes_value(b"not a private key")?;
        let result =
            context.destack_tls_context_set_identity_pem(handle, certificate_chain, private_key);

        context.destack_tls_context_close(handle)?;

        assert_invalid_argument_value(result)
    });
}

/// Reject one invalid trust anchor bundle when verification material is installed.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_trust_anchors_pem_rejects_invalid_pem() {
    with_harness_context(|mut context| {
        let options = context.default_client_context_options()?;
        let handle = context.destack_tls_context_open(options)?;

        let trust_anchors = context.bytes_value(b"not a certificate")?;
        let result = context.destack_tls_context_set_trust_anchors_pem(handle, trust_anchors);

        context.destack_tls_context_close(handle)?;

        assert_invalid_argument_value(result)
    });
}

/// Reject unsupported cipher suite names even when some entries are valid.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_cipher_suites_rejects_unknown_names() {
    with_harness_context(|mut context| {
        let options = context.default_client_context_options()?;
        let handle = context.destack_tls_context_open(options)?;

        let suites = context.string_slice_value(&["TLS13_AES_128_GCM_SHA256", "NOT_A_SUITE"])?;
        let result = context.destack_tls_context_set_cipher_suites(handle, suites);

        context.destack_tls_context_close(handle)?;

        assert_invalid_argument_value(result)
    });
}

/// Reject unsupported key exchange groups even when some entries are valid.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_groups_rejects_unknown_names() {
    with_harness_context(|mut context| {
        let options = context.default_client_context_options()?;
        let handle = context.destack_tls_context_open(options)?;

        let groups = context.string_slice_value(&["X25519", "NOT_A_GROUP"])?;
        let result = context.destack_tls_context_set_groups(handle, groups);

        context.destack_tls_context_close(handle)?;

        assert_invalid_argument_value(result)
    });
}

/// Reject signature scheme names that the selected provider cannot verify.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_signature_algorithms_rejects_unsupported_provider_schemes() {
    with_harness_context(|mut context| {
        let options = context.default_client_context_options()?;
        let handle = context.destack_tls_context_open(options)?;

        let algorithms = context.string_slice_value(&["ED448"])?;
        let result = context.destack_tls_context_set_signature_algorithms(handle, algorithms);

        context.destack_tls_context_close(handle)?;

        assert_invalid_argument_value(result)
    });
}
