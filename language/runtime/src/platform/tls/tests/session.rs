use super::{
    TlsHarnessContext, assert_invalid_argument_value, placeholder_context_handle,
    placeholder_session_handle, placeholder_socket_handle, with_harness_context,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource;
use crate::platform::tls::{
    TlsHandshakeStatus, TlsRole, TlsSessionResumptionMode, TlsSessionResumptionState, TlsVersion,
};

const TEST_CA_CERT_PEM: &[u8] = include_bytes!("fixtures/tls-test-ca.cert.pem");
const TEST_SERVER_CERT_PEM: &[u8] = include_bytes!("fixtures/tls-test-server.cert.pem");
const TEST_SERVER_KEY_PEM: &[u8] = include_bytes!("fixtures/tls-test-server.key.pem");
const TEST_CLIENT_CERT_PEM: &[u8] = include_bytes!("fixtures/tls-test-client.cert.pem");
const TEST_CLIENT_KEY_PEM: &[u8] = include_bytes!("fixtures/tls-test-client.key.pem");

/// Return invalidArgumentValue when opening one session with unknown handles.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_open_invalid_argument_value() {
    with_harness_context(|mut context| {
        let server_name = context.string_value("example.com");
        assert_invalid_argument_value(context.destack_tls_session_open(
            placeholder_context_handle(),
            placeholder_socket_handle(),
            server_name,
        ))
    });
}

/// Return invalidArgumentValue when closing one unknown TLS session handle.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_close_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_session_close(placeholder_session_handle()),
        )
    });
}

/// Return invalidArgumentValue when advancing one unknown TLS session handshake.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_handshake_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_session_handshake(placeholder_session_handle()),
        )
    });
}

/// Return invalidArgumentValue when reading ALPN from one unknown TLS session.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_negotiated_alpn_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_session_negotiated_alpn(placeholder_session_handle()),
        )
    });
}

/// Return invalidArgumentValue when reading peer certs from one unknown TLS session.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_peer_certificates_pem_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_session_peer_certificates_pem(placeholder_session_handle()),
        )
    });
}

/// Return invalidArgumentValue when exporting keying material from one unknown session.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_export_keying_material_invalid_argument_value() {
    with_harness_context(|mut context| {
        let label = context.string_value("EXPORTER-TEST");
        let argument_context = context.bytes_value(&[1u8, 2u8])?;
        assert_invalid_argument_value(context.destack_tls_session_export_keying_material(
            placeholder_session_handle(),
            label,
            argument_context,
            16,
        ))
    });
}

/// Return invalidArgumentValue when reading plaintext from one unknown TLS session.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_read_invalid_argument_value() {
    with_harness_context(|mut context| {
        let buffer = context.zeroed_bytes_value(8)?;
        assert_invalid_argument_value(
            context.destack_tls_session_read(placeholder_session_handle(), buffer),
        )
    });
}

/// Return invalidArgumentValue when writing plaintext to one unknown TLS session.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_write_invalid_argument_value() {
    with_harness_context(|mut context| {
        let bytes = context.bytes_value(&[1u8, 2u8, 3u8])?;
        assert_invalid_argument_value(
            context.destack_tls_session_write(placeholder_session_handle(), bytes),
        )
    });
}

/// Return invalidArgumentValue when reading resumption state from one unknown session.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_resumption_state_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_session_resumption_state(placeholder_session_handle()),
        )
    });
}

/// Return invalidArgumentValue when shutting down one unknown TLS session.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_shutdown_invalid_argument_value() {
    with_harness_context(|mut context| {
        assert_invalid_argument_value(
            context.destack_tls_session_shutdown(placeholder_session_handle()),
        )
    });
}

fn platform_error_code(error: &RuntimeError) -> PlatformErrorCode {
    let platform = error
        .platform_error()
        .expect("error should contain one platform error");

    platform.code
}

