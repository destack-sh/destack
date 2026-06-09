use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{Constant, GlobalReference, Lifetime, LocalReference, Value, ValueReference};

/// Root storage for one MIR place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaceOrigin {
    /// A function-local stack slot.
    Local(LocalReference),
    /// A module global.
    Global(GlobalReference),
    /// An opaque reference value.
    Value(ValueReference),
}

impl PlaceOrigin {
    /// Replace value references inside this origin.
    fn replace_value(&mut self, from: Value, to: Value) {
        let Self::Value(value) = self else {
            return;
        };

        value.replace_value(from, to);
    }
}

/// One projection in a MIR path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        index: ValueReference,
    },
    /// An unknown element projection.
    AnyElement,
    /// A runtime slice projection.
    Slice {
        /// The runtime start index value.
        start: ValueReference,
        /// The runtime length value.
        length: ValueReference,
    },
    /// A variant payload projection.
    Variant {
        /// The selected variant tag.
        tag: Constant,
    },
}

impl Projection {
    /// Replace value references inside this projection.
    fn replace_value(&mut self, from: Value, to: Value) {
        match self {
            Self::Field { .. } | Self::Element { .. } | Self::AnyElement | Self::Variant { .. } => {
            }
            Self::Index { index } => index.replace_value(from, to),
            Self::Slice { start, length } => {
                start.replace_value(from, to);
                length.replace_value(from, to);
            }
        }
    }

    /// Append value references used by this projection.
    fn append_value_references(&self, values: &mut SmallVec<[ValueReference; 4]>) {
        match self {
            Self::Field { .. } | Self::Element { .. } | Self::AnyElement | Self::Variant { .. } => {
            }
            Self::Index { index } => values.push(*index),
            Self::Slice { start, length } => {
                values.push(*start);
                values.push(*length);
            }
        }
    }
}

/// A rootless path through a MIR value or type shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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

    /// Return whether this path is rooted at the value itself.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.projections.is_empty()
    }
}

/// One borrowed reference-like component in a type shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BorrowedPath {
    /// Path to the borrowed component.
    pub path: Path,
    /// Lifetime carried by the borrowed component.
    pub lifetime: Lifetime,
}

/// A MIR memory place.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Place {
    /// Root storage for the place.
    pub origin: PlaceOrigin,
    /// Path from the root storage.
    pub path: Path,
}

impl Place {
    /// Create a place from one origin.
    #[inline]
    pub fn new(origin: PlaceOrigin) -> Self {
        Self {
            origin,
            path: Path::root(),
        }
    }

    /// Create a place rooted in a local.
    #[inline]
    pub fn local(local: LocalReference) -> Self {
        Self::new(PlaceOrigin::Local(local))
    }

    /// Create a place rooted in a global.
    #[inline]
    pub fn global(global: GlobalReference) -> Self {
        Self::new(PlaceOrigin::Global(global))
    }

    /// Create a place rooted in an opaque value.
    #[inline]
    pub fn value(value: ValueReference) -> Self {
        Self::new(PlaceOrigin::Value(value))
    }

    /// Append one projection.
    #[inline]
    pub fn push(&mut self, projection: Projection) {
        self.path.push(projection);
    }

    /// Return this place with one extra projection.
    #[inline]
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.push(projection);
        self
    }

    /// Return whether this place is proven disjoint from another place.
    pub fn is_definitely_disjoint(&self, other: &Self) -> bool {
        if self.origin != other.origin {
            return !matches!(self.origin, PlaceOrigin::Value(_))
                && !matches!(other.origin, PlaceOrigin::Value(_));
        }

        for (left, right) in self.path.projections.iter().zip(&other.path.projections) {
            // separate fields are disjoint
            if let (Projection::Field { index: left }, Projection::Field { index: right }) =
                (left, right)
            {
                return left != right;
            }

            // separate fixed elements are disjoint
            if let (Projection::Element { index: left }, Projection::Element { index: right }) =
                (left, right)
            {
                return left != right;
            }
        }

        false
    }

    /// Return whether this place contains another place.
    pub fn contains(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.path.projections.len() <= other.path.projections.len()
            && self
                .path
                .projections
                .iter()
                .zip(&other.path.projections)
                .all(|(left, right)| left == right)
    }

    /// Return whether this place may overlap another place.
    #[inline]
    pub fn may_overlap(&self, other: &Self) -> bool {
        !self.is_definitely_disjoint(other)
    }

    /// Replace value references inside this place.
    pub fn replace_value(&mut self, from: Value, to: Value) {
        self.origin.replace_value(from, to);

        for projection in &mut self.path.projections {
            projection.replace_value(from, to);
        }
    }

    /// Return value references used by this place.
    pub fn value_references(&self) -> SmallVec<[ValueReference; 4]> {
        let mut values = SmallVec::new();

        // include value origins
        if let PlaceOrigin::Value(value) = self.origin {
            values.push(value);
        }

        // include dynamic projection operands
        for projection in &self.path.projections {
            projection.append_value_references(&mut values);
        }

        values
    }
}

/// Place table effect produced by one instruction.
pub(crate) enum PlaceEffect {
    /// Assign a concrete place to one SSA value.
    Root {
        /// The value receiving the place.
        value: Value,
        /// The assigned place.
        place: Place,
    },
    /// Project a place from an existing SSA value.
    Projection {
        /// The value receiving the projected place.
        value: Value,
        /// The base value whose place is projected.
        base: Value,
        /// The projection to append.
        projection: Projection,
    },
    /// Copy a place from one SSA value to another.
    Copy {
        /// The value receiving the copied place.
        value: Value,
        /// The source value whose place is copied.
        source: Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Local, LocalNodeId};

    #[test]
    fn test_fields_are_disjoint() {
        let local = LocalNodeId::<Local>::new(0);
        let left = Place::local(local.into()).with_projection(Projection::Field { index: 0 });
        let right = Place::local(local.into()).with_projection(Projection::Field { index: 1 });

        assert!(left.is_definitely_disjoint(&right));
        assert!(!left.may_overlap(&right));
    }

    #[test]
    fn test_prefix_places_overlap() {
        let local = LocalNodeId::<Local>::new(0);
        let root = Place::local(local.into());
        let field = root.clone().with_projection(Projection::Field { index: 0 });

        assert!(!root.is_definitely_disjoint(&field));
        assert!(root.may_overlap(&field));
    }

    #[test]
    fn test_index_projection_is_conservative() {
        let local = LocalNodeId::<Local>::new(0);
        let dynamic = Place::local(local.into()).with_projection(Projection::Index {
            index: Value::new(0).into(),
        });
        let fixed = Place::local(local.into()).with_projection(Projection::Element { index: 0 });

        assert!(!dynamic.is_definitely_disjoint(&fixed));
        assert!(dynamic.may_overlap(&fixed));
    }

    #[test]
    fn test_slice_projection_tracks_operands() {
        let local = LocalNodeId::<Local>::new(0);
        let place = Place::local(local.into()).with_projection(Projection::Slice {
            start: Value::new(0).into(),
            length: Value::new(1).into(),
        });

        assert_eq!(
            place.value_references().as_slice(),
            &[Value::new(0).into(), Value::new(1).into()]
        );
    }
}
