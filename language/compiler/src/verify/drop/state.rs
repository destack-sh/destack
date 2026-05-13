use destack_mir as mir;

use super::owned::OwnedValues;

/// Ownership state used while planning drops.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct DropState {
    /// Whole owned values still available.
    pub(super) owned: OwnedValues,
    /// Moved subplaces inside available owned values.
    pub(super) moved: Vec<mir::Place>,
}

impl DropState {
    /// Build function entry state.
    pub(super) fn parameters(function: &mir::Function, owned: &OwnedValues) -> Self {
        Self {
            owned: OwnedValues::parameters(function, owned),
            moved: Vec::new(),
        }
    }

    /// Intersect ownership with another predecessor state.
    pub(super) fn intersect_with(&mut self, other: &Self) {
        self.owned.intersect_with(&other.owned);

        // keep partial moves observed on any surviving owned value
        for moved in &other.moved {
            if !self.moved.contains(moved) {
                self.moved.push(moved.clone());
            }
        }
        self.retain_owned_moved();
    }

    /// Retain values known to be owned.
    pub(super) fn retain_owned(&mut self, owned: &OwnedValues) {
        self.owned.retain_owned(owned);
        self.retain_owned_moved();
    }

    /// Return whether one value is available.
    pub(super) fn contains(&self, value: mir::Value) -> bool {
        self.owned.contains(value)
    }

    /// Mark consumed whole values as unavailable.
    pub(super) fn remove_consumed(&mut self, consumed: &OwnedValues) {
        for value in consumed.values() {
            self.move_value(value);
        }
    }

    /// Insert one owned instruction destination.
    pub(super) fn insert_destination(
        &mut self,
        instruction: &mir::Instruction,
        owned: &OwnedValues,
    ) {
        let Some(destination) = instruction
            .destination()
            .and_then(mir::ValueReference::value)
        else {
            return;
        };
        if !owned.contains(destination) {
            return;
        }

        self.owned.insert(destination);
        self.moved.retain(|moved| {
            !matches!(moved.origin, mir::PlaceOrigin::Value(value) if value.value() == Some(destination))
        });
    }

    /// Mark one place as moved.
    pub(super) fn move_place(&mut self, place: mir::Place) {
        let mir::PlaceOrigin::Value(value) = place.origin else {
            return;
        };
        let Some(value) = value.value() else {
            return;
        };
        if !self.owned.contains(value) {
            return;
        }

        // move the whole value
        if place.projections.is_empty() {
            self.move_value(value);
            return;
        }

        // avoid redundant child moves
        if self.moved.iter().any(|moved| moved.contains(&place)) {
            return;
        }

        self.moved.retain(|moved| !place.contains(moved));
        self.moved.push(place);
    }

    /// Mark one whole value as moved.
    fn move_value(&mut self, value: mir::Value) {
        self.owned.remove(value);
        self.moved.retain(|moved| {
            !matches!(moved.origin, mir::PlaceOrigin::Value(origin) if origin.value() == Some(value))
        });
    }

    /// Remove moved places outside the surviving owned values.
    fn retain_owned_moved(&mut self) {
        self.moved.retain(|moved| match moved.origin {
            mir::PlaceOrigin::Value(value) => value
                .value()
                .is_some_and(|value| self.owned.contains(value)),
            mir::PlaceOrigin::Local(_) | mir::PlaceOrigin::Global(_) => false,
        });
    }
}
