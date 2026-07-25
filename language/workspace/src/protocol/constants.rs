use super::ProtocolVersion;

/// One kibibyte in bytes.
const KIB_BYTES_U64: u64 = 1024;

/// One mebibyte in bytes.
const MIB_BYTES_U64: u64 = 1024 * KIB_BYTES_U64;

/// One kibibyte in bytes.
const KIB_BYTES_USIZE: usize = 1024;

/// Current workspace protocol version.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(2, 0, 0);

/// Minimum compatible workspace protocol version.
pub const MIN_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(2, 0, 0);

/// Magic identifier for protocol frames.
pub const FRAME_MAGIC: [u8; 4] = *b"DSWP";

/// Frame format version.
pub const FRAME_VERSION: u16 = 1;

/// Frame header size in bytes.
pub const FRAME_HEADER_SIZE_BYTES: usize = 12;

/// Default maximum transport frame size in bytes.
pub const DEFAULT_MAX_FRAME_BYTES: u64 = 16 * MIB_BYTES_U64;

/// Default maximum encoded protocol message size in bytes.
pub const DEFAULT_MAX_PAYLOAD_BYTES: u64 = MIB_BYTES_U64;

/// Default maximum inline payload size in bytes.
pub const DEFAULT_INLINE_PAYLOAD_MAX_BYTES: usize = 64 * KIB_BYTES_USIZE;

/// Bytes reserved for inline payload metadata.
pub const INLINE_PAYLOAD_OVERHEAD_BYTES: usize = 4 * KIB_BYTES_USIZE;

/// Bytes reserved for payload chunk headroom.
pub const PAYLOAD_CHUNK_HEADROOM_BYTES: usize = 4 * KIB_BYTES_USIZE;