fn complete_handshake(
    context: &mut TlsHarnessContext<'_>,
    client_session: resource::TlsSessionHandle,
    server_session: resource::TlsSessionHandle,
) -> RuntimeResult<()> {
    for _ in 0..256 {
        let client = context.destack_tls_session_handshake(client_session)?;
        let server = context.destack_tls_session_handshake(server_session)?;
        if matches!(client, TlsHandshakeStatus::Complete)
            && matches!(server, TlsHandshakeStatus::Complete)
        {
            return Ok(());
        }
    }

    panic!("handshake did not complete in bounded steps");
}

fn write_payload(
    context: &mut TlsHarnessContext<'_>,
    session: resource::TlsSessionHandle,
    payload: &[u8],
) -> RuntimeResult<()> {
    let mut offset = 0usize;
    for _ in 0..256 {
        if offset >= payload.len() {
            return Ok(());
        }

        let bytes = context.bytes_value(&payload[offset..])?;
        match context.destack_tls_session_write(session, bytes) {
            Ok(written) => {
                assert!(written > 0);
                offset += written as usize;
            }
            Err(error) => {
                if platform_error_code(error.as_ref()) == PlatformErrorCode::IoWouldBlock {
                    continue;
                }
                return Err(error);
            }
        }
    }

    panic!("write did not complete in bounded steps");
}

fn read_payload(
    context: &mut TlsHarnessContext<'_>,
    session: resource::TlsSessionHandle,
    expected_length: usize,
) -> RuntimeResult<Vec<u8>> {
    for _ in 0..256 {
        let buffer = context.zeroed_bytes_value(expected_length)?;
        let (buffer_for_read, buffer_for_decode) = context.duplicate_value(buffer);
        match context.destack_tls_session_read(session, buffer_for_read) {
            Ok(read) => {
                if read == 0 {
                    continue;
                }
                let mut bytes = context.bytes_from_value(buffer_for_decode)?;
                bytes.truncate(read as usize);
                return Ok(bytes);
            }
            Err(error) => {
                if platform_error_code(error.as_ref()) == PlatformErrorCode::IoWouldBlock {
                    continue;
                }
                return Err(error);
            }
        }
    }

    panic!("read did not complete in bounded steps");
}

/// Complete handshake, ALPN negotiation, and bidirectional payload exchange.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_roundtrip_and_alpn() {
    with_harness_context(|mut context| {
        let client_options = context.context_options_value(
            TlsRole::Client,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            true,
            &[b"h2"],
        )?;
        let server_options = context.context_options_value(
            TlsRole::Server,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            false,
            &[b"h2"],
        )?;
        let client_context = context.destack_tls_context_open(client_options)?;
        let server_context = context.destack_tls_context_open(server_options)?;

        let server_cert = context.bytes_value(TEST_SERVER_CERT_PEM)?;
        let server_key = context.bytes_value(TEST_SERVER_KEY_PEM)?;
        context.destack_tls_context_set_identity_pem(server_context, server_cert, server_key)?;
        let trust_anchors = context.bytes_value(TEST_CA_CERT_PEM)?;
        context.destack_tls_context_set_trust_anchors_pem(client_context, trust_anchors)?;

        let (client_socket, server_socket) = context.socket_pair()?;
        context.set_socket_nonblocking(client_socket, true)?;
        context.set_socket_nonblocking(server_socket, true)?;

        let client_name = context.string_value("localhost");
        let server_name = context.string_value("");
        let client_session =
            context.destack_tls_session_open(client_context, client_socket, client_name)?;
        let server_session =
            context.destack_tls_session_open(server_context, server_socket, server_name)?;

        complete_handshake(&mut context, client_session, server_session)?;
        context.set_socket_nonblocking(client_socket, false)?;
        context.set_socket_nonblocking(server_socket, false)?;

        let client_alpn = context.destack_tls_session_negotiated_alpn(client_session)?;
        let client_alpn = context.bytes_from_value(client_alpn)?;
        assert_eq!(client_alpn, b"h2");

        let server_alpn = context.destack_tls_session_negotiated_alpn(server_session)?;
        let server_alpn = context.bytes_from_value(server_alpn)?;
        assert_eq!(server_alpn, b"h2");

        write_payload(&mut context, client_session, b"ping")?;
        let server_read = read_payload(&mut context, server_session, 4)?;
        assert_eq!(server_read, b"ping");

        write_payload(&mut context, server_session, b"pong")?;
        let client_read = read_payload(&mut context, client_session, 4)?;
        assert_eq!(client_read, b"pong");

        context.destack_tls_session_close(client_session)?;
        context.destack_tls_session_close(server_session)?;
        context.close_socket(client_socket)?;
        context.close_socket(server_socket)?;
        context.destack_tls_context_close(client_context)?;
        context.destack_tls_context_close(server_context)
    });
}

