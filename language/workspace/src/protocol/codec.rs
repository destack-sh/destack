use super::{FrameError, ProtocolLimits, ProtocolMessage};

/// Errors produced by protocol encoding and decoding.
#[derive(Debug)]
pub enum ProtocolCodecError {
    /// Frame errors.
    Frame(FrameError),
    /// Serialization error.
    Serialize(destack_serde::Error),
    /// Deserialization error.
    Deserialize(destack_serde::Error),
    /// Payload exceeds negotiated limits.
    PayloadTooLarge { limit: usize, actual: usize },
}

impl std::fmt::Display for ProtocolCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format codec errors
        match self {
            ProtocolCodecError::Frame(error) => write!(f, "{error}"),
            ProtocolCodecError::Serialize(error) => write!(f, "serialize error: {error}"),
            ProtocolCodecError::Deserialize(error) => write!(f, "deserialize error: {error}"),
            ProtocolCodecError::PayloadTooLarge { limit, actual } => {
                write!(f, "payload too large, limit {limit}, actual {actual}")
            }
        }
    }
}

impl std::error::Error for ProtocolCodecError {}

impl From<FrameError> for ProtocolCodecError {
    fn from(error: FrameError) -> Self {
        ProtocolCodecError::Frame(error)
    }
}

/// Protocol message codec with payload limits.
#[derive(Debug, Clone)]
pub struct ProtocolCodec {
    /// Maximum payload size in bytes.
    pub max_payload_bytes: usize,
}

impl ProtocolCodec {
    /// Create a new codec with a payload limit.
    pub fn new(max_payload_bytes: usize) -> Self {
        Self { max_payload_bytes }
    }

    /// Encode a protocol message into binary bytes.
    pub fn encode_message(&self, message: &ProtocolMessage) -> Result<Vec<u8>, ProtocolCodecError> {
        // serialize the message payload
        let payload = destack_serde::to_vec(message).map_err(ProtocolCodecError::Serialize)?;

        // enforce payload limits
        if payload.len() > self.max_payload_bytes {
            return Err(ProtocolCodecError::PayloadTooLarge {
                limit: self.max_payload_bytes,
                actual: payload.len(),
            });
        }

        Ok(payload)
    }

    /// Decode a protocol message from binary bytes.
    pub fn decode_message(&self, payload: &[u8]) -> Result<ProtocolMessage, ProtocolCodecError> {
        // enforce payload limits
        if payload.len() > self.max_payload_bytes {
            return Err(ProtocolCodecError::PayloadTooLarge {
                limit: self.max_payload_bytes,
                actual: payload.len(),
            });
        }

        // deserialize the message payload
        destack_serde::from_slice(payload).map_err(ProtocolCodecError::Deserialize)
    }
}

