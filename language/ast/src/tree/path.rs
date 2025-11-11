use std::fmt::Debug;

use dyst_source::SmallVec;

use crate::StringId;

/// A Path is static path to a named definition in a namespace.
/// In the case of a Use declaration, the Path excludes the items.
///
/// Examples:
/// ```
/// foo
/// foobar
/// foo.bar.baz.qux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: SmallVec<StringId, 3>,
}
