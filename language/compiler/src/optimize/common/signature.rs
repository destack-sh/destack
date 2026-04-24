use std::collections::HashSet;

use destack_mir as mir;
use destack_mir::function_signature_parts;

use crate::optimize::common::TypeKey;

/// Signature key used for matching function types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureKey {
    /// Parameter type keys for the signature.
    pub parameters: Vec<TypeKey>,
    /// Return type key for the signature.
    pub result: TypeKey,
}

impl SignatureKey {
    /// Build a signature key from a function definition.
    pub fn from_function(tree: &mir::NodeTree, function: &mir::Function) -> Option<Self> {
        // collect parameter type keys
        let parameters = function
            .parameters
            .iter()
            .map(|param| Some(TypeKey::from_type(param.ty.ty()?, tree)))
            .collect::<Option<Vec<_>>>()?;

        // collect result type key
        let result = TypeKey::from_type(function.return_type.ty()?, tree);

        Some(Self { parameters, result })
    }

    /// Build a signature key from a function pointer type.
    pub fn from_signature_type(
        tree: &mir::NodeTree,
        signature: impl Into<mir::TypeReference>,
    ) -> Option<Self> {
        let signature = signature.into().ty()?;

        // resolve the function pointer signature
        let Some((parameters, result)) = function_signature_parts(tree.get(signature)) else {
            return None;
        };

        // collect parameter type keys
        let parameters = parameters
            .iter()
            .map(|param| Some(TypeKey::from_type(param.ty()?, tree)))
            .collect::<Option<Vec<_>>>()?;

        // collect result type key
        let result = TypeKey::from_type(result.ty()?, tree);

        Some(Self { parameters, result })
    }
}

/// Remapping information for removed parameters.
#[derive(Debug, Clone)]
pub struct ParameterRemap {
    /// Parameter indices removed from the signature.
    removal_indices: Vec<usize>,
    /// Fast lookup set for removed indices.
    removal_set: HashSet<usize>,
}

impl ParameterRemap {
    /// Create a new remapping from removal indices.
    pub fn new(removals: &[usize]) -> Self {
        // copy and sort the removal indices
        let mut removal_indices: Vec<usize> = removals.to_vec();
        removal_indices.sort_unstable();

        // build the removal set for lookups
        let removal_set: HashSet<usize> = removal_indices.iter().copied().collect();

        Self {
            removal_indices,
            removal_set,
        }
    }

    /// Return the sorted removal indices.
    pub fn removal_indices(&self) -> &[usize] {
        &self.removal_indices
    }

    /// Return the removal set for fast lookups.
    pub fn removal_set(&self) -> &HashSet<usize> {
        &self.removal_set
    }

    /// Filter a list by removing indices in the removal set.
    pub fn filter_by_index<T: Clone>(&self, items: &[T]) -> Vec<T> {
        // build the filtered list
        let mut filtered = Vec::with_capacity(items.len().saturating_sub(self.removal_set.len()));
        for (index, item) in items.iter().enumerate() {
            if !self.removal_set.contains(&index) {
                filtered.push(item.clone());
            }
        }

        filtered
    }

    /// Remap a parameter index after removals.
    pub fn remap_parameter_index(&self, index: u32) -> Option<u32> {
        // convert the index to usize for comparisons
        let index = index as usize;

        // reject indices that were removed
        if self.removal_indices.binary_search(&index).is_ok() {
            return None;
        }

        // compute the shift from earlier removals
        let shift = self
            .removal_indices
            .iter()
            .take_while(|removed| **removed < index)
            .count();

        Some((index - shift) as u32)
    }

    /// Remap return borrow-region indices after removals.
    pub fn remap_return_region(&self, region: &mir::BorrowRegion) -> Option<mir::BorrowRegion> {
        // only remap explicit parameter lifetimes
        let mir::BorrowRegion::Parameters(indices) = region else {
            return Some(region.clone());
        };

        // translate each index through the removal map
        let mut remapped = Vec::new();
        for index in indices {
            remapped.push(self.remap_parameter_index(*index)?);
        }

        // drop empty parameter sets
        if remapped.is_empty() {
            return None;
        }

        Some(mir::BorrowRegion::Parameters(remapped))
    }

    /// Remap allocation size parameter indices after removals.
    pub fn remap_allocation_size(
        &self,
        allocation_size: Option<mir::AllocationSize>,
    ) -> Option<mir::AllocationSize> {
        // read the existing allocation size metadata
        let allocation_size = allocation_size?;

        // remap the required stride index
        let stride_index = self.remap_parameter_index(allocation_size.stride_index)?;

        // remap the optional element count index
        let element_count_index = match allocation_size.element_count_index {
            Some(index) => Some(self.remap_parameter_index(index)?),
            None => None,
        };

        Some(mir::AllocationSize::new(stride_index, element_count_index))
    }
}

/// Build a function pointer signature type for a function.
pub fn build_signature_type(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::NodeTree,
) -> mir::LocalNodeId<mir::Type> {
    // collect parameter types from the function signature
    let function = tree.get(function_id);
    let parameters = function.parameters.iter().map(|param| param.ty).collect();

    // insert the function pointer type
    tree.insert_type(mir::Type::FunctionSignature {
        parameters,
        result: function.return_type,
    })
}

/// Collect parameter indices that must be preserved by metadata.
pub fn required_parameter_indices(function: &mir::Function) -> HashSet<usize> {
    // gather required indices from metadata
    let mut required = HashSet::new();

    // include explicit return-region parameters
    if let mir::BorrowRegion::Parameters(indices) = &function.return_region {
        for index in indices {
            required.insert(*index as usize);
        }
    }

    // include allocation size indices
    if let Some(allocation_size) = function.allocation_size {
        required.insert(allocation_size.stride_index as usize);
        if let Some(count_index) = allocation_size.element_count_index {
            required.insert(count_index as usize);
        }
    }

    required
}
