use std::collections::{HashMap, HashSet};

use crate::analyze::common::TypeContext;
use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Expression, GlobalSymbolId, Key, LocalNodeId, LocalTypeId, MatchCase, MatchSelector, NodeTree,
    NormalizationMode, Pattern, PatternField, PrimitiveType, ScalarLiteral, StaticKey, StringId,
    SymbolType, Type, TypeExpression, TypeField, TypeLiteral, TypeTable,
};

/// Coverage summary for a match pattern.
#[derive(Debug)]
enum MatchPatternCoverage<T> {
    /// The pattern covers all candidates.
    All,
    /// The pattern covers a specific set of candidates.
    Values(Vec<T>),
}

/// Literal values used for exhaustiveness checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MatchLiteral {
    /// The null literal.
    Null,
    /// The undefined literal.
    Undefined,
    /// A boolean literal.
    Boolean(bool),
    /// An integer literal.
    Integer(i64),
    /// A bigint literal.
    Bigint(i64),
    /// A float literal (stored as bits).
    Float(u64),
    /// A character literal.
    Character(char),
    /// A string literal.
    String(StringId),
}

/// Exhaustiveness targets for match expressions.
#[derive(Debug)]
enum MatchExhaustiveTarget {
    /// Enum fields must be covered by match patterns.
    Enum {
        symbol: GlobalSymbolId,
        fields: Vec<GlobalSymbolId>,
        field_set: HashSet<GlobalSymbolId>,
    },
    /// Literal unions must be covered by match patterns.
    LiteralUnion { values: HashSet<MatchLiteral> },
    /// Discriminated unions must cover every discriminant value.
    DiscriminantUnion {
        key: StaticKey,
        values: HashSet<MatchLiteral>,
    },
    /// Tuple unions can be exhaustive when a fixed element index is a literal discriminant.
    TupleDiscriminantUnion {
        index: usize,
        values: HashSet<MatchLiteral>,
    },
}

