use destack_mir as mir;

/// Owned values tracked by drop insertion.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct OwnedValues {
    /// Dense ownership bits keyed by SSA value id.
    values: Vec<bool>,
}

impl OwnedValues {
    /// Create an empty set sized for one function.
    pub(super) fn new(value_count: usize) -> Self {
        Self {
            values: vec![false; value_count],
        }
    }

    /// Build available owned values from function parameters.
    pub(super) fn parameters(function: &mir::Function, owned: &Self) -> Self {
        let mut values = Self::new(function.value_types().len());

        // seed parameters available at function entry
        for parameter in &function.parameters {
            if owned.contains(parameter.value) {
                values.insert(parameter.value);
            }
        }

        values
    }

    /// Return whether the set contains a value.
    pub(super) fn contains(&self, value: mir::Value) -> bool {
        self.values
            .get(value.id() as usize)
            .copied()
            .unwrap_or(false)
    }

    /// Return copied values in this set.
    pub(super) fn values(&self) -> impl DoubleEndedIterator<Item = mir::Value> + '_ {
        self.values
            .iter()
            .enumerate()
            .filter_map(|(index, is_owned)| is_owned.then_some(mir::Value::new(index as u32)))
    }

    /// Insert one value.
    pub(super) fn insert(&mut self, value: mir::Value) {
        let index = value.id() as usize;
        if index >= self.values.len() {
            self.values.resize(index + 1, false);
        }

        self.values[index] = true;
    }

    /// Remove one value.
    pub(super) fn remove(&mut self, value: mir::Value) {
        let Some(slot) = self.values.get_mut(value.id() as usize) else {
            return;
        };

        *slot = false;
    }

    /// Insert one value when it is tracked as owned.
    pub(super) fn insert_reference(&mut self, value: mir::Value, owned: &Self) {
        // skip values outside the owned set
        if owned.contains(value) {
            self.insert(value);
        }
    }

    /// Retain values that are also present in another set.
    pub(super) fn intersect_with(&mut self, other: &Self) {
        for (index, is_owned) in self.values.iter_mut().enumerate() {
            if *is_owned && !other.contains(mir::Value::new(index as u32)) {
                *is_owned = false;
            }
        }
    }

    /// Retain values known to be owned.
    pub(super) fn retain_owned(&mut self, owned: &Self) {
        for (index, is_owned) in self.values.iter_mut().enumerate() {
            if *is_owned && !owned.contains(mir::Value::new(index as u32)) {
                *is_owned = false;
            }
        }
    }
}
