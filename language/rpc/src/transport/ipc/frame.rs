use crate::TransportError;

/// RPC byte-stream frame magic.
const FRAME_MAGIC: [u8; 4] = *b"DSRP";

/// Current RPC byte-stream frame version.
const FRAME_VERSION: u16 = 1;

/// Encoded RPC byte-stream frame header length.
pub(super) const FRAME_HEADER_BYTES: usize = 12;

/// One RPC byte-stream frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Frame {
    /// Frame format version.
    version: u16,
    /// Reserved frame flags.
    flags: u16,
    /// Encoded message byte length.
    message_bytes: u32,
}

impl Frame {
    /// Create one frame header for an encoded message.
    pub(super) fn new(message_bytes: usize) -> Result<Self, FrameError> {
        let message_bytes = u32::try_from(message_bytes).map_err(|_| FrameError::TooLarge {
            limit: u32::MAX as usize,
            actual: message_bytes,
        })?;

        Ok(Self {
            version: FRAME_VERSION,
            flags: 0,
            message_bytes,
        })
    }

    /// Return the encoded message byte length.
    pub(super) const fn message_bytes(self) -> u32 {
        self.message_bytes
    }

    /// Encode this frame header.
    pub(super) fn encode(self) -> [u8; FRAME_HEADER_BYTES] {
        let mut bytes = [0; FRAME_HEADER_BYTES];
        bytes[0..4].copy_from_slice(&FRAME_MAGIC);
        bytes[4..6].copy_from_slice(&self.version.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.flags.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.message_bytes.to_le_bytes());

        bytes
    }

    /// Decode one exact frame header.
    pub(super) fn decode(bytes: [u8; FRAME_HEADER_BYTES]) -> Result<Self, FrameError> {
        let mut magic = [0; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if magic != FRAME_MAGIC {
            return Err(FrameError::InvalidMagic(magic));
        }

        // decode and validate the framing grammar
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != FRAME_VERSION {
            return Err(FrameError::UnsupportedVersion(version));
        }
        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        if flags != 0 {
            return Err(FrameError::UnsupportedFlags(flags));
        }

        let message_bytes = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        Ok(Self {
            version,
            flags,
            message_bytes,
        })
    }
}

/// Failure to decode one IPC byte-stream frame.
#[derive(Debug)]
pub(super) enum FrameError {
    /// Frame magic does not identify TS++ RPC.
    InvalidMagic([u8; 4]),
    /// Frame version is not supported.
    UnsupportedVersion(u16),
    /// Reserved flags are not supported.
    UnsupportedFlags(u16),
    /// Message exceeds the representable frame length.
    TooLarge {
        /// Representable byte limit.
        limit: usize,
        /// Actual message byte length.
        actual: usize,
    },
}

impl std::fmt::Display for FrameError {
    /// Format this IPC frame failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic(magic) => write!(formatter, "invalid RPC frame magic {magic:?}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported RPC frame version {version}")
            }
            Self::UnsupportedFlags(flags) => {
                write!(formatter, "unsupported RPC frame flags {flags:#x}")
            }
            Self::TooLarge { limit, actual } => {
                write!(formatter, "RPC frame exceeds {limit} bytes: {actual}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

impl From<FrameError> for TransportError {
    /// Convert one private IPC frame failure into a transport failure.
    fn from(error: FrameError) -> Self {
        match error {
            FrameError::TooLarge { limit, actual } => Self::MessageTooLarge { limit, actual },
            error => Self::InvalidMessage(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAME_HEADER_BYTES, Frame};

    /// Encode and decode the exact stable IPC frame header.
    #[test]
    fn test_roundtrip_frame() {
        let frame = Frame::new(3).expect("create frame");
        let encoded = frame.encode();
        let expected: [u8; FRAME_HEADER_BYTES] = [b'D', b'S', b'R', b'P', 1, 0, 0, 0, 3, 0, 0, 0];

        assert_eq!(encoded, expected);
        assert_eq!(Frame::decode(encoded).expect("decode frame"), frame);
    }
}
