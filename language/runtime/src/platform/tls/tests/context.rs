use super::{assert_invalid_argument_value, placeholder_context_handle, with_harness_context};
use crate::platform::tls::{TlsHostnameVerificationMode, TlsSessionResumptionMode};

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

/// Return invalidArgumentValue when toggling keylog on one unknown context.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_context_set_keylog_enabled_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_context_set_keylog_enabled(placeholder_context_handle(), true),
        )
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
