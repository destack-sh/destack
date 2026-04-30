use std::io::{Read, Write};

use postcard::Error as PostcardError;
use serde::{Deserialize, Serialize};

use super::{ProtocolMessage, Transport, TransportError};

/// Magic identifier for protocol frames.
pub const FRAME_MAGIC: [u8; 4] = *b"DSDP";
/// Frame format version.
pub const FRAME_VERSION: u16 = 1;
/// Default maximum frame size.
pub const DEFAULT_MAX_FRAME_SIZE_BYTES: usize = 64 * 1024 * 1024;
/// Frame header size in bytes.
pub const FRAME_HEADER_SIZE_BYTES: usize = 12;

/// Frame header metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeader {
    /// Magic bytes for protocol frames.
    pub magic: [u8; 4],
    /// Frame format version.
    pub version: u16,
    /// Reserved flags.
    pub flags: u16,
    /// Payload length in bytes.
    pub payload_len: u32,
}

impl FrameHeader {
    /// Create a new frame header for a payload.
    pub fn new(payload_len: usize) -> Result<Self, FrameError> {
        // validate payload length
        let payload_len =
            u32::try_from(payload_len).map_err(|_| FrameError::SizeLimitExceeded {
                limit: u32::MAX as usize,
                actual: payload_len,
            })?;

        Ok(Self {
            magic: FRAME_MAGIC,
            version: FRAME_VERSION,
            flags: 0,
            payload_len,
        })
    }

    /// Encode the header to bytes.
    pub fn encode(self) -> [u8; FRAME_HEADER_SIZE_BYTES] {
        // write header fields in little endian
        let mut out = [0u8; FRAME_HEADER_SIZE_BYTES];
        out[..4].copy_from_slice(&self.magic);
        out[4..6].copy_from_slice(&self.version.to_le_bytes());
        out[6..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..12].copy_from_slice(&self.payload_len.to_le_bytes());

        out
    }

    /// Decode a header from bytes.
    pub fn decode(bytes: [u8; FRAME_HEADER_SIZE_BYTES]) -> Result<Self, FrameError> {
        // parse header fields from little endian bytes
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[..4]);
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        let payload_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        // validate magic
        if magic != FRAME_MAGIC {
            return Err(FrameError::InvalidMagic {
                expected: FRAME_MAGIC,
                found: magic,
            });
        }

        // validate version
        if version != FRAME_VERSION {
            return Err(FrameError::UnsupportedVersion {
                expected: FRAME_VERSION,
                found: version,
            });
        }

        Ok(Self {
            magic,
            version,
            flags,
            payload_len,
        })
    }
}

/// Errors produced by frame encoding or decoding.
#[derive(Debug)]
pub enum FrameError {
    /// Frame magic did not match.
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },
    /// Frame version did not match.
    UnsupportedVersion { expected: u16, found: u16 },
    /// Frame exceeded configured size limits.
    SizeLimitExceeded { limit: usize, actual: usize },
    /// Frame bytes ended unexpectedly.
    UnexpectedEof,
    /// Frame io error.
    Io(std::io::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format frame errors
        match self {
            FrameError::InvalidMagic { expected, found } => {
                write!(
                    f,
                    "invalid frame magic, expected {expected:?}, found {found:?}"
                )
            }
            FrameError::UnsupportedVersion { expected, found } => {
                write!(
                    f,
                    "unsupported frame version, expected {expected}, found {found}"
                )
            }
            FrameError::SizeLimitExceeded { limit, actual } => {
                write!(
                    f,
                    "frame size limit exceeded, limit {limit}, actual {actual}"
                )
            }
            FrameError::UnexpectedEof => write!(f, "unexpected eof while reading frame"),
            FrameError::Io(error) => write!(f, "frame io error: {error}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<std::io::Error> for FrameError {
    fn from(error: std::io::Error) -> Self {
        FrameError::Io(error)
    }
}

/// Frame encoder and decoder.
#[derive(Debug, Clone)]
pub struct FrameCodec {
    /// Maximum frame size in bytes.
    pub max_frame_size: usize,
}

impl FrameCodec {
    /// Create a new frame codec with a maximum size.
    pub fn new(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }

    /// Encode a payload into a framed byte vector.
    pub fn encode(&self, payload: &[u8]) -> Result<Vec<u8>, FrameError> {
        // enforce size limits
        if payload.len() > self.max_frame_size {
            return Err(FrameError::SizeLimitExceeded {
                limit: self.max_frame_size,
                actual: payload.len(),
            });
        }

        // build header
        let header = FrameHeader::new(payload.len())?;
        let header_bytes = header.encode();

        // assemble the frame bytes
        let mut out = Vec::with_capacity(FRAME_HEADER_SIZE_BYTES + payload.len());
        out.extend_from_slice(&header_bytes);
        out.extend_from_slice(payload);

        Ok(out)
    }

    /// Decode a framed payload from bytes.
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, FrameError> {
        // validate the header size
        if bytes.len() < FRAME_HEADER_SIZE_BYTES {
            return Err(FrameError::UnexpectedEof);
        }

        // parse header and validate size
        let mut header_bytes = [0u8; FRAME_HEADER_SIZE_BYTES];
        header_bytes.copy_from_slice(&bytes[..FRAME_HEADER_SIZE_BYTES]);
        let header = FrameHeader::decode(header_bytes)?;
        let payload_len = header.payload_len as usize;
        if payload_len > self.max_frame_size {
            return Err(FrameError::SizeLimitExceeded {
                limit: self.max_frame_size,
                actual: payload_len,
            });
        }

        // validate payload length
        let required_len = FRAME_HEADER_SIZE_BYTES + payload_len;
        if bytes.len() < required_len {
            return Err(FrameError::UnexpectedEof);
        }

        Ok(bytes[FRAME_HEADER_SIZE_BYTES..required_len].to_vec())
    }

    /// Read a frame from a reader.
    pub fn read_from<R: Read>(&self, reader: &mut R) -> Result<Vec<u8>, FrameError> {
        // read the frame header
        let mut header_bytes = [0u8; FRAME_HEADER_SIZE_BYTES];
        if let Err(error) = reader.read_exact(&mut header_bytes) {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                return Err(FrameError::UnexpectedEof);
            }
            return Err(FrameError::Io(error));
        }

        // decode the header
        let header = FrameHeader::decode(header_bytes)?;
        let payload_len = header.payload_len as usize;
        if payload_len > self.max_frame_size {
            return Err(FrameError::SizeLimitExceeded {
                limit: self.max_frame_size,
                actual: payload_len,
            });
        }

        // read the payload
        let mut payload = vec![0u8; payload_len];
        if let Err(error) = reader.read_exact(&mut payload) {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                return Err(FrameError::UnexpectedEof);
            }
            return Err(FrameError::Io(error));
        }

        Ok(payload)
    }

    /// Write a frame to a writer.
    pub fn write_to<W: Write>(&self, writer: &mut W, payload: &[u8]) -> Result<(), FrameError> {
        // encode the frame bytes
        let bytes = self.encode(payload)?;

        // write the frame
        writer.write_all(&bytes)?;
        Ok(())
    }
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self {
            max_frame_size: DEFAULT_MAX_FRAME_SIZE_BYTES,
        }
    }
}

