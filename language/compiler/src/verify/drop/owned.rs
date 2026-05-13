use std::collections::BTreeSet;

use destack_mir as mir;

/// Owned values tracked by drop insertion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct OwnedValues {
    /// Values in the set.
    values: BTreeSet<mir::Value>,
}

impl OwnedValues {
    /// Build available owned values from function parameters.
    pub(super) fn parameters(function: &mir::Function, owned: &Self) -> Self {
        let mut values = Self::default();

        // seed parameters available at function entry
        for parameter in &function.parameters {
            let Some(value) = parameter.value.value() else {
                continue;
            };
            if owned.contains(value) {
                values.insert(value);
            }
        }

        values
    }

    /// Return whether the set contains a value.
    pub(super) fn contains(&self, value: mir::Value) -> bool {
        self.values.contains(&value)
    }

    /// Return copied values in this set.
    pub(super) fn values(&self) -> impl Iterator<Item = mir::Value> + '_ {
        self.values.iter().copied()
    }

    /// Insert one value.
    pub(super) fn insert(&mut self, value: mir::Value) {
        self.values.insert(value);
    }

    /// Remove one value.
    pub(super) fn remove(&mut self, value: mir::Value) {
        self.values.remove(&value);
    }

    /// Add one value reference when it is owned.
    pub(super) fn insert_reference(&mut self, value: mir::ValueReference, owned: &Self) {
        let Some(value) = value.value() else {
            return;
        };

        // skip nonowned references without lifetime markers
        if owned.contains(value) {
            self.insert(value);
        }
    }

    /// Retain values that are also present in another set.
    pub(super) fn intersect_with(&mut self, other: &Self) {
        self.values.retain(|value| other.contains(*value));
    }

    /// Retain values known to be owned.
    pub(super) fn retain_owned(&mut self, owned: &Self) {
        self.values.retain(|value| owned.contains(*value));
    }
}