/// Report resumed state on second TLS 1.2 handshake with shared contexts.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_resumption_stateful() {
    with_harness_context(|mut context| {
        let client_options = context.context_options_value(
            TlsRole::Client,
            TlsVersion::Tls12,
            TlsVersion::Tls12,
            true,
            &[],
        )?;
        let server_options = context.context_options_value(
            TlsRole::Server,
            TlsVersion::Tls12,
            TlsVersion::Tls12,
            false,
            &[],
        )?;
        let client_context = context.destack_tls_context_open(client_options)?;
        let server_context = context.destack_tls_context_open(server_options)?;

        context.destack_tls_context_set_session_resumption(
            client_context,
            TlsSessionResumptionMode::Stateful,
        )?;
        context.destack_tls_context_set_session_resumption(
            server_context,
            TlsSessionResumptionMode::Stateful,
        )?;
        let server_cert = context.bytes_value(TEST_SERVER_CERT_PEM)?;
        let server_key = context.bytes_value(TEST_SERVER_KEY_PEM)?;
        context.destack_tls_context_set_identity_pem(server_context, server_cert, server_key)?;
        let trust_anchors = context.bytes_value(TEST_CA_CERT_PEM)?;
        context.destack_tls_context_set_trust_anchors_pem(client_context, trust_anchors)?;

        let (first_client_socket, first_server_socket) = context.socket_pair()?;
        context.set_socket_nonblocking(first_client_socket, true)?;
        context.set_socket_nonblocking(first_server_socket, true)?;
        let first_client_name = context.string_value("localhost");
        let first_server_name = context.string_value("");
        let first_client_session = context.destack_tls_session_open(
            client_context,
            first_client_socket,
            first_client_name,
        )?;
        let first_server_session = context.destack_tls_session_open(
            server_context,
            first_server_socket,
            first_server_name,
        )?;
        complete_handshake(&mut context, first_client_session, first_server_session)?;
        let first_state = context.destack_tls_session_resumption_state(first_client_session)?;
        assert_eq!(first_state, TlsSessionResumptionState::Fresh);
        context.destack_tls_session_close(first_client_session)?;
        context.destack_tls_session_close(first_server_session)?;
        context.close_socket(first_client_socket)?;
        context.close_socket(first_server_socket)?;

        let (second_client_socket, second_server_socket) = context.socket_pair()?;
        context.set_socket_nonblocking(second_client_socket, true)?;
        context.set_socket_nonblocking(second_server_socket, true)?;
        let second_client_name = context.string_value("localhost");
        let second_server_name = context.string_value("");
        let second_client_session = context.destack_tls_session_open(
            client_context,
            second_client_socket,
            second_client_name,
        )?;
        let second_server_session = context.destack_tls_session_open(
            server_context,
            second_server_socket,
            second_server_name,
        )?;
        complete_handshake(&mut context, second_client_session, second_server_session)?;
        let second_state = context.destack_tls_session_resumption_state(second_client_session)?;
        assert_eq!(second_state, TlsSessionResumptionState::Resumed);
        context.destack_tls_session_close(second_client_session)?;
        context.destack_tls_session_close(second_server_session)?;
        context.close_socket(second_client_socket)?;
        context.close_socket(second_server_socket)?;
        context.destack_tls_context_close(client_context)?;
        context.destack_tls_context_close(server_context)
    });
}