/// Errors produced by protocol encoding and decoding.
#[derive(Debug)]
pub enum ProtocolCodecError {
    /// Frame errors.
    Frame(FrameError),
    /// Serialization error.
    Serialize(PostcardError),
    /// Deserialization error.
    Deserialize(PostcardError),
    /// Payload exceeds negotiated limits.
    PayloadTooLarge { limit: usize, actual: usize },
    /// Transport error.
    Transport(TransportError),
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
            ProtocolCodecError::Transport(error) => write!(f, "transport error: {error}"),
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

    /// Encode a protocol message into postcard bytes.
    pub fn encode_message(&self, message: &ProtocolMessage) -> Result<Vec<u8>, ProtocolCodecError> {
        // serialize the message payload
        let payload = postcard::to_allocvec(message).map_err(ProtocolCodecError::Serialize)?;

        // enforce payload limits
        if payload.len() > self.max_payload_bytes {
            return Err(ProtocolCodecError::PayloadTooLarge {
                limit: self.max_payload_bytes,
                actual: payload.len(),
            });
        }

        Ok(payload)
    }

    /// Decode a protocol message from postcard bytes.
    pub fn decode_message(&self, payload: &[u8]) -> Result<ProtocolMessage, ProtocolCodecError> {
        // enforce payload limits
        if payload.len() > self.max_payload_bytes {
            return Err(ProtocolCodecError::PayloadTooLarge {
                limit: self.max_payload_bytes,
                actual: payload.len(),
            });
        }

        // deserialize the message payload
        postcard::from_bytes(payload).map_err(ProtocolCodecError::Deserialize)
    }

    /// Send a protocol message via transport.
    pub fn send_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
        message: &ProtocolMessage,
    ) -> Result<(), ProtocolCodecError> {
        // serialize the message payload
        let payload = self.encode_message(message)?;

        // send the payload via transport
        transport
            .send(&payload)
            .map_err(ProtocolCodecError::Transport)?;
        Ok(())
    }

    /// Receive a protocol message via transport.
    pub fn recv_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
    ) -> Result<ProtocolMessage, ProtocolCodecError> {
        // receive the payload via transport
        let payload = transport.recv().map_err(ProtocolCodecError::Transport)?;

        // deserialize the payload
        self.decode_message(&payload)
    }
}

