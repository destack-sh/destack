use destack_mir::{self as mir, PlaceMap};

use super::owned::OwnedValues;

/// Ownership state used while planning drops.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct DropState {
    /// Whole owned values still available.
    pub(super) owned: OwnedValues,
    /// Known places keyed by SSA value.
    places: PlaceMap,
}

impl DropState {
    /// Build function entry state.
    pub(super) fn parameters(function: &mir::Function, owned: &OwnedValues) -> Self {
        Self {
            owned: OwnedValues::parameters(function, owned),
            places: PlaceMap::new(function.value_types().len()),
        }
    }

    /// Intersect ownership with another predecessor state.
    pub(super) fn intersect_with(&mut self, other: &Self) {
        self.owned.intersect_with(&other.owned);
        self.places.intersect(&other.places);
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

        self.places.bind(argument, parameter);
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

    /// Return values with known places derived from one owner value.
    pub(super) fn values_derived_from(
        &self,
        owner: mir::Value,
    ) -> impl Iterator<Item = mir::Value> + '_ {
        self.places.derived_from(owner)
    }

    /// Apply one instruction's place bindings.
    pub(super) fn apply_instruction(&mut self, instruction: &mir::Instruction) {
        self.places.apply(instruction);
    }

    /// Mark one whole value as moved.
    pub(super) fn remove(&mut self, value: mir::Value) {
        self.owned.remove(value);
    }
}
