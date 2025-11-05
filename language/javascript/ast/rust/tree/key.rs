use dyst_source::StringId;

/// A Name is a regular or string identifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Name {
    /// A regular identifier (regular `x` or `someThing`).
    Identifier(StringId),
    /// A string identifier (like `["Content-Type"]`, only in certain contexts).
    String(StringId),
}
