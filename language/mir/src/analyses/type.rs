use crate as mir;

use super::{Analysis, FunctionCache, Mutation};

/// Value and local type lookup for one MIR function.
#[derive(Debug, Clone)]
pub struct ValueTypeTable {
    /// SSA value types indexed by value id.
    values: Vec<Option<mir::LocalNodeId<mir::Type>>>,
    /// Local types indexed by local id.
    locals: Vec<Option<mir::LocalNodeId<mir::Type>>>,
}

impl ValueTypeTable {
    /// Build value types for a function.
    pub fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        // seed value types from the function table
        let values = function.value_types().to_vec();

        // seed local types from the local table
        let local_count = function.local_capacity();
        let mut locals = vec![None; local_count];
        for &local_id in function.locals() {
            let local = tree.get(local_id);
            locals[local_id.id as usize] = Some(local.ty);
        }

        Self { values, locals }
    }

    /// Return the type of a value.
    pub fn value_type(&self, value: impl Into<mir::Value>) -> Option<mir::LocalNodeId<mir::Type>> {
        let value = value.into();
        self.values.get(value.0 as usize).copied().flatten()
    }

    /// Return the expected type of a value.
    pub fn expect_value_type(&self, value: impl Into<mir::Value>) -> mir::LocalNodeId<mir::Type> {
        let value = value.into();
        match self.value_type(value) {
            Some(type_id) => type_id,
            None => panic!("missing value type for {value:?}"),
        }
    }

    /// Return the type of a local when available.
    pub fn local_type(
        &self,
        local: impl Into<mir::LocalId>,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        let local = local.into();

        self.locals.get(local.id as usize).copied().flatten()
    }

    /// Return the expected type of a local.
    pub fn expect_local_type(&self, local: impl Into<mir::LocalId>) -> mir::LocalNodeId<mir::Type> {
        let local = local.into();
        match self.local_type(local) {
            Some(type_id) => type_id,
            None => panic!("missing type for local {local:?}"),
        }
    }

    /// Resolve the pointee type of an address-bearing value when statically known.
    pub fn pointee_type(
        &self,
        address: impl Into<mir::Value>,
        tree: &mir::Tree,
    ) -> Option<mir::TypeId> {
        let type_id = self.expect_value_type(address);
        match tree.get(type_id) {
            mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => {
                Some(*pointee)
            }
            mir::Type::Slice { element, .. }
            | mir::Type::Tensor { element, .. }
            | mir::Type::TensorView { element, .. } => Some(*element),
            _ => None,
        }
    }

    /// Resolve a reference's storage when statically known.
    pub fn reference_storage(
        &self,
        reference: impl Into<mir::Value>,
        tree: &mir::Tree,
    ) -> Option<mir::Storage> {
        let type_id = self.expect_value_type(reference);
        tree.get(type_id).reference_storage()
    }

    /// Resolve a reference's kind when statically known.
    pub fn reference_kind(
        &self,
        reference: impl Into<mir::Value>,
        tree: &mir::Tree,
    ) -> Option<mir::ReferenceKind> {
        let type_id = self.expect_value_type(reference);
        tree.get(type_id).reference_kind()
    }

    /// Return the unsigned integer width for a value when it is known.
    pub fn unsigned_int_width(
        &self,
        value: mir::Value,
        pointer_width_bits: u16,
        tree: &mir::Tree,
    ) -> Option<u16> {
        let type_id = self.expect_value_type(value);

        // accept unsigned integer types
        match tree.get(type_id) {
            mir::Type::Int {
                width,
                is_signed: false,
            } => Some(*width),
            mir::Type::Usize => Some(pointer_width_bits),
            _ => None,
        }
    }

    /// Return whether one value can safely replace another value.
    pub fn can_substitute(&self, destination: mir::Value, replacement: mir::Value) -> bool {
        let destination_type = self.expect_value_type(destination);
        let replacement_type = self.expect_value_type(replacement);

        destination_type == replacement_type
    }

    /// Return the raw value type table.
    pub fn values(&self) -> &[Option<mir::LocalNodeId<mir::Type>>] {
        &self.values
    }
}

impl Analysis for ValueTypeTable {
    const INVALIDATED_BY: Mutation = Mutation::VALUE;
}

impl ValueTypeTable {
    /// Compute value and local types for one function.
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        _analyses: &mut FunctionCache,
    ) -> Self {
        Self::build(function, tree)
    }
}
