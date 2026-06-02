use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{GlobalReference, LocalReference, Value, ValueReference};

/// Opaque id for one MIR place.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlaceId(pub u32);

impl PlaceId {
    /// Create a place id.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the numeric id.
    #[inline]
    pub const fn id(self) -> u32 {
        self.0
    }
}

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

/// One projection applied to a MIR place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaceProjection {
    /// A fixed concrete field projection.
    Field {
        /// The zero-based field index.
        index: u32,
    },
    /// A fixed concrete element projection.
    Element {
        /// The zero-based element index.
        index: u32,
    },
    /// A runtime element projection.
    Index {
        /// The runtime index value.
        index: ValueReference,
    },
    /// A runtime slice projection.
    Slice {
        /// The runtime start index value.
        start: ValueReference,
        /// The runtime length value.
        length: ValueReference,
    },
}

impl PlaceProjection {
    /// Replace value references inside this projection.
    fn replace_value(&mut self, from: Value, to: Value) {
        match self {
            Self::Field { .. } | Self::Element { .. } => {}
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
            Self::Field { .. } | Self::Element { .. } => {}
            Self::Index { index } => values.push(*index),
            Self::Slice { start, length } => {
                values.push(*start);
                values.push(*length);
            }
        }
    }
}

/// A MIR memory place.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Place {
    /// Root storage for the place.
    pub origin: PlaceOrigin,
    /// Projections from the root storage.
    pub projections: Vec<PlaceProjection>,
}

