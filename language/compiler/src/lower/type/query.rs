use std::collections::HashSet;

use crate::lower::ModuleLowerer;
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Return the anchored node id for a type source.
    pub(crate) fn type_anchor(&self, type_id: dir::LocalTypeId) -> dir::AnchoredGlobalNodeId {
        let source = self.types.get_type_source(type_id);
        source.into_anchored(self.module_id, Some(self.profile))
    }

    /// Return true when two type ids are structurally equivalent.
    pub(crate) fn types_are_equivalent(
        &self,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> bool {
        dir::are_types_equal(left, right, self.types)
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
            (
                dir::Type::Function {
                    asynchrony: left_async,
                    cardinality: left_cardinality,
                    generic_parameters: left_static,
                    parameters: left_dynamic,
                    return_type: left_return,
                    ..
                },
                dir::Type::Function {
                    asynchrony: right_async,
                    cardinality: right_cardinality,
                    generic_parameters: right_static,
                    parameters: right_dynamic,
                    return_type: right_return,
                    ..
                },
            ) => {
                // require matching function modifiers
                if left_async != right_async || left_cardinality != right_cardinality {
                    return false;
                }

                // require matching static parameter counts
                if left_static.len() != right_static.len() {
                    return false;
                }

                // compare static parameter types
                for (left_param, right_param) in left_static.iter().zip(right_static.iter()) {
                    if !self.types_are_equivalent(*left_param, *right_param) {
                        return false;
                    }
                }

                // require matching dynamic parameter counts
                if left_dynamic.len() != right_dynamic.len() {
                    return false;
                }

                // compare dynamic parameter types
                for (left_param, right_param) in left_dynamic.iter().zip(right_dynamic.iter()) {
                    if !self.types_are_equivalent(*left_param, *right_param) {
                        return false;
                    }
                }

                // compare return types
                match (left_return, right_return) {
                    (Some(left_return), Some(right_return)) => {
                        self.types_are_equivalent(*left_return, *right_return)
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
            if let dir::Type::Reference {
                symbol: target_symbol,
                ..
            } = dir_type
                && *target_symbol == symbol
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
