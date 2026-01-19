use std::collections::HashSet;

use destack_dir::{
    GlobalSymbolId, LocalTypeId, NodeTree, StaticArgument, StaticExpression, StaticProperty,
    SymbolTable, SymbolType, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::{AnalyzeResult, Compiler};

impl Compiler {
    /// Evaluate a type id in place when it is unevaluated.
    pub(crate) fn evaluate_unevaluated_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        if matches!(types.get_type(type_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, profile, type_id, tree, symbols, types)?;
        }
        Ok(type_id)
    }

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

    /// Resolve an enum symbol from a type when possible.
    pub(crate) fn enum_symbol_for_type(
        &self,
        ty: &Type,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        match ty {
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Enum => Some(*symbol),
            Type::Value { value } => {
                let inner = types.get_type(*value);
                self.enum_symbol_for_type(inner, types)
            }
            Type::Intersection { elements } => elements.iter().find_map(|element_id| {
                let element_ty = types.get_type(*element_id);
                self.enum_symbol_for_type(element_ty, types)
            }),
            _ => None,
        }
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

    /// Check whether a type contains references without resolved instance types.
    pub(crate) fn type_contains_unresolved_reference(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // avoid infinite recursion in self-referential types
        if !visited.insert(ty_id) {
            return false;
        }

        match types.get_type(ty_id) {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if types.get_instance_type_id(*symbol).is_none() {
                    return true;
                }

                static_arguments.as_ref().is_some_and(|arguments| {
                    arguments.iter().any(|argument| {
                        self.static_argument_contains_unresolved_reference(argument, types, visited)
                    })
                })
            }
            Type::Value { value } => {
                self.type_contains_unresolved_reference(*value, types, visited)
            }
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                self.type_contains_unresolved_reference(*left, types, visited)
                    || self.type_contains_unresolved_reference(*right, types, visited)
                    || self.type_contains_unresolved_reference(*then_type, types, visited)
                    || self.type_contains_unresolved_reference(*else_type, types, visited)
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.type_contains_unresolved_reference(parameter.constraint, types, visited)
                    || parameter.key_remap.is_some_and(|key_remap| {
                        self.type_contains_unresolved_reference(key_remap, types, visited)
                    })
                    || self.type_contains_unresolved_reference(*value, types, visited)
            }
            Type::Index { left, index } => {
                self.type_contains_unresolved_reference(*left, types, visited)
                    || self.type_contains_unresolved_reference(*index, types, visited)
            }
            Type::TemplateLiteral { spans, .. } => spans
                .iter()
                .any(|span| self.type_contains_unresolved_reference(*span, types, visited)),
            Type::Import {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments.iter().any(|argument| {
                    self.static_argument_contains_unresolved_reference(argument, types, visited)
                })
            }),
            Type::Unary { right, .. }
            | Type::Mutable { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                self.type_contains_unresolved_reference(*right, types, visited)
            }
            Type::Binary { left, right, .. } => {
                self.type_contains_unresolved_reference(*left, types, visited)
                    || self.type_contains_unresolved_reference(*right, types, visited)
            }
            Type::ArraySized { element, .. } => {
                self.type_contains_unresolved_reference(*element, types, visited)
            }
            Type::Array { element } => element.is_some_and(|element| {
                self.type_contains_unresolved_reference(element, types, visited)
            }),
            Type::Tuple { elements } => elements
                .iter()
                .any(|element| self.type_contains_unresolved_reference(element.ty, types, visited)),
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                fields
                    .iter()
                    .any(|field| self.type_contains_unresolved_reference(field.ty, types, visited))
                    || call_signatures.iter().any(|signature| {
                        self.type_contains_unresolved_reference(*signature, types, visited)
                    })
                    || construct_signatures.iter().any(|signature| {
                        self.type_contains_unresolved_reference(*signature, types, visited)
                    })
                    || index_signatures.iter().any(|signature| {
                        self.type_contains_unresolved_reference(signature.key_type, types, visited)
                            || self.type_contains_unresolved_reference(
                                signature.value_type,
                                types,
                                visited,
                            )
                    })
            }
            Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                static_parameters.iter().any(|parameter| {
                    self.type_contains_unresolved_reference(*parameter, types, visited)
                }) || this_parameter.is_some_and(|parameter| {
                    self.type_contains_unresolved_reference(parameter, types, visited)
                }) || dynamic_parameters.iter().any(|parameter| {
                    self.type_contains_unresolved_reference(*parameter, types, visited)
                }) || return_type.is_some_and(|return_type| {
                    self.type_contains_unresolved_reference(return_type, types, visited)
                })
            }
            Type::Union { elements } | Type::Intersection { elements } => elements
                .iter()
                .any(|element| self.type_contains_unresolved_reference(*element, types, visited)),
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Error
            | Type::Unevaluated(_)
            | Type::Infer { .. }
            | Type::Predicate { .. }
            | Type::This => false,
        }
    }

    /// Check whether a static argument contains unresolved references.
    pub(crate) fn static_argument_contains_unresolved_reference(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // inspect the argument payload
        match argument {
            StaticArgument::Unevaluated { .. } => false,
            StaticArgument::Evaluated { value, .. } => {
                self.static_expression_contains_unresolved_reference(value, types, visited)
            }
        }
    }

    /// Check whether a static expression contains unresolved references.
    pub(crate) fn static_expression_contains_unresolved_reference(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // inspect the expression structure
        match expression {
            StaticExpression::Type { ty } => {
                self.type_contains_unresolved_reference(*ty, types, visited)
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => static_arguments.as_ref().is_some_and(|arguments| {
                arguments.iter().any(|argument| {
                    self.static_argument_contains_unresolved_reference(argument, types, visited)
                })
            }),
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => elements.iter().any(|element| {
                self.static_expression_contains_unresolved_reference(element, types, visited)
            }),
            StaticExpression::ObjectExpression { properties } => {
                properties.iter().any(|property| match property {
                    StaticProperty::Unevaluated { .. } => false,
                    StaticProperty::Field { value, default, .. } => {
                        self.static_expression_contains_unresolved_reference(value, types, visited)
                            || default.as_ref().is_some_and(|default| {
                                self.static_expression_contains_unresolved_reference(
                                    default, types, visited,
                                )
                            })
                    }
                    StaticProperty::Method { body, .. } => {
                        self.static_expression_contains_unresolved_reference(body, types, visited)
                    }
                })
            }
            StaticExpression::Unevaluated { .. }
            | StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. }
            | StaticExpression::RangeExpression { .. } => false,
        }
    }
}
