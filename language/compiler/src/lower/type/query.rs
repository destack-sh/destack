use std::collections::HashSet;

use crate::lower::ModuleLowerer;
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Return the anchored node id for a type source.
    pub(crate) fn type_anchor(&self, type_id: dir::LocalTypeId) -> dir::AnchoredGlobalNodeId {
        let source = self.types.get_type_source(type_id);
        source.into_anchored(self.module_id, Some(self.profile))
    }

    /// Return true when two type ids are the same canonical type.
    pub(crate) fn types_are_equivalent(
        &self,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> bool {
        left == right
    }

    /// Return true when two method signature types are equivalent.
    pub(crate) fn method_signatures_equivalent(
        &self,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> bool {
        // load the signature types
        let left_type = self.types.get_type(left);
        let right_type = self.types.get_type(right);

        // compare function signatures while ignoring this
        match (left_type, right_type) {
            (dir::Type::Function(left), dir::Type::Function(right)) => {
                // require matching function modifiers
                if left.asynchrony != right.asynchrony || left.is_generator != right.is_generator {
                    return false;
                }

                // require matching static parameter counts
                if left.generic_parameters.len() != right.generic_parameters.len() {
                    return false;
                }

                // compare static parameter types
                for (left_param, right_param) in left
                    .generic_parameters
                    .iter()
                    .zip(right.generic_parameters.iter())
                {
                    if !self.types_are_equivalent(*left_param, *right_param) {
                        return false;
                    }
                }

                // require matching dynamic parameter counts
                if left.parameters.len() != right.parameters.len() {
                    return false;
                }

                // compare dynamic parameter types
                for (left_param, right_param) in left.parameters.iter().zip(right.parameters.iter())
                {
                    if !self.types_are_equivalent(*left_param, *right_param) {
                        return false;
                    }
                }

                // compare return types
                match (left.return_type, right.return_type) {
                    (Some(left_return), Some(right_return)) => {
                        self.types_are_equivalent(left_return, right_return)
                    }
                    (None, None) => true,
                    _ => false,
                }
            }
            _ => self.types_are_equivalent(left, right),
        }
    }

    /// Find nominal reference type ids for a symbol by scanning known types.
    pub(crate) fn nominal_reference_type_ids_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::LocalTypeId> {
        // scan all known types for matching reference ids
        let mut seen = HashSet::new();
        let mut reference_ids = Vec::new();
        let type_count = self.types.type_count();
        for index in 0..type_count {
            let type_id = dir::LocalTypeId::new(index);
            let dir_type = self.types.get_type(type_id);
            if let dir::Type::Named(reference) = dir_type
                && reference.symbol == symbol
                && seen.insert(type_id)
            {
                reference_ids.push(type_id);
            }
        }
        reference_ids
    }

    /// Find a nominal reference type id for a symbol by scanning known types.
    pub(crate) fn nominal_reference_type_id_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LocalTypeId> {
        // return the first nominal reference id
        self.nominal_reference_type_ids_for_symbol(symbol)
            .into_iter()
            .next()
    }
}
