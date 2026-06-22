use destack_serde::Schema;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use super::{FRAME_HEADER_SIZE_BYTES, FRAME_MAGIC, FRAME_VERSION, ProtocolLimits};

/// Frame header metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
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
            max_frame_size: ProtocolLimits::default().max_frame_bytes as usize,
        }
    }
}
