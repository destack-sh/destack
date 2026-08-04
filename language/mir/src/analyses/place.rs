use crate::{GlobalId, Instruction, LocalId, Path, Projection, Value};

/// Root storage for one analyzed MIR place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceOrigin {
    /// A function-local stack slot.
    Local(LocalId),
    /// A module global.
    Global(GlobalId),
    /// Storage reached through an opaque reference value.
    Value(Value),
}

impl PlaceOrigin {
    /// Replace one SSA value.
    fn replace_value(&mut self, from: Value, to: Value) {
        let Self::Value(value) = self else {
            return;
        };

        if *value == from {
            *value = to;
        }
    }
}

/// One storage location derived from MIR address values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    /// The root storage.
    pub origin: PlaceOrigin,
    /// The structural path from the root.
    pub path: Path,
}

impl Place {
    /// Create a place from one origin.
    pub fn new(origin: PlaceOrigin) -> Self {
        Self {
            origin,
            path: Path::root(),
        }
    }

    /// Create a place rooted in one local.
    pub fn local(local: LocalId) -> Self {
        Self::new(PlaceOrigin::Local(local))
    }

    /// Create a place rooted in one global.
    pub fn global(global: GlobalId) -> Self {
        Self::new(PlaceOrigin::Global(global))
    }

    /// Create a place rooted in one opaque reference value.
    pub fn value(value: Value) -> Self {
        Self::new(PlaceOrigin::Value(value))
    }

    /// Return this place with one extra projection.
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.path.push(projection);

        self
    }

    /// Append one projection.
    pub fn push(&mut self, projection: Projection) {
        self.path.push(projection);
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

    /// Replace one SSA value throughout the place.
    pub fn replace_value(&mut self, from: Value, to: Value) {
        self.origin.replace_value(from, to);
        self.path.replace_value(from, to);
    }
}

/// Dense analyzed places keyed by SSA value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlaceMap {
    /// The known place for each SSA value.
    places: Vec<Option<Place>>,
}

impl PlaceMap {
    /// Create an empty map sized for one function.
    pub fn new(value_count: usize) -> Self {
        Self {
            places: vec![None; value_count],
        }
    }

    /// Return the known place for one value.
    pub fn get(&self, value: Value) -> Option<&Place> {
        self.places
            .get(value.id() as usize)
            .and_then(Option::as_ref)
    }

    /// Return the known place or an opaque value root.
    pub fn resolve(&self, value: Value) -> Place {
        self.get(value)
            .cloned()
            .unwrap_or_else(|| Place::value(value))
    }

    /// Return values derived from one opaque root.
    pub fn derived_from(&self, owner: Value) -> impl Iterator<Item = Value> + '_ {
        self.places
            .iter()
            .enumerate()
            .filter_map(move |(index, place)| {
                let place = place.as_ref()?;
                matches!(place.origin, PlaceOrigin::Value(value) if value == owner)
                    .then_some(Value::new(index as u32))
            })
    }

    /// Bind a successor parameter to one predecessor argument.
    pub fn bind(&mut self, argument: Value, parameter: Value) {
        let Some(place) = self.get(argument).cloned() else {
            self.clear(parameter);
            return;
        };
        if place == Place::value(argument) {
            self.clear(parameter);
            return;
        }

        self.set(parameter, place);
    }

    /// Apply one instruction's address derivation.
    pub fn apply(&mut self, instruction: &Instruction) {
        match instruction {
            Instruction::LocalAddr {
                destination, local, ..
            } => self.set(*destination, Place::local(*local)),
            Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => self.set(*destination, Place::global(*global)),
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewComplete { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. }
            | Instruction::FunctionEnvironment { destination, .. }
            | Instruction::FunctionEnvironmentCurrent { destination } => {
                self.set(*destination, Place::value(*destination));
            }
            Instruction::FieldAddr {
                destination,
                aggregate,
                field,
                ..
            } => self.project(
                *destination,
                *aggregate,
                Projection::Field { index: *field },
            ),
            Instruction::ElementAddr {
                destination,
                base,
                index,
                ..
            } => self.project(*destination, *base, Projection::Index { index: *index }),
            Instruction::VariantPayloadAddr {
                destination,
                variant,
                case,
                ..
            } => self.project(*destination, *variant, Projection::Variant { case: *case }),
            Instruction::SliceView {
                destination,
                source,
                start,
                length,
                ..
            } => self.project(
                *destination,
                *source,
                Projection::Slice {
                    start: *start,
                    length: *length,
                },
            ),
            Instruction::Cast {
                destination,
                argument,
                ..
            }
            | Instruction::TensorCast {
                destination,
                tensor: argument,
            }
            | Instruction::TensorView {
                destination,
                view: argument,
                ..
            }
            | Instruction::Pin {
                destination,
                value: argument,
                ..
            } => self.copy(*destination, *argument),
            _ => {}
        }
    }

    /// Retain only places also known by another control-flow state.
    pub fn intersect(&mut self, other: &Self) {
        for index in 0..self.places.len() {
            let is_stable = self.places[index]
                .as_ref()
                .is_some_and(|place| other.at(index) == Some(place));
            if !is_stable {
                self.places[index] = None;
            }
        }
    }

    /// Return one place by dense value index.
    fn at(&self, index: usize) -> Option<&Place> {
        self.places.get(index).and_then(Option::as_ref)
    }

    /// Clear one value's place.
    fn clear(&mut self, value: Value) {
        let index = self.resize(value);

        self.places[index] = None;
    }

    /// Set one value's place.
    fn set(&mut self, value: Value, place: Place) {
        let index = self.resize(value);

        self.places[index] = Some(place);
    }

    /// Derive one projected place.
    fn project(&mut self, value: Value, base: Value, projection: Projection) {
        let place = self.resolve(base).with_projection(projection);

        self.set(value, place);
    }

    /// Copy one known place.
    fn copy(&mut self, value: Value, source: Value) {
        let Some(place) = self.get(source).cloned() else {
            self.clear(value);
            return;
        };

        self.set(value, place);
    }

    /// Resize storage for one value.
    fn resize(&mut self, value: Value) -> usize {
        let index = value.id() as usize;
        let value_count = index + 1;

        if self.places.len() < value_count {
            self.places.resize(value_count, None);
        }

        index
    }
}
