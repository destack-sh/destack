use destack_dir::{GlobalSymbolId, LocalTypeId, Type, TypeLiteral, TypeTable};

use crate::Compiler;

impl Compiler {
    /// Resolve a type symbol from a type reference or type-as-value.
    pub(crate) fn unwrap_type_value_symbol(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // unwrap direct references
        if let Type::Reference { symbol, .. } = types.get_type(type_id) {
            return Some(*symbol);
        }

        // unwrap references stored in type-as-value wrappers
        if let Type::Value { value } = types.get_type(type_id)
            && let Type::Reference { symbol, .. } = types.get_type(*value)
        {
            return Some(*symbol);
        }

        None
    }

    /// Build a union type from two type ids.
    pub(crate) fn union_type(
        &self,
        left: LocalTypeId,
        right: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // reuse the left source for the combined union
        self.union_type_from_list(vec![left, right], left, types)
    }

    /// Build a union type from a list of elements.
    pub(crate) fn union_type_from_list(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // flatten nested unions and keep elements unique
        let mut flattened = Vec::new();
        for element_id in elements {
            match types.get_type(element_id) {
                Type::Union { elements: union } => {
                    for element_id in union {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    if !flattened.contains(&element_id) {
                        flattened.push(element_id);
                    }
                }
            }
        }

        // collapse any or unknown and remove never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating any or unknown
        if let Some(any_type) = any_type {
            return any_type;
        }
        if let Some(unknown_type) = unknown_type {
            return unknown_type;
        }

        // fall back to never when the union is empty
        if filtered.is_empty() {
            return never_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    types.get_type_source(source_type_id),
                )
            });
        }

        // avoid rebuilding when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // construct the union type
        let union = Type::Union { elements: filtered };
        types.insert_type_from_any(union, types.get_type_source(source_type_id))
    }
}