impl Default for ProtocolCodec {
    fn default() -> Self {
        Self {
            max_payload_bytes: ProtocolLimits::default().max_payload_bytes as usize,
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_repository::TraceView;
    use destack_source::{ByteRange, Edit, TextPatch, Uri};

    use crate::{
        BuildInput, BuildOutputs, CommandInput, CommandRevision, FileOperation, SourceUpdate,
    };

    use crate::protocol::{
        FileOperationRequest, FrameCodec, ProtocolCodec, ProtocolLimits, ProtocolMessage,
        ProtocolRequest, RequestId, RequestOptions, RootId, SourceUpdateRequest, WorkspaceRequest,
    };

    #[test]
    fn test_frame_roundtrip() {
        // build a payload
        let payload = b"hello".to_vec();
        let codec = FrameCodec::default();

        // encode and decode the payload
        let frame = codec.encode(&payload).expect("encode");
        let decoded = codec.decode(&frame).expect("decode");

        // assert the payload roundtrips
        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_protocol_roundtrip() {
        // build a message payload
        let request = ProtocolRequest {
            id: RequestId::new(7),
            options: RequestOptions::default(),
            payload: WorkspaceRequest::Build {
                handle: RootId::new(2),
                input: BuildInput {
                    revision: CommandRevision::Current,
                    inputs: vec![CommandInput::File {
                        path: "/root/app.ds".into(),
                    }],
                    config_inputs: false,
                    cwd: None,
                    manifest: None,
                    target: Some("app".to_string()),
                    target_overrides: None,
                    profile: None,
                    env: Vec::new(),
                    overrides: Vec::new(),
                    watch: false,
                    dry_run: false,
                    trace: TraceView::Summary,
                    product: Some("app".to_string()),
                    outputs: BuildOutputs {
                        products: true,
                        bundles: true,
                        programs: true,
                        assets: false,
                    },
                },
            },
        };
        let message = ProtocolMessage::Request(Box::new(request));
        let codec = ProtocolCodec::default();

        // encode to bytes and decode back
        let bytes = codec.encode_message(&message).expect("encode");
        let decoded = codec.decode_message(&bytes).expect("decode");

        // assert the roundtrip preserves the message
        assert_eq!(format!("{decoded:?}"), format!("{message:?}"));
    }

    #[test]
    fn test_frame_rejects_invalid_magic() {
        // build a framed payload
        let payload = b"hello".to_vec();
        let codec = FrameCodec::default();
        let mut frame = codec.encode(&payload).expect("encode");

        // corrupt the magic bytes
        frame[0] = b'X';

        // assert invalid magic is rejected
        let result = codec.decode(&frame);
        assert!(result.is_err());
    }

    #[test]
    fn test_protocol_read_write() {
        // build a message payload
        let request = ProtocolRequest {
            id: RequestId::new(4),
            options: RequestOptions::default(),
            payload: WorkspaceRequest::ApplyFileOperation(FileOperationRequest {
                handle: RootId::new(1),
                operation: FileOperation::ChangeText {
                    path: "/root/app.ds".into(),
                    uri: Uri::from_file_path("/root/app.ds"),
                    version: 1,
                    content: "let x = 1".to_string(),
                },
            }),
        };
        let message = ProtocolMessage::Request(Box::new(request));
        let codec = ProtocolCodec::default();

        // encode and decode through the protocol codec
        let encoded = codec.encode_message(&message).expect("encode");
        let decoded = codec.decode_message(&encoded).expect("decode");

        // assert roundtrip works
        assert_eq!(format!("{decoded:?}"), format!("{message:?}"));
    }

    #[test]
    fn test_protocol_source_update_roundtrip() {
        // build a source update payload
        let request = ProtocolRequest {
            id: RequestId::new(5),
            options: RequestOptions::default(),
            payload: WorkspaceRequest::ApplySourceUpdate(SourceUpdateRequest {
                handle: RootId::new(1),
                update: SourceUpdate {
                    base: None,
                    edits: vec![
                        Edit::SetText {
                            path: "/root/app.ds".into(),
                            text: "let x = 1".to_string(),
                        },
                        Edit::EditText {
                            path: "/root/app.ds".into(),
                            patches: vec![TextPatch {
                                range: ByteRange { start: 4, end: 5 },
                                text: "y".to_string(),
                            }],
                        },
                    ],
                },
            }),
        };
        let message = ProtocolMessage::Request(Box::new(request));
        let codec = ProtocolCodec::default();

        // encode to bytes and decode back
        let bytes = codec.encode_message(&message).expect("encode");
        let decoded = codec.decode_message(&bytes).expect("decode");

        assert_eq!(format!("{decoded:?}"), format!("{message:?}"));
    }

    #[test]
    fn test_frame_size_limit() {
        // build an oversized payload
        let payload = vec![0u8; 16];
        let codec = FrameCodec::new(8);

        // assert size limit is enforced
        let result = codec.encode(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_protocol_payload_limit() {
        // build a payload that exceeds the limit
        let request = ProtocolRequest {
            id: RequestId::new(9),
            options: RequestOptions::default(),
            payload: WorkspaceRequest::Ping,
        };
        let message = ProtocolMessage::Request(Box::new(request));
        let codec = ProtocolCodec::new(1);

        // assert payload limit is enforced
        let result = codec.encode_message(&message);
        assert!(result.is_err());

        // sanity check default allows the message
        let codec = ProtocolCodec::new(ProtocolLimits::default().max_payload_bytes as usize);
        let result = codec.encode_message(&message);
        assert!(result.is_ok());
    }

    #[test]
    fn test_protocol_payload_limit_on_decode() {
        // build an oversized payload
        let payload = vec![0u8; 2];
        let codec = ProtocolCodec::new(1);

        // assert payload limit is enforced on decode
        let result = codec.decode_message(&payload);
        assert!(result.is_err());
    }
}
