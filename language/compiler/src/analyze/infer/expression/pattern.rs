use std::collections::HashMap;

use super::member::MemberLookupMode;

use crate::analyze::common::CanonicalSymbolMode;
use crate::{AnalyzeResult, Compiler, InferContext};
use destack_dir::{
    Declaration, Expression, GlobalSymbolId, InferTable, LocalNodeId, LocalTypeId, NodeTree,
    NormalizationMode, Pattern, PatternField, ScalarLiteral, StaticKey, SymbolTable, SymbolType,
    Type, TypeElement, TypeKind, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Literal coverage for tuple pattern discriminant filtering.
#[derive(Debug)]
enum TuplePatternLiteralCoverage {
    /// The pattern covers every literal value.
    All,
    /// The pattern covers a concrete set of literal values.
    Values(Vec<ScalarLiteral>),
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer a dependency item.
    pub(crate) fn infer_pattern(
        &self,
        module: &Module,
        pattern_id: LocalNodeId<Pattern>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let pattern = tree.get(pattern_id);
        match pattern {
            Pattern::Wildcard => {
                // nothing to do
            }
            Pattern::Must(pattern_id) => {
                let binding_ty_id = binding_ty_id.and_then(|binding_ty_id| {
                    let (non_nullish, _) = self.strip_nullish_from_union(binding_ty_id, types);
                    non_nullish.or(Some(binding_ty_id))
                });
                self.infer_pattern(
                    module,
                    *pattern_id,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::ReferenceOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(
                    module,
                    *right,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::ValueOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(
                    module,
                    *right,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Binding {
                mutability: _,
                name: _,
                symbol,
                pattern,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    let binding_symbol = symbol.into_global(module.id);
                    types.set_value_type(binding_symbol, ty_id);

                    // mirror onto the canonical symbol to avoid lookup misses
                    let canonical_symbol = self.canonical_symbol_id(
                        module,
                        symbols,
                        ctx.profile,
                        binding_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    if canonical_symbol != binding_symbol {
                        types.set_value_type(canonical_symbol, ty_id);
                    }
                }
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Expression { value } => {
                let value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                // ensure the pattern expression is compatible with the binding type
                if let Some(binding_ty_id) = binding_ty_id {
                    let assignable = self.is_type_assignable(
                        module,
                        ctx.profile,
                        symbols,
                        binding_ty_id,
                        value_ty_id,
                        types,
                        &ctx.options,
                    );
                    if !assignable.is_assignable() {
                        let _reported = self.report_unassignable_type_for_types(
                            module,
                            ctx.profile,
                            value.into_any(),
                            binding_ty_id,
                            value_ty_id,
                            types,
                        );
                    }
                }
            }
            Pattern::Tuple { fields } => {
                self.infer_pattern_sequence(
                    module,
                    fields,
                    binding_ty_id,
                    |rest_types| Type::Tuple {
                        elements: rest_types.into_iter().map(TypeElement::new).collect(),
                        is_readonly: false,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::TaggedTuple { ty, fields } => {
                // prefer union variants from the binding type when available
                let mut ty_id =
                    self.evaluate_pattern_tag_type(module, *ty, tree, symbols, types, ctx)?;
                if let Some(binding_ty_id) = binding_ty_id
                    && let Some(union_ty_id) =
                        self.select_union_variant_for_tagged_pattern(types, binding_ty_id, ty_id)
                {
                    ty_id = union_ty_id;
                }
                // handle scalar tagged patterns like `UserId(value)`
                if fields.len() == 1 {
                    let field = tree.get(fields[0]);
                    if let PatternField::Positional { pattern, default } = field {
                        self.infer_pattern(
                            module,
                            *pattern,
                            Some(ty_id),
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                        if let Some(default) = default {
                            self.infer_expression(
                                module, *default, tree, symbols, types, infer, ctx,
                            )?;
                        }
                        return Ok(());
                    }
                }

                self.infer_pattern_sequence(
                    module,
                    fields,
                    Some(ty_id),
                    |rest_types| Type::Tuple {
                        elements: rest_types.into_iter().map(TypeElement::new).collect(),
                        is_readonly: false,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Array { fields } => {
                self.infer_pattern_sequence(
                    module,
                    fields,
                    binding_ty_id,
                    |rest_types| Type::Array {
                        element: rest_types.first().cloned(),
                        is_readonly: false,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Object { fields } => {
                // reject bare object patterns against nominal object values
                if let Some(binding_ty_id) = binding_ty_id
                    && self.is_nominal_object_pattern_target(
                        module,
                        ctx.profile,
                        pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                    )?
                {
                    let object_ty_id = types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Object,
                        },
                        pattern_id.into_any(),
                    );
                    let _reported = self.report_unassignable_type_for_types(
                        module,
                        ctx.profile,
                        pattern_id.into_any(),
                        binding_ty_id,
                        object_ty_id,
                        types,
                    );
                }

                for field_id in fields {
                    self.infer_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::TaggedObject { ty, fields } => {
                // prefer union variants from the binding type when available
                let mut ty_id =
                    self.evaluate_pattern_tag_type(module, *ty, tree, symbols, types, ctx)?;
                if let Some(binding_ty_id) = binding_ty_id
                    && let Some(union_ty_id) =
                        self.select_union_variant_for_tagged_pattern(types, binding_ty_id, ty_id)
                {
                    ty_id = union_ty_id;
                }
                for field_id in fields {
                    self.infer_pattern_field(
                        module,
                        *field_id,
                        Some(ty_id),
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Narrow a match pattern binding type using union member compatibility.
    pub(crate) fn narrow_match_pattern_binding_type(
        &self,
        module: &Module,
        profile: ProfileId,
        binding_ty_id: LocalTypeId,
        pattern_id: LocalNodeId<Pattern>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // keep the original type when we cannot narrow it
        let original_ty_id = binding_ty_id;

        // try both flow and assign normalization modes to preserve compatibility
        let normalized_flow_ty_id = self.normalize_type(
            module,
            profile,
            binding_ty_id,
            symbols,
            types,
            NormalizationMode::Flow,
        );
        let normalized_assign_ty_id = self.normalize_type(
            module,
            profile,
            binding_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
        );

        // narrow against union members using the match pattern shape
        let narrowed_members = self
            .pattern_filter_union_members_for_match_pattern(
                module,
                profile,
                pattern_id,
                normalized_flow_ty_id,
                tree,
                symbols,
                types,
            )
            .or_else(|| {
                self.pattern_filter_union_members_for_match_pattern(
                    module,
                    profile,
                    pattern_id,
                    normalized_assign_ty_id,
                    tree,
                    symbols,
                    types,
                )
            });

        // return the original binding type when no narrowing candidates exist
        let Some(narrowed_members) = narrowed_members else {
            return original_ty_id;
        };
        if narrowed_members.is_empty() {
            return original_ty_id;
        }

        // return a single narrowed member directly when possible
        if narrowed_members.len() == 1 {
            return narrowed_members[0];
        }

        // materialize the narrowed union for downstream pattern inference
        self.union_type_from_list(narrowed_members, original_ty_id, types)
    }

    /// Filter union members that are compatible with a match pattern.
    fn pattern_filter_union_members_for_match_pattern(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        binding_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        // only union-like bindings can be narrowed by member filtering
        let union_members = self.pattern_union_member_types_for_binding(binding_ty_id, types)?;
        if union_members.is_empty() {
            return None;
        }

        // keep union members whose shape is compatible with the pattern
        let mut narrowed_members = Vec::new();
        for member_ty_id in union_members {
            if self.pattern_matches_type_for_narrowing(
                module,
                profile,
                pattern_id,
                member_ty_id,
                tree,
                symbols,
                types,
            ) {
                narrowed_members.push(member_ty_id);
            }
        }

        Some(narrowed_members)
    }

    /// Resolve union member types for a binding when available.
    fn pattern_union_member_types_for_binding(
        &self,
        binding_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        let binding_ty = types.get_type(binding_ty_id).clone();
        match binding_ty {
            // return union members directly
            Type::Union { elements } => Some(elements),

            // unwrap type-as-value wrappers
            Type::Value { value } => self.pattern_union_member_types_for_binding(value, types),

            // unwrap references to their declared value types
            Type::Reference { symbol, .. } => types
                .get_value_type_id(symbol)
                .and_then(|type_id| self.pattern_union_member_types_for_binding(type_id, types)),

            _ => None,
        }
    }

    /// Check whether a pattern can match a specific candidate type.
    fn pattern_matches_type_for_narrowing(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        candidate_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let pattern = tree.get(pattern_id);
        match pattern {
            // wildcard and binding patterns accept every candidate
            Pattern::Wildcard | Pattern::Binding { .. } => true,

            // wrappers preserve the inner pattern compatibility
            Pattern::Must(pattern_id)
            | Pattern::ReferenceOf {
                mutability: _,
                right: pattern_id,
            }
            | Pattern::ValueOf {
                mutability: _,
                right: pattern_id,
            } => self.pattern_matches_type_for_narrowing(
                module,
                profile,
                *pattern_id,
                candidate_ty_id,
                tree,
                symbols,
                types,
            ),

            // expression patterns narrow by literal or reference symbol identity
            Pattern::Expression { value } => {
                if let Some(literal) = self.pattern_scalar_literal_for_expression(*value, tree) {
                    self.pattern_type_contains_scalar_literal(candidate_ty_id, &literal, types)
                } else if let Some(pattern_symbol) =
                    self.reference_symbol_for_expression(module, *value, profile, tree, symbols)
                {
                    self.pattern_type_contains_reference_symbol(
                        candidate_ty_id,
                        pattern_symbol,
                        types,
                    )
                } else {
                    true
                }
            }

            // union patterns match when any branch matches
            Pattern::Union { patterns } => patterns.iter().any(|pattern_id| {
                self.pattern_matches_type_for_narrowing(
                    module,
                    profile,
                    *pattern_id,
                    candidate_ty_id,
                    tree,
                    symbols,
                    types,
                )
            }),

            // tuple patterns narrow tuple-like candidate types
            Pattern::Tuple { fields } => self.pattern_tuple_fields_match_type_for_narrowing(
                module,
                profile,
                fields,
                candidate_ty_id,
                tree,
                symbols,
                types,
            ),

            // object patterns narrow object-like candidate types by field patterns
            Pattern::Object { fields } => self.pattern_object_fields_match_type_for_narrowing(
                module,
                profile,
                fields,
                candidate_ty_id,
                tree,
                symbols,
                types,
            ),

            // tagged patterns are handled by downstream tagged pattern inference
            Pattern::TaggedTuple { .. } | Pattern::TaggedObject { .. } => true,

            // keep unsupported pattern kinds conservative for narrowing
            Pattern::Array { .. } => true,
        }
    }

    /// Check whether a candidate type contains a scalar literal.
    fn pattern_type_contains_scalar_literal(
        &self,
        candidate_ty_id: LocalTypeId,
        literal: &ScalarLiteral,
        types: &mut TypeTable,
    ) -> bool {
        let candidate_ty_id = types.unwrap_value_type_id(candidate_ty_id);
        let candidate_ty = types.get_type(candidate_ty_id).clone();
        match candidate_ty {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(candidate_literal),
            } => candidate_literal == *literal,
            Type::Union { elements } => elements.iter().any(|element_ty_id| {
                self.pattern_type_contains_scalar_literal(*element_ty_id, literal, types)
            }),
            Type::Value { value } => {
                self.pattern_type_contains_scalar_literal(value, literal, types)
            }
            Type::Reference { symbol, .. } => {
                types.get_value_type_id(symbol).is_some_and(|type_id| {
                    self.pattern_type_contains_scalar_literal(type_id, literal, types)
                })
            }
            _ => false,
        }
    }

    /// Check whether a candidate type contains a reference symbol.
    fn pattern_type_contains_reference_symbol(
        &self,
        candidate_ty_id: LocalTypeId,
        expected_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> bool {
        let candidate_ty_id = types.unwrap_value_type_id(candidate_ty_id);
        let candidate_ty = types.get_type(candidate_ty_id).clone();
        match candidate_ty {
            Type::Reference { symbol, .. } => {
                symbol == expected_symbol
                    || types.get_value_type_id(symbol).is_some_and(|type_id| {
                        self.pattern_type_contains_reference_symbol(type_id, expected_symbol, types)
                    })
            }
            Type::Union { elements } => elements.iter().any(|element_ty_id| {
                self.pattern_type_contains_reference_symbol(*element_ty_id, expected_symbol, types)
            }),
            Type::Value { value } => {
                self.pattern_type_contains_reference_symbol(value, expected_symbol, types)
            }
            _ => false,
        }
    }

    /// Check tuple pattern compatibility for a candidate type.
    fn pattern_tuple_fields_match_type_for_narrowing(
        &self,
        module: &Module,
        profile: ProfileId,
        fields: &[LocalNodeId<PatternField>],
        candidate_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let Some(element_types) =
            self.pattern_fixed_tuple_element_types_for_narrowing(candidate_ty_id, types)
        else {
            return false;
        };

        let mut tuple_index = 0;
        for field_id in fields {
            let field = tree.get(*field_id);
            match field {
                PatternField::Positional { pattern, .. } => {
                    let Some(field_ty_id) = element_types.get(tuple_index).copied() else {
                        return false;
                    };
                    if !self.pattern_matches_type_for_narrowing(
                        module,
                        profile,
                        *pattern,
                        field_ty_id,
                        tree,
                        symbols,
                        types,
                    ) {
                        return false;
                    }
                    tuple_index += 1;
                }
                PatternField::Elision => {
                    tuple_index += 1;
                }
                PatternField::Spread { .. }
                | PatternField::Named { .. }
                | PatternField::Alias { .. }
                | PatternField::Computed { .. } => {
                    return true;
                }
            }
        }

        true
    }

    /// Resolve fixed tuple element types for narrowing checks.
    fn pattern_fixed_tuple_element_types_for_narrowing(
        &self,
        candidate_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        let candidate_ty_id = types.unwrap_value_type_id(candidate_ty_id);
        let candidate_ty = types.get_type(candidate_ty_id).clone();
        match candidate_ty {
            Type::Tuple { elements, .. } => {
                Some(elements.iter().map(|element| element.ty).collect())
            }
            Type::Reference { symbol, .. } => types.get_value_type_id(symbol).and_then(|type_id| {
                self.pattern_fixed_tuple_element_types_for_narrowing(type_id, types)
            }),
            Type::Value { value } => {
                self.pattern_fixed_tuple_element_types_for_narrowing(value, types)
            }
            _ => None,
        }
    }

    /// Check object pattern compatibility for a candidate type.
    fn pattern_object_fields_match_type_for_narrowing(
        &self,
        module: &Module,
        profile: ProfileId,
        fields: &[LocalNodeId<PatternField>],
        candidate_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        for field_id in fields {
            let field = tree.get(*field_id);
            match field {
                PatternField::Named {
                    name,
                    pattern,
                    default: _,
                    mutability: _,
                } => {
                    let Some(field_ty_id) = self.pattern_member_type_for_narrowing(
                        module,
                        profile,
                        *field_id,
                        candidate_ty_id,
                        StaticKey::Name(*name),
                        symbols,
                        types,
                    ) else {
                        return false;
                    };
                    if let Some(pattern_id) = pattern
                        && !self.pattern_matches_type_for_narrowing(
                            module,
                            profile,
                            *pattern_id,
                            field_ty_id,
                            tree,
                            symbols,
                            types,
                        )
                    {
                        return false;
                    }
                }
                PatternField::Alias {
                    name,
                    alias: _,
                    default: _,
                    mutability: _,
                    symbol: _,
                } => {
                    if self
                        .pattern_member_type_for_narrowing(
                            module,
                            profile,
                            *field_id,
                            candidate_ty_id,
                            StaticKey::Name(*name),
                            symbols,
                            types,
                        )
                        .is_none()
                    {
                        return false;
                    }
                }
                PatternField::Computed { .. } | PatternField::Spread { .. } => {
                    return true;
                }
                PatternField::Positional { .. } | PatternField::Elision => {
                    return false;
                }
            }
        }

        true
    }

    /// Resolve a member type for object pattern narrowing.
    fn pattern_member_type_for_narrowing(
        &self,
        module: &Module,
        profile: ProfileId,
        field_id: LocalNodeId<PatternField>,
        candidate_ty_id: LocalTypeId,
        key: StaticKey,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let receiver_ty = types.get_type(candidate_ty_id).clone();
        let mut visited = Vec::new();
        self.infer_member_of_type(
            module,
            profile,
            field_id.into_any(),
            symbols,
            &receiver_ty,
            &key,
            MemberLookupMode::Any,
            types,
            &mut visited,
        )
        .ok()
        .flatten()
    }

    /// Select a union variant for a tagged pattern to preserve static arguments.
    fn select_union_variant_for_tagged_pattern(
        &self,
        types: &TypeTable,
        binding_ty_id: LocalTypeId,
        tag_ty_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        let tag_symbol = self.unwrap_type_value_symbol(types, tag_ty_id)?;

        // unwrap value wrappers before inspecting the binding union
        let binding_ty_id = types.unwrap_value_type_id(binding_ty_id);
        let Type::Union { elements } = types.get_type(binding_ty_id) else {
            return None;
        };

        for element_id in elements {
            if self
                .unwrap_type_value_symbol(types, *element_id)
                .is_some_and(|symbol| symbol == tag_symbol)
            {
                return Some(*element_id);
            }
        }

        None
    }

    /// Check whether a binding type is a nominal object for untagged patterns.
    fn is_nominal_object_pattern_target(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        binding_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<bool> {
        let binding_ty = types.get_type(binding_ty_id);
        if self.is_definitely_struct_type(binding_ty) {
            return Ok(true);
        }

        let Type::Reference { symbol, .. } = binding_ty else {
            return Ok(false);
        };
        if symbol.ty() != SymbolType::Newtype {
            return Ok(false);
        }

        // resolve the underlying newtype target to determine object shape
        let Some(target_ty_id) = self.alias_target_type_id_for_symbol(
            module,
            profile,
            *symbol,
            pattern_id.into_any(),
            symbols,
            types,
        ) else {
            return Ok(false);
        };
        let target_ty_id =
            self.ensure_type_evaluated(module, profile, target_ty_id, tree, symbols, types)?;

        let target_ty = types.get_type(target_ty_id);
        let is_object = matches!(target_ty, Type::Object { .. });
        let is_struct_ref = matches!(
            target_ty,
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Struct
        );
        Ok(is_object || is_struct_ref)
    }

    /// Infer a sequence of pattern fields (with spread syntax support).
    pub(crate) fn infer_pattern_sequence(
        &self,
        module: &Module,
        fields: &Vec<LocalNodeId<PatternField>>,
        binding_ty_id: Option<LocalTypeId>,
        to_rest_type: impl Fn(Vec<LocalTypeId>) -> Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // use tuple union members directly when all fields are positional or elided
        if let Some(binding_ty_id) = binding_ty_id
            && fields.iter().all(|field_id| {
                matches!(
                    tree.get(*field_id),
                    PatternField::Positional { .. } | PatternField::Elision
                )
            })
        {
            let normalized_ty_id = self.normalize_type(
                module,
                ctx.profile,
                binding_ty_id,
                symbols,
                types,
                NormalizationMode::Flow,
            );
            let normalized_assignability_ty_id = self.normalize_type(
                module,
                ctx.profile,
                binding_ty_id,
                symbols,
                types,
                NormalizationMode::Assign,
            );
            let member_elements = self
                .pattern_union_tuple_member_elements(normalized_ty_id, types)
                .or_else(|| {
                    self.pattern_union_tuple_member_elements(normalized_assignability_ty_id, types)
                });
            if let Some(member_elements) = member_elements {
                let mut candidate_indexes: Vec<usize> = (0..member_elements.len()).collect();
                let mut tuple_index = 0;
                for field_id in fields {
                    let field = tree.get(*field_id);
                    let field_ty = match field {
                        PatternField::Positional { pattern, .. } => {
                            let field_ty = self.pattern_union_slot_type_for_candidates(
                                &member_elements,
                                &candidate_indexes,
                                tuple_index,
                                *field_id,
                                types,
                            );

                            if let Some(coverage) =
                                self.pattern_literal_coverage_for_tuple_pattern(*pattern, tree)
                            {
                                self.pattern_filter_tuple_union_candidates(
                                    &member_elements,
                                    &mut candidate_indexes,
                                    tuple_index,
                                    &coverage,
                                    types,
                                );
                            }

                            tuple_index += 1;
                            field_ty
                        }
                        PatternField::Elision => {
                            tuple_index += 1;
                            None
                        }
                        _ => None,
                    };
                    self.infer_pattern_field(
                        module, *field_id, field_ty, tree, symbols, types, infer, ctx,
                    )?;
                }

                return Ok(());
            }
        }

        // map the binding type into sequence-friendly pieces
        let (binding_ty_fields, binding_array_element) = binding_ty_id
            .map(|ty_id| {
                let normalized_ty_id = self.normalize_type(
                    module,
                    ctx.profile,
                    ty_id,
                    symbols,
                    types,
                    NormalizationMode::Flow,
                );
                self.pattern_sequence_binding_types(normalized_ty_id, types)
            })
            .unwrap_or_else(|| (Vec::new(), None));

        // allow a direct fallback only for a single field on non-sequence types
        let allow_direct_binding_fallback = binding_ty_id.is_some()
            && binding_ty_fields.is_empty()
            && binding_array_element.is_none()
            && fields.len() == 1
            && !matches!(
                tree.get(fields[0]),
                PatternField::Spread { .. } | PatternField::Elision
            );
        let direct_binding_fallback = allow_direct_binding_fallback
            .then_some(binding_ty_id)
            .flatten();

        // nothing to infer when the sequence has no fields
        if fields.is_empty() {
            return Ok(());
        }

        // reject non-sequence bindings that cannot use the guarded fallback
        if let Some(binding_ty_id) = binding_ty_id
            && binding_ty_fields.is_empty()
            && binding_array_element.is_none()
            && !allow_direct_binding_fallback
        {
            let first_field_id = fields[0];
            let unknown_ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            let unknown_ty_id = types.insert_type_from(unknown_ty, first_field_id);
            let actual_elements = fields
                .iter()
                .map(|_| TypeElement::new(unknown_ty_id))
                .collect();
            let actual_ty = Type::Tuple {
                elements: actual_elements,
                is_readonly: false,
            };
            let actual_ty_id = types.insert_type_from(actual_ty, first_field_id);
            let _reported = self.report_unassignable_type_for_types(
                module,
                ctx.profile,
                first_field_id.into_any(),
                binding_ty_id,
                actual_ty_id,
                types,
            );
        }

        let spread_len = binding_ty_fields.len().saturating_sub(fields.len() - 1);
        let mut ty_idx = 0;
        for field_id in fields {
            let field = tree.get(*field_id);
            let field_ty = match field {
                PatternField::Named { .. }
                | PatternField::Alias { .. }
                | PatternField::Positional { .. }
                | PatternField::Computed { .. } => {
                    if !binding_ty_fields.is_empty() {
                        let ty = binding_ty_fields.get(ty_idx).cloned();
                        ty_idx += 1;
                        ty
                    } else if let Some(element_ty_id) = binding_array_element {
                        Some(element_ty_id)
                    } else {
                        direct_binding_fallback
                    }
                }
                PatternField::Spread { .. } => {
                    if !binding_ty_fields.is_empty() {
                        let rest_types = binding_ty_fields
                            .get(ty_idx..ty_idx + spread_len)
                            .map(|s| s.to_vec())
                            .unwrap_or_default();
                        ty_idx += spread_len;
                        let rest_ty = to_rest_type(rest_types);
                        Some(types.insert_type_from(rest_ty, *field_id))
                    } else if let Some(element_ty_id) = binding_array_element {
                        let rest_ty = to_rest_type(vec![element_ty_id]);
                        Some(types.insert_type_from(rest_ty, *field_id))
                    } else {
                        None
                    }
                }
                PatternField::Elision => {
                    // elision skips a type position
                    if !binding_ty_fields.is_empty() {
                        ty_idx += 1;
                    }
                    None
                }
            };
            self.infer_pattern_field(
                module, *field_id, field_ty, tree, symbols, types, infer, ctx,
            )?;
        }
        Ok(())
    }

    /// Resolve tuple member element types for a union of fixed tuples.
    fn pattern_union_tuple_member_elements(
        &self,
        binding_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<Vec<Vec<LocalTypeId>>> {
        let binding_ty = types.get_type(binding_ty_id).clone();
        match binding_ty {
            Type::Union { elements } => {
                let mut member_elements: Vec<Vec<LocalTypeId>> = Vec::with_capacity(elements.len());
                let mut tuple_len: Option<usize> = None;
                for element_type_id in elements {
                    let element_type_id = types.unwrap_value_type_id(element_type_id);
                    let Type::Tuple { elements, .. } = types.get_type(element_type_id) else {
                        return None;
                    };
                    let element_types: Vec<LocalTypeId> =
                        elements.iter().map(|element| element.ty).collect();

                    if let Some(expected_len) = tuple_len {
                        if element_types.len() != expected_len {
                            return None;
                        }
                    } else {
                        tuple_len = Some(element_types.len());
                    }
                    member_elements.push(element_types);
                }
                Some(member_elements)
            }
            Type::Reference { symbol, .. } => types
                .get_value_type_id(symbol)
                .and_then(|type_id| self.pattern_union_tuple_member_elements(type_id, types)),
            Type::Value { value } => self.pattern_union_tuple_member_elements(value, types),
            _ => None,
        }
    }

    /// Resolve a tuple slot type for the current union member candidates.
    fn pattern_union_slot_type_for_candidates(
        &self,
        member_elements: &[Vec<LocalTypeId>],
        candidate_indexes: &[usize],
        tuple_index: usize,
        field_id: LocalNodeId<PatternField>,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let mut slot_types: Vec<LocalTypeId> = Vec::new();
        for candidate_index in candidate_indexes {
            let slot_type_id = member_elements[*candidate_index][tuple_index];
            if !slot_types.contains(&slot_type_id) {
                slot_types.push(slot_type_id);
            }
        }

        if slot_types.is_empty() {
            return None;
        }
        if slot_types.len() == 1 {
            return Some(slot_types[0]);
        }

        let union_ty_id = types.insert_type_from(
            Type::Union {
                elements: slot_types,
            },
            field_id,
        );
        Some(union_ty_id)
    }

    /// Filter tuple union candidates using literal coverage for a specific slot.
    fn pattern_filter_tuple_union_candidates(
        &self,
        member_elements: &[Vec<LocalTypeId>],
        candidate_indexes: &mut Vec<usize>,
        tuple_index: usize,
        coverage: &TuplePatternLiteralCoverage,
        types: &mut TypeTable,
    ) {
        if matches!(coverage, TuplePatternLiteralCoverage::All) {
            return;
        }

        let TuplePatternLiteralCoverage::Values(values) = coverage else {
            return;
        };
        candidate_indexes.retain(|candidate_index| {
            self.pattern_tuple_slot_scalar_literal(
                member_elements,
                *candidate_index,
                tuple_index,
                types,
            )
            .is_some_and(|literal| values.contains(&literal))
        });
    }

    /// Resolve a scalar literal for a tuple member slot when available.
    fn pattern_tuple_slot_scalar_literal(
        &self,
        member_elements: &[Vec<LocalTypeId>],
        candidate_index: usize,
        tuple_index: usize,
        types: &mut TypeTable,
    ) -> Option<ScalarLiteral> {
        let slot_type_id = member_elements[candidate_index][tuple_index];
        let slot_type_id = types.unwrap_value_type_id(slot_type_id);
        let slot_type = types.get_type(slot_type_id);
        match slot_type {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(value),
            } => Some(value.clone()),
            _ => None,
        }
    }

    /// Resolve literal coverage for tuple discriminant pattern filtering.
    fn pattern_literal_coverage_for_tuple_pattern(
        &self,
        pattern_id: LocalNodeId<Pattern>,
        tree: &NodeTree,
    ) -> Option<TuplePatternLiteralCoverage> {
        let pattern = tree.get(pattern_id);
        match pattern {
            Pattern::Wildcard | Pattern::Binding { .. } => Some(TuplePatternLiteralCoverage::All),
            Pattern::Must(pattern_id)
            | Pattern::ReferenceOf {
                mutability: _,
                right: pattern_id,
            }
            | Pattern::ValueOf {
                mutability: _,
                right: pattern_id,
            } => self.pattern_literal_coverage_for_tuple_pattern(*pattern_id, tree),
            Pattern::Expression { value } => self
                .pattern_scalar_literal_for_expression(*value, tree)
                .map(|value| TuplePatternLiteralCoverage::Values(vec![value])),
            Pattern::Union { patterns } => {
                let mut values = Vec::new();
                for pattern_id in patterns {
                    match self.pattern_literal_coverage_for_tuple_pattern(*pattern_id, tree)? {
                        TuplePatternLiteralCoverage::All => {
                            return Some(TuplePatternLiteralCoverage::All);
                        }
                        TuplePatternLiteralCoverage::Values(part) => {
                            for value in part {
                                if !values.contains(&value) {
                                    values.push(value);
                                }
                            }
                        }
                    }
                }
                Some(TuplePatternLiteralCoverage::Values(values))
            }
            _ => None,
        }
    }

    /// Resolve a scalar literal from a pattern expression.
    fn pattern_scalar_literal_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<ScalarLiteral> {
        let expression = tree.get(expression_id);
        match expression {
            Expression::ScalarLiteral { value } => Some(value.clone()),
            _ => None,
        }
    }

    /// Resolve sequence element binding types for tuple and array pattern targets.
    fn pattern_sequence_binding_types(
        &self,
        binding_ty_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> (Vec<LocalTypeId>, Option<LocalTypeId>) {
        let binding_ty = types.get_type(binding_ty_id).clone();
        match binding_ty {
            // tuple patterns bind each tuple element positionally
            Type::Tuple { elements, .. } => {
                (elements.iter().map(|element| element.ty).collect(), None)
            }

            // fixed arrays bind the repeated element type
            Type::ArraySized { element, .. } => (Vec::new(), Some(element)),

            // dynamic arrays bind the optional element type
            Type::Array { element, .. } => (Vec::new(), element),

            // union patterns support tuple union discriminants and per-slot narrowing
            Type::Union { elements } => self
                .pattern_union_tuple_element_types(&elements, binding_ty_id, types)
                .map_or((Vec::new(), None), |fields| (fields, None)),

            // value wrappers defer to the wrapped type
            Type::Value { value } => self.pattern_sequence_binding_types(value, types),

            _ => (Vec::new(), None),
        }
    }

    /// Resolve per-position tuple element types for a union of tuple members.
    fn pattern_union_tuple_element_types(
        &self,
        element_type_ids: &[LocalTypeId],
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        // collect tuple member elements from the union first
        let union_ty_id = types.insert_type_from_type(
            Type::Union {
                elements: element_type_ids.to_vec(),
            },
            source_type_id,
        );
        let member_elements = self.pattern_union_tuple_member_elements(union_ty_id, types)?;

        // collect tuple member element vectors
        let tuple_len = member_elements.first().map_or(0, Vec::len);
        let mut merged_fields: Vec<LocalTypeId> = Vec::with_capacity(tuple_len);

        // merge each tuple slot into a field type
        for index in 0..tuple_len {
            let mut slot_types: Vec<LocalTypeId> = Vec::new();
            for member in &member_elements {
                let slot_type_id = member[index];
                if !slot_types.contains(&slot_type_id) {
                    slot_types.push(slot_type_id);
                }
            }

            // keep a single slot type when possible
            if slot_types.len() == 1 {
                merged_fields.push(slot_types[0]);
            }
            // otherwise use a slot union type
            else {
                let union_type_id = types.insert_type_from_type(
                    Type::Union {
                        elements: slot_types,
                    },
                    source_type_id,
                );
                merged_fields.push(union_type_id);
            }
        }

        Some(merged_fields)
    }

    /// Infer a pattern field and propagate type to bound symbol.
    pub(crate) fn infer_pattern_field(
        &self,
        module: &Module,
        field_id: LocalNodeId<PatternField>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        match field {
            PatternField::Named {
                mutability: _,
                name,
                default,
                pattern,
            } => {
                // resolve the field type from the binding type when possible
                let field_ty_id = self.pattern_field_binding_type(
                    module,
                    field_id,
                    binding_ty_id,
                    StaticKey::Name(*name),
                    types,
                    ctx,
                )?;

                // infer default expressions for named fields
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }

                // propagate the field type into nested patterns
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        field_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            PatternField::Computed {
                key,
                pattern,
                default,
                ..
            } => {
                self.infer_expression(module, *key, tree, symbols, types, infer, ctx)?;
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        None,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            PatternField::Alias {
                mutability: _,
                name,
                alias: _,
                default,
                symbol,
            } => {
                // resolve the field type from the binding type when possible
                let field_ty_id = self.pattern_field_binding_type(
                    module,
                    field_id,
                    binding_ty_id,
                    StaticKey::Name(*name),
                    types,
                    ctx,
                )?;
                if let Some(ty_id) = field_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }

                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }
            }
            PatternField::Positional { pattern, default } => {
                self.infer_pattern(
                    module,
                    *pattern,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }
            }
            PatternField::Spread {
                mutability: _,
                pattern,
            } => {
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            PatternField::Elision => {
                // elision doesn't bind anything
            }
        }
        Ok(())
    }

    /// Resolve the binding type for a named pattern field.
    fn pattern_field_binding_type(
        &self,
        module: &Module,
        field_id: LocalNodeId<PatternField>,
        binding_ty_id: Option<LocalTypeId>,
        field_key: StaticKey,
        types: &mut TypeTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when there is no binding type to inspect
        let Some(binding_ty_id) = binding_ty_id else {
            return Ok(None);
        };

        // resolve field types when the binding type is an object or reference
        let receiver_ty = types.get_type(binding_ty_id).clone();
        let symbols = module.dir(ctx.profile).symbols.read();
        let mut visited = Vec::new();
        let field_ty_id = self.infer_member_of_type(
            module,
            ctx.profile,
            field_id.into_any(),
            &symbols,
            &receiver_ty,
            &field_key,
            MemberLookupMode::Any,
            types,
            &mut visited,
        )?;

        // allow match patterns to bind fields from matching union variants
        let match_union_field =
            if field_ty_id.is_none() && (ctx.in_match.is_some() || ctx.in_switch.is_some()) {
                let union_receiver_ty = match receiver_ty {
                    Type::Reference { symbol, .. } => self
                        .apparent_instance_type(
                            module,
                            ctx.profile,
                            field_id.into_any(),
                            symbol,
                            &symbols,
                            types,
                        )
                        .map(|type_id| types.get_type(type_id).clone())
                        .unwrap_or(receiver_ty.clone()),
                    Type::Value { value } => types.get_type(value).clone(),
                    _ => receiver_ty.clone(),
                };
                if let Type::Union { elements } = union_receiver_ty {
                    let mut field_types = Vec::new();
                    for element_id in elements {
                        let element_ty = types.get_type(element_id).clone();
                        if let Some(field_ty) = self.infer_member_of_type(
                            module,
                            ctx.profile,
                            field_id.into_any(),
                            &symbols,
                            &element_ty,
                            &field_key,
                            MemberLookupMode::Any,
                            types,
                            &mut visited,
                        )? {
                            field_types.push(field_ty);
                        }
                    }
                    if field_types.is_empty() {
                        None
                    } else if field_types.len() == 1 {
                        Some(field_types[0])
                    } else {
                        Some(self.union_type_from_list(field_types, binding_ty_id, types))
                    }
                } else {
                    None
                }
            } else {
                None
            };

        // fall back to the binding type for non-object patterns
        Ok(Some(
            match_union_field.unwrap_or_else(|| field_ty_id.unwrap_or(binding_ty_id)),
        ))
    }

    /// Resolve a tagged pattern target type from an expression.
    fn evaluate_pattern_tag_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // evaluate the tag expression as a type
        let ty_id = self.resolve_declared_type_expression(
            module,
            ctx.profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
            true,
        )?;

        // unwrap type-as-value wrappers when present
        let ty_id = match types.get_type(ty_id) {
            Type::Value { value } => *value,
            _ => ty_id,
        };

        // return non-reference tag types directly
        let Type::Reference {
            symbol,
            static_arguments,
        } = types.get_type(ty_id).clone()
        else {
            return Ok(ty_id);
        };

        // skip remote symbols
        if symbol.module_id != module.id {
            return Ok(ty_id);
        }

        // skip non-newtype symbols
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        if symbol_entry.ty != SymbolType::Newtype {
            return Ok(ty_id);
        }

        // load the local nominal declaration for the tag
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(ty_id);
        };
        if primary_declaration.module_id != module.id {
            return Ok(ty_id);
        }
        let Ok(declaration_id) = primary_declaration.try_into_typed::<Declaration>() else {
            return Ok(ty_id);
        };
        let declaration_id: LocalNodeId<Declaration> = declaration_id.into();
        let Declaration::Type {
            kind: TypeKind::Nominal,
            value,
            ..
        } = tree.get(declaration_id)
        else {
            return Ok(ty_id);
        };

        // resolve the declared type for the nominal alias
        let value_id = value.into_global_any(module.id);
        let Some(declared_ty_id) = types.get_declared_type_id(value_id) else {
            return Ok(ty_id);
        };

        // evaluate unevaluated declared types
        if matches!(types.get_type(declared_ty_id), Type::Unevaluated(_)) {
            self.resolve_declared_type(module, ctx.profile, declared_ty_id, tree, symbols, types)?;
        }

        let mut declared_ty_id = declared_ty_id;

        // apply static arguments when provided
        if let Some(static_arguments) = static_arguments {
            let source_id = types.get_type_source(ty_id);
            let resolved_arguments = self.resolve_type_reference_static_arguments(
                module,
                ctx.profile,
                source_id,
                symbol,
                Some(static_arguments.as_slice()),
                true,
                &ctx.options,
                tree,
                symbols,
                types,
            )?;
            if let Some(resolved_arguments) = resolved_arguments
                && !resolved_arguments.is_empty()
            {
                let substitutions = self.build_type_parameter_substitutions_for_symbol(
                    module,
                    ctx.profile,
                    symbol,
                    source_id,
                    &resolved_arguments,
                    tree,
                    symbols,
                    types,
                );
                if !substitutions.is_empty() {
                    let mut cache = HashMap::new();
                    declared_ty_id = self.substitute_static_parameters(
                        declared_ty_id,
                        &substitutions,
                        types,
                        &mut cache,
                    );
                }
            }
        }

        Ok(declared_ty_id)
    }
}
