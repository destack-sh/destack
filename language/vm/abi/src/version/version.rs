/// Current metadata version for VM ABI tables.
pub const METADATA_VERSION: u32 = 1;

/// Byte order for serialized metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    /// Little-endian encoding.
    Little,
    /// Big-endian encoding.
    Big,
}

/// Header for serialized VM metadata blobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataHeader {
    /// Metadata version identifier.
    pub version: u32,
    /// Target triple for the build.
    pub target_triple: String,
    /// Pointer width in bits.
    pub pointer_width: u8,
    /// Endianness for encoded data.
    pub endianness: Endianness,
}
