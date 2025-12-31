use std::collections::HashMap;

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NormalizationMode, StaticArgument, SymbolTable,
    SymbolType, Type, TypeElement, TypeField, TypeIndexSignature, TypeLiteral, TypeTable,
    TypeUnaryOperator,
};
use destack_workspace::{Module, ProfileId};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize a type id for the given mode.
    pub(crate) fn normalize_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
    ) -> LocalTypeId {
        let mut visited = Vec::new();
        self.normalize_type_inner(module, profile, type_id, symbols, types, mode, &mut visited)
    }

    /// Normalize a type id with a recursion guard.
    pub(super) fn normalize_type_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // reuse cached normalization when available
        if let Some(normalized) = types.normalized_type(mode, type_id) {
            return normalized;
        }

        // avoid infinite recursion on self referential types
        if visited.contains(&type_id) {
            return type_id;
        }
        visited.push(type_id);

        // keep the source id for any normalized replacement
        let source_id = types.get_type_source(type_id);
        // clone to avoid holding a borrow across recursive normalization
        let ty = types.get_type(type_id).clone();

        // normalize based on structural shape
        let normalized_id = match ty {
            Type::Union { elements } => self.normalize_union_type(
                module, profile, type_id, &elements, symbols, types, mode, visited,
            ),
            Type::Intersection { elements } => self.normalize_intersection_type(
                module, profile, type_id, &elements, symbols, types, mode, visited,
            ),
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // rewrite well known references to canonical shapes
                if let Some(normalized) = self.normalize_well_known_type_reference(
                    module,
                    symbols,
                    profile,
                    symbol,
                    static_arguments.as_deref(),
                    types,
                ) {
                    let normalized_id = types.insert_type_from_any(normalized, source_id);
                    self.normalize_type_inner(
                        module,
                        profile,
                        normalized_id,
                        symbols,
                        types,
                        mode,
                        visited,
                    )
                }
                // expand type aliases with static arguments
                else if symbol.ty() == SymbolType::TypeAlias {
                    let arguments = static_arguments.as_deref().unwrap_or(&[]);
                    let expanded = self.normalize_type_alias_reference_with_arguments(
                        module, profile, source_id, symbol, arguments, symbols, types, mode,
                        visited,
                    );
                    if let Some(expanded) = expanded {
                        expanded
                    } else {
                        let unwrapped = self.unwrap_normalization_alias_reference(type_id, types);
                        if unwrapped != type_id {
                            self.normalize_type_inner(
                                module, profile, unwrapped, symbols, types, mode, visited,
                            )
                        } else {
                            type_id
                        }
                    }
                } else {
                    let unwrapped = self.unwrap_normalization_alias_reference(type_id, types);
                    if unwrapped != type_id {
                        self.normalize_type_inner(
                            module, profile, unwrapped, symbols, types, mode, visited,
                        )
                    } else {
                        type_id
                    }
                }
            }
            Type::Array { element } => {
                // normalize the optional element type
                let original_element = element;
                let normalized_element = original_element.map(|element_id| {
                    self.normalize_type_inner(
                        module, profile, element_id, symbols, types, mode, visited,
                    )
                });

                if normalized_element == original_element {
                    type_id
                } else {
                    let normalized = Type::Array {
                        element: normalized_element,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::ArraySized { element, count } => {
                // normalize the array element type
                let original_element = element;
                let normalized_element = self.normalize_type_inner(
                    module,
                    profile,
                    original_element,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                if normalized_element == original_element {
                    type_id
                } else {
                    let normalized = Type::ArraySized {
                        element: normalized_element,
                        count,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Tuple { elements } => {
                // normalize tuple element types
                let mut normalized_elements = Vec::with_capacity(elements.len());
                let mut did_change = false;
                for element in elements {
                    let normalized_ty = self.normalize_type_inner(
                        module, profile, element.ty, symbols, types, mode, visited,
                    );
                    if normalized_ty != element.ty {
                        did_change = true;
                    }
                    normalized_elements.push(TypeElement {
                        ty: normalized_ty,
                        ..element
                    });
                }

                if !did_change {
                    type_id
                } else {
                    let normalized = Type::Tuple {
                        elements: normalized_elements,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                // track object member changes
                let mut did_change = false;

                // normalize object fields
                let mut normalized_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let normalized_ty = self.normalize_type_inner(
                        module, profile, field.ty, symbols, types, mode, visited,
                    );
                    if normalized_ty != field.ty {
                        did_change = true;
                    }
                    normalized_fields.push(TypeField {
                        ty: normalized_ty,
                        ..field
                    });
                }

                // normalize callable signatures
                let normalized_calls = self.normalize_type_list(
                    module,
                    profile,
                    &call_signatures,
                    symbols,
                    types,
                    mode,
                    visited,
                    &mut did_change,
                );
                // normalize construct signatures
                let normalized_constructs = self.normalize_type_list(
                    module,
                    profile,
                    &construct_signatures,
                    symbols,
                    types,
                    mode,
                    visited,
                    &mut did_change,
                );

                // normalize index signatures
                let mut normalized_indexes = Vec::with_capacity(index_signatures.len());
                for signature in index_signatures {
                    let normalized_key = self.normalize_type_inner(
                        module,
                        profile,
                        signature.key_type,
                        symbols,
                        types,
                        mode,
                        visited,
                    );
                    let normalized_value = self.normalize_type_inner(
                        module,
                        profile,
                        signature.value_type,
                        symbols,
                        types,
                        mode,
                        visited,
                    );

                    if normalized_key != signature.key_type
                        || normalized_value != signature.value_type
                    {
                        did_change = true;
                    }

                    normalized_indexes.push(TypeIndexSignature {
                        key_type: normalized_key,
                        value_type: normalized_value,
                        ..signature
                    });
                }

                if !did_change {
                    type_id
                } else {
                    let normalized = Type::Object {
                        fields: normalized_fields,
                        call_signatures: normalized_calls,
                        construct_signatures: normalized_constructs,
                        index_signatures: normalized_indexes,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                // track function member changes
                let mut did_change = false;
                // normalize type parameter and parameter lists
                let normalized_static = self.normalize_type_list(
                    module,
                    profile,
                    &static_parameters,
                    symbols,
                    types,
                    mode,
                    visited,
                    &mut did_change,
                );
                let normalized_dynamic = self.normalize_type_list(
                    module,
                    profile,
                    &dynamic_parameters,
                    symbols,
                    types,
                    mode,
                    visited,
                    &mut did_change,
                );
                // normalize the this parameter when present
                let normalized_this = match this_parameter {
                    Some(type_id) => {
                        let normalized = self.normalize_type_inner(
                            module, profile, type_id, symbols, types, mode, visited,
                        );
                        if normalized != type_id {
                            did_change = true;
                        }
                        Some(normalized)
                    }
                    None => None,
                };
                // normalize the return type when present
                let normalized_return = match return_type {
                    Some(type_id) => {
                        let normalized = self.normalize_type_inner(
                            module, profile, type_id, symbols, types, mode, visited,
                        );
                        if normalized != type_id {
                            did_change = true;
                        }
                        Some(normalized)
                    }
                    None => None,
                };

                if !did_change {
                    type_id
                } else {
                    let normalized = Type::Function {
                        asynchrony,
                        cardinality,
                        static_parameters: normalized_static,
                        this_parameter: normalized_this,
                        dynamic_parameters: normalized_dynamic,
                        return_type: normalized_return,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => self.normalize_conditional_type(
                module, profile, source_id, left, right, then_type, else_type, symbols, types,
                mode, visited,
            ),
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => self.normalize_mapped_type(
                module, profile, source_id, parameter, modifiers, value, symbols, types, mode,
                visited,
            ),
            Type::Index { left, index } => self.normalize_index_type(
                module, profile, source_id, left, index, symbols, types, mode, visited,
            ),
            Type::TemplateLiteral { strings, spans } => {
                // normalize template literal spans
                let mut did_change = false;
                let normalized_spans = self.normalize_type_list(
                    module,
                    profile,
                    &spans,
                    symbols,
                    types,
                    mode,
                    visited,
                    &mut did_change,
                );
                if !did_change {
                    type_id
                } else {
                    let normalized = Type::TemplateLiteral {
                        strings,
                        spans: normalized_spans,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Unevaluated(_) | Type::Import { .. } => type_id,
            Type::Infer { name, constraint } => {
                // normalize the inference constraint when present
                let original_constraint = constraint;
                let constraint = original_constraint.map(|type_id| {
                    self.normalize_type_inner(
                        module, profile, type_id, symbols, types, mode, visited,
                    )
                });

                if constraint == original_constraint {
                    type_id
                } else {
                    let normalized = Type::Infer { name, constraint };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                // normalize the predicate target when present
                let original_target = target;
                let target = original_target.map(|type_id| {
                    self.normalize_type_inner(
                        module, profile, type_id, symbols, types, mode, visited,
                    )
                });
                if target == original_target {
                    type_id
                } else {
                    let normalized = Type::Predicate {
                        asserts,
                        subject,
                        target,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Unary { operator, right } => match operator {
                TypeUnaryOperator::Keyof => self.normalize_keyof_type(
                    module, profile, source_id, right, symbols, types, mode, visited,
                ),
                _ => {
                    // normalize unary operand
                    let original_right = right;
                    let right = self.normalize_type_inner(
                        module,
                        profile,
                        original_right,
                        symbols,
                        types,
                        mode,
                        visited,
                    );
                    if right == original_right {
                        type_id
                    } else {
                        let normalized = Type::Unary { operator, right };
                        types.insert_type_from_any(normalized, source_id)
                    }
                }
            },
            Type::Binary {
                left,
                operator,
                right,
            } => {
                // normalize binary operands
                let original_left = left;
                let original_right = right;
                let left = self.normalize_type_inner(
                    module,
                    profile,
                    original_left,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                let right = self.normalize_type_inner(
                    module,
                    profile,
                    original_right,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                if left == original_left && right == original_right {
                    type_id
                } else {
                    let normalized = Type::Binary {
                        left,
                        operator,
                        right,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Mutable { mutability, right } => {
                // normalize mutable target
                let original_right = right;
                let right = self.normalize_type_inner(
                    module,
                    profile,
                    original_right,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::Mutable { mutability, right };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                // normalize value of target
                let original_right = right;
                let right = self.normalize_type_inner(
                    module,
                    profile,
                    original_right,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::ValueOf {
                        mutability,
                        variance,
                        right,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                // normalize reference of target
                let original_right = right;
                let right = self.normalize_type_inner(
                    module,
                    profile,
                    original_right,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::ReferenceOf {
                        mutability,
                        variance,
                        right,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::PointerOf { mutability, right } => {
                // normalize pointer target
                let original_right = right;
                let right = self.normalize_type_inner(
                    module,
                    profile,
                    original_right,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::PointerOf { mutability, right };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Value { value } => {
                // normalize type value target
                let original_value = value;
                let value = self.normalize_type_inner(
                    module,
                    profile,
                    original_value,
                    symbols,
                    types,
                    mode,
                    visited,
                );
                if value == original_value {
                    type_id
                } else {
                    let normalized = Type::Value { value };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::TypeLiteral { .. } | Type::InferVar { .. } | Type::This | Type::Error => type_id,
        };

        // release the recursion guard for this type
        visited.pop();
        // cache the normalized result for reuse
        types.set_normalized_type(mode, type_id, normalized_id);
        normalized_id
    }

    /// Normalize type alias references with static arguments.
    fn normalize_type_alias_reference_with_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        arguments: &[StaticArgument],
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> Option<LocalTypeId> {
        // fetch the alias instance type
        let instance_type_id = types.get_instance_type_id(symbol)?;

        // prefer resolved instance arguments when available
        let mut resolved_arguments = self
            .resolved_static_arguments_for_reference(module, source_id, types)
            .unwrap_or_default();

        // resolve arguments on demand when instances are not registered yet
        if resolved_arguments.is_empty() {
            let tree = module.dir(profile).tree.read();
            let options = self.analyze_context_options_for_module(module.id);
            match self.resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                symbol,
                Some(arguments),
                &options,
                &tree,
                symbols,
                types,
            ) {
                Ok(Some(arguments)) => {
                    resolved_arguments = arguments;
                }
                Ok(None) => {}
                Err(error) => {
                    self.error(error);
                }
            }
        }
        if resolved_arguments.is_empty() {
            return Some(self.normalize_type_inner(
                module,
                profile,
                instance_type_id,
                symbols,
                types,
                mode,
                visited,
            ));
        }

        // build type parameter substitutions
        let tree = module.dir(profile).tree.read();
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module,
            profile,
            symbol,
            &resolved_arguments,
            &tree,
            symbols,
            types,
        );
        if substitutions.is_empty() {
            return Some(instance_type_id);
        }

        // substitute parameters inside the instance type
        let mut cache = HashMap::new();
        let substituted =
            self.substitute_static_parameters(instance_type_id, &substitutions, types, &mut cache);

        // normalize the substituted shape
        Some(self.normalize_type_inner(module, profile, substituted, symbols, types, mode, visited))
    }

    /// Resolve static arguments from registered instances.
    fn resolved_static_arguments_for_reference(
        &self,
        module: &Module,
        source_id: LocalNodeIdAny,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        // return resolved instance arguments when available
        let instance_id = types.get_instance_for_node(source_id.into_global(module.id))?;
        let instance = types.get_instance(instance_id);
        Some(instance.static_arguments.clone())
    }

    /// Normalize union types by flattening and collapsing special cases.
    fn normalize_union_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        elements: &[LocalTypeId],
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // prepare union element collection
        let mut flattened = Vec::new();

        // normalize and collect union elements
        for element_id in elements {
            let normalized = self.normalize_type_inner(
                module,
                profile,
                *element_id,
                symbols,
                types,
                mode,
                visited,
            );
            // flatten nested unions
            match types.get_type(normalized) {
                Type::Union { elements: union } => {
                    // keep elements unique
                    for element_id in union {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    // keep elements unique
                    if !flattened.contains(&normalized) {
                        flattened.push(normalized);
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

        // no matches means never
        if filtered.is_empty() {
            return never_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    types.get_type_source(type_id),
                )
            });
        }

        // short circuit when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // reuse existing union id when unchanged
        if filtered == elements {
            return type_id;
        }

        let normalized = Type::Union { elements: filtered };
        types.insert_type_from_any(normalized, types.get_type_source(type_id))
    }

    /// Normalize intersection types by flattening and collapsing special cases.
    fn normalize_intersection_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        elements: &[LocalTypeId],
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // prepare intersection element collection
        let mut flattened = Vec::new();

        // normalize and collect intersection elements
        for element_id in elements {
            let normalized = self.normalize_type_inner(
                module,
                profile,
                *element_id,
                symbols,
                types,
                mode,
                visited,
            );
            // flatten nested intersections
            match types.get_type(normalized) {
                Type::Intersection {
                    elements: intersection,
                } => {
                    // keep elements unique
                    for element_id in intersection {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    // keep elements unique
                    if !flattened.contains(&normalized) {
                        flattened.push(normalized);
                    }
                }
            }
        }

        // collapse any or unknown and handle never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        // split special literals from remaining elements
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

        // honor dominating never or any
        if let Some(never_type) = never_type {
            return never_type;
        }
        if let Some(any_type) = any_type {
            return any_type;
        }

        // fall back to unknown when the intersection collapses
        if filtered.is_empty() {
            return unknown_type.unwrap_or_else(|| {
                types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    types.get_type_source(type_id),
                )
            });
        }

        // short circuit when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // reuse existing intersection id when unchanged
        if filtered == elements {
            return type_id;
        }

        let normalized = Type::Intersection { elements: filtered };
        types.insert_type_from_any(normalized, types.get_type_source(type_id))
    }

    /// Unwrap structural type aliases using cached instance types.
    pub(super) fn unwrap_normalization_alias_reference(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> LocalTypeId {
        // track alias chains as we walk
        let mut current_id = type_id;
        // record visited symbols to avoid cycles
        let mut visited = Vec::new();
        loop {
            // exit when the current type is not a reference
            let Type::Reference { symbol, .. } = types.get_type(current_id) else {
                break;
            };
            // exit when this is not a type alias
            if symbol.ty() != SymbolType::TypeAlias {
                break;
            }
            // exit on alias cycles
            if visited.contains(symbol) {
                break;
            }
            visited.push(*symbol);

            // exit when the alias has no instance type yet
            let Some(instance_id) = types.get_instance_type_id(*symbol) else {
                break;
            };
            current_id = instance_id;
        }

        current_id
    }

    /// Normalize a list of type ids, updating the change flag.
    fn normalize_type_list(
        &self,
        module: &Module,
        profile: ProfileId,
        type_ids: &[LocalTypeId],
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
        did_change: &mut bool,
    ) -> Vec<LocalTypeId> {
        let mut normalized = Vec::with_capacity(type_ids.len());
        // normalize each element and track changes
        for type_id in type_ids {
            let normalized_id =
                self.normalize_type_inner(module, profile, *type_id, symbols, types, mode, visited);
            if normalized_id != *type_id {
                *did_change = true;
            }
            normalized.push(normalized_id);
        }
        normalized
    }
}
