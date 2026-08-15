use crate as mir;
use destack_core::BitSet;

/// Remapping for removed function parameters.
#[derive(Debug, Clone)]
pub struct ParameterRemap {
    /// Parameter indices removed from the signature.
    removal_indices: Vec<usize>,
}

impl ParameterRemap {
    /// Create a remapping from removed parameter indices.
    pub fn new(removals: &[usize]) -> Self {
        let mut removal_indices = removals.to_vec();
        removal_indices.sort_unstable();
        removal_indices.dedup();

        Self { removal_indices }
    }

    /// Return the sorted removed parameter indices.
    pub fn removal_indices(&self) -> &[usize] {
        &self.removal_indices
    }

    /// Remove remapped parameter positions from one list.
    pub fn filter_by_index<T: Clone>(&self, items: &[T]) -> Vec<T> {
        let mut filtered =
            Vec::with_capacity(items.len().saturating_sub(self.removal_indices.len()));

        for (index, item) in items.iter().enumerate() {
            if self.removal_indices.binary_search(&index).is_err() {
                filtered.push(item.clone());
            }
        }

        filtered
    }

    /// Remap one surviving parameter index.
    pub fn remap_parameter_index(&self, index: u32) -> Option<u32> {
        let index = index as usize;

        if self.removal_indices.binary_search(&index).is_ok() {
            return None;
        }

        let shift = self
            .removal_indices
            .partition_point(|removed| *removed < index);

        Some((index - shift) as u32)
    }

    /// Collect parameter indices retained by the result lifetime.
    pub fn required_indices(function: &mir::Function, tree: &mir::Tree) -> BitSet {
        let mut required = BitSet::new(function.parameters.len());

        if let Some(lifetime) = tree.type_lifetime(function.return_type) {
            for index in lifetime.slot_indices() {
                required.insert(index as usize);
            }
        }

        required
    }
}
