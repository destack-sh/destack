use std::collections::HashSet;

use crate::lower::ModuleLowerer;
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Return true when two type ids are structurally equivalent.
    pub(crate) fn types_are_equivalent(
        &self,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> bool {
        dir::are_types_equal(left, right, self.types)
    }

    /// Find nominal reference type ids for a symbol by walking its value type.
    pub(crate) fn nominal_reference_type_ids_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::LocalTypeId> {
        // get the value type id as the search root
        let Some(value_type_id) = self.types.get_value_type_id(symbol) else {
            return Vec::new();
        };

        // walk the type graph to collect reference ids
        let mut visited = HashSet::new();
        let mut seen = HashSet::new();
        let mut reference_ids = Vec::new();
        self.collect_reference_type_ids(
            value_type_id,
            symbol,
            &mut visited,
            &mut seen,
            &mut reference_ids,
        );
        reference_ids
    }

    /// Find a nominal reference type id for a symbol by walking its value type.
    pub(crate) fn nominal_reference_type_id_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LocalTypeId> {
        // return the first nominal reference id
        self.nominal_reference_type_ids_for_symbol(symbol)
            .into_iter()
            .next()
    }

    /// Collect reference type ids for a symbol by walking nested type nodes.
    fn collect_reference_type_ids(
        &self,
        type_id: dir::LocalTypeId,
        symbol: dir::GlobalSymbolId,
        visited: &mut HashSet<dir::LocalTypeId>,
        seen: &mut HashSet<dir::LocalTypeId>,
        reference_ids: &mut Vec<dir::LocalTypeId>,
    ) {
        // skip visited nodes to avoid cycles
        if !visited.insert(type_id) {
            return;
        }

        // walk the type based on its structure
        match self.types.get_type(type_id) {
            dir::Type::Reference {
                symbol: target_symbol,
                ..
            } => {
                if *target_symbol == symbol && seen.insert(type_id) {
                    reference_ids.push(type_id);
                }
            }
            dir::Type::Value { value } => {
                self.collect_reference_type_ids(*value, symbol, visited, seen, reference_ids);
            }
            dir::Type::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                self.collect_reference_type_ids(*left, symbol, visited, seen, reference_ids);
                self.collect_reference_type_ids(*right, symbol, visited, seen, reference_ids);
                self.collect_reference_type_ids(*then_type, symbol, visited, seen, reference_ids);
                self.collect_reference_type_ids(*else_type, symbol, visited, seen, reference_ids);
            }
            dir::Type::Mapped {
                parameter, value, ..
            } => {
                self.collect_reference_type_ids(
                    parameter.constraint,
                    symbol,
                    visited,
                    seen,
                    reference_ids,
                );
                if let Some(remap) = parameter.key_remap {
                    self.collect_reference_type_ids(remap, symbol, visited, seen, reference_ids);
                }
                self.collect_reference_type_ids(*value, symbol, visited, seen, reference_ids);
            }
            dir::Type::Index { left, index } => {
                self.collect_reference_type_ids(*left, symbol, visited, seen, reference_ids);
                self.collect_reference_type_ids(*index, symbol, visited, seen, reference_ids);
            }
            dir::Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.collect_reference_type_ids(*span, symbol, visited, seen, reference_ids);
                }
            }
            dir::Type::Infer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    self.collect_reference_type_ids(
                        *constraint,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
            }
            dir::Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.collect_reference_type_ids(*target, symbol, visited, seen, reference_ids);
                }
            }
            dir::Type::Unary { right, .. } => {
                self.collect_reference_type_ids(*right, symbol, visited, seen, reference_ids);
            }
            dir::Type::Mutable { right, .. } => {
                self.collect_reference_type_ids(*right, symbol, visited, seen, reference_ids);
            }
            dir::Type::ValueOf { right, .. } => {
                self.collect_reference_type_ids(*right, symbol, visited, seen, reference_ids);
            }
            dir::Type::ReferenceOf { right, .. } => {
                self.collect_reference_type_ids(*right, symbol, visited, seen, reference_ids);
            }
            dir::Type::PointerOf { right, .. } => {
                self.collect_reference_type_ids(*right, symbol, visited, seen, reference_ids);
            }
            dir::Type::Binary { left, right, .. } => {
                self.collect_reference_type_ids(*left, symbol, visited, seen, reference_ids);
                self.collect_reference_type_ids(*right, symbol, visited, seen, reference_ids);
            }
            dir::Type::ArraySized { element, .. } => {
                self.collect_reference_type_ids(*element, symbol, visited, seen, reference_ids);
            }
            dir::Type::Array { element } => {
                if let Some(element) = element {
                    self.collect_reference_type_ids(*element, symbol, visited, seen, reference_ids);
                }
            }
            dir::Type::Tuple { elements } => {
                for element in elements {
                    self.collect_reference_type_ids(
                        element.ty,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
            }
            dir::Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.collect_reference_type_ids(field.ty, symbol, visited, seen, reference_ids);
                }
                for signature in call_signatures {
                    self.collect_reference_type_ids(
                        *signature,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
                for signature in construct_signatures {
                    self.collect_reference_type_ids(
                        *signature,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
                for signature in index_signatures {
                    self.collect_reference_type_ids(
                        signature.key_type,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                    self.collect_reference_type_ids(
                        signature.value_type,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
            }
            dir::Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                for parameter in static_parameters {
                    self.collect_reference_type_ids(
                        *parameter,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
                if let Some(this_parameter) = this_parameter {
                    self.collect_reference_type_ids(
                        *this_parameter,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
                for parameter in dynamic_parameters {
                    self.collect_reference_type_ids(
                        *parameter,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
                if let Some(return_type) = return_type {
                    self.collect_reference_type_ids(
                        *return_type,
                        symbol,
                        visited,
                        seen,
                        reference_ids,
                    );
                }
            }
            dir::Type::Union { elements } | dir::Type::Intersection { elements } => {
                for element in elements {
                    self.collect_reference_type_ids(*element, symbol, visited, seen, reference_ids);
                }
            }
            dir::Type::TypeLiteral { .. }
            | dir::Type::InferVar { .. }
            | dir::Type::This
            | dir::Type::Unevaluated(_)
            | dir::Type::Import { .. }
            | dir::Type::Error => {}
        }
    }
}