/// Fail session open when peer verification is enabled without trust anchors.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_handshake_fails_without_trust_anchors() {
    with_harness_context(|mut context| {
        let client_options = context.context_options_value(
            TlsRole::Client,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            true,
            &[],
        )?;
        let server_options = context.context_options_value(
            TlsRole::Server,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            false,
            &[],
        )?;
        let client_context = context.destack_tls_context_open(client_options)?;
        let server_context = context.destack_tls_context_open(server_options)?;

        let server_cert = context.bytes_value(TEST_SERVER_CERT_PEM)?;
        let server_key = context.bytes_value(TEST_SERVER_KEY_PEM)?;
        context.destack_tls_context_set_identity_pem(server_context, server_cert, server_key)?;

        let (client_socket, server_socket) = context.socket_pair()?;
        let client_name = context.string_value("localhost");
        let error = context.destack_tls_session_open(client_context, client_socket, client_name);
        let error = error.expect_err("session open should fail without trust anchors");
        assert_eq!(
            platform_error_code(error.as_ref()),
            PlatformErrorCode::InvalidArgumentValue
        );

        context.close_socket(client_socket)?;
        context.close_socket(server_socket)?;
        context.destack_tls_context_close(client_context)?;
        context.destack_tls_context_close(server_context)
    });
}

/// Complete mutual TLS handshake when both peers present trusted identities.
#[cfg(any(unix, windows))]
#[test]
fn test_tls_session_mutual_tls_handshake() {
    with_harness_context(|mut context| {
        let client_options = context.context_options_value(
            TlsRole::Client,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            true,
            &[],
        )?;
        let server_options = context.context_options_value(
            TlsRole::Server,
            TlsVersion::Tls12,
            TlsVersion::Tls13,
            true,
            &[],
        )?;
        let client_context = context.destack_tls_context_open(client_options)?;
        let server_context = context.destack_tls_context_open(server_options)?;

        let client_cert = context.bytes_value(TEST_CLIENT_CERT_PEM)?;
        let client_key = context.bytes_value(TEST_CLIENT_KEY_PEM)?;
        context.destack_tls_context_set_identity_pem(client_context, client_cert, client_key)?;
        let server_cert = context.bytes_value(TEST_SERVER_CERT_PEM)?;
        let server_key = context.bytes_value(TEST_SERVER_KEY_PEM)?;
        context.destack_tls_context_set_identity_pem(server_context, server_cert, server_key)?;
        let client_trust = context.bytes_value(TEST_CA_CERT_PEM)?;
        context.destack_tls_context_set_trust_anchors_pem(client_context, client_trust)?;
        let server_trust = context.bytes_value(TEST_CA_CERT_PEM)?;
        context.destack_tls_context_set_trust_anchors_pem(server_context, server_trust)?;

        let (client_socket, server_socket) = context.socket_pair()?;
        context.set_socket_nonblocking(client_socket, true)?;
        context.set_socket_nonblocking(server_socket, true)?;
        let client_name = context.string_value("localhost");
        let server_name = context.string_value("");
        let client_session =
            context.destack_tls_session_open(client_context, client_socket, client_name)?;
        let server_session =
            context.destack_tls_session_open(server_context, server_socket, server_name)?;

        complete_handshake(&mut context, client_session, server_session)?;

        context.destack_tls_session_close(client_session)?;
        context.destack_tls_session_close(server_session)?;
        context.close_socket(client_socket)?;
        context.close_socket(server_socket)?;
        context.destack_tls_context_close(client_context)?;
        context.destack_tls_context_close(server_context)
    });
}