/// Rest sequence shapes used by irrefutable rest checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SequenceRestPatternKind {
    /// Rest behaves like a fixed tuple suffix.
    Tuple,
    /// Rest behaves like an unsized array.
    Array,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Validate a single pattern node.
    pub(super) fn validate_pattern(&self, ctx: &mut TypeContext<'_>, pattern: &Pattern) {
        match pattern {
            Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => {
                self.validate_object_pattern_spreads(&mut ctx.reborrow(), fields);
            }
            Pattern::Array { fields }
            | Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. } => {
                self.validate_sequence_pattern_fields(&mut ctx.reborrow(), fields);
            }
            _ => {}
        }
    }

    /// Validate match exhaustiveness for supported value shapes.
    pub(super) fn validate_match_exhaustiveness(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
    ) {
        // scan selectors for fallback and guard usage
        let mut has_fallback = false;
        let mut has_guard = false;
        for case_id in cases {
            let selector = match ctx.tree.get(*case_id) {
                MatchCase::Expression { selector, .. } | MatchCase::Block { selector, .. } => {
                    selector
                }
            };

            match selector {
                MatchSelector::Default => {
                    has_fallback = true;
                }
                MatchSelector::Pattern { guard, .. } => {
                    if guard.is_some() {
                        has_guard = true;
                    }
                }
            }
        }

        // fallback arms satisfy exhaustiveness requirements
        if has_fallback {
            return;
        }

        // resolve the match value type
        let value_type_id = ctx
            .types
            .get_inferred_type_id(value_id.into_global_any(ctx.module.id))
            .or_else(|| {
                let symbol =
                    self.reference_symbol_for_expression(ctx.tree_symbol_view(), value_id)?;
                ctx.types.get_value_type_id(symbol)
            });
        let Some(value_type_id) = value_type_id else {
            return;
        };
        let value_type_id = self.unwrap_type_value(value_type_id, ctx.types);
        let value_type_id = match ctx.types.get_type(value_type_id) {
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Void => ctx
                .types
                .get_value_type_id(*symbol)
                .unwrap_or(value_type_id),
            _ => value_type_id,
        };
        // preserve nominal types for irrefutable checks
        let irrefutable_type_id = value_type_id;
        let normalized_type_id =
            self.normalize_type(&mut ctx.reborrow(), value_type_id, NormalizationMode::Flow);

        // allow irrefutable patterns to satisfy exhaustiveness
        if self.match_has_irrefutable_pattern(&mut ctx.reborrow(), irrefutable_type_id, cases) {
            return;
        }

        // guards require a fallback because exhaustiveness cannot be proven
        if has_guard {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::NonExhaustiveMatch { node });
            return;
        }

        // resolve the exhaustiveness target
        let target = self
            .match_exhaustiveness_target(&mut ctx.reborrow(), irrefutable_type_id)
            .or_else(|| self.match_exhaustiveness_target(&mut ctx.reborrow(), normalized_type_id));
        let Some(target) = target else {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::NonExhaustiveMatch { node });
            return;
        };

        // track covered entries based on the target
        let mut covered_literals: HashSet<MatchLiteral> = HashSet::new();
        let mut covered_fields: HashSet<GlobalSymbolId> = HashSet::new();
        let mut is_provable = true;
        for case_id in cases {
            let selector = match ctx.tree.get(*case_id) {
                MatchCase::Expression { selector, .. } | MatchCase::Block { selector, .. } => {
                    selector
                }
            };

            // default or guarded cases block exhaustiveness reasoning
            let MatchSelector::Pattern { pattern, guard } = selector else {
                is_provable = false;
                break;
            };
            if guard.is_some() {
                is_provable = false;
                break;
            }

            match &target {
                MatchExhaustiveTarget::Enum {
                    symbol, field_set, ..
                } => {
                    let coverage = self.enum_pattern_coverage(
                        &mut ctx.reborrow(),
                        *symbol,
                        *pattern,
                        field_set,
                    );
                    let Some(coverage) = coverage else {
                        is_provable = false;
                        break;
                    };
                    match coverage {
                        MatchPatternCoverage::All => return,
                        MatchPatternCoverage::Values(fields) => {
                            covered_fields.extend(fields);
                        }
                    }
                }
                MatchExhaustiveTarget::LiteralUnion { values } => {
                    let coverage = self
                        .literal_pattern_coverage(*pattern, ctx.tree)
                        .and_then(|coverage| self.filter_literal_coverage(values, coverage));
                    let Some(coverage) = coverage else {
                        is_provable = false;
                        break;
                    };
                    match coverage {
                        MatchPatternCoverage::All => return,
                        MatchPatternCoverage::Values(literals) => {
                            covered_literals.extend(literals);
                        }
                    }
                }
                MatchExhaustiveTarget::DiscriminantUnion { key, values } => {
                    let coverage = self.discriminant_pattern_coverage(
                        &mut ctx.reborrow(),
                        *pattern,
                        *key,
                        values,
                    );
                    let Some(coverage) = coverage else {
                        is_provable = false;
                        break;
                    };
                    match coverage {
                        MatchPatternCoverage::All => return,
                        MatchPatternCoverage::Values(literals) => {
                            covered_literals.extend(literals);
                        }
                    }
                }
                MatchExhaustiveTarget::TupleDiscriminantUnion { index, values } => {
                    let coverage = self
                        .tuple_discriminant_pattern_coverage(*pattern, *index, values, ctx.tree);
                    let Some(coverage) = coverage else {
                        is_provable = false;
                        break;
                    };
                    match coverage {
                        MatchPatternCoverage::All => return,
                        MatchPatternCoverage::Values(literals) => {
                            covered_literals.extend(literals);
                        }
                    }
                }
            }
        }

        // require fallback when coverage cannot be proven
        if !is_provable {
            let node = expression_id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::NonExhaustiveMatch { node });
            return;
        }

        // check for missing fields
        let is_exhaustive = match target {
            MatchExhaustiveTarget::Enum { fields, .. } => covered_fields.len() == fields.len(),
            MatchExhaustiveTarget::LiteralUnion { values } => covered_literals == values,
            MatchExhaustiveTarget::DiscriminantUnion { values, .. } => covered_literals == values,
            MatchExhaustiveTarget::TupleDiscriminantUnion { values, .. } => {
                covered_literals == values
            }
        };
        if is_exhaustive {
            return;
        }

        let node = expression_id
            .into_global_any(ctx.module.id)
            .into_anchored(Some(ctx.profile));
        self.error(AnalyzeError::NonExhaustiveMatch { node });
    }

    /// Check if a match contains an irrefutable pattern for the given type.
    fn match_has_irrefutable_pattern(
        &self,
        ctx: &mut TypeContext<'_>,
        value_type_id: LocalTypeId,
        cases: &[LocalNodeId<MatchCase>],
    ) -> bool {
        for case_id in cases {
            let selector = match ctx.tree.get(*case_id) {
                MatchCase::Expression { selector, .. } | MatchCase::Block { selector, .. } => {
                    selector
                }
            };

            let MatchSelector::Pattern { pattern, guard } = selector else {
                continue;
            };

            if guard.is_some() {
                continue;
            }

            if self.is_irrefutable_pattern_for_type(&mut ctx.reborrow(), *pattern, value_type_id) {
                return true;
            }
        }

        false
    }

    /// Check whether a pattern matches all values of the given type.
    pub(crate) fn is_irrefutable_pattern_for_type(
        &self,
        ctx: &mut TypeContext<'_>,
        pattern_id: LocalNodeId<Pattern>,
        value_type_id: LocalTypeId,
    ) -> bool {
        let mut visited = HashSet::new();
        self.is_irrefutable_pattern_for_type_inner(
            &mut ctx.reborrow(),
            pattern_id,
            value_type_id,
            &mut visited,
        )
    }

    /// Check whether a pattern matches all values of the given type.
    fn is_irrefutable_pattern_for_type_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        pattern_id: LocalNodeId<Pattern>,
        value_type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let value_type_id = ctx.types.unwrap_value_type_id(value_type_id);
        if !visited.insert(value_type_id) {
            return false;
        }

        let is_irrefutable = match ctx.tree.get(pattern_id) {
            Pattern::Wildcard => true,
            Pattern::Assign { pattern, .. } => self.is_irrefutable_pattern_for_type_inner(
                &mut ctx.reborrow(),
                *pattern,
                value_type_id,
                visited,
            ),
            Pattern::Binding { pattern, .. } => match pattern {
                None => {
                    let value_type = ctx.types.get_type(value_type_id);
                    if let Some(enum_symbol) = self.enum_symbol_for_type(value_type, ctx.types) {
                        let Pattern::Binding { name, .. } = ctx.tree.get(pattern_id) else {
                            return true;
                        };
                        if self
                            .query_enum_field_symbol_for_name(ctx.type_view(), enum_symbol, *name)
                            .is_some()
                        {
                            return false;
                        }
                    }

                    true
                }
                Some(inner) => self.is_irrefutable_pattern_for_type_inner(
                    &mut ctx.reborrow(),
                    *inner,
                    value_type_id,
                    visited,
                ),
            },
            Pattern::ReferenceOf { right, .. } => {
                let inner = match ctx.types.get_type(value_type_id) {
                    Type::ReferenceOf { right: inner, .. } => *inner,
                    _ => return false,
                };
                self.is_irrefutable_pattern_for_type_inner(
                    &mut ctx.reborrow(),
                    *right,
                    inner,
                    visited,
                )
            }
            Pattern::ValueOf { right, .. } => {
                let inner = match ctx.types.get_type(value_type_id) {
                    Type::ValueOf { right: inner, .. } => *inner,
                    _ => return false,
                };
                self.is_irrefutable_pattern_for_type_inner(
                    &mut ctx.reborrow(),
                    *right,
                    inner,
                    visited,
                )
            }
            Pattern::Union { patterns } => patterns.iter().any(|inner| {
                self.is_irrefutable_pattern_for_type_inner(
                    &mut ctx.reborrow(),
                    *inner,
                    value_type_id,
                    visited,
                )
            }),
            Pattern::Tuple { fields } => self.is_irrefutable_sequence_pattern_for_type(
                &mut ctx.reborrow(),
                fields,
                value_type_id,
                SequenceRestPatternKind::Tuple,
                visited,
            ),
            Pattern::Array { fields } => self.is_irrefutable_sequence_pattern_for_type(
                &mut ctx.reborrow(),
                fields,
                value_type_id,
                SequenceRestPatternKind::Array,
                visited,
            ),
            Pattern::TaggedTuple { ty, fields } => self.is_irrefutable_tagged_tuple_pattern(
                &mut ctx.reborrow(),
                *ty,
                fields,
                value_type_id,
                visited,
            ),
            Pattern::Object { fields } => self.is_irrefutable_object_pattern_for_type(
                &mut ctx.reborrow(),
                fields,
                value_type_id,
                visited,
            ),
            Pattern::TaggedObject { ty, fields } => self.is_irrefutable_tagged_object_pattern(
                &mut ctx.reborrow(),
                *ty,
                fields,
                value_type_id,
                visited,
            ),
            Pattern::Must(_) | Pattern::Expression { .. } | Pattern::TypeExpression { .. } => false,
        };

        // clear the path marker after finishing this branch
        visited.remove(&value_type_id);

        is_irrefutable
    }

    /// Check if a sequence pattern matches all values of a fixed-size sequence type.
    fn is_irrefutable_sequence_pattern_for_type(
        &self,
        ctx: &mut TypeContext<'_>,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        rest_pattern_kind: SequenceRestPatternKind,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // resolve fixed element types for the matched value
        let Some(element_types) =
            self.fixed_sequence_element_types(&mut ctx.reborrow(), value_type_id)
        else {
            return false;
        };

        self.is_irrefutable_sequence_pattern_for_fixed_elements(
            &mut ctx.reborrow(),
            fields,
            &element_types,
            rest_pattern_kind,
            visited,
        )
    }

    /// Check if a sequence pattern matches all values of fixed element types.
    fn is_irrefutable_sequence_pattern_for_fixed_elements(
        &self,
        ctx: &mut TypeContext<'_>,
        fields: &[LocalNodeId<PatternField>],
        element_types: &[LocalTypeId],
        rest_pattern_kind: SequenceRestPatternKind,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // locate a rest field when present
        let mut rest_index = None;
        for (index, field_id) in fields.iter().enumerate() {
            if matches!(ctx.tree.get(*field_id), PatternField::Spread { .. }) {
                if rest_index.is_some() {
                    return false;
                }
                rest_index = Some(index);
            }
        }

        // rest fields must be trailing
        if let Some(rest_index) = rest_index
            && rest_index + 1 != fields.len()
        {
            return false;
        }

        // fixed length patterns must cover all elements without rest
        if rest_index.is_none() && element_types.len() != fields.len() {
            return false;
        }

        // rest patterns must cover at least the prefix length
        if let Some(rest_index) = rest_index
            && element_types.len() < rest_index
        {
            return false;
        }

        // check prefix fields against fixed element types
        let prefix_len = rest_index.unwrap_or(fields.len());
        for (field_id, element_type_id) in fields.iter().take(prefix_len).zip(element_types.iter())
        {
            let field = ctx.tree.get(*field_id);
            match field {
                PatternField::Positional { pattern, .. } => {
                    if !self.is_irrefutable_pattern_for_type_inner(
                        &mut ctx.reborrow(),
                        *pattern,
                        *element_type_id,
                        visited,
                    ) {
                        return false;
                    }
                }
                PatternField::Named { pattern, .. } => {
                    if let Some(pattern_id) = pattern
                        && !self.is_irrefutable_pattern_for_type_inner(
                            &mut ctx.reborrow(),
                            *pattern_id,
                            *element_type_id,
                            visited,
                        )
                    {
                        return false;
                    }
                }
                PatternField::Elision => {}
                PatternField::Computed { .. } | PatternField::Spread { .. } => {
                    return false;
                }
            }
        }

        // validate the rest binding when present
        if let Some(rest_index) = rest_index {
            let rest_elements = &element_types[rest_index..];

            // validate the rest pattern when provided
            let PatternField::Spread { pattern, .. } = ctx.tree.get(fields[rest_index]) else {
                return false;
            };
            if let Some(pattern_id) = pattern
                && !self.is_irrefutable_sequence_rest_pattern(
                    &mut ctx.reborrow(),
                    *pattern_id,
                    rest_elements,
                    rest_pattern_kind,
                    visited,
                )
            {
                return false;
            }
        }

        true
    }

    /// Check whether one rest pattern is irrefutable for a virtual sequence rest shape.
    fn is_irrefutable_sequence_rest_pattern(
        &self,
        ctx: &mut TypeContext<'_>,
        pattern_id: LocalNodeId<Pattern>,
        rest_elements: &[LocalTypeId],
        rest_pattern_kind: SequenceRestPatternKind,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        match ctx.tree.get(pattern_id) {
            Pattern::Assign { pattern, .. } => self.is_irrefutable_sequence_rest_pattern(
                &mut ctx.reborrow(),
                *pattern,
                rest_elements,
                rest_pattern_kind,
                visited,
            ),
            Pattern::Wildcard => true,
            Pattern::Binding { pattern, .. } => {
                if let Some(inner_pattern_id) = pattern {
                    return self.is_irrefutable_sequence_rest_pattern(
                        &mut ctx.reborrow(),
                        *inner_pattern_id,
                        rest_elements,
                        rest_pattern_kind,
                        visited,
                    );
                }

                true
            }
            Pattern::Union { patterns } => patterns.iter().any(|inner_pattern_id| {
                self.is_irrefutable_sequence_rest_pattern(
                    &mut ctx.reborrow(),
                    *inner_pattern_id,
                    rest_elements,
                    rest_pattern_kind,
                    visited,
                )
            }),
            Pattern::Tuple { fields } => {
                if rest_pattern_kind != SequenceRestPatternKind::Tuple {
                    return false;
                }

                self.is_irrefutable_sequence_pattern_for_fixed_elements(
                    &mut ctx.reborrow(),
                    fields,
                    rest_elements,
                    SequenceRestPatternKind::Tuple,
                    visited,
                )
            }
            Pattern::Array { fields } => {
                if rest_pattern_kind != SequenceRestPatternKind::Tuple {
                    return false;
                }

                self.is_irrefutable_sequence_pattern_for_fixed_elements(
                    &mut ctx.reborrow(),
                    fields,
                    rest_elements,
                    SequenceRestPatternKind::Array,
                    visited,
                )
            }
            Pattern::ReferenceOf { .. }
            | Pattern::ValueOf { .. }
            | Pattern::TaggedTuple { .. }
            | Pattern::Object { .. }
            | Pattern::TaggedObject { .. }
            | Pattern::Must(_)
            | Pattern::Expression { .. }
            | Pattern::TypeExpression { .. } => false,
        }
    }

    /// Check whether a tagged tuple pattern is irrefutable for a nominal type.
    fn is_irrefutable_tagged_tuple_pattern(
        &self,
        ctx: &mut TypeContext<'_>,
        ty: LocalNodeId<TypeExpression>,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(tag_symbol) =
            self.reference_symbol_for_type_expression(ctx.tree_symbol_view(), ty)
        else {
            return false;
        };

        if !self.value_type_matches_tag_symbol(&mut ctx.reborrow(), value_type_id, tag_symbol) {
            return false;
        }

        if tag_symbol.ty() != SymbolType::Newtype {
            return false;
        }

        let Some(target_type_id) =
            self.committed_alias_target_type_id_for_symbol(ctx.symbol_type_view(), tag_symbol)
        else {
            return false;
        };

        let target_type_id = ctx.types.unwrap_value_type_id(target_type_id);

        // scalar newtypes use a single field
        if fields.len() == 1 {
            let field = ctx.tree.get(fields[0]);
            let nested_pattern = match field {
                PatternField::Positional { pattern, .. } => Some(*pattern),
                PatternField::Named { pattern, .. } => *pattern,
                PatternField::Elision => None,
                PatternField::Computed { .. } | PatternField::Spread { .. } => return false,
            };

            if let Some(pattern_id) = nested_pattern {
                return self.is_irrefutable_pattern_for_type_inner(
                    &mut ctx.reborrow(),
                    pattern_id,
                    target_type_id,
                    visited,
                );
            }

            return true;
        }

        self.is_irrefutable_sequence_pattern_for_type(
            &mut ctx.reborrow(),
            fields,
            target_type_id,
            SequenceRestPatternKind::Tuple,
            visited,
        )
    }

    /// Check whether an object pattern is irrefutable for a structural object type.
    fn is_irrefutable_object_pattern_for_type(
        &self,
        ctx: &mut TypeContext<'_>,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(field_map) = self.object_field_map_for_type(&mut ctx.reborrow(), value_type_id)
        else {
            return false;
        };

        self.object_pattern_is_irrefutable(&mut ctx.reborrow(), fields, &field_map, visited)
    }

    /// Check whether a tagged object pattern is irrefutable for a nominal object type.
    fn is_irrefutable_tagged_object_pattern(
        &self,
        ctx: &mut TypeContext<'_>,
        ty: LocalNodeId<TypeExpression>,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(tag_symbol) =
            self.reference_symbol_for_type_expression(ctx.tree_symbol_view(), ty)
        else {
            return false;
        };

        if !self.value_type_matches_tag_symbol(&mut ctx.reborrow(), value_type_id, tag_symbol) {
            return false;
        }

        let Some(object_type_id) =
            self.object_type_id_for_tag_symbol(&mut ctx.reborrow(), tag_symbol)
        else {
            return false;
        };

        let Some(field_map) = self.object_field_map_for_object_type(ctx.types, object_type_id)
        else {
            return false;
        };

        self.object_pattern_is_irrefutable(&mut ctx.reborrow(), fields, &field_map, visited)
    }

    /// Check whether a pattern field list is irrefutable against a field map.
    fn object_pattern_is_irrefutable(
        &self,
        ctx: &mut TypeContext<'_>,
        fields: &[LocalNodeId<PatternField>],
        field_map: &HashMap<StaticKey, TypeField>,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        for field_id in fields {
            let field = ctx.tree.get(*field_id);
            match field {
                PatternField::Spread { .. } => {}
                PatternField::Named { name, pattern, .. } => {
                    let key = StaticKey::Name(*name);
                    let Some(field_ty) = field_map.get(&key) else {
                        return false;
                    };
                    if field_ty.is_optional {
                        return false;
                    }
                    if let Some(inner) = pattern
                        && !self.is_irrefutable_pattern_for_type_inner(
                            &mut ctx.reborrow(),
                            *inner,
                            field_ty.ty,
                            visited,
                        )
                    {
                        return false;
                    }
                }
                PatternField::Computed { key, pattern, .. } => {
                    let Some(key) = self.static_key_from_key(
                        ctx.compiler_context.revision(),
                        ctx.profile,
                        ctx.tree,
                        ctx.symbols,
                        ctx.types,
                        Key::Expression(*key),
                    ) else {
                        return false;
                    };
                    let Some(field_ty) = field_map.get(&key) else {
                        return false;
                    };
                    if field_ty.is_optional {
                        return false;
                    }
                    if !self.is_irrefutable_pattern_for_type_inner(
                        &mut ctx.reborrow(),
                        *pattern,
                        field_ty.ty,
                        visited,
                    ) {
                        return false;
                    }
                }
                PatternField::Positional { .. } | PatternField::Elision => {
                    return false;
                }
            }
        }

        true
    }

    /// Resolve fixed sequence element types for tuple and sized array values.
    fn fixed_sequence_element_types(
        &self,
        ctx: &mut TypeContext<'_>,
        value_type_id: LocalTypeId,
    ) -> Option<Vec<LocalTypeId>> {
        let value_type = ctx.types.get_type(value_type_id).clone();
        match value_type {
            Type::Tuple { elements, .. } => {
                if elements
                    .iter()
                    .any(|element| element.is_optional || element.is_rest)
                {
                    return None;
                }

                let types = elements.iter().map(|element| element.ty).collect();
                Some(types)
            }
            Type::ArraySized { element, count, .. } => {
                let length = self.fixed_array_count_value(count, ctx.types)?;
                let mut elements = Vec::with_capacity(length);
                for _ in 0..length {
                    elements.push(element);
                }
                Some(elements)
            }
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::TypeAlias => {
                let target_id =
                    self.committed_alias_target_type_id_for_symbol(ctx.symbol_type_view(), symbol)?;
                let target_id = ctx.types.unwrap_value_type_id(target_id);
                self.fixed_sequence_element_types(&mut ctx.reborrow(), target_id)
            }
            Type::Value { value } => self.fixed_sequence_element_types(&mut ctx.reborrow(), value),
            _ => None,
        }
    }

    /// Resolve a fixed array count to a literal length.
    fn fixed_array_count_value(&self, count: LocalTypeId, types: &TypeTable) -> Option<usize> {
        let count_id = types.unwrap_value_type_id(count);
        let value = self.integer_literal_value_for_type_id(count_id, types)?;
        usize::try_from(value).ok()
    }

    /// Resolve a structural object field map for a value type.
    fn object_field_map_for_type(
        &self,
        ctx: &mut TypeContext<'_>,
        value_type_id: LocalTypeId,
    ) -> Option<HashMap<StaticKey, TypeField>> {
        let value_type = ctx.types.get_type(value_type_id).clone();
        match value_type {
            Type::Object { fields, .. } => {
                let mut map = HashMap::new();
                for field in &fields {
                    map.insert(field.key, field.clone());
                }
                Some(map)
            }
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::TypeAlias => {
                let target_id =
                    self.committed_alias_target_type_id_for_symbol(ctx.symbol_type_view(), symbol)?;
                let target_id = ctx.types.unwrap_value_type_id(target_id);
                self.object_field_map_for_type(&mut ctx.reborrow(), target_id)
            }
            Type::Value { value } => self.object_field_map_for_type(&mut ctx.reborrow(), value),
            _ => None,
        }
    }

    /// Resolve an object type id for a tagged pattern symbol.
    fn object_type_id_for_tag_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        tag_symbol: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        match tag_symbol.ty() {
            SymbolType::Struct | SymbolType::Interface | SymbolType::Class => {
                ctx.types.get_instance_type_id(tag_symbol)
            }
            SymbolType::Newtype | SymbolType::TypeAlias => {
                let target_id = self.committed_alias_target_type_id_for_symbol(
                    ctx.symbol_type_view(),
                    tag_symbol,
                )?;
                let target_id = ctx.types.unwrap_value_type_id(target_id);
                self.object_type_id_for_type(ctx.types, target_id)
            }
            _ => None,
        }
    }

    /// Resolve an object type id from a structural value.
    fn object_type_id_for_type(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        match types.get_type(type_id) {
            Type::Object { .. } => Some(type_id),
            Type::Reference { symbol, .. } => types.get_instance_type_id(*symbol),
            Type::Value { value } => self.object_type_id_for_type(types, *value),
            _ => None,
        }
    }

    /// Build a field map for a concrete object type id.
    fn object_field_map_for_object_type(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<HashMap<StaticKey, TypeField>> {
        let Type::Object { fields, .. } = types.get_type(type_id) else {
            return None;
        };

        let mut map = HashMap::new();
        for field in fields {
            map.insert(field.key, field.clone());
        }
        Some(map)
    }

    /// Check whether a value type resolves to a tagged pattern symbol.
    fn value_type_matches_tag_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        value_type_id: LocalTypeId,
        tag_symbol: GlobalSymbolId,
    ) -> bool {
        let mut visited = HashSet::new();
        let mut current_id = value_type_id;

        loop {
            current_id = ctx.types.unwrap_value_type_id(current_id);
            if !visited.insert(current_id) {
                return false;
            }

            match ctx.types.get_type(current_id) {
                Type::Reference { symbol, .. } => {
                    if *symbol == tag_symbol {
                        return true;
                    }

                    if matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype) {
                        let Some(target_id) = self.committed_alias_target_type_id_for_symbol(
                            ctx.symbol_type_view(),
                            *symbol,
                        ) else {
                            return false;
                        };
                        current_id = target_id;
                        continue;
                    }

                    return false;
                }
                Type::Value { value } => {
                    current_id = *value;
                }
                _ => return false,
            }
        }
    }

    /// Resolve the exhaustiveness target for a match value.
    fn match_exhaustiveness_target(
        &self,
        ctx: &mut TypeContext<'_>,
        value_type_id: LocalTypeId,
    ) -> Option<MatchExhaustiveTarget> {
        // enum exhaustiveness
        let value_type = ctx.types.get_type(value_type_id);
        if let Some(enum_symbol) = self.enum_symbol_for_type(value_type, ctx.types) {
            let enum_fields = self.enum_field_symbols_for_enum(ctx.type_view(), enum_symbol);
            if !enum_fields.is_empty() {
                let field_set = enum_fields.iter().copied().collect();
                return Some(MatchExhaustiveTarget::Enum {
                    symbol: enum_symbol,
                    fields: enum_fields,
                    field_set,
                });
            }
        }

        // literal unions
        if let Some(values) = self.literal_union_values_for_type(value_type_id, ctx.types) {
            return Some(MatchExhaustiveTarget::LiteralUnion { values });
        }

        // discriminated unions
        if let Some((key, values)) =
            self.discriminant_union_values_for_type(value_type_id, ctx.types)
        {
            return Some(MatchExhaustiveTarget::DiscriminantUnion { key, values });
        }

        // tuple discriminated unions
        if let Some((index, values)) =
            self.tuple_discriminant_union_values_for_type(value_type_id, ctx.types)
        {
            return Some(MatchExhaustiveTarget::TupleDiscriminantUnion { index, values });
        }

        None
    }

    /// Resolve tuple discriminant values for a union when possible.
    fn tuple_discriminant_union_values_for_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<(usize, HashSet<MatchLiteral>)> {
        match types.get_type(type_id) {
            Type::Reference { symbol, .. } => {
                if symbol.ty() == SymbolType::TypeAlias
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.tuple_discriminant_union_values_for_type(target, types);
                }
                if symbol.ty() == SymbolType::Newtype
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.tuple_discriminant_union_values_for_type(target, types);
                }
                if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                    if instance_id == type_id {
                        None
                    } else {
                        self.tuple_discriminant_union_values_for_type(instance_id, types)
                    }
                } else {
                    None
                }
            }
            Type::Value { value } => self.tuple_discriminant_union_values_for_type(*value, types),
            Type::Union { elements } => {
                // collect tuple element lists for each union member
                let mut tuple_elements = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element_types =
                        self.fixed_tuple_element_types_for_discriminant(*element_id, types)?;
                    tuple_elements.push(element_types);
                }

                // require at least one tuple member
                let first = tuple_elements.first()?;
                let tuple_length = first.len();
                if tuple_length == 0 {
                    return None;
                }

                // all members must have the same tuple arity
                if tuple_elements
                    .iter()
                    .any(|elements| elements.len() != tuple_length)
                {
                    return None;
                }

                // pick the first index where every member has a literal
                for index in 0..tuple_length {
                    let mut values = HashSet::new();
                    let mut is_candidate = true;
                    for element_types in &tuple_elements {
                        let Some(literal) =
                            self.match_literal_from_type(types, element_types[index])
                        else {
                            is_candidate = false;
                            break;
                        };
                        values.insert(literal);
                    }
                    if is_candidate && !values.is_empty() {
                        return Some((index, values));
                    }
                }

                None
            }
            _ => None,
        }
    }

    /// Resolve fixed tuple element types for tuple union discriminants.
    fn fixed_tuple_element_types_for_discriminant(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        match types.get_type(type_id) {
            Type::Tuple { elements, .. } => {
                if elements
                    .iter()
                    .any(|element| element.is_optional || element.is_rest)
                {
                    return None;
                }

                Some(elements.iter().map(|element| element.ty).collect())
            }
            Type::Reference { symbol, .. } => {
                if symbol.ty() == SymbolType::TypeAlias
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.fixed_tuple_element_types_for_discriminant(target, types);
                }
                if symbol.ty() == SymbolType::Newtype
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.fixed_tuple_element_types_for_discriminant(target, types);
                }
                None
            }
            Type::Value { value } => self.fixed_tuple_element_types_for_discriminant(*value, types),
            _ => None,
        }
    }

    /// Extract literal union values when possible.
    fn literal_union_values_for_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<HashSet<MatchLiteral>> {
        let ty = types.get_type(type_id);
        match ty {
            // treat boolean as finite literal union
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            } => {
                let mut set = HashSet::new();
                set.insert(MatchLiteral::Boolean(true));
                set.insert(MatchLiteral::Boolean(false));
                Some(set)
            }
            Type::TypeLiteral { value } => {
                let literal = self.match_literal_from_type_literal(value)?;
                let mut set = HashSet::new();
                set.insert(literal);
                Some(set)
            }
            Type::Union { elements } => {
                let mut set = HashSet::new();
                for element_id in elements {
                    let element = types.get_type(*element_id);
                    match element {
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                        } => {
                            set.insert(MatchLiteral::Boolean(true));
                            set.insert(MatchLiteral::Boolean(false));
                        }
                        Type::TypeLiteral { value } => {
                            let literal = self.match_literal_from_type_literal(value)?;
                            set.insert(literal);
                        }
                        _ => return None,
                    };
                }
                if set.is_empty() { None } else { Some(set) }
            }
            _ => None,
        }
    }

    /// Resolve discriminant values for a union when possible.
    fn discriminant_union_values_for_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<(StaticKey, HashSet<MatchLiteral>)> {
        match types.get_type(type_id) {
            Type::Reference { symbol, .. } => {
                if symbol.ty() == SymbolType::TypeAlias
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.discriminant_union_values_for_type(target, types);
                }
                if symbol.ty() == SymbolType::Newtype
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.discriminant_union_values_for_type(target, types);
                }
                if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                    if instance_id == type_id {
                        None
                    } else {
                        self.discriminant_union_values_for_type(instance_id, types)
                    }
                } else {
                    None
                }
            }
            Type::Value { value } => self.discriminant_union_values_for_type(*value, types),
            Type::Union { elements } => {
                // continue below
                // collect discriminant maps for each union element
                let mut maps = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let map = self.discriminant_fields_for_type(types, *element_id)?;
                    maps.push(map);
                }

                let (first, rest) = maps.split_first()?;

                // pick the first key shared by every element
                let mut candidate_keys: Vec<StaticKey> = first.keys().copied().collect();
                candidate_keys.retain(|key| rest.iter().all(|map| map.contains_key(key)));
                let discriminant_key = candidate_keys.into_iter().next()?;

                let mut values = HashSet::new();
                for map in maps {
                    if let Some(value) = map.get(&discriminant_key) {
                        values.insert(*value);
                    } else {
                        return None;
                    }
                }

                if values.is_empty() {
                    None
                } else {
                    Some((discriminant_key, values))
                }
            }
            _ => None,
        }
    }

    /// Collect discriminant literal fields for an object-like type.
    fn discriminant_fields_for_type(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<HashMap<StaticKey, MatchLiteral>> {
        let mut visited = HashSet::new();
        self.discriminant_fields_for_type_inner(types, type_id, &mut visited)
    }

    /// Collect discriminant fields with recursion and alias expansion.
    fn discriminant_fields_for_type_inner(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Option<HashMap<StaticKey, MatchLiteral>> {
        // stop recursion on cycles
        if !visited.insert(type_id) {
            return Some(HashMap::new());
        }

        match types.get_type(type_id) {
            Type::Reference { symbol, .. } => {
                if symbol.ty() == SymbolType::TypeAlias
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.discriminant_fields_for_type_inner(types, target, visited);
                }

                if let Some(instance_id) = types.get_instance_type_id(*symbol) {
                    return self.discriminant_fields_for_type_inner(types, instance_id, visited);
                }

                None
            }
            Type::Object { fields, .. } => {
                let mut map = HashMap::new();
                for field in fields {
                    if field.is_optional {
                        continue;
                    }

                    if let Some(literal) = self.match_literal_from_type(types, field.ty) {
                        map.insert(field.key, literal);
                    }
                }

                Some(map)
            }
            Type::Intersection { elements } => {
                let mut maps = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let map =
                        self.discriminant_fields_for_type_inner(types, *element_id, visited)?;
                    maps.push(map);
                }

                let Some((first, rest)) = maps.split_first() else {
                    return Some(HashMap::new());
                };
                let mut merged = first.clone();
                for map in rest {
                    merged.retain(|key, literal| {
                        map.get(key).is_some_and(|other| *other == *literal)
                    });
                }

                Some(merged)
            }
            Type::Value { value } => {
                self.discriminant_fields_for_type_inner(types, *value, visited)
            }
            _ => None,
        }
    }

    /// Summarize coverage for a single enum pattern.
    fn enum_pattern_coverage(
        &self,
        ctx: &mut TypeContext<'_>,
        enum_symbol: GlobalSymbolId,
        pattern_id: LocalNodeId<Pattern>,
        enum_fields: &HashSet<GlobalSymbolId>,
    ) -> Option<MatchPatternCoverage<GlobalSymbolId>> {
        match ctx.tree.get(pattern_id) {
            Pattern::Wildcard => Some(MatchPatternCoverage::All),
            Pattern::Binding { name, pattern, .. } => match pattern {
                Some(pattern) => self.enum_pattern_coverage(
                    &mut ctx.reborrow(),
                    enum_symbol,
                    *pattern,
                    enum_fields,
                ),
                None => {
                    if let Some(field_symbol) =
                        self.query_enum_field_symbol_for_name(ctx.type_view(), enum_symbol, *name)
                    {
                        Some(MatchPatternCoverage::Values(vec![field_symbol]))
                    } else {
                        Some(MatchPatternCoverage::All)
                    }
                }
            },
            Pattern::Union { patterns } => {
                let mut covered_fields: HashSet<GlobalSymbolId> = HashSet::new();
                for pattern in patterns {
                    let coverage = self.enum_pattern_coverage(
                        &mut ctx.reborrow(),
                        enum_symbol,
                        *pattern,
                        enum_fields,
                    )?;
                    match coverage {
                        MatchPatternCoverage::All => return Some(MatchPatternCoverage::All),
                        MatchPatternCoverage::Values(fields) => {
                            covered_fields.extend(fields);
                        }
                    }
                }
                Some(MatchPatternCoverage::Values(
                    covered_fields.into_iter().collect(),
                ))
            }
            Pattern::TaggedTuple { ty, .. } | Pattern::TaggedObject { ty, .. } => {
                let field_symbol = self.enum_field_symbol_for_pattern_type(
                    &mut ctx.reborrow(),
                    enum_symbol,
                    *ty,
                    enum_fields,
                )?;
                Some(MatchPatternCoverage::Values(vec![field_symbol]))
            }
            Pattern::Expression { value } => {
                let field_symbol = self.enum_field_symbol_for_pattern_value(
                    &mut ctx.reborrow(),
                    enum_symbol,
                    *value,
                    enum_fields,
                )?;
                Some(MatchPatternCoverage::Values(vec![field_symbol]))
            }
            _ => None,
        }
    }

    /// Summarize coverage for a literal union pattern.
    fn literal_pattern_coverage(
        &self,
        pattern_id: LocalNodeId<Pattern>,
        tree: &NodeTree,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match tree.get(pattern_id) {
            Pattern::Wildcard => Some(MatchPatternCoverage::All),
            Pattern::Binding { pattern, .. } => match pattern {
                Some(pattern) => self.literal_pattern_coverage(*pattern, tree),
                None => Some(MatchPatternCoverage::All),
            },
            Pattern::Union { patterns } => {
                let mut covered: HashSet<MatchLiteral> = HashSet::new();
                for pattern in patterns {
                    let coverage = self.literal_pattern_coverage(*pattern, tree)?;
                    match coverage {
                        MatchPatternCoverage::All => return Some(MatchPatternCoverage::All),
                        MatchPatternCoverage::Values(values) => covered.extend(values),
                    }
                }
                Some(MatchPatternCoverage::Values(covered.into_iter().collect()))
            }
            Pattern::Expression { value } => {
                let literal = self.match_literal_from_expression(*value, tree)?;
                Some(MatchPatternCoverage::Values(vec![literal]))
            }
            _ => None,
        }
    }

    /// Summarize coverage for a discriminated union pattern.
    fn discriminant_pattern_coverage(
        &self,
        ctx: &mut TypeContext<'_>,
        pattern_id: LocalNodeId<Pattern>,
        key: StaticKey,
        values: &HashSet<MatchLiteral>,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match ctx.tree.get(pattern_id) {
            Pattern::Wildcard => Some(MatchPatternCoverage::All),
            Pattern::Binding { pattern, .. } => match pattern {
                Some(pattern) => {
                    self.discriminant_pattern_coverage(&mut ctx.reborrow(), *pattern, key, values)
                }
                None => Some(MatchPatternCoverage::All),
            },
            Pattern::Union { patterns } => {
                let mut covered: HashSet<MatchLiteral> = HashSet::new();
                for pattern in patterns {
                    let coverage = self.discriminant_pattern_coverage(
                        &mut ctx.reborrow(),
                        *pattern,
                        key,
                        values,
                    )?;
                    match coverage {
                        MatchPatternCoverage::All => return Some(MatchPatternCoverage::All),
                        MatchPatternCoverage::Values(values) => covered.extend(values),
                    }
                }
                Some(MatchPatternCoverage::Values(covered.into_iter().collect()))
            }
            Pattern::Object { fields } => {
                let coverage = if let Some(field_id) =
                    self.discriminant_pattern_field(&mut ctx.reborrow(), key, fields)
                {
                    self.discriminant_pattern_field_coverage(field_id, ctx.tree)?
                } else {
                    MatchPatternCoverage::All
                };
                self.filter_literal_coverage(values, coverage)
            }
            Pattern::TaggedObject { ty, fields } => {
                // prefer explicit discriminant fields in the pattern
                if let Some(field_id) =
                    self.discriminant_pattern_field(&mut ctx.reborrow(), key, fields)
                {
                    let coverage = self.discriminant_pattern_field_coverage(field_id, ctx.tree)?;
                    return self.filter_literal_coverage(values, coverage);
                }

                // fall back to the discriminant value on the tag type
                let tag_symbol =
                    self.reference_symbol_for_type_expression(ctx.tree_symbol_view(), *ty)?;
                let tag_type_id = ctx.types.get_instance_type_id(tag_symbol)?;
                let discriminants = self.discriminant_fields_for_type(ctx.types, tag_type_id)?;
                let literal = *discriminants.get(&key)?;
                let coverage = MatchPatternCoverage::Values(vec![literal]);
                self.filter_literal_coverage(values, coverage)
            }
            _ => None,
        }
    }

    /// Summarize coverage for tuple discriminant patterns.
    fn tuple_discriminant_pattern_coverage(
        &self,
        pattern_id: LocalNodeId<Pattern>,
        index: usize,
        values: &HashSet<MatchLiteral>,
        tree: &NodeTree,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match tree.get(pattern_id) {
            Pattern::Wildcard => Some(MatchPatternCoverage::All),
            Pattern::Binding { pattern, .. } => match pattern {
                Some(pattern) => {
                    self.tuple_discriminant_pattern_coverage(*pattern, index, values, tree)
                }
                None => Some(MatchPatternCoverage::All),
            },
            Pattern::Union { patterns } => {
                let mut covered: HashSet<MatchLiteral> = HashSet::new();
                for pattern in patterns {
                    let coverage =
                        self.tuple_discriminant_pattern_coverage(*pattern, index, values, tree)?;
                    match coverage {
                        MatchPatternCoverage::All => return Some(MatchPatternCoverage::All),
                        MatchPatternCoverage::Values(values) => covered.extend(values),
                    }
                }
                Some(MatchPatternCoverage::Values(covered.into_iter().collect()))
            }
            Pattern::Tuple { fields } | Pattern::Array { fields } => {
                let field_id = fields.get(index)?;
                let coverage = self.tuple_discriminant_field_coverage(*field_id, tree)?;
                self.filter_literal_coverage(values, coverage)
            }
            _ => None,
        }
    }

    /// Resolve discriminant literal coverage for a tuple field.
    fn tuple_discriminant_field_coverage(
        &self,
        field_id: LocalNodeId<PatternField>,
        tree: &NodeTree,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match tree.get(field_id) {
            PatternField::Positional { pattern, .. } => {
                self.literal_pattern_coverage(*pattern, tree)
            }
            PatternField::Named { pattern, .. } => {
                if let Some(pattern_id) = pattern {
                    self.literal_pattern_coverage(*pattern_id, tree)
                } else {
                    Some(MatchPatternCoverage::All)
                }
            }
            PatternField::Elision => Some(MatchPatternCoverage::All),
            PatternField::Computed { .. } | PatternField::Spread { .. } => None,
        }
    }

    /// Filter literal coverage to known union values.
    fn filter_literal_coverage(
        &self,
        values: &HashSet<MatchLiteral>,
        coverage: MatchPatternCoverage<MatchLiteral>,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match coverage {
            MatchPatternCoverage::All => Some(MatchPatternCoverage::All),
            MatchPatternCoverage::Values(items) => {
                let mut filtered = Vec::new();
                for item in items {
                    if values.contains(&item) {
                        filtered.push(item);
                    }
                }
                if filtered.is_empty() {
                    None
                } else {
                    Some(MatchPatternCoverage::Values(filtered))
                }
            }
        }
    }

    /// Resolve the discriminant field for an object pattern.
    fn discriminant_pattern_field(
        &self,
        ctx: &mut TypeContext<'_>,
        key: StaticKey,
        fields: &[LocalNodeId<PatternField>],
    ) -> Option<LocalNodeId<PatternField>> {
        // locate the matching field by key
        for field_id in fields {
            let field_key = match ctx.tree.get(*field_id) {
                PatternField::Named { name, .. } => Some(StaticKey::Name(*name)),
                PatternField::Computed { key: field_key, .. } => self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    Key::Expression(*field_key),
                ),
                _ => None,
            };

            if let Some(field_key) = field_key
                && field_key.matches(&key)
            {
                return Some(*field_id);
            }
        }

        None
    }

    /// Resolve literal coverage for a discriminant pattern field.
    fn discriminant_pattern_field_coverage(
        &self,
        field_id: LocalNodeId<PatternField>,
        tree: &NodeTree,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match tree.get(field_id) {
            PatternField::Named { pattern, .. } => {
                let Some(pattern_id) = *pattern else {
                    return Some(MatchPatternCoverage::All);
                };
                self.literal_pattern_coverage(pattern_id, tree)
            }
            PatternField::Computed { pattern, .. } => self.literal_pattern_coverage(*pattern, tree),
            _ => None,
        }
    }

    /// Extract a literal value from an expression.
    fn match_literal_from_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<MatchLiteral> {
        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);
        match tree.get(expression_id) {
            Expression::ScalarLiteral { value } => self.match_literal_from_scalar(value),
            Expression::TypeLiteral { value } => self.match_literal_from_type_literal(value),
            _ => None,
        }
    }

    /// Extract a literal value from a type id.
    fn match_literal_from_type(
        &self,
        types: &TypeTable,
        type_id: LocalTypeId,
    ) -> Option<MatchLiteral> {
        match types.get_type(type_id) {
            Type::TypeLiteral { value } => self.match_literal_from_type_literal(value),
            Type::Reference { symbol, .. } => {
                if symbol.ty() == SymbolType::TypeAlias
                    && let Some(target) = types.get_alias_target_type_id(*symbol)
                {
                    return self.match_literal_from_type(types, target);
                }
                None
            }
            Type::Value { value } => self.match_literal_from_type(types, *value),
            _ => None,
        }
    }

    /// Extract a literal value from a type literal.
    fn match_literal_from_type_literal(&self, value: &TypeLiteral) -> Option<MatchLiteral> {
        match value {
            TypeLiteral::Null => Some(MatchLiteral::Null),
            TypeLiteral::Undefined => Some(MatchLiteral::Undefined),
            TypeLiteral::ScalarLiteral(literal) => self.match_literal_from_scalar(literal),
            _ => None,
        }
    }

    /// Extract a literal value from a scalar literal.
    fn match_literal_from_scalar(&self, literal: &ScalarLiteral) -> Option<MatchLiteral> {
        match literal {
            ScalarLiteral::Null => Some(MatchLiteral::Null),
            ScalarLiteral::Boolean(value) => Some(MatchLiteral::Boolean(*value)),
            ScalarLiteral::Integer(value) => Some(MatchLiteral::Integer(*value)),
            ScalarLiteral::Bigint(value) => Some(MatchLiteral::Bigint(*value)),
            ScalarLiteral::Float(value) => Some(MatchLiteral::Float(value.to_bits())),
            ScalarLiteral::Character(value) => Some(MatchLiteral::Character(*value)),
            ScalarLiteral::String(value) => Some(MatchLiteral::String(*value)),
            ScalarLiteral::RegexString { .. } => None,
        }
    }

    /// Resolve enum field symbols from pattern expressions.
    fn enum_field_symbol_for_pattern_value(
        &self,
        ctx: &mut TypeContext<'_>,
        enum_symbol: GlobalSymbolId,
        expression_id: LocalNodeId<Expression>,
        enum_fields: &HashSet<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        let expression_id = self.unwrap_parenthesized_expression(expression_id, ctx.tree);
        match ctx.tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                let left_id = self.unwrap_parenthesized_expression(*left, ctx.tree);
                let left_symbol =
                    self.reference_symbol_for_expression(ctx.tree_symbol_view(), left_id)?;
                if left_symbol != enum_symbol {
                    return None;
                }
                let name = (*name)?;
                let field_symbol =
                    self.query_enum_field_symbol_for_name(ctx.type_view(), enum_symbol, name)?;
                if enum_fields.contains(&field_symbol) {
                    Some(field_symbol)
                } else {
                    None
                }
            }
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                if enum_fields.contains(target_symbol) {
                    Some(*target_symbol)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Resolve enum field symbols from tagged pattern type expressions.
    fn enum_field_symbol_for_pattern_type(
        &self,
        ctx: &mut TypeContext<'_>,
        enum_symbol: GlobalSymbolId,
        type_expression_id: LocalNodeId<TypeExpression>,
        enum_fields: &HashSet<GlobalSymbolId>,
    ) -> Option<GlobalSymbolId> {
        let type_expression_id =
            self.unwrap_parenthesized_type_expression(type_expression_id, ctx.tree);

        match ctx.tree.get(type_expression_id) {
            TypeExpression::Member { left, name, .. } => {
                let left_id = self.unwrap_parenthesized_type_expression(*left, ctx.tree);
                let left_symbol =
                    self.reference_symbol_for_type_expression(ctx.tree_symbol_view(), left_id)?;
                if left_symbol != enum_symbol {
                    return None;
                }

                let field_symbol =
                    self.query_enum_field_symbol_for_name(ctx.type_view(), enum_symbol, *name)?;
                if enum_fields.contains(&field_symbol) {
                    Some(field_symbol)
                } else {
                    None
                }
            }
            TypeExpression::LocalReference { target_symbol, .. }
            | TypeExpression::ModuleReference { target_symbol, .. }
            | TypeExpression::GlobalReference { target_symbol, .. } => {
                if enum_fields.contains(target_symbol) {
                    Some(*target_symbol)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Validate object pattern spread placement.
    fn validate_object_pattern_spreads(
        &self,
        ctx: &mut TypeContext<'_>,
        fields: &[LocalNodeId<PatternField>],
    ) {
        // locate the first spread field and report duplicates
        let mut spread_index = None;
        for (index, field_id) in fields.iter().enumerate() {
            if matches!(ctx.tree.get(*field_id), PatternField::Spread { .. }) {
                let error_node = field_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                if spread_index.is_some() {
                    self.error(AnalyzeError::ObjectPatternMultipleSpreads { node: error_node });
                } else {
                    spread_index = Some(index);
                }
            }
        }

        // validate rest targets
        if !ctx.module.language_type.is_destack() {
            for field_id in fields {
                let PatternField::Spread { pattern, .. } = ctx.tree.get(*field_id) else {
                    continue;
                };
                let is_identifier = pattern.is_some_and(|pattern_id| {
                    matches!(
                        ctx.tree.get(pattern_id),
                        Pattern::Binding { pattern: None, .. }
                    )
                });
                if !is_identifier {
                    let node = field_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::ObjectPatternRestNotIdentifier { node });
                }
            }
        }

        // spread must be the last field
        if let Some(index) = spread_index
            && index + 1 < fields.len()
        {
            let error_node = fields[index + 1]
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::ObjectPatternSpreadNotLast { node: error_node });
        }
    }

    /// Validate named fields in array and tuple patterns.
    fn validate_sequence_pattern_fields(
        &self,
        ctx: &mut TypeContext<'_>,
        fields: &[LocalNodeId<PatternField>],
    ) {
        // Destack tuple and array patterns may use named fields
        if ctx.module.language_type.is_destack() {
            return;
        }

        // reject named or aliased fields in array and tuple patterns
        for field_id in fields {
            if matches!(ctx.tree.get(*field_id), PatternField::Named { .. }) {
                let node = field_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::InvalidPatternNamedField { node });
                return;
            }
        }
    }

    /// Check whether a pattern contains a definite assignment assertion.
    pub(super) fn pattern_has_definite_assignment(
        &self,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
    ) -> bool {
        let pattern = tree.get(pattern_id);

        // unwrap and scan nested patterns
        match pattern {
            Pattern::Must(_) => true,
            Pattern::Assign { pattern, .. } => self.pattern_has_definite_assignment(tree, *pattern),
            Pattern::ReferenceOf { right, .. } | Pattern::ValueOf { right, .. } => {
                self.pattern_has_definite_assignment(tree, *right)
            }
            Pattern::Binding { pattern, .. } => {
                pattern.is_some_and(|inner| self.pattern_has_definite_assignment(tree, inner))
            }
            Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. }
            | Pattern::Array { fields }
            | Pattern::Object { fields }
            | Pattern::TaggedObject { fields, .. } => fields
                .iter()
                .any(|field_id| self.pattern_field_has_definite_assignment(tree, *field_id)),
            Pattern::Union { patterns } => patterns
                .iter()
                .any(|inner| self.pattern_has_definite_assignment(tree, *inner)),
            Pattern::Wildcard | Pattern::Expression { .. } | Pattern::TypeExpression { .. } => {
                false
            }
        }
    }

    /// Check whether a pattern field contains a definite assignment assertion.
    fn pattern_field_has_definite_assignment(
        &self,
        tree: &NodeTree,
        field_id: LocalNodeId<PatternField>,
    ) -> bool {
        let field = tree.get(field_id);

        // scan nested patterns inside fields
        match field {
            PatternField::Named { pattern, .. } => {
                pattern.is_some_and(|inner| self.pattern_has_definite_assignment(tree, inner))
            }
            PatternField::Computed { pattern, .. } => {
                self.pattern_has_definite_assignment(tree, *pattern)
            }
            PatternField::Positional { pattern, .. } => {
                self.pattern_has_definite_assignment(tree, *pattern)
            }
            PatternField::Elision => false,
            PatternField::Spread { pattern, .. } => {
                pattern.is_some_and(|inner| self.pattern_has_definite_assignment(tree, inner))
            }
        }
    }

    /// Check whether a pattern is a destructuring pattern.
    pub(super) fn is_destructuring_pattern(
        &self,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
    ) -> bool {
        match tree.get(pattern_id) {
            Pattern::Array { .. }
            | Pattern::Object { .. }
            | Pattern::Tuple { .. }
            | Pattern::TaggedTuple { .. }
            | Pattern::TaggedObject { .. } => true,
            Pattern::Assign { pattern, .. } => self.is_destructuring_pattern(tree, *pattern),
            Pattern::Binding {
                pattern: Some(inner),
                ..
            } => self.is_destructuring_pattern(tree, *inner),
            Pattern::Must(inner)
            | Pattern::ReferenceOf { right: inner, .. }
            | Pattern::ValueOf { right: inner, .. } => self.is_destructuring_pattern(tree, *inner),
            _ => false,
        }
    }
}