impl Place {
    /// Create a place from one origin.
    #[inline]
    pub fn new(origin: PlaceOrigin) -> Self {
        Self {
            origin,
            projections: Vec::new(),
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
    pub fn push(&mut self, projection: PlaceProjection) {
        self.projections.push(projection);
    }

    /// Return this place with one extra projection.
    #[inline]
    pub fn with_projection(mut self, projection: PlaceProjection) -> Self {
        self.push(projection);
        self
    }

    /// Return whether this place is proven disjoint from another place.
    pub fn is_definitely_disjoint(&self, other: &Self) -> bool {
        if self.origin != other.origin {
            return !matches!(self.origin, PlaceOrigin::Value(_))
                && !matches!(other.origin, PlaceOrigin::Value(_));
        }

        for (left, right) in self.projections.iter().zip(&other.projections) {
            // separate fields are disjoint
            if let (
                PlaceProjection::Field { index: left },
                PlaceProjection::Field { index: right },
            ) = (left, right)
            {
                return left != right;
            }

            // separate fixed elements are disjoint
            if let (
                PlaceProjection::Element { index: left },
                PlaceProjection::Element { index: right },
            ) = (left, right)
            {
                return left != right;
            }
        }

        false
    }

    /// Return whether this place contains another place.
    pub fn contains(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.projections.len() <= other.projections.len()
            && self
                .projections
                .iter()
                .zip(&other.projections)
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

        for projection in &mut self.projections {
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
        for projection in &self.projections {
            projection.append_value_references(&mut values);
        }

        values
    }
}

/// Places keyed by SSA value inside one function.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlaceTable {
    /// Stored places.
    places: Vec<Place>,
    /// Place assigned to each SSA value.
    places_by_value: Vec<Option<PlaceId>>,
}

impl PlaceTable {
    /// Create an empty place table.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the place for one id.
    #[inline]
    pub fn place(&self, id: PlaceId) -> Option<&Place> {
        self.places.get(id.0 as usize)
    }

    /// Return the place id assigned to one value.
    #[inline]
    pub fn value_place_id(&self, value: Value) -> Option<PlaceId> {
        self.places_by_value
            .get(value.0 as usize)
            .copied()
            .flatten()
    }

    /// Assign a place to one value.
    pub fn set_value_place(&mut self, value: Value, place: Place) -> PlaceId {
        let id = self.insert(place);
        self.bind_value(value, id);
        id
    }

    /// Assign a local place to one value.
    #[inline]
    pub fn set_local(&mut self, value: Value, local: LocalReference) -> PlaceId {
        self.set_value_place(value, Place::local(local))
    }

    /// Assign a global place to one value.
    #[inline]
    pub fn set_global(&mut self, value: Value, global: GlobalReference) -> PlaceId {
        self.set_value_place(value, Place::global(global))
    }

    /// Assign an opaque value place to one value.
    #[inline]
    pub fn set_value(&mut self, value: Value, origin: ValueReference) -> PlaceId {
        self.set_value_place(value, Place::value(origin))
    }

    /// Assign a projected place to one value.
    pub fn set_projection(
        &mut self,
        value: Value,
        base: Value,
        projection: PlaceProjection,
    ) -> PlaceId {
        let place = self
            .value_place_id(base)
            .and_then(|id| self.place(id).cloned())
            .unwrap_or_else(|| Place::value(base.into()))
            .with_projection(projection);

        self.set_value_place(value, place)
    }

    /// Assign a copied place to one value.
    pub fn set_from_value(&mut self, value: Value, source: Value) -> Option<PlaceId> {
        let place = self
            .value_place_id(source)
            .and_then(|id| self.place(id).cloned())?;

        Some(self.set_value_place(value, place))
    }

    /// Replace value references inside the table.
    pub fn replace_value(&mut self, from: Value, to: Value) {
        if from == to {
            return;
        }

        for place in &mut self.places {
            place.replace_value(from, to);
        }

        let from_index = from.0 as usize;
        let from_place = self.places_by_value.get(from_index).copied().flatten();

        let to_index = to.0 as usize;
        if to_index >= self.places_by_value.len() {
            self.places_by_value.resize(to_index + 1, None);
        }

        if self.places_by_value[to_index].is_none() {
            self.places_by_value[to_index] = from_place;
        }

        if let Some(slot) = self.places_by_value.get_mut(from_index) {
            *slot = None;
        }
    }

    /// Insert one place.
    fn insert(&mut self, place: Place) -> PlaceId {
        let id = PlaceId::new(self.places.len() as u32);
        self.places.push(place);
        id
    }

    /// Bind an existing place id to one value.
    fn bind_value(&mut self, value: Value, id: PlaceId) {
        let index = value.0 as usize;
        if index >= self.places_by_value.len() {
            self.places_by_value.resize(index + 1, None);
        }

        self.places_by_value[index] = Some(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Local, LocalNodeId};

    #[test]
    fn test_fields_are_disjoint() {
        let local = LocalNodeId::<Local>::new(0);
        let left = Place::local(local.into()).with_projection(PlaceProjection::Field { index: 0 });
        let right = Place::local(local.into()).with_projection(PlaceProjection::Field { index: 1 });

        assert!(left.is_definitely_disjoint(&right));
        assert!(!left.may_overlap(&right));
    }

    #[test]
    fn test_prefix_places_overlap() {
        let local = LocalNodeId::<Local>::new(0);
        let root = Place::local(local.into());
        let field = root
            .clone()
            .with_projection(PlaceProjection::Field { index: 0 });

        assert!(!root.is_definitely_disjoint(&field));
        assert!(root.may_overlap(&field));
    }

    #[test]
    fn test_index_projection_is_conservative() {
        let local = LocalNodeId::<Local>::new(0);
        let dynamic = Place::local(local.into()).with_projection(PlaceProjection::Index {
            index: Value::new(0).into(),
        });
        let fixed =
            Place::local(local.into()).with_projection(PlaceProjection::Element { index: 0 });

        assert!(!dynamic.is_definitely_disjoint(&fixed));
        assert!(dynamic.may_overlap(&fixed));
    }

    #[test]
    fn test_slice_projection_tracks_operands() {
        let local = LocalNodeId::<Local>::new(0);
        let place = Place::local(local.into()).with_projection(PlaceProjection::Slice {
            start: Value::new(0).into(),
            length: Value::new(1).into(),
        });

        assert_eq!(
            place.value_references().as_slice(),
            &[Value::new(0).into(), Value::new(1).into()]
        );
    }
}
