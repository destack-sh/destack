use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Access, Lifetime, Reference, Value};

/// One projection in a MIR type path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Projection {
    /// Any element of a repeated type.
    Elements,
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
    /// A runtime slice projection.
    Slice {
        /// The runtime start index value.
        start: Value,
        /// The runtime length value.
        length: Value,
    },
    /// The whole pointee behind a unique reference.
    Deref,
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

    /// Return the first projection, absent at the root.
    pub fn first(&self) -> Option<&Projection> {
        self.projections.first()
    }

    /// Return whether this path is rooted at the value itself.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.projections.is_empty()
    }

    /// Return whether this path contains another path.
    pub fn contains(&self, other: &Self) -> bool {
        self.projections.len() <= other.projections.len()
            && self
                .projections
                .iter()
                .zip(&other.projections)
                .all(|(left, right)| left.contains(right))
    }

    /// Return whether the paths can select overlapping storage.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.projections
            .iter()
            .zip(&other.projections)
            .all(|(left, right)| left.overlaps(right))
    }

    /// Remove one matching structural prefix from this path.
    pub fn strip_prefix(&self, prefix: &Self) -> Option<Self> {
        if prefix.projections.len() > self.projections.len() || !prefix.overlaps(self) {
            return None;
        }
        let projections = self.projections[prefix.projections.len()..].to_vec();

        Some(Self { projections })
    }

    /// Map each value reference inside this path.
    pub fn map_values(&mut self, mut map: impl FnMut(Value) -> Value) {
        for projection in &mut self.projections {
            match projection {
                Projection::Index { index } => *index = map(*index),
                Projection::Slice { start, length } => {
                    *start = map(*start);
                    *length = map(*length);
                }
                Projection::Field { .. }
                | Projection::Element { .. }
                | Projection::Variant { .. }
                | Projection::Deref => {}
            }
        }
    }

    /// Replace value references inside this path.
    pub fn replace_value(&mut self, from: Value, to: Value) {
        self.map_values(|value| if value == from { to } else { value });
    }
}

/// One reference-like component in a type shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct BorrowedPath {
    /// The path to the component.
    pub path: Path,
    /// The lifetime carried by the component.
    pub lifetime: Lifetime,
    /// Access granted by the component.
    pub access: Access,
    /// The reference kind of the component.
    pub kind: Reference,
}
