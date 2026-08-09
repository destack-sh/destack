use super::arithmetic::{ArithmeticService, NumberRequest};
use crate::protocol::message::{CallStart, Message};
use crate::protocol::payload::Payload;
use crate::protocol::{Handshake, HandshakeCodec, HandshakeRequest, MessageCodec, ProtocolVersion};
use crate::{CallId, Code, Limits, Peer, Request, ServiceId};

/// Accept only positive negotiated limits bounded by the caller's offer.
#[test]
fn test_validate_selected_limits() {
    let offered = Limits {
        max_message_bytes: 1024,
        max_payload_bytes: 4096,
        max_concurrent_calls: 8,
        stream_window: 4,
    };
    let selected = Limits {
        max_message_bytes: 512,
        max_payload_bytes: 2048,
        max_concurrent_calls: 4,
        stream_window: 2,
    };

    assert_eq!(offered.validate_selected(selected), Ok(()));

    let zero = Limits {
        stream_window: 0,
        ..selected
    };
    let status = offered
        .validate_selected(zero)
        .expect_err("reject zero selected window");
    assert_eq!(status.code, Code::InvalidArgument);
    assert_eq!(status.message, "stream window must be positive");

    let exceeded = Limits {
        stream_window: offered.stream_window + 1,
        ..selected
    };
    let status = offered
        .validate_selected(exceeded)
        .expect_err("reject selected window above offer");
    assert_eq!(status.code, Code::FailedPrecondition);
    assert_eq!(
        status.message,
        "selected RPC limits exceed the caller's advertised limits"
    );
}

/// Encode and decode the exact wire request vocabulary.
#[test]
fn test_roundtrip_request_message() {
    let service = ArithmeticService::new();
    let start = CallStart {
        call: CallId::new(7),
        service: service.schema.id(),
        method: service.double,
        request: Request::new(
            Payload::encode(&NumberRequest { value: 21 }).expect("encode request"),
        ),
    };
    let message = Message::Start(start);
    let codec = MessageCodec::new(ProtocolVersion::CURRENT, 1024 * 1024);

    let bytes = codec.encode(&message).expect("encode message");
    let expected = vec![
        0, 7, 205, 166, 131, 229, 137, 128, 212, 196, 71, 182, 226, 191, 130, 138, 167, 145, 159,
        9, 0, 0, 1, 21,
    ];
    let decoded = codec.decode(&bytes).expect("decode message");

    assert_eq!(bytes, expected);
    assert_eq!(decoded, message);
}

/// Encode one exact stable connection negotiation message.
#[test]
fn test_roundtrip_handshake() {
    let handshake = Handshake::Request(HandshakeRequest {
        versions: vec![ProtocolVersion::CURRENT],
        limits: Limits {
            max_message_bytes: 1024,
            max_payload_bytes: 4096,
            max_concurrent_calls: 8,
            stream_window: 4,
        },
        peer: Peer {
            name: "client".to_string(),
            version: "1".to_string(),
            build_id: None,
        },
        services: vec![ServiceId::new(9)],
    });
    let codec = HandshakeCodec::new(1024);

    let bytes = codec.encode(&handshake).expect("encode handshake");
    let expected = vec![
        0, 1, 1, 128, 8, 128, 32, 8, 4, 6, 99, 108, 105, 101, 110, 116, 1, 49, 0, 1, 9,
    ];
    let decoded = codec.decode(&bytes).expect("decode handshake");

    assert_eq!(bytes, expected);
    assert_eq!(decoded, handshake);
}
