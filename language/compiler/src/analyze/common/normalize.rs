use std::collections::{HashMap, HashSet};

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NormalizationMode, StaticArgument,
    StaticParameterKind, Symbol, SymbolSpace, SymbolTable, SymbolType, Type, TypeElement,
    TypeField, TypeIndexSignature, TypeLiteral, TypeTable, TypeUnaryOperator, WellKnownSymbol,
};
use destack_workspace::{Module, ProfileId};

use super::{AnalyzeDependencyStage, CanonicalSymbolMode, RelationMode, TypeTablesContext};
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
        self.with_module_symbols_base_at_stage(
            module,
            profile,
            symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |owner_module, symbols| {
                let symbol_entry = symbols.get_symbol(symbol.local_id).clone();
                self.normalize_reference_symbol_id_with_symbols(
                    module,
                    profile,
                    owner_module,
                    symbol,
                    symbols,
                    symbol_entry,
                )
            },
        )
        .unwrap_or(symbol)
    }

    /// Normalize a reference symbol id using a symbol table snapshot.
    fn normalize_reference_symbol_id_with_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        owner_module: &Module,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        symbol_entry: Symbol,
    ) -> GlobalSymbolId {
        // normalize to the declared symbol type
        let normalized =
            GlobalSymbolId::new(owner_module.id, symbol.local_id.with_type(symbol_entry.ty));

        // preserve static parameter identity across symbol space normalization
        if symbol_entry.is_static_parameter() {
            return normalized;
        }

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

        // canonicalize candidates: normalize symbol typing, dedupe, then prefer local-module symbols
        let mut normalized_candidates = candidates
            .into_iter()
            .map(|candidate| {
                let candidate_module = self.program.modules.get(candidate.module_id);
                let candidate_module = candidate_module.read();
                let candidate_symbols = candidate_module.dir_base().symbols.read();
                let candidate_entry = candidate_symbols.get_symbol(candidate.local_id);
                GlobalSymbolId::new(
                    candidate.module_id,
                    candidate.local_id.with_type(candidate_entry.ty),
                )
            })
            .collect::<Vec<_>>();
        normalized_candidates.sort_unstable();
        normalized_candidates.dedup();
        normalized_candidates
            .sort_unstable_by_key(|candidate| (candidate.module_id != module.id, *candidate));
        if let Some(candidate) = normalized_candidates.into_iter().next() {
            return candidate;
        }

        normalized
    }

    /// Normalize a type id for the given mode.
    pub(crate) fn normalize_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
    ) -> LocalTypeId {
        self.normalize_type_with_relation(
            &mut tables.reborrow(),
            type_id,
            mode,
            RelationMode::ASSIGN,
        )
    }

    /// Normalize a type id for the given mode and relation.
    pub(crate) fn normalize_type_with_relation(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_NORMALIZE);

        let mut visited = Vec::new();
        self.normalize_type_inner(
            &mut tables.reborrow(),
            type_id,
            mode,
            relation_mode,
            &mut visited,
        )
    }

    /// Normalize a type for assignability checks.
    pub(crate) fn normalize_type_for_assignability(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // normalize the apparent return type first
        let normalized = self.normalize_apparent_type(
            &mut type_tables.reborrow(),
            type_id,
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
        } = type_tables.types.get_type(type_id).clone()
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
        let source_id = type_tables.types.get_type_source(type_id);
        let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(&mut type_tables.reborrow(), symbol, source_id)
        else {
            return normalized;
        };

        // resolve static arguments for substitution
        let resolved_arguments = self
            .resolve_type_reference_static_arguments(
                &mut type_tables.reborrow(),
                source_id,
                symbol,
                Some(arguments.as_slice()),
                false,
            )
            .ok()
            .flatten();
        let arguments = resolved_arguments.as_deref().unwrap_or(arguments);

        // substitute parameters into the alias target
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut type_tables.reborrow(),
            symbol,
            source_id,
            arguments,
        );
        if substitutions.is_empty() {
            return self.normalize_type(
                &mut type_tables.reborrow(),
                alias_target_id,
                NormalizationMode::Assign,
            );
        }

        // apply substitutions and normalize the result
        let mut cache = HashMap::new();
        let substituted = self.substitute_static_parameters(
            alias_target_id,
            &substitutions,
            type_tables.types,
            &mut cache,
        );
        self.normalize_type(
            &mut type_tables.reborrow(),
            substituted,
            NormalizationMode::Assign,
        )
    }

    /// Normalize a type id with a recursion guard.
    pub(crate) fn normalize_type_inner(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // NOTE #Suspicious: normalization caches only ASSIGN, but normalize_type_inner handles CONSTRAINT too
        // reuse cached normalization when available
        let relation_key = relation_mode.cache_key();
        if relation_mode.is_cacheable()
            && let Some(entry) = tables
                .types
                .get_normalized_type(mode, relation_key, type_id)
        {
            for (dependency_id, _) in &entry.dependency_versions.type_versions {
                tables.types.record_normalization_dependency(*dependency_id);
            }
            for (dependency_id, _) in &entry.dependency_versions.symbol_versions {
                tables
                    .types
                    .record_normalization_symbol_dependency(*dependency_id);
            }
            return entry.normalized_type;
        }

        // avoid infinite recursion on self referential tables.types
        if visited.contains(&type_id) {
            tables.types.record_normalization_dependency(type_id);
            return type_id;
        }

        // collect dependencies for this normalization pass
        tables.types.push_normalization_dependency_scope();
        tables.types.record_normalization_dependency(type_id);
        visited.push(type_id);

        // keep the source id for any normalized replacement
        let source_id = tables.types.get_type_source(type_id);
        let ty = tables.types.get_type(type_id).clone();

        // normalize based on structural shape
        let normalized_id = match ty {
            Type::Union { elements } => self.normalize_union_type(
                &mut tables.reborrow(),
                type_id,
                &elements,
                mode,
                relation_mode,
                visited,
            ),
            Type::Intersection { elements } => self.normalize_intersection_type(
                &mut tables.reborrow(),
                type_id,
                &elements,
                mode,
                relation_mode,
                visited,
            ),
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                // normalize reference tables.symbols to their declared type
                let normalized_symbol =
                    self.normalize_reference_symbol_id(tables.module, tables.profile, symbol);
                if normalized_symbol != symbol {
                    let normalized = Type::Reference {
                        symbol: normalized_symbol,
                        static_arguments,
                    };
                    let normalized_id = tables.types.insert_type_from_any(normalized, source_id);
                    self.normalize_type_inner(
                        &mut tables.reborrow(),
                        normalized_id,
                        mode,
                        relation_mode,
                        visited,
                    )
                } else {
                    let symbol = normalized_symbol;

                    // follow import targets while preserving alias identity
                    let symbol = self.canonical_symbol_id(
                        tables.module,
                        tables.symbols,
                        tables.profile,
                        symbol,
                        CanonicalSymbolMode::PreserveAliases,
                    );

                    // rewrite well known references to canonical shapes
                    if let Some(well_known) = self.well_known_array_kind(tables.profile, symbol)
                        && let Some(normalized) = self.normalize_well_known_type_reference(
                            tables.module,
                            tables.symbols,
                            tables.profile,
                            source_id,
                            symbol,
                            well_known,
                            static_arguments.as_deref(),
                            tables.types,
                        )
                    {
                        let normalized_id =
                            tables.types.insert_type_from_any(normalized, source_id);
                        self.normalize_type_inner(
                            &mut tables.reborrow(),
                            normalized_id,
                            mode,
                            relation_mode,
                            visited,
                        )
                    }
                    // expand type aliases with static arguments
                    else if symbol.ty() == SymbolType::TypeAlias {
                        let arguments = static_arguments.as_deref().unwrap_or(&[]);
                        let expanded = self.normalize_type_alias_reference_with_arguments(
                            &mut tables.reborrow(),
                            source_id,
                            symbol,
                            arguments,
                            mode,
                            relation_mode,
                            visited,
                        );
                        if let Some(expanded) = expanded {
                            self.normalize_type_inner(
                                &mut tables.reborrow(),
                                expanded,
                                mode,
                                relation_mode,
                                visited,
                            )
                        } else {
                            let unwrapped =
                                self.unwrap_normalization_alias_reference(type_id, tables.types);
                            if unwrapped != type_id {
                                self.normalize_type_inner(
                                    &mut tables.reborrow(),
                                    unwrapped,
                                    mode,
                                    relation_mode,
                                    visited,
                                )
                            } else {
                                type_id
                            }
                        }
                    } else {
                        let unwrapped =
                            self.unwrap_normalization_alias_reference(type_id, tables.types);
                        if unwrapped != type_id {
                            self.normalize_type_inner(
                                &mut tables.reborrow(),
                                unwrapped,
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
                        &mut tables.reborrow(),
                        element_id,
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
                    tables.types.insert_type_from_any(normalized, source_id)
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
                    &mut tables.reborrow(),
                    original_element,
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
                    tables.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                // normalize tuple element tables.types
                let mut normalized_elements = Vec::with_capacity(elements.len());
                let mut did_change = false;
                for element in elements {
                    let (normalized, element_changed) = self.normalize_tuple_element(
                        &mut tables.reborrow(),
                        element,
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
                    tables.types.insert_type_from_any(normalized, source_id)
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
                        &mut tables.reborrow(),
                        field.ty,
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
                    &mut tables.reborrow(),
                    &call_signatures,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                // normalize construct signatures
                let normalized_constructs = self.normalize_type_list(
                    &mut tables.reborrow(),
                    &construct_signatures,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );

                // normalize index signatures
                let mut normalized_indexes = Vec::with_capacity(index_signatures.len());
                for signature in index_signatures {
                    let normalized_key = self.normalize_type_inner(
                        &mut tables.reborrow(),
                        signature.key_type,
                        mode,
                        relation_mode,
                        visited,
                    );
                    let normalized_value = self.normalize_type_inner(
                        &mut tables.reborrow(),
                        signature.value_type,
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
                    tables.types.insert_type_from_any(normalized, source_id)
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
                    &mut tables.reborrow(),
                    &static_parameters,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                let normalized_dynamic = self.normalize_type_list(
                    &mut tables.reborrow(),
                    &dynamic_parameters,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                // normalize the this parameter when present
                let normalized_this = match this_parameter {
                    Some(type_id) => {
                        let normalized = self.normalize_type_inner(
                            &mut tables.reborrow(),
                            type_id,
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
                            &mut tables.reborrow(),
                            type_id,
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
                    tables.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => self.normalize_conditional_type(
                &mut tables.reborrow(),
                type_id,
                source_id,
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
                mode,
                relation_mode,
                visited,
            ),
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => self.normalize_mapped_type(
                &mut tables.reborrow(),
                source_id,
                parameter,
                modifiers,
                value,
                mode,
                relation_mode,
                visited,
            ),
            Type::Index { left, index } => self.normalize_index_type(
                &mut tables.reborrow(),
                source_id,
                left,
                index,
                mode,
                relation_mode,
                visited,
            ),
            Type::TemplateLiteral { strings, spans } => {
                // normalize template literal spans
                let mut did_change = false;
                let normalized_spans = self.normalize_type_list(
                    &mut tables.reborrow(),
                    &spans,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                // collapse template literals containing never
                if normalized_spans.iter().any(|span_id| {
                    matches!(
                        tables.types.get_type(*span_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        }
                    )
                }) {
                    tables.types.insert_type_from_any(
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
                    tables.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Unevaluated(_) => {
                // treat unevaluated tables.types as unknown during assignability normalization
                if matches!(mode, NormalizationMode::Assign) {
                    tables.types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        source_id,
                    )
                } else {
                    type_id
                }
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                let resolved = self.query_import_type_reference(
                    tables.module,
                    tables.profile,
                    source_id,
                    target,
                    qualifier.as_ref(),
                    static_arguments.as_deref(),
                    tables.types,
                );
                if let Some(resolved) = resolved {
                    self.normalize_type_inner(
                        &mut tables.reborrow(),
                        resolved,
                        mode,
                        relation_mode,
                        visited,
                    )
                } else {
                    type_id
                }
            }
            Type::Infer { name, constraint } => {
                // normalize the inference constraint when present
                let original_constraint = constraint;
                let constraint = original_constraint.map(|type_id| {
                    self.normalize_type_inner(
                        &mut tables.reborrow(),
                        type_id,
                        mode,
                        relation_mode,
                        visited,
                    )
                });

                if constraint == original_constraint {
                    type_id
                } else {
                    let normalized = Type::Infer { name, constraint };
                    tables.types.insert_type_from_any(normalized, source_id)
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
                        &mut tables.reborrow(),
                        type_id,
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
                    tables.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Unary { operator, right } => match operator {
                TypeUnaryOperator::Keyof => self.normalize_keyof_type(
                    &mut tables.reborrow(),
                    source_id,
                    Some(type_id),
                    right,
                    mode,
                    relation_mode,
                    visited,
                ),
                TypeUnaryOperator::Readonly => {
                    // materialize readonly modifiers during normalization
                    let normalized_right = self.normalize_type_inner(
                        &mut tables.reborrow(),
                        right,
                        mode,
                        relation_mode,
                        visited,
                    );
                    let deep_readonly = self
                        .analyze_context_options_for_module(tables.module.id)
                        .deep_readonly;
                    self.materialize_readonly_type(
                        source_id,
                        normalized_right,
                        tables.types,
                        deep_readonly,
                    )
                }
                TypeUnaryOperator::AsConst => {
                    // materialize const modifiers with deep readonly
                    let normalized_right = self.normalize_type_inner(
                        &mut tables.reborrow(),
                        right,
                        mode,
                        relation_mode,
                        visited,
                    );
                    self.materialize_readonly_type(source_id, normalized_right, tables.types, true)
                }
                _ => {
                    // normalize unary operand
                    let original_right = right;
                    let right = self.normalize_type_inner(
                        &mut tables.reborrow(),
                        original_right,
                        mode,
                        relation_mode,
                        visited,
                    );
                    if right == original_right {
                        type_id
                    } else {
                        let normalized = Type::Unary { operator, right };
                        tables.types.insert_type_from_any(normalized, source_id)
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
                    &mut tables.reborrow(),
                    original_left,
                    mode,
                    relation_mode,
                    visited,
                );
                let right = self.normalize_type_inner(
                    &mut tables.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );

                // reduce decidable type operators to boolean literals
                let normalized_id = self.normalize_decidable_type_operator(
                    &mut tables.reborrow(),
                    source_id,
                    operator,
                    left,
                    right,
                    mode,
                    relation_mode,
                );
                if let Some(normalized_id) = normalized_id {
                    normalized_id
                } else if left == original_left && right == original_right {
                    type_id
                } else {
                    let normalized = Type::Binary {
                        left,
                        operator,
                        right,
                    };
                    tables.types.insert_type_from_any(normalized, source_id)
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
                    &mut tables.reborrow(),
                    original_right,
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
                    tables.types.insert_type_from_any(normalized, source_id)
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
                    &mut tables.reborrow(),
                    original_right,
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
                    tables.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::PointerOf { mutability, right } => {
                // normalize pointer target
                let original_right = right;
                let right = self.normalize_type_inner(
                    &mut tables.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::PointerOf { mutability, right };
                    tables.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Value { value } => {
                // normalize type value target
                let original_value = value;
                let value = self.normalize_type_inner(
                    &mut tables.reborrow(),
                    original_value,
                    mode,
                    relation_mode,
                    visited,
                );
                if value == original_value {
                    type_id
                } else {
                    let normalized = Type::Value { value };
                    tables.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::TypeLiteral { .. } | Type::InferVar { .. } | Type::This | Type::Error => type_id,
        };

        // release the recursion guard for this type
        visited.pop();
        let dependencies = tables.types.pop_normalization_dependency_scope();
        // cache the normalized result for reuse
        if relation_mode.is_cacheable() {
            let dependency_versions = tables.types.collect_dependency_versions(dependencies);
            tables.types.set_normalized_type(
                mode,
                relation_key,
                type_id,
                normalized_id,
                dependency_versions,
            );
        }
        normalized_id
    }

    /// Normalize type alias references with static arguments.
    pub(crate) fn normalize_type_alias_reference_with_arguments(
        &self,
        tables: &mut TypeTablesContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        arguments: &[StaticArgument],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> Option<LocalTypeId> {
        // normalize reference symbols to their declared type
        let symbol = self.normalize_reference_symbol_id(tables.module, tables.profile, symbol);

        // return cached normalization results when available
        let relation_key = relation_mode.cache_key();
        if relation_mode.is_cacheable()
            && let Some(entry) =
                tables
                    .types
                    .get_normalized_alias_reference(symbol, mode, relation_key, arguments)
        {
            for (dependency_id, _) in &entry.dependency_versions.type_versions {
                tables.types.record_normalization_dependency(*dependency_id);
            }
            for (dependency_id, _) in &entry.dependency_versions.symbol_versions {
                tables
                    .types
                    .record_normalization_symbol_dependency(*dependency_id);
            }
            return Some(entry.normalized_type);
        }

        // report recursion when already resolving the same alias
        if tables.types.is_normalization_alias_in_progress(symbol) {
            let alias_target_id =
                self.alias_target_type_id_for_symbol(&mut tables.reborrow(), symbol, source_id);
            let is_direct_self_reference = alias_target_id.is_some_and(|alias_target_id| {
                matches!(
                    tables.types.get_type(alias_target_id),
                    Type::Reference { symbol: target_symbol, .. } if *target_symbol == symbol
                )
            });
            if is_direct_self_reference {
                return None;
            }

            let node = source_id
                .into_global(tables.module.id)
                .into_anchored(Some(tables.profile));
            self.error(AnalyzeError::RecursiveTypeInstantiation { node });
            let error_id = tables.types.insert_type_from_any(Type::Error, source_id);
            return Some(error_id);
        }
        tables.types.mark_normalization_alias_in_progress(symbol);
        tables.types.push_normalization_dependency_scope();

        let normalized = {
            // ensure remote declarations are ready before reading instance types
            if symbol.module_id != tables.module.id {
                let _ = self.require_analyze_module_declare(symbol.module_id, tables.profile);
            }

            // resolve the instance type, including alias targets and remote imports
            let instance_type_id =
                self.instance_type_id_for_normalization(&mut tables.reborrow(), symbol, source_id)?;

            // materialize unevaluated alias targets before normalization
            let materialized_instance = self.materialize_alias_instance_for_normalization(
                &mut tables.reborrow(),
                symbol,
                instance_type_id,
            );

            // select the static arguments to substitute
            let resolved_arguments = self.resolved_static_arguments_for_normalization(
                &mut tables.reborrow(),
                source_id,
                symbol,
                arguments,
            );

            if resolved_arguments.is_empty() {
                Some(self.normalize_type_inner(
                    &mut tables.reborrow(),
                    materialized_instance,
                    mode,
                    relation_mode,
                    visited,
                ))
            } else {
                // build type parameter substitutions
                let substitutions = self.build_type_parameter_substitutions_for_symbol(
                    &mut tables.reborrow(),
                    symbol,
                    source_id,
                    &resolved_arguments,
                );
                if substitutions.is_empty() {
                    // normalize the instance type (even when no substitutions are available)
                    Some(self.normalize_type_inner(
                        &mut tables.reborrow(),
                        materialized_instance,
                        mode,
                        relation_mode,
                        visited,
                    ))
                } else {
                    // substitute and normalize
                    let mut cache = HashMap::new();
                    let substituted = self.substitute_static_parameters(
                        materialized_instance,
                        &substitutions,
                        tables.types,
                        &mut cache,
                    );
                    let normalized_id = self.normalize_type_inner(
                        &mut tables.reborrow(),
                        substituted,
                        mode,
                        relation_mode,
                        visited,
                    );
                    Some(normalized_id)
                }
            }
        };

        let dependencies = tables.types.pop_normalization_dependency_scope();
        tables.types.clear_normalization_alias_in_progress(symbol);
        if let Some(normalized_id) = normalized
            && relation_mode.is_cacheable()
        {
            let dependency_versions = tables.types.collect_dependency_versions(dependencies);
            tables.types.set_normalized_alias_reference(
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

    /// Resolve the instance type id used for alias normalization.
    fn instance_type_id_for_normalization(
        &self,
        tables: &mut TypeTablesContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> Option<LocalTypeId> {
        // fetch or import the alias target type when available
        if let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(&mut tables.reborrow(), symbol, source_id)
        {
            return Some(alias_target_id);
        }

        // resolve the apparent instance type through the normal require gate
        self.apparent_instance_type(&mut tables.reborrow(), source_id, symbol)
    }

    /// Resolve the static arguments used for alias normalization.
    fn resolved_static_arguments_for_normalization(
        &self,
        tables: &mut TypeTablesContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        arguments: &[StaticArgument],
    ) -> Vec<StaticArgument> {
        // prefer resolved instance arguments when no arguments are present
        if arguments.is_empty()
            && let Some(resolved) = self.query_instance_arguments_for_node(
                source_id.into_global(tables.module.id),
                Some(symbol),
                tables.types,
            )
        {
            return resolved;
        }

        // prefer resolved instance arguments when unevaluated arguments are present
        if arguments
            .iter()
            .any(|argument| matches!(argument, StaticArgument::Unevaluated { .. }))
        {
            // resolve unevaluated arguments using the full reference resolver
            if let Ok(Some(resolved)) = self.resolve_type_reference_static_arguments(
                &mut tables.reborrow(),
                source_id,
                symbol,
                Some(arguments),
                false,
            ) {
                return resolved;
            }

            // fall back to local materialization when resolution is incomplete
            let resolved = self.materialize_static_arguments_for_reference(
                &mut tables.reborrow(),
                symbol,
                source_id,
                arguments,
            );
            if resolved != arguments {
                return resolved;
            }

            return self
                .query_instance_arguments_for_node(
                    source_id.into_global(tables.module.id),
                    Some(symbol),
                    tables.types,
                )
                .unwrap_or_else(|| arguments.to_vec());
        }

        arguments.to_vec()
    }

    /// Materialize alias targets before normalization when needed.
    fn materialize_alias_instance_for_normalization(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        symbol: GlobalSymbolId,
        instance_type_id: LocalTypeId,
    ) -> LocalTypeId {
        if symbol.module_id == type_tables.module.id {
            // skip materialization when the alias instance is already stable
            if !self.alias_instance_needs_materialization(
                type_tables.module,
                type_tables.profile,
                symbol,
                instance_type_id,
                type_tables.types,
            ) {
                return instance_type_id;
            }

            // materialize static arguments using the alias module context
            let mut materialize_cache = HashMap::new();
            self.materialize_static_arguments_in_type(
                &mut type_tables.reborrow(),
                instance_type_id,
                &mut materialize_cache,
            )
        } else {
            self.with_module_tree_symbols_at_stage(
                type_tables.module,
                type_tables.profile,
                symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |owner_module, tree, symbols| {
                    // skip materialization when the alias instance is already stable
                    if !self.alias_instance_needs_materialization(
                        owner_module,
                        type_tables.profile,
                        symbol,
                        instance_type_id,
                        type_tables.types,
                    ) {
                        return instance_type_id;
                    }

                    // materialize static arguments using the alias module context
                    let mut materialize_cache = HashMap::new();
                    let options = self.analyze_context_options_for_module(owner_module.id);
                    let mut type_tables = type_tables.reborrow_for_module_with_options(
                        owner_module,
                        &options,
                        tree,
                        symbols,
                    );
                    self.materialize_static_arguments_in_type(
                        &mut type_tables,
                        instance_type_id,
                        &mut materialize_cache,
                    )
                },
            )
            .unwrap_or(instance_type_id)
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
        // check for unresolved static-evaluation convergence state
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        if self.type_requires_static_evaluation_convergence(
            module,
            profile,
            instance_type_id,
            &symbols,
            types,
        ) {
            return true;
        }

        // skip value materialization when the alias has no value parameters
        let Some(parameters) =
            self.collect_static_parameter_symbols(module, symbol, profile, &tree, &symbols, types)
        else {
            return false;
        };
        let has_value_parameters = parameters.iter().any(|parameter_symbol| {
            self.static_parameter_metadata_for_symbol_in_module(*parameter_symbol, &tree, &symbols)
                .0
                == StaticParameterKind::Value
        });
        if !has_value_parameters {
            return false;
        }

        self.type_has_unevaluated_value_static_arguments(
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
        tables: &mut TypeTablesContext<'_>,
        element: TypeElement,
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
        } = tables.types.get_type(element_ty_id)
        {
            element_ty_id = *right;
            element_is_readonly = true;
            did_change = true;
        }

        // normalize the tuple element type
        let normalized_ty = self.normalize_type_inner(
            &mut tables.reborrow(),
            element_ty_id,
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
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        elements: &[LocalTypeId],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // prepare union element collection
        let mut flattened = Vec::new();

        // normalize and collect union elements
        for element_id in elements {
            let normalized = self.normalize_type_inner(
                &mut tables.reborrow(),
                *element_id,
                mode,
                relation_mode,
                visited,
            );
            // flatten nested unions
            match tables.types.get_type(normalized) {
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
            match tables.types.get_type(element_id) {
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
                tables.types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    tables.types.get_type_source(type_id),
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

        tables.types.intern_union_type(filtered, type_id)
    }

    /// Normalize intersection types by flattening and collapsing special cases.
    fn normalize_intersection_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        elements: &[LocalTypeId],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // prepare intersection element collection
        let mut flattened = Vec::new();

        // normalize and collect intersection elements
        for element_id in elements {
            let normalized = self.normalize_type_inner(
                &mut tables.reborrow(),
                *element_id,
                mode,
                relation_mode,
                visited,
            );
            // flatten nested intersections
            match tables.types.get_type(normalized) {
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
            match tables.types.get_type(element_id) {
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
        let source_id = tables.types.get_type_source(type_id);
        let mut empty_object_id = None;
        let mut remaining = Vec::new();
        for element_id in filtered {
            let is_empty_object = match tables.types.get_type(element_id) {
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
                match tables.types.get_type(element_id) {
                    Type::TypeLiteral {
                        value: TypeLiteral::Null | TypeLiteral::Undefined,
                    } => {
                        return tables.types.insert_type_from_any(
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
                            match tables.types.get_type(*union_id) {
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
                            return tables.types.insert_type_from_any(
                                Type::TypeLiteral {
                                    value: TypeLiteral::Never,
                                },
                                source_id,
                            );
                        }

                        if non_nullish.len() == 1 {
                            stripped.push(non_nullish[0]);
                        } else {
                            stripped.push(tables.types.insert_type_from_any(
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
                tables.types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    tables.types.get_type_source(type_id),
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

        tables.types.intern_intersection_type(filtered, type_id)
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
        tables: &mut TypeTablesContext<'_>,
        type_ids: &[LocalTypeId],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
        did_change: &mut bool,
    ) -> Vec<LocalTypeId> {
        let mut normalized = Vec::with_capacity(type_ids.len());
        // normalize each element and track changes
        for type_id in type_ids {
            let normalized_id = self.normalize_type_inner(
                &mut tables.reborrow(),
                *type_id,
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
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_APPARENT);

        // skip apparent type expansion when the relation mode says so
        if !relation_mode.flags.use_apparent_type {
            return type_id;
        }

        // unwrap cached alias instances first
        let type_id = self.unwrap_normalization_alias_reference(type_id, tables.types);

        // expand alias references with static arguments before apparent resolution
        let ty = tables.types.get_type(type_id).clone();
        if let Type::Reference {
            symbol,
            static_arguments: Some(static_arguments),
        } = ty
            && matches!(symbol.ty(), SymbolType::TypeAlias)
        {
            let source_id = tables.types.get_type_source(type_id);
            let mut visited = Vec::new();
            if let Some(expanded_id) = self.normalize_type_alias_reference_with_arguments(
                &mut tables.reborrow(),
                source_id,
                symbol,
                &static_arguments,
                NormalizationMode::Assign,
                relation_mode,
                &mut visited,
            ) {
                return expanded_id;
            }
        }

        // prefer apparent instance types for references and type as value wrappers
        if let Some(symbol) = self.unwrap_type_value_symbol(tables.types, type_id) {
            let source_id = tables.types.get_type_source(type_id);
            if let Some(apparent_id) =
                self.apparent_instance_type(&mut tables.reborrow(), source_id, symbol)
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
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        // normalize the input type first
        let type_id = {
            let mut normalize_visited = Vec::new();
            self.normalize_type_inner(
                &mut tables.reborrow(),
                type_id,
                mode,
                relation_mode,
                &mut normalize_visited,
            )
        };

        // resolve apparent types after normalization
        let type_id = self.apparent_type_for_assignability(
            tables.module,
            tables.profile,
            type_id,
            tables.symbols,
            tables.types,
            relation_mode,
        );

        // normalize again after apparent type expansion
        let mut normalize_visited = Vec::new();
        self.normalize_type_inner(
            &mut tables.reborrow(),
            type_id,
            mode,
            relation_mode,
            &mut normalize_visited,
        )
    }
}
