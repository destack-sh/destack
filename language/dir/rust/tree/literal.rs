use crate::StringId;

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarLiteral {
    /// Boolean value.
    Boolean(bool),
    /// Byte value.
    Byte(u8),
    /// Integer value.
    Integer(i64),
    /// Float value.
    Float(f64),
    /// Character value.
    Character(char),
    /// String value.
    String(StringId),
    /// Byte string value.
    ByteString(Vec<u8>),
}