impl Default for ProtocolCodec {
    fn default() -> Self {
        Self {
            max_payload_bytes: DEFAULT_MAX_FRAME_SIZE_BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_source::{
        Diagnostic, DiagnosticSeverity, FileId, FileType, LabeledSpan, Span, Uri,
    };

    use super::{DEFAULT_MAX_FRAME_SIZE_BYTES, FrameCodec, ProtocolCodec, ProtocolMessage};
    use crate::protocol::{
        CommandBuildOptions, CommandInput, CommandPayload, CommandRequest, CommandResponse,
        CommonCommandOptions, DaemonRequest, DaemonResponse, DiagnosticBatch, FileUpdate,
        FileUpdateImage, FileUpdateKind, FileUpdateRequest, ProtocolRequest, ProtocolResponse,
        RequestId, RequestOptions, RootHandleId, loopback_transport_pair,
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
            payload: DaemonRequest::Command(Box::new(CommandRequest {
                handle: RootHandleId::new(2),
                common: CommonCommandOptions {
                    inputs: vec![CommandInput::File {
                        path: "/root/app.ds".into(),
                    }],
                    allow_destack_config_fallback: false,
                    target: Some("app".to_string()),
                    ..CommonCommandOptions::default()
                },
                payload: CommandPayload::Build(CommandBuildOptions::default()),
            })),
        };
        let message = ProtocolMessage::Request(Box::new(request));
        let codec = ProtocolCodec::default();

        // encode to bytes and decode back
        let bytes = codec.encode_message(&message).expect("encode");
        let decoded = codec.decode_message(&bytes).expect("decode");

        // assert the roundtrip preserves the message
        assert_eq!(decoded, message);
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
            payload: DaemonRequest::ApplyFileUpdate(FileUpdateRequest {
                handle: RootHandleId::new(1),
                update: FileUpdate {
                    path: "/root/app.ds".into(),
                    update: FileUpdateKind::Text {
                        content: "let x = 1".to_string(),
                    },
                    write_to_disk: false,
                },
            }),
        };
        let message = ProtocolMessage::Request(Box::new(request));
        let codec = ProtocolCodec::default();

        // send through a loopback transport
        let (client, server) = loopback_transport_pair(4);
        codec.send_message(&client, &message).expect("send");
        let decoded = codec.recv_message(&server).expect("recv");

        // assert roundtrip works
        assert_eq!(decoded, message);
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
            payload: DaemonRequest::Ping,
        };
        let message = ProtocolMessage::Request(Box::new(request));
        let codec = ProtocolCodec::new(1);

        // assert payload limit is enforced
        let result = codec.encode_message(&message);
        assert!(result.is_err());

        // sanity check default allows the message
        let codec = ProtocolCodec::new(DEFAULT_MAX_FRAME_SIZE_BYTES);
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

    /// Encode and decode command responses without loss.
    #[test]
    fn test_protocol_command_response_roundtrip() {
        // build a minimal diagnostic payload
        let file_id = FileId::new(1);
        let diagnostic = Diagnostic {
            code: "E001".to_string(),
            original_code: None,
            severity: DiagnosticSeverity::Error,
            original_severity: None,
            message: "example error".to_string(),
            file_id,
            primary_span: LabeledSpan::new(Span::new(file_id, 0, 1), "primary"),
            primary_highlight_spans: None,
            secondary_spans: None,
            suggestions: None,
        };
        let diagnostics = vec![DiagnosticBatch {
            file_id,
            diagnostics: vec![diagnostic],
        }];

        // build an image for diagnostics rendering
        let image = FileUpdateImage {
            id: file_id,
            name: "main.ds".to_string(),
            uri: Uri::from_string("file:///main.ds"),
            path: Some("/root/main.ds".into()),
            file_type: FileType::Destack,
            content: Some("export const answer = 42;\n".to_string()),
        };

        // build base response data
        let base = CommandResponse {
            handle: RootHandleId::new(1),
            success: true,
            exit_code: 0,
            diagnostics: Vec::new(),
            files: Vec::new(),
            messages: Vec::new(),
            output: Vec::new(),
            outputs: Vec::new(),
            module_count: 1,
            profile_count: 1,
            target_count: 0,
            data: None,
        };

        // roundtrip a response with no diagnostics or files
        assert_command_roundtrip("empty", base.clone());

        // roundtrip a response with diagnostics only
        let mut with_diagnostics = base.clone();
        with_diagnostics.diagnostics = diagnostics;
        assert_command_roundtrip("diagnostics", with_diagnostics);

        // roundtrip a response with file images only
        let mut with_files = base.clone();
        with_files.files = vec![image];
        assert_command_roundtrip("files", with_files);
    }

    /// Roundtrip a command response through the protocol codec.
    fn assert_command_roundtrip(label: &str, response: CommandResponse) {
        // build a protocol response message
        let response = ProtocolResponse {
            id: RequestId::new(1),
            payload: DaemonResponse::CommandResult(response),
        };
        let message = ProtocolMessage::Response(Box::new(response));
        let codec = ProtocolCodec::default();

        // assert the response survives a roundtrip
        let bytes = codec
            .encode_message(&message)
            .unwrap_or_else(|error| panic!("{label} encode failed: {error}"));
        let decoded = codec
            .decode_message(&bytes)
            .unwrap_or_else(|error| panic!("{label} decode failed: {error}"));
        assert_eq!(decoded, message);
    }
}
