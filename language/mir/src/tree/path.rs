use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Lifetime, Value};

/// One projection in a MIR type path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Projection {
    /// A fixed concrete field projection.
    Field {
        /// The zero-based field index.
        index: u32,
    },
    /// A fixed element projection.
    Element {
        /// The zero-based element index.
        index: u32,
    },
    /// A runtime element projection.
    Index {
        /// The runtime index value.
        index: Value,
    },
    /// One statically selected variant case.
    Variant {
        /// The zero-based case index.
        case: u32,
    },
    /// Any element of an indexed container.
    AnyElement,
    /// A runtime slice projection.
    Slice {
        /// The runtime start index value.
        start: Value,
        /// The runtime length value.
        length: Value,
    },
}

impl Projection {
    /// Replace value references inside this projection.
    fn replace_value(&mut self, from: Value, to: Value) {
        match self {
            Self::Field { .. } | Self::Element { .. } | Self::Variant { .. } | Self::AnyElement => {
            }
            Self::Index { index } => replace_value(index, from, to),
            Self::Slice { start, length } => {
                replace_value(start, from, to);
                replace_value(length, from, to);
            }
        }
    }
}

/// A rootless path through a MIR value or type shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub struct Path {
    /// Projections from the root value.
    pub projections: Vec<Projection>,
}

impl Path {
    /// Create a root path.
    #[inline]
    pub fn root() -> Self {
        Self::default()
    }

    /// Append one projection.
    #[inline]
    pub fn push(&mut self, projection: Projection) {
        self.projections.push(projection);
    }

    /// Return this path with one extra projection.
    #[inline]
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.push(projection);

        self
    }

    /// Return this path with another path appended.
    #[inline]
    pub fn with_path(mut self, path: &Path) -> Self {
        self.projections.extend(path.projections.iter().cloned());

        self
    }

    /// Return whether this path is rooted at the value itself.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.projections.is_empty()
    }

    /// Replace value references inside this path.
    pub fn replace_value(&mut self, from: Value, to: Value) {
        for projection in &mut self.projections {
            projection.replace_value(from, to);
        }
    }
}

/// One borrowed reference-like component in a type shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BorrowedPath {
    /// The path to the borrowed component.
    pub path: Path,
    /// The lifetime carried by the borrowed component.
    pub lifetime: Lifetime,
}

/// Replace one value id.
fn replace_value(value: &mut Value, from: Value, to: Value) {
    if *value == from {
        *value = to;
    }
}
