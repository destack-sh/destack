use std::collections::{HashMap, HashSet};

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NormalizationMode, PrimitiveType, ScalarLiteral,
    StaticArgument, StaticParameterKind, SymbolTable, SymbolType, Type, TypeBinaryOperator,
    TypeElement, TypeField, TypeIndexSignature, TypeLiteral, TypeTable, TypeUnaryOperator,
};
use destack_workspace::{Module, ProfileId};

use super::CanonicalSymbolMode;
use crate::Compiler;
use crate::analyze::infer::Assignability;

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

    /// Normalize a type for assignability checks.
    pub(crate) fn normalize_type_for_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // normalize the apparent return type first
        let normalized = self.normalize_apparent_type(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
        );

        // stop when normalization already expanded the type
        if normalized != type_id {
            return normalized;
        }

        // expand alias targets when the return type stays wrapped
        let Type::Reference {
            symbol,
            static_arguments,
        } = types.get_type(type_id).clone()
        else {
            return normalized;
        };
        if symbol.ty() != SymbolType::TypeAlias {
            return normalized;
        }
        let Some(arguments) = static_arguments.as_ref() else {
            return normalized;
        };

        // load the alias target for substitution
        let source_id = types.get_type_source(type_id);
        let Some(alias_target_id) = self
            .alias_target_type_id_for_symbol(module, profile, symbol, source_id, symbols, types)
        else {
            return normalized;
        };

        // resolve static arguments for substitution
        let tree = module.dir(profile).tree.read();
        let options = self.analyze_context_options_for_module(module.id);
        let resolved_arguments = self
            .resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                symbol,
                Some(arguments),
                false,
                &options,
                &tree,
                symbols,
                types,
            )
            .ok()
            .flatten();
        let arguments = resolved_arguments.as_deref().unwrap_or(arguments);

        // substitute parameters into the alias target
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module, profile, symbol, source_id, arguments, &tree, symbols, types,
        );
        if substitutions.is_empty() {
            return self.normalize_type(
                module,
                profile,
                alias_target_id,
                symbols,
                types,
                NormalizationMode::Assign,
            );
        }

        // apply substitutions and normalize the result
        let mut cache = HashMap::new();
        let substituted =
            self.substitute_static_parameters(alias_target_id, &substitutions, types, &mut cache);
        self.normalize_type(
            module,
            profile,
            substituted,
            symbols,
            types,
            NormalizationMode::Assign,
        )
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
        // NOTE #Performance: clone to avoid holding a borrow across recursive normalization
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
                // follow import targets while preserving alias identity
                let symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    symbol,
                    CanonicalSymbolMode::PreserveAliases,
                );

                // rewrite well known references to canonical shapes
                if let Some(normalized) = self.normalize_well_known_type_reference(
                    module,
                    symbols,
                    profile,
                    source_id,
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
                        self.normalize_type_inner(
                            module, profile, expanded, symbols, types, mode, visited,
                        )
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
            Type::Array {
                element,
                is_readonly,
            } => {
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
                        is_readonly,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
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
                        is_readonly,
                    };
                    types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                // normalize tuple element types
                let mut normalized_elements = Vec::with_capacity(elements.len());
                let mut did_change = false;
                for element in elements {
                    let (normalized, element_changed) = self.normalize_tuple_element(
                        module, profile, element, symbols, types, mode, visited,
                    );
                    if element_changed {
                        did_change = true;
                    }
                    normalized_elements.push(normalized);
                }

                if !did_change {
                    type_id
                } else {
                    let normalized = Type::Tuple {
                        elements: normalized_elements,
                        is_readonly,
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
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => self.normalize_conditional_type(
                module,
                profile,
                type_id,
                source_id,
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
                symbols,
                types,
                mode,
                visited,
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
                // collapse template literals containing never
                if normalized_spans.iter().any(|span_id| {
                    matches!(
                        types.get_type(*span_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        }
                    )
                }) {
                    return types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        },
                        source_id,
                    );
                }

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
            Type::Unevaluated(_) => {
                // treat unevaluated types as unknown during assignability normalization
                if matches!(mode, NormalizationMode::Assign) {
                    types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        source_id,
                    )
                } else {
                    type_id
                }
            }
            Type::Import { .. } => type_id,
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
                TypeUnaryOperator::Readonly | TypeUnaryOperator::AsConst => {
                    // materialize readonly modifiers during normalization
                    let normalized_right = self.normalize_type_inner(
                        module, profile, right, symbols, types, mode, visited,
                    );
                    self.materialize_readonly_type(source_id, normalized_right, types)
                }
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

                // reduce decidable type operators to boolean literals
                if let Some(normalized_id) = self.normalize_decidable_type_operator(
                    module, profile, source_id, operator, left, right, symbols, types, mode,
                ) {
                    return normalized_id;
                }

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

    /// Normalize decidable type operators into boolean literal types.
    fn normalize_decidable_type_operator(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        operator: TypeBinaryOperator,
        left: LocalTypeId,
        right: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
    ) -> Option<LocalTypeId> {
        if !matches!(
            operator,
            TypeBinaryOperator::In
                | TypeBinaryOperator::Is
                | TypeBinaryOperator::InstanceOf
                | TypeBinaryOperator::Extends
                | TypeBinaryOperator::Implements
        ) {
            return None;
        }

        // unwrap type values before assignability checks
        let unwrap_value = |ty_id: LocalTypeId, types: &TypeTable| match types.get_type(ty_id) {
            Type::Value { value } => *value,
            _ => ty_id,
        };
        let left = unwrap_value(left, types);
        let right = unwrap_value(right, types);

        // treat static-dependent checks as undecidable
        let mut static_visited = HashSet::new();
        let left_contains_static = self.type_contains_static_parameters(
            module,
            profile,
            left,
            symbols,
            types,
            &mut static_visited,
        );
        let mut static_visited = HashSet::new();
        let right_contains_static = self.type_contains_static_parameters(
            module,
            profile,
            right,
            symbols,
            types,
            &mut static_visited,
        );
        let is_decidable = !(left_contains_static || right_contains_static);
        let options = self.analyze_context_options_for_module(module.id);

        // compute assignability for operator semantics
        let assignability = if operator == TypeBinaryOperator::In {
            let mut key_visited = Vec::new();
            let key_type_id = self.normalize_keyof_type(
                module,
                profile,
                source_id,
                right,
                symbols,
                types,
                mode,
                &mut key_visited,
            );
            self.is_type_assignable(module, profile, symbols, key_type_id, left, types, &options)
        } else {
            self.is_type_assignable(module, profile, symbols, right, left, types, &options)
        };

        let ty = if !is_decidable {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }
        } else {
            let value = matches!(assignability, Assignability::Assignable);
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(value)),
            }
        };
        Some(types.insert_type_from_any(ty, source_id))
    }

    /// Normalize type alias references with static arguments.
    pub(crate) fn normalize_type_alias_reference_with_arguments(
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
        // return cached normalization results when available
        if let Some(normalized) = types.normalized_alias_reference(symbol, mode, arguments) {
            return Some(normalized);
        }

        // skip alias expansion when already resolving the same alias
        if types.is_normalization_alias_in_progress(symbol) {
            return None;
        }
        types.mark_normalization_alias_in_progress(symbol);

        let normalized = (|| {
            // ensure remote declarations are ready before reading instance types
            if symbol.module_id != module.id {
                let _ = self.require_analyze_module_declare(symbol.module_id, profile);
            }

            // resolve the instance type, including alias targets and remote imports
            let instance_type_id = self.instance_type_id_for_normalization(
                module, profile, symbol, source_id, symbols, types,
            )?;

            // select the static arguments to substitute
            let resolved_arguments = self.resolved_static_arguments_for_normalization(
                module, profile, source_id, symbol, arguments, symbols, types,
            );

            // materialize unevaluated alias targets before normalization
            let materialized_instance = self.materialize_alias_instance_for_normalization(
                module,
                profile,
                symbol,
                instance_type_id,
                types,
            );

            if resolved_arguments.is_empty() {
                return Some(self.normalize_type_inner(
                    module,
                    profile,
                    materialized_instance,
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
                source_id,
                &resolved_arguments,
                &tree,
                symbols,
                types,
            );
            if substitutions.is_empty() {
                // normalize the instance type (even when no substitutions are available)
                return Some(self.normalize_type_inner(
                    module,
                    profile,
                    materialized_instance,
                    symbols,
                    types,
                    mode,
                    visited,
                ));
            }

            // substitute and normalize
            let mut cache = HashMap::new();
            let substituted = self.substitute_static_parameters(
                materialized_instance,
                &substitutions,
                types,
                &mut cache,
            );
            let normalized_id = self.normalize_type_inner(
                module,
                profile,
                substituted,
                symbols,
                types,
                mode,
                visited,
            );
            Some(normalized_id)
        })();

        types.clear_normalization_alias_in_progress(symbol);
        if let Some(normalized_id) = normalized {
            types.set_normalized_alias_reference(symbol, mode, arguments.to_vec(), normalized_id);
        }
        normalized
    }

    /// Resolve static arguments from registered instances.
    fn resolved_static_arguments_for_reference(
        &self,
        module: &Module,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        types: &TypeTable,
    ) -> Option<Vec<StaticArgument>> {
        // return resolved instance arguments when available
        let instance_id = types.get_instance_for_node(source_id.into_global(module.id))?;
        let instance = types.get_instance(instance_id);
        if instance.symbol_id != symbol {
            return None;
        }
        Some(instance.static_arguments.clone())
    }

    /// Resolve the instance type id used for alias normalization.
    fn instance_type_id_for_normalization(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // fetch or import the alias target type when available
        if let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(module, profile, symbol, source_id, symbols, types)
        {
            return Some(alias_target_id);
        }

        // fall back to instance types when alias targets are unavailable
        if let Some(instance_type_id) = types.get_instance_type_id(symbol) {
            return Some(instance_type_id);
        }
        if let Ok(Some(instance_type_id)) =
            self.resolve_instance_type_for_symbol(module, profile, source_id, symbol, types)
        {
            return Some(instance_type_id);
        }

        // import remote instance types when needed
        if symbol.module_id != module.id {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let remote_types = remote_module.dir(profile).types.read();
            let remote_instance_id = remote_types.get_instance_type_id(symbol)?;
            let remote_instance_ty = remote_types.get_type(remote_instance_id);
            let local_instance_id = self.import_type_from_remote_for_node(
                source_id,
                remote_instance_ty,
                &remote_types,
                symbol,
                types,
            );
            types.set_instance_type(symbol, local_instance_id);
            return Some(local_instance_id);
        }

        None
    }

    /// Resolve the static arguments used for alias normalization.
    fn resolved_static_arguments_for_normalization(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        arguments: &[StaticArgument],
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Vec<StaticArgument> {
        // prefer resolved instance arguments when unevaluated arguments are present
        if arguments
            .iter()
            .any(|argument| matches!(argument, StaticArgument::Unevaluated { .. }))
        {
            // resolve unevaluated arguments using the full reference resolver
            let options = self.analyze_context_options_for_module(module.id);
            let tree = module.dir(profile).tree.read();
            if let Ok(Some(resolved)) = self.resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                symbol,
                Some(arguments),
                false,
                &options,
                &tree,
                symbols,
                types,
            ) {
                return resolved;
            }

            // fall back to local materialization when resolution is incomplete
            let tree = module.dir(profile).tree.read();
            let resolved = self.materialize_static_arguments_for_reference(
                module, profile, symbol, source_id, arguments, &tree, symbols, types,
            );
            if resolved != arguments {
                return resolved;
            }

            return self
                .resolved_static_arguments_for_reference(module, source_id, symbol, types)
                .unwrap_or_else(|| arguments.to_vec());
        }

        arguments.to_vec()
    }

    /// Materialize alias targets before normalization when needed.
    fn materialize_alias_instance_for_normalization(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        instance_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        if symbol.module_id == module.id {
            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();

            // skip materialization when the alias instance is already stable
            if !self.alias_instance_needs_materialization(
                module,
                profile,
                symbol,
                instance_type_id,
                types,
            ) {
                return instance_type_id;
            }

            // materialize static arguments using the alias module context
            let mut materialize_cache = HashMap::new();
            self.materialize_static_arguments_in_type(
                module,
                profile,
                instance_type_id,
                &tree,
                &symbols,
                types,
                &mut materialize_cache,
            )
        } else {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let tree = remote_module.dir(profile).tree.read();
            let symbols = remote_module.dir(profile).symbols.read();

            // skip materialization when the alias instance is already stable
            if !self.alias_instance_needs_materialization(
                &remote_module,
                profile,
                symbol,
                instance_type_id,
                types,
            ) {
                return instance_type_id;
            }

            // materialize static arguments using the alias module context
            let mut materialize_cache = HashMap::new();
            self.materialize_static_arguments_in_type(
                &remote_module,
                profile,
                instance_type_id,
                &tree,
                &symbols,
                types,
                &mut materialize_cache,
            )
        }
    }

    /// Return true when an alias instance needs value materialization.
    fn alias_instance_needs_materialization(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        instance_type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        // check for unevaluated targets or static value arguments
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        if matches!(types.get_type(instance_type_id), Type::Unevaluated(_)) {
            return true;
        }

        // skip value materialization when the alias has no value parameters
        let Some(parameters) =
            self.collect_static_parameter_symbols(module, symbol, profile, &tree, &symbols)
        else {
            return false;
        };
        let has_value_parameters = parameters.iter().any(|parameter_symbol| {
            self.static_parameter_kind_for_symbol_in_module(*parameter_symbol, &tree, &symbols)
                == StaticParameterKind::Value
        });
        if !has_value_parameters {
            return false;
        }

        self.type_contains_unevaluated_value_static_arguments(
            module,
            profile,
            instance_type_id,
            &tree,
            &symbols,
            types,
            &mut HashSet::new(),
        )
    }

    /// Normalize a tuple element, lifting readonly modifiers into flags.
    fn normalize_tuple_element(
        &self,
        module: &Module,
        profile: ProfileId,
        element: TypeElement,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> (TypeElement, bool) {
        // unwrap readonly/const modifiers into tuple element flags
        let mut element_ty_id = element.ty;
        let mut element_is_readonly = element.is_readonly;
        let mut did_change = false;
        if let Type::Unary {
            operator: TypeUnaryOperator::Readonly | TypeUnaryOperator::AsConst,
            right,
        } = types.get_type(element_ty_id)
        {
            element_ty_id = *right;
            element_is_readonly = true;
            did_change = true;
        }

        // normalize the tuple element type
        let normalized_ty = self.normalize_type_inner(
            module,
            profile,
            element_ty_id,
            symbols,
            types,
            mode,
            visited,
        );
        if normalized_ty != element.ty || element_is_readonly != element.is_readonly {
            did_change = true;
        }

        let normalized_element = TypeElement {
            ty: normalized_ty,
            is_readonly: element_is_readonly,
            ..element
        };
        (normalized_element, did_change)
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

        // strip nullish values when intersecting with empty object types
        let source_id = types.get_type_source(type_id);
        let mut empty_object_id = None;
        let mut remaining = Vec::new();
        for element_id in filtered {
            let is_empty_object = match types.get_type(element_id) {
                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                } => {
                    fields.is_empty()
                        && call_signatures.is_empty()
                        && construct_signatures.is_empty()
                        && index_signatures.is_empty()
                }
                _ => false,
            };

            if is_empty_object {
                if empty_object_id.is_none() {
                    empty_object_id = Some(element_id);
                }
            } else {
                remaining.push(element_id);
            }
        }

        let filtered = if let Some(empty_object_id) = empty_object_id {
            if remaining.is_empty() {
                return empty_object_id;
            }

            let mut stripped = Vec::new();
            for element_id in remaining {
                match types.get_type(element_id) {
                    Type::TypeLiteral {
                        value: TypeLiteral::Null | TypeLiteral::Undefined,
                    } => {
                        return types.insert_type_from_any(
                            Type::TypeLiteral {
                                value: TypeLiteral::Never,
                            },
                            source_id,
                        );
                    }
                    Type::Union { elements } => {
                        let mut non_nullish = Vec::new();
                        let mut has_nullish = false;
                        for union_id in elements {
                            match types.get_type(*union_id) {
                                Type::TypeLiteral {
                                    value: TypeLiteral::Null | TypeLiteral::Undefined,
                                } => {
                                    has_nullish = true;
                                }
                                _ => non_nullish.push(*union_id),
                            }
                        }

                        if !has_nullish {
                            stripped.push(element_id);
                            continue;
                        }

                        if non_nullish.is_empty() {
                            return types.insert_type_from_any(
                                Type::TypeLiteral {
                                    value: TypeLiteral::Never,
                                },
                                source_id,
                            );
                        }

                        if non_nullish.len() == 1 {
                            stripped.push(non_nullish[0]);
                        } else {
                            stripped.push(types.insert_type_from_any(
                                Type::Union {
                                    elements: non_nullish,
                                },
                                source_id,
                            ));
                        }
                    }
                    _ => stripped.push(element_id),
                }
            }

            stripped
        } else {
            remaining
        };

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
        let mut current_id = type_id;
        let mut visited = Vec::new();
        loop {
            // exit when the current type is not a reference
            let Type::Reference {
                symbol,
                static_arguments,
            } = types.get_type(current_id)
            else {
                break;
            };
            // exit when this is not a type alias
            if symbol.ty() != SymbolType::TypeAlias {
                break;
            }
            // avoid unwrapping aliases with explicit static arguments
            if static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
            {
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

    /// Resolve the apparent type for type operations.
    pub(crate) fn apparent_type(
        &self,
        _module: &Module,
        _profile: ProfileId,
        type_id: LocalTypeId,
        _symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // unwrap cached alias instances
        self.unwrap_normalization_alias_reference(type_id, types)
    }

    /// Resolve the apparent type for assignability checks.
    pub(crate) fn apparent_type_for_assignability(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // start with the original type id
        let mut apparent_id = type_id;

        // substitute static parameter constraints when available
        if let Type::Reference { symbol, .. } = types.get_type(apparent_id) {
            let source_id = types.get_type_source(apparent_id);
            if let Some(constraint_id) = self.static_parameter_constraint_type(
                module, profile, *symbol, source_id, symbols, types,
            ) && !matches!(
                types.get_type(constraint_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown
                }
            ) && constraint_id != apparent_id
            {
                apparent_id = constraint_id;
            }
        }

        // unwrap cached alias instances
        self.unwrap_normalization_alias_reference(apparent_id, types)
    }

    /// Normalize and resolve the apparent type.
    pub(crate) fn normalize_apparent_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
    ) -> LocalTypeId {
        // normalize the input type first
        let type_id = self.normalize_type(module, profile, type_id, symbols, types, mode);

        // resolve apparent types after normalization
        let type_id =
            self.apparent_type_for_assignability(module, profile, type_id, symbols, types);

        // normalize again after apparent type expansion
        self.normalize_type(module, profile, type_id, symbols, types, mode)
    }
}
