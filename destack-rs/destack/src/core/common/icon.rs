//! destack.core.common.icon@2025.08.15.1

#![destack::partial(destack.core.common.icon, file)]

use crate::Color;

#[destack::generated(Icon, -, block)]
/// An icon to be displayed in some view.
pub struct Icon {
    pub r#type: IconType,
    pub emoji: Option<String>,
    pub fa_name: Option<String>,
    pub vsc_name: Option<String>,
    pub file: Option<i64 /* TODO */>,
    pub file_url: Option<String>,
    pub color: Option<Color>,
}

#[destack::generated(PrimitiveType, -, block)]
/// A fundamental scalar data type.
pub enum PrimitiveType {
    /// Null value
    None = 1,
    /// Boolean flag (True or False, 1 byte)
    /// Range: False, True
    Boolean = 2,
    /// 8-bit signed integer
    /// Range: -2^7 to 2^7-1
    Int8 = 3,
    /// 16-bit signed integer
    /// Range: -2^15 to 2^15-1
    Int16 = 4,
    /// 32-bit signed integer
    /// Range: -2^31 to 2^31-1
    Int32 = 5,
    /// 64-bit signed integer
    /// Range: -2^63 to 2^63-1
    Int64 = 6,
    /// 128-bit signed integer
    /// Range: -2^127 to 2^127-1
    Int128 = 7,
    /// 8-bit unsigned integer
    /// Range: 0 to 2^8-1
    Uint8 = 10,
    /// 16-bit unsigned integer
    /// Range: 0 to 2^16-1
    Uint16 = 11,
    /// 32-bit unsigned integer
    /// Range: 0 to 2^32-1
    Uint32 = 12,
    /// 64-bit unsigned integer
    /// Range: 0 to 2^64-1
    Uint64 = 13,
    /// 128-bit unsigned integer
    /// Range: 0 to 2^128-1
    Uint128 = 14,
    /// 32-bit single-precision float
    /// Range: ±2^-126 to ±2^127-1
    Float32 = 23,
    /// 64-bit double-precision float
    /// Range: ±2^-1022 to ±2^1023-1
    Float64 = 24,
    /// DateTime in signed 64-bit microsecond precision since epoch (UTC)
    /// Range: ±292,277 years
    Datetime = 40,
    /// Date in signed 64-bit day precision since epoch
    /// Range: ±2.525x10^16 days
    Date = 41,
    /// Time in unsigned 64-bit nanosecond precision
    /// Range: 00:00:00.000000000 to 23:59:59.999999999
    Time = 42,
    /// Timestamp in unsigned 64-bit nanosecond precision since epoch (UTC)
    /// Range: 1970-01-01 to 2554-07-21 UTC
    Timestamp = 43,
    /// Duration in signed 64-bit nanosecond precision
    /// Range: ±292.277 years
    Duration = 44,
    /// Plain text (UTF-8, 32-bit variable length)
    /// Range: 0 to 2^32-1
    String = 50,
    /// Single character (UTF-8, 32-bit)
    /// Range: 0 to 2^32-1
    Character = 51,
    /// Universally unique identifier (UUID7, 128-bit)
    /// The zero UUID is invalid (00000000-0000-0000-0000-000000000000).
    /// Range: 0 to 2^128-1
    Uuid = 54,
    /// JSON (32-bit variable length)
    /// Range: 0 to 2^32-1
    Json = 57,
}

#[destack::generated(TypeCardinality, -, block)]
/// The order of a Type (scalar, list, map, etc.).
pub enum TypeCardinality {
    /// Single value
    Scalar = 1,
    /// Dynamic sequence of homogeneous values
    List = 2,
    /// Fixed sequence of heterogeneous values
    Tuple = 3,
    /// Mapping of homogenous keys to homogeneous values
    Map = 10,
}

#[destack::generated(ScalarType, -, block)]
/// The type of a scalar (single value like primitive, enum, struct, etc.).
pub enum ScalarType {
    /// Primitive value (boolean, number, time, string, etc.)
    Primitive = 1,
    /// Enum value (enumeration of options)
    Enum = 2,
    /// Node as a value (Node)
    Node = 3,
    /// Reference to a Node (id only, for internal use)
    NodeRaw = 4,
    /// Reference to a Node (type + id, for internal use)
    NodeIdentity = 5,
    /// Reference to a Node (type + id + space, assumed time)
    NodeSpatial = 6,
    /// Reference to a Node (type + id + space + time)
    NodeTemporal = 7,
    /// Struct value (structured data)
    Struct = 8,
    /// Handle (runtime-only)
    Handle = 9,
    /// Tagged union of heterogeneous values
    Union = 10,
}

#[destack::generated(ValueFactory, -, block)]
/// The factory to use for generating values.
pub enum ValueFactory {
    /// Generate a random (time-sorted) UUIDv7
    Uuid7 = 1,
    /// Get the current timestamp (system)
    Now = 10,
    /// Get the current logical time (system)
    RemoteEpoch = 11,
    /// Get the current logical time (system)
    LocalEpoch = 12,
    /// Get the current Actor
    Actor = 20,
    /// Get the current Client
    Client = 21,
    /// Get the current Client nonce
    ClientNonce = 22,
    /// Get the current Region
    Region = 30,
    /// Get the current Node
    SelfNode = 40,
    /// Get the current Space
    CurrentSpace = 41,
    /// Get the current Branch
    CurrentBranch = 42,
    /// Get the current Snapshot
    CurrentSnapshot = 43,
    /// Generate a relevant name
    Name = 50,
}

#[destack::generated(PropertyZone, -, block)]
/// PropertyZone
pub enum PropertyZone {
    Member = 1,
    Input = 10,
    Output = 11,
}

#[destack::generated(ReferenceType, -, block)]
/// The type of a Node reference.
pub enum ReferenceType {
    /// Raw untyped reference (id only, for internal use)
    Raw = 1,
    /// Identity reference (type + id, for internal use)
    Identity = 2,
    /// Identity + Space reference (type + id + space, assumed time)
    Spatial = 3,
    /// Identity + Space + time reference (type + id + space + time)
    Temporal = 4,
}

#[destack::generated(IconType, -, block)]
/// IconType
pub enum IconType {
    Emoji = 1,
    File = 10,
    FileUrl = 11,
}
