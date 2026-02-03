use std::collections::{HashMap, HashSet};

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NormalizationMode, PrimitiveType, ScalarLiteral,
    StaticArgument, StaticParameterKind, SymbolSpace, SymbolTable, SymbolType, Type,
    TypeBinaryOperator, TypeElement, TypeField, TypeIndexSignature, TypeLiteral, TypeTable,
    TypeUnaryOperator, WellKnownSymbol,
};
use destack_workspace::{Module, ProfileId};

use super::{CanonicalSymbolMode, RelationMode};
use crate::analyze::infer::Assignability;
use crate::timing::tags;
use crate::{AnalyzeError, Compiler};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize a reference symbol id to a type space symbol when possible.
    pub(crate) fn normalize_reference_symbol_id(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        if let Some(cached) = self.cached_normalized_reference_symbol(profile, module.id, symbol) {
            return cached;
        }

        let normalized = if symbol.module_id == module.id {
            let symbols = module.dir_base().symbols.read();
            let symbol_entry = symbols.get_symbol(symbol.local_id).clone();
            self.normalize_reference_symbol_id_with_symbols(
                module,
                profile,
                module,
                symbol,
                &symbols,
                symbol_entry,
            )
        } else {
            let remote_module = self.program.modules.get(symbol.module_id);
            let remote_module = remote_module.read();
            let symbols = remote_module.dir_base().symbols.read();
            let symbol_entry = symbols.get_symbol(symbol.local_id).clone();
            self.normalize_reference_symbol_id_with_symbols(
                module,
                profile,
                &remote_module,
                symbol,
                &symbols,
                symbol_entry,
            )
        };

        self.set_cached_normalized_reference_symbol(profile, module.id, symbol, normalized);
        normalized
    }

    /// Normalize a reference symbol id using a symbol table snapshot.
    fn normalize_reference_symbol_id_with_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        owner_module: &Module,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        symbol_entry: destack_dir::Symbol,
    ) -> GlobalSymbolId {
        // normalize to the declared symbol type
        let normalized =
            GlobalSymbolId::new(owner_module.id, symbol.local_id.with_type(symbol_entry.ty));
        if matches!(
            symbol_entry.space,
            SymbolSpace::Type | SymbolSpace::TypeValue
        ) {
            return normalized;
        }

        // map well known values to their type symbols
        for well_known in WellKnownSymbol::all() {
            if self.is_well_known_symbol(profile, symbol, well_known)
                && let Some(type_symbol) = self.get_well_known_type_symbol(profile, well_known)
            {
                return type_symbol;
            }
        }

        // skip when the symbol has no name key
        let Some(key) = symbol_entry.key else {
            return normalized;
        };

        // prefer type space symbols from the same scope
        let scope = symbols.get_scope_by_id(symbol_entry.scope.0);
        for (candidate_key, candidate_id) in symbols.active_named_symbols(scope) {
            if candidate_key != key {
                continue;
            }
            let candidate_entry = symbols.get_symbol(candidate_id);
            if !matches!(
                candidate_entry.space,
                SymbolSpace::Type | SymbolSpace::TypeValue
            ) {
                continue;
            }

            return GlobalSymbolId::new(
                owner_module.id,
                candidate_id.with_type(candidate_entry.ty),
            );
        }

        // gather global and ambient type symbols by key
        let mut candidates = Vec::new();
        if let Some(group) =
            self.get_global_symbol_group(module.id, profile, key, SymbolSpace::Type)
        {
            candidates.extend(group);
        }
        if let Some(group) =
            self.get_global_symbol_group(module.id, profile, key, SymbolSpace::TypeValue)
        {
            candidates.extend(group);
        }
        if let Some(ambient) =
            self.get_ambient_lib_symbol_sources_for_merge(profile, key, SymbolSpace::Type)
        {
            candidates.extend(ambient);
        }
        if let Some(ambient) =
            self.get_ambient_lib_symbol_sources_for_merge(profile, key, SymbolSpace::TypeValue)
        {
            candidates.extend(ambient);
        }

        // select the first global match
        if let Some(candidate) = candidates.into_iter().next() {
            let candidate_module = self.program.modules.get(candidate.module_id);
            let candidate_module = candidate_module.read();
            let candidate_symbols = candidate_module.dir_base().symbols.read();
            let candidate_entry = candidate_symbols.get_symbol(candidate.local_id);
            return GlobalSymbolId::new(
                candidate.module_id,
                candidate.local_id.with_type(candidate_entry.ty),
            );
        }

        normalized
    }

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
        self.normalize_type_with_relation(
            module,
            profile,
            type_id,
            symbols,
            types,
            mode,
            RelationMode::ASSIGN,
        )
    }

    /// Normalize a type id for the given mode and relation.
    pub(crate) fn normalize_type_with_relation(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_NORMALIZE);

        let mut visited = Vec::new();
        self.normalize_type_inner(
            module,
            profile,
            type_id,
            symbols,
            types,
            mode,
            relation_mode,
            &mut visited,
        )
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
            RelationMode::ASSIGN,
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
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // NOTE #Suspicious: normalization caches only ASSIGN, but normalize_type_inner handles TYPE_OPS too
        // reuse cached normalization when available
        let relation_key = relation_mode.cache_key();
        if relation_mode.is_cacheable()
            && let Some(entry) = types.normalized_type(mode, relation_key, type_id)
        {
            for (dependency_id, _) in &entry.dependency_versions.type_versions {
                types.record_normalization_dependency(*dependency_id);
            }
            for (dependency_id, _) in &entry.dependency_versions.symbol_versions {
                types.record_normalization_symbol_dependency(*dependency_id);
            }
            return entry.normalized_type;
        }

        // avoid infinite recursion on self referential types
        if visited.contains(&type_id) {
            types.record_normalization_dependency(type_id);
            return type_id;
        }

        // collect dependencies for this normalization pass
        types.push_normalization_dependency_scope();
        types.record_normalization_dependency(type_id);
        visited.push(type_id);

        // keep the source id for any normalized replacement
        let source_id = types.get_type_source(type_id);
        let ty = types.get_type(type_id).clone();

        // normalize based on structural shape
        let normalized_id = match ty {
            Type::Union { elements } => self.normalize_union_type(
                module,
                profile,
                type_id,
                &elements,
                symbols,
                types,
                mode,
                relation_mode,
                visited,
            ),
            Type::Intersection { elements } => self.normalize_intersection_type(
                module,
                profile,
                type_id,
                &elements,
                symbols,
                types,
                mode,
                relation_mode,
                visited,
            ),
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // normalize reference symbols to their declared type
                let normalized_symbol = self.normalize_reference_symbol_id(module, profile, symbol);
                if normalized_symbol != symbol {
                    let normalized = Type::Reference {
                        symbol: normalized_symbol,
                        static_arguments,
                    };
                    let normalized_id = types.insert_type_from_any(normalized, source_id);
                    self.normalize_type_inner(
                        module,
                        profile,
                        normalized_id,
                        symbols,
                        types,
                        mode,
                        relation_mode,
                        visited,
                    )
                } else {
                    let symbol = normalized_symbol;

                    // follow import targets while preserving alias identity
                    let symbol = self.canonical_symbol_id(
                        module,
                        symbols,
                        profile,
                        symbol,
                        CanonicalSymbolMode::PreserveAliases,
                    );

                    // rewrite well known references to canonical shapes
                    if let Some(well_known) = self.well_known_array_kind(profile, symbol)
                        && let Some(normalized) = self.normalize_well_known_type_reference(
                            module,
                            symbols,
                            profile,
                            source_id,
                            symbol,
                            well_known,
                            static_arguments.as_deref(),
                            types,
                        )
                    {
                        let normalized_id = types.insert_type_from_any(normalized, source_id);
                        self.normalize_type_inner(
                            module,
                            profile,
                            normalized_id,
                            symbols,
                            types,
                            mode,
                            relation_mode,
                            visited,
                        )
                    }
                    // expand type aliases with static arguments
                    else if symbol.ty() == SymbolType::TypeAlias {
                        let arguments = static_arguments.as_deref().unwrap_or(&[]);
                        let expanded = self.normalize_type_alias_reference_with_arguments(
                            module,
                            profile,
                            source_id,
                            symbol,
                            arguments,
                            symbols,
                            types,
                            mode,
                            relation_mode,
                            visited,
                        );
                        if let Some(expanded) = expanded {
                            self.normalize_type_inner(
                                module,
                                profile,
                                expanded,
                                symbols,
                                types,
                                mode,
                                relation_mode,
                                visited,
                            )
                        } else {
                            let unwrapped =
                                self.unwrap_normalization_alias_reference(type_id, types);
                            if unwrapped != type_id {
                                self.normalize_type_inner(
                                    module,
                                    profile,
                                    unwrapped,
                                    symbols,
                                    types,
                                    mode,
                                    relation_mode,
                                    visited,
                                )
                            } else {
                                type_id
                            }
                        }
                    } else {
                        let unwrapped = self.unwrap_normalization_alias_reference(type_id, types);
                        if unwrapped != type_id {
                            self.normalize_type_inner(
                                module,
                                profile,
                                unwrapped,
                                symbols,
                                types,
                                mode,
                                relation_mode,
                                visited,
                            )
                        } else {
                            type_id
                        }
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
                        module,
                        profile,
                        element_id,
                        symbols,
                        types,
                        mode,
                        relation_mode,
                        visited,
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
                    relation_mode,
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
                        module,
                        profile,
                        element,
                        symbols,
                        types,
                        mode,
                        relation_mode,
                        visited,
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
                        module,
                        profile,
                        field.ty,
                        symbols,
                        types,
                        mode,
                        relation_mode,
                        visited,
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
                    relation_mode,
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
                    relation_mode,
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
                        relation_mode,
                        visited,
                    );
                    let normalized_value = self.normalize_type_inner(
                        module,
                        profile,
                        signature.value_type,
                        symbols,
                        types,
                        mode,
                        relation_mode,
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
                    relation_mode,
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
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                // normalize the this parameter when present
                let normalized_this = match this_parameter {
                    Some(type_id) => {
                        let normalized = self.normalize_type_inner(
                            module,
                            profile,
                            type_id,
                            symbols,
                            types,
                            mode,
                            relation_mode,
                            visited,
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
                            module,
                            profile,
                            type_id,
                            symbols,
                            types,
                            mode,
                            relation_mode,
                            visited,
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
                relation_mode,
                visited,
            ),
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => self.normalize_mapped_type(
                module,
                profile,
                source_id,
                parameter,
                modifiers,
                value,
                symbols,
                types,
                mode,
                relation_mode,
                visited,
            ),
            Type::Index { left, index } => self.normalize_index_type(
                module,
                profile,
                source_id,
                left,
                index,
                symbols,
                types,
                mode,
                relation_mode,
                visited,
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
                    relation_mode,
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
                    types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        },
                        source_id,
                    )
                } else if !did_change {
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
                        module,
                        profile,
                        type_id,
                        symbols,
                        types,
                        mode,
                        relation_mode,
                        visited,
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
                        module,
                        profile,
                        type_id,
                        symbols,
                        types,
                        mode,
                        relation_mode,
                        visited,
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
                    module,
                    profile,
                    source_id,
                    Some(type_id),
                    right,
                    symbols,
                    types,
                    mode,
                    relation_mode,
                    visited,
                ),
                TypeUnaryOperator::Readonly | TypeUnaryOperator::AsConst => {
                    // materialize readonly modifiers during normalization
                    let normalized_right = self.normalize_type_inner(
                        module,
                        profile,
                        right,
                        symbols,
                        types,
                        mode,
                        relation_mode,
                        visited,
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
                        relation_mode,
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
                    relation_mode,
                    visited,
                );
                let right = self.normalize_type_inner(
                    module,
                    profile,
                    original_right,
                    symbols,
                    types,
                    mode,
                    relation_mode,
                    visited,
                );

                // reduce decidable type operators to boolean literals
                if let Some(normalized_id) = self.normalize_decidable_type_operator(
                    module,
                    profile,
                    source_id,
                    operator,
                    left,
                    right,
                    symbols,
                    types,
                    mode,
                    relation_mode,
                ) {
                    normalized_id
                } else if left == original_left && right == original_right {
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
                    relation_mode,
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
                    relation_mode,
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
                    relation_mode,
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
                    relation_mode,
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
        let dependencies = types.pop_normalization_dependency_scope();
        // cache the normalized result for reuse
        if relation_mode.is_cacheable() {
            let dependency_versions = types.collect_dependency_versions(dependencies);
            types.set_normalized_type(
                mode,
                relation_key,
                type_id,
                normalized_id,
                dependency_versions,
            );
        }
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
        relation_mode: RelationMode,
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

        // TODO #Cleanup: move this instantiation gate into the evaluation boundary once normalization is split
        // treat instantiation dependent checks as undecidable
        let left_needs_instantiation =
            self.type_needs_instantiation(module, profile, left, symbols, types);
        let right_needs_instantiation =
            self.type_needs_instantiation(module, profile, right, symbols, types);
        let is_decidable = !(left_needs_instantiation || right_needs_instantiation);
        let options = self.analyze_context_options_for_module(module.id);

        // compute assignability for operator semantics
        let assignability = if operator == TypeBinaryOperator::In {
            let mut key_visited = Vec::new();
            let key_type_id = self.normalize_keyof_type(
                module,
                profile,
                source_id,
                None,
                right,
                symbols,
                types,
                mode,
                relation_mode,
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
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> Option<LocalTypeId> {
        // normalize reference symbols to their declared type
        let symbol = self.normalize_reference_symbol_id(module, profile, symbol);

        // return cached normalization results when available
        let relation_key = relation_mode.cache_key();
        if relation_mode.is_cacheable()
            && let Some(entry) =
                types.normalized_alias_reference(symbol, mode, relation_key, arguments)
        {
            for (dependency_id, _) in &entry.dependency_versions.type_versions {
                types.record_normalization_dependency(*dependency_id);
            }
            for (dependency_id, _) in &entry.dependency_versions.symbol_versions {
                types.record_normalization_symbol_dependency(*dependency_id);
            }
            return Some(entry.normalized_type);
        }

        // report recursion when already resolving the same alias
        if types.is_normalization_alias_in_progress(symbol) {
            let alias_target_id = self.alias_target_type_id_for_symbol(
                module, profile, symbol, source_id, symbols, types,
            );
            let is_direct_self_reference = alias_target_id.is_some_and(|alias_target_id| {
                matches!(
                    types.get_type(alias_target_id),
                    Type::Reference { symbol: target_symbol, .. } if *target_symbol == symbol
                )
            });
            if is_direct_self_reference {
                return None;
            }

            let node = source_id
                .into_global(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::RecursiveTypeInstantiation { node });
            let error_id = types.insert_type_from_any(Type::Error, source_id);
            return Some(error_id);
        }
        types.mark_normalization_alias_in_progress(symbol);
        types.push_normalization_dependency_scope();

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
                    relation_mode,
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
                    relation_mode,
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
                relation_mode,
                visited,
            );
            Some(normalized_id)
        })();

        let dependencies = types.pop_normalization_dependency_scope();
        types.clear_normalization_alias_in_progress(symbol);
        if let Some(normalized_id) = normalized
            && relation_mode.is_cacheable()
        {
            let dependency_versions = types.collect_dependency_versions(dependencies);
            types.set_normalized_alias_reference(
                symbol,
                mode,
                relation_key,
                arguments.to_vec(),
                normalized_id,
                dependency_versions,
            );
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

        // resolve the apparent instance type through the normal require gate
        self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
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
        // prefer resolved instance arguments when no arguments are present
        if arguments.is_empty()
            && let Some(resolved) =
                self.resolved_static_arguments_for_reference(module, source_id, symbol, types)
        {
            return resolved;
        }

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
        // check for unevaluated targets or static arguments
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        if matches!(types.get_type(instance_type_id), Type::Unevaluated(_)) {
            return true;
        }
        if self.type_contains_unevaluated_static_arguments(
            instance_type_id,
            types,
            &mut HashSet::new(),
        ) {
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
        relation_mode: RelationMode,
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
            relation_mode,
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
        relation_mode: RelationMode,
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
                relation_mode,
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

        types.intern_union_type(filtered, type_id)
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
        relation_mode: RelationMode,
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
                relation_mode,
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

        types.intern_intersection_type(filtered, type_id)
    }

    /// Unwrap structural type aliases using cached instance types.
    pub(super) fn unwrap_normalization_alias_reference(
        &self,
        type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut current_id = type_id;
        let mut visited = Vec::new();
        while let Type::Reference {
            symbol,
            static_arguments,
        } = types.get_type(current_id)
        {
            let symbol = *symbol;
            let static_arguments = static_arguments.clone();

            // exit when this is not a type alias
            if symbol.ty() != SymbolType::TypeAlias {
                break;
            };
            // avoid unwrapping aliases with explicit static arguments
            if static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
            {
                break;
            }
            // exit on alias cycles
            if visited.contains(&symbol) {
                break;
            }
            visited.push(symbol);

            // exit when the alias has no instance type yet
            types.record_normalization_symbol_dependency(symbol);
            let Some(instance_id) = types.get_instance_type_id(symbol) else {
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
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
        did_change: &mut bool,
    ) -> Vec<LocalTypeId> {
        let mut normalized = Vec::with_capacity(type_ids.len());
        // normalize each element and track changes
        for type_id in type_ids {
            let normalized_id = self.normalize_type_inner(
                module,
                profile,
                *type_id,
                symbols,
                types,
                mode,
                relation_mode,
                visited,
            );
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
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_APPARENT);

        // skip apparent type expansion when the relation mode says so
        if !relation_mode.flags.use_apparent_type {
            return type_id;
        }

        // unwrap cached alias instances first
        let type_id = self.unwrap_normalization_alias_reference(type_id, types);

        // expand alias references with static arguments before apparent resolution
        let ty = types.get_type(type_id).clone();
        if let Type::Reference {
            symbol,
            static_arguments: Some(static_arguments),
        } = ty
            && matches!(symbol.ty(), SymbolType::TypeAlias)
        {
            let source_id = types.get_type_source(type_id);
            let mut visited = Vec::new();
            if let Some(expanded_id) = self.normalize_type_alias_reference_with_arguments(
                module,
                profile,
                source_id,
                symbol,
                &static_arguments,
                symbols,
                types,
                NormalizationMode::Assign,
                relation_mode,
                &mut visited,
            ) {
                return expanded_id;
            }
        }

        // prefer apparent instance types for references and type as value wrappers
        if let Some(symbol) = self.unwrap_type_value_symbol(types, type_id) {
            let source_id = types.get_type_source(type_id);
            if let Some(apparent_id) =
                self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
            {
                return apparent_id;
            }
        }

        type_id
    }

    /// Resolve the apparent type for assignability checks.
    pub(crate) fn apparent_type_for_assignability(
        &self,
        _module: &Module,
        _profile: ProfileId,
        type_id: LocalTypeId,
        _symbols: &SymbolTable,
        types: &mut TypeTable,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        // skip apparent type expansion when the relation mode says so
        if !relation_mode.flags.use_apparent_type {
            return type_id;
        }

        // unwrap cached alias instances only
        self.unwrap_normalization_alias_reference(type_id, types)
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
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        // normalize the input type first
        let type_id = self.normalize_type_with_relation(
            module,
            profile,
            type_id,
            symbols,
            types,
            mode,
            relation_mode,
        );

        // resolve apparent types after normalization
        let type_id = self.apparent_type_for_assignability(
            module,
            profile,
            type_id,
            symbols,
            types,
            relation_mode,
        );

        // normalize again after apparent type expansion
        self.normalize_type_with_relation(
            module,
            profile,
            type_id,
            symbols,
            types,
            mode,
            relation_mode,
        )
    }
}
