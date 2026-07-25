use destack_mir as mir;

use super::owned::OwnedValues;

/// Ownership state used while planning drops.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct DropState {
    /// Whole owned values still available.
    pub(super) owned: OwnedValues,
    /// Known places keyed by SSA value.
    places: Vec<Option<mir::Place>>,
}

impl DropState {
    /// Build function entry state.
    pub(super) fn parameters(function: &mir::Function, owned: &OwnedValues) -> Self {
        Self {
            owned: OwnedValues::parameters(function, owned),
            places: vec![None; function.value_types().len()],
        }
    }

    /// Intersect ownership with another predecessor state.
    pub(super) fn intersect_with(&mut self, other: &Self) {
        self.owned.intersect_with(&other.owned);
        self.intersect_places_with(other);
    }

    /// Retain values known to be owned.
    pub(super) fn retain_owned(&mut self, owned: &OwnedValues) {
        self.owned.retain_owned(owned);
    }

    /// Return whether one value is available.
    pub(super) fn contains(&self, value: mir::Value) -> bool {
        self.owned.contains(value)
    }

    /// Mark consumed whole values as unavailable.
    pub(super) fn remove_consumed(&mut self, consumed: &OwnedValues) {
        for value in consumed.values() {
            self.remove(value);
        }
    }

    /// Bind a successor parameter to one predecessor argument.
    pub(super) fn bind(
        &mut self,
        argument: mir::Value,
        parameter: mir::Value,
        owned: &OwnedValues,
    ) {
        if self.owned.contains(argument) && owned.contains(parameter) {
            self.remove(argument);
            self.owned.insert(parameter);
        }

        self.bind_place(argument, parameter);
    }

    /// Insert one owned instruction destination.
    pub(super) fn insert_destination(
        &mut self,
        instruction: &mir::Instruction,
        owned: &OwnedValues,
    ) {
        let Some(destination) = instruction.destination() else {
            return;
        };
        if !owned.contains(destination) {
            return;
        }

        self.owned.insert(destination);
    }

    /// Return the known place for one value.
    pub(super) fn value_place(&self, value: mir::Value) -> Option<&mir::Place> {
        self.places
            .get(value.id() as usize)
            .and_then(Option::as_ref)
    }

    /// Return the best known place for one value.
    pub(super) fn place_for_value(&self, value: mir::Value) -> mir::Place {
        self.value_place(value)
            .cloned()
            .unwrap_or_else(|| mir::Place::value(value))
    }

    /// Return values with known places derived from one owner value.
    pub(super) fn values_derived_from(
        &self,
        owner: mir::Value,
    ) -> impl Iterator<Item = mir::Value> + '_ {
        self.places
            .iter()
            .enumerate()
            .filter_map(move |(index, place)| {
                let place = place.as_ref()?;
                matches!(place.origin, mir::PlaceOrigin::Value(value) if value == owner)
                    .then_some(mir::Value::new(index as u32))
            })
    }

    /// Apply one instruction's place bindings.
    pub(super) fn apply_instruction(&mut self, instruction: &mir::Instruction) {
        match instruction {
            mir::Instruction::LocalAddr {
                destination, local, ..
            } => self.set_place(*destination, mir::Place::local(*local)),
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => self.set_place(*destination, mir::Place::global(*global)),
            mir::Instruction::NewZeroed { destination, .. }
            | mir::Instruction::NewUninit { destination, .. }
            | mir::Instruction::NewComplete { destination, .. }
            | mir::Instruction::NewSliceZeroed { destination, .. }
            | mir::Instruction::NewSliceUninit { destination, .. }
            | mir::Instruction::FunctionEnvironment { destination, .. }
            | mir::Instruction::FunctionEnvironmentCurrent { destination } => {
                self.set_place(*destination, mir::Place::value(*destination))
            }
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                field,
                ..
            } => self.set_projected_place(
                *destination,
                *aggregate,
                mir::Projection::Field { index: *field },
            ),
            mir::Instruction::ElementAddr {
                destination,
                base,
                index,
                ..
            } => self.set_projected_place(
                *destination,
                *base,
                mir::Projection::Index { index: *index },
            ),
            mir::Instruction::SliceView {
                destination,
                source,
                start,
                length,
                ..
            } => self.set_projected_place(
                *destination,
                *source,
                mir::Projection::Slice {
                    start: *start,
                    length: *length,
                },
            ),
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            }
            | mir::Instruction::TensorCast {
                destination,
                tensor: argument,
            }
            | mir::Instruction::TensorView {
                destination,
                view: argument,
                ..
            }
            | mir::Instruction::Pin {
                destination,
                value: argument,
                ..
            } => self.copy_place(*destination, *argument),
            _ => {}
        }
    }

    /// Mark one whole value as moved.
    pub(super) fn remove(&mut self, value: mir::Value) {
        self.owned.remove(value);
    }

    /// Keep only places also known in another predecessor state.
    fn intersect_places_with(&mut self, other: &Self) {
        for index in 0..self.places.len() {
            let is_stable = self.places[index]
                .as_ref()
                .is_some_and(|place| other.place_at(index) == Some(place));
            if !is_stable {
                self.places[index] = None;
            }
        }
    }

    /// Return the place at one dense value index.
    fn place_at(&self, index: usize) -> Option<&mir::Place> {
        self.places.get(index).and_then(Option::as_ref)
    }

    /// Bind a place to a successor parameter.
    fn bind_place(&mut self, argument: mir::Value, parameter: mir::Value) {
        let Some(place) = self.value_place(argument).cloned() else {
            self.clear_place(parameter);
            return;
        };
        if place == mir::Place::value(argument) {
            self.clear_place(parameter);
            return;
        }

        self.set_place(parameter, place);
    }

    /// Clear a value's known place.
    fn clear_place(&mut self, value: mir::Value) {
        let index = self.resize_places(value);

        self.places[index] = None;
    }

    /// Set a value's known place.
    fn set_place(&mut self, value: mir::Value, place: mir::Place) {
        let index = self.resize_places(value);

        self.places[index] = Some(place);
    }

    /// Set a value's projected place.
    fn set_projected_place(
        &mut self,
        value: mir::Value,
        base: mir::Value,
        projection: mir::Projection,
    ) {
        let place = self.place_for_value(base).with_projection(projection);

        self.set_place(value, place);
    }

    /// Copy a value's known place.
    fn copy_place(&mut self, value: mir::Value, source: mir::Value) {
        let Some(place) = self.value_place(source).cloned() else {
            self.clear_place(value);
            return;
        };

        self.set_place(value, place);
    }

    /// Resize place storage for one value.
    fn resize_places(&mut self, value: mir::Value) -> usize {
        let index = value.id() as usize;
        let value_count = index + 1;

        if self.places.len() < value_count {
            self.places.resize(value_count, None);
        }

        index
    }
}
