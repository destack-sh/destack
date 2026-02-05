use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use crate::{AnalyzeError, Compiler};
use destack_dir::{
    DynamicKey, Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalTypeId, MatchCase,
    MatchSelector, NodeTree, NormalizationMode, Pattern, PatternField, PrimitiveType,
    ScalarLiteral, StaticExpression, StaticKey, StringId, SymbolTable, SymbolType, Type,
    TypeElement, TypeField, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

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
}

/// Pattern analysis helper for match and destructuring validation.
struct PatternAnalysis<'a> {
    /// The compiler providing shared helpers.
    compiler: &'a Compiler,
}

/// Pattern analysis with typed context.
struct PatternAnalysisContext<'a> {
    /// The shared analysis helpers.
    analysis: PatternAnalysis<'a>,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The AST for this module.
    tree: &'a NodeTree,
    /// The symbol table for this module.
    symbols: &'a SymbolTable,
    /// The type table for this module.
    types: &'a mut TypeTable,
}

impl<'a> PatternAnalysis<'a> {
    /// Create a new pattern analysis helper.
    fn new(compiler: &'a Compiler) -> Self {
        Self { compiler }
    }
}

impl<'a> Deref for PatternAnalysis<'a> {
    type Target = Compiler;

    fn deref(&self) -> &Compiler {
        self.compiler
    }
}

impl<'a> PatternAnalysisContext<'a> {
    /// Create a new pattern analysis context.
    fn new(
        analysis: PatternAnalysis<'a>,
        module: &'a Module,
        profile: ProfileId,
        tree: &'a NodeTree,
        symbols: &'a SymbolTable,
        types: &'a mut TypeTable,
    ) -> Self {
        Self {
            analysis,
            module,
            profile,
            tree,
            symbols,
            types,
        }
    }

    /// Validate match exhaustiveness with the current context.
    fn validate_match_exhaustiveness(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
    ) {
        self.validate_match_exhaustiveness_inner(expression_id, value_id, cases);
    }

    /// Check whether a pattern is irrefutable for a given value type.
    fn is_irrefutable_pattern_for_type(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
        value_type: LocalTypeId,
    ) -> bool {
        self.analysis.is_irrefutable_pattern_for_type(
            self.module,
            self.profile,
            pattern_id,
            value_type,
            self.tree,
            self.symbols,
            self.types,
        )
    }

    /// Check if a match contains an irrefutable pattern for the given type.
    fn match_has_irrefutable_pattern(
        &mut self,
        value_type_id: LocalTypeId,
        cases: &[LocalNodeId<MatchCase>],
    ) -> bool {
        self.analysis.match_has_irrefutable_pattern(
            self.module,
            self.profile,
            value_type_id,
            cases,
            self.tree,
            self.symbols,
            self.types,
        )
    }

    /// Resolve the exhaustiveness target for a match value.
    fn match_exhaustiveness_target(
        &mut self,
        value_type_id: LocalTypeId,
    ) -> Option<MatchExhaustiveTarget> {
        self.analysis.match_exhaustiveness_target(
            self.module,
            self.profile,
            value_type_id,
            self.tree,
            self.symbols,
            self.types,
        )
    }

    /// Summarize coverage for a single enum pattern.
    fn enum_pattern_coverage(
        &mut self,
        enum_symbol: GlobalSymbolId,
        pattern_id: LocalNodeId<Pattern>,
        enum_fields: &HashSet<GlobalSymbolId>,
    ) -> Option<MatchPatternCoverage<GlobalSymbolId>> {
        self.analysis.enum_pattern_coverage(
            self.module,
            self.profile,
            enum_symbol,
            pattern_id,
            enum_fields,
            self.tree,
            self.symbols,
        )
    }

    /// Summarize coverage for a literal pattern.
    fn literal_pattern_coverage_for_values(
        &self,
        values: &HashSet<MatchLiteral>,
        pattern_id: LocalNodeId<Pattern>,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        self.analysis
            .literal_pattern_coverage_for_values(values, pattern_id, self.tree)
    }

    /// Summarize coverage for a discriminant pattern.
    fn discriminant_pattern_coverage(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
        key: StaticKey,
        values: &HashSet<MatchLiteral>,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        self.analysis.discriminant_pattern_coverage(
            self.module,
            self.profile,
            pattern_id,
            key,
            values,
            self.tree,
            self.symbols,
            self.types,
        )
    }

    /// Validate match exhaustiveness for supported value shapes.
    fn validate_match_exhaustiveness_inner(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
    ) {
        // scan selectors for fallback and guard usage
        let mut has_fallback = false;
        let mut has_guard = false;
        for case_id in cases {
            let selector = match self.tree.get(*case_id) {
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
        let value_type_id = self
            .types
            .get_inferred_type_id(value_id.into_global_any(self.module.id))
            .or_else(|| {
                let symbol = self.analysis.reference_symbol_for_expression(
                    self.module,
                    value_id,
                    self.profile,
                    self.tree,
                    self.symbols,
                )?;
                self.types.get_value_type_id(symbol)
            });
        let Some(value_type_id) = value_type_id else {
            return;
        };
        let value_type_id = self.analysis.unwrap_type_value(value_type_id, self.types);
        let value_type_id = match self.types.get_type(value_type_id) {
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Void => self
                .types
                .get_value_type_id(*symbol)
                .unwrap_or(value_type_id),
            _ => value_type_id,
        };
        // preserve nominal types for irrefutable checks
        let irrefutable_type_id = value_type_id;
        let normalized_type_id = self.analysis.normalize_type(
            self.module,
            self.profile,
            value_type_id,
            self.symbols,
            self.types,
            NormalizationMode::Flow,
        );

        // allow irrefutable patterns to satisfy exhaustiveness
        if self.match_has_irrefutable_pattern(irrefutable_type_id, cases) {
            return;
        }

        // guards require a fallback because exhaustiveness cannot be proven
        if has_guard {
            let node = expression_id
                .into_global_any(self.module.id)
                .into_anchored(Some(self.profile));
            self.analysis
                .error(AnalyzeError::NonExhaustiveMatch { node });
            return;
        }

        // resolve the exhaustiveness target
        let target = self
            .match_exhaustiveness_target(irrefutable_type_id)
            .or_else(|| self.match_exhaustiveness_target(normalized_type_id));
        let Some(target) = target else {
            let node = expression_id
                .into_global_any(self.module.id)
                .into_anchored(Some(self.profile));
            self.analysis
                .error(AnalyzeError::NonExhaustiveMatch { node });
            return;
        };

        // track covered entries based on the target
        let mut covered_literals: HashSet<MatchLiteral> = HashSet::new();
        let mut covered_fields: HashSet<GlobalSymbolId> = HashSet::new();
        let mut is_provable = true;
        for case_id in cases {
            let selector = match self.tree.get(*case_id) {
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
                    let coverage = self.enum_pattern_coverage(*symbol, *pattern, field_set);
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
                    let coverage = self.literal_pattern_coverage_for_values(values, *pattern);
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
                    let coverage = self.discriminant_pattern_coverage(*pattern, *key, values);
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
                .into_global_any(self.module.id)
                .into_anchored(Some(self.profile));
            self.analysis
                .error(AnalyzeError::NonExhaustiveMatch { node });
            return;
        }

        // check for missing fields
        let is_exhaustive = match target {
            MatchExhaustiveTarget::Enum { fields, .. } => covered_fields.len() == fields.len(),
            MatchExhaustiveTarget::LiteralUnion { values } => covered_literals == values,
            MatchExhaustiveTarget::DiscriminantUnion { values, .. } => covered_literals == values,
        };
        if is_exhaustive {
            return;
        }

        let node = expression_id
            .into_global_any(self.module.id)
            .into_anchored(Some(self.profile));
        self.analysis
            .error(AnalyzeError::NonExhaustiveMatch { node });
    }
}

impl<'a> PatternAnalysis<'a> {
    /// Validate a single pattern node.
    pub(super) fn validate_pattern(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        pattern: &Pattern,
    ) {
        match pattern {
            Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => {
                self.validate_object_pattern_spreads(module, profile, tree, fields);
            }
            Pattern::Array { fields }
            | Pattern::Tuple { fields }
            | Pattern::TaggedTuple { fields, .. } => {
                self.validate_sequence_pattern_fields(module, profile, tree, fields);
            }
            _ => {}
        }
    }

    /// Check if a match contains an irrefutable pattern for the given type.
    fn match_has_irrefutable_pattern(
        &self,
        module: &Module,
        profile: ProfileId,
        value_type_id: LocalTypeId,
        cases: &[LocalNodeId<MatchCase>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        for case_id in cases {
            let selector = match tree.get(*case_id) {
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

            if self.is_irrefutable_pattern_for_type(
                module,
                profile,
                *pattern,
                value_type_id,
                tree,
                symbols,
                types,
            ) {
                return true;
            }
        }

        false
    }

    /// Check whether a pattern matches all values of the given type.
    pub(crate) fn is_irrefutable_pattern_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        value_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let mut visited = HashSet::new();
        self.is_irrefutable_pattern_for_type_inner(
            module,
            profile,
            pattern_id,
            value_type_id,
            tree,
            symbols,
            types,
            &mut visited,
        )
    }

    /// Check whether a pattern matches all values of the given type.
    fn is_irrefutable_pattern_for_type_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        value_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let value_type_id = types.unwrap_value_type_id(value_type_id);
        if !visited.insert(value_type_id) {
            return false;
        }

        match tree.get(pattern_id) {
            Pattern::Wildcard => true,
            Pattern::Binding { pattern, .. } => match pattern {
                None => {
                    let value_type = types.get_type(value_type_id);
                    if let Some(enum_symbol) = self.enum_symbol_for_type(value_type, types) {
                        let Pattern::Binding { name, .. } = tree.get(pattern_id) else {
                            return true;
                        };
                        if self
                            .enum_field_symbol_for_name(
                                module,
                                profile,
                                enum_symbol,
                                *name,
                                tree,
                                symbols,
                            )
                            .is_some()
                        {
                            return false;
                        }
                    }

                    true
                }
                Some(inner) => self.is_irrefutable_pattern_for_type_inner(
                    module,
                    profile,
                    *inner,
                    value_type_id,
                    tree,
                    symbols,
                    types,
                    visited,
                ),
            },
            Pattern::ReferenceOf { right, .. } => {
                let Type::ReferenceOf { right: inner, .. } = types.get_type(value_type_id) else {
                    return false;
                };
                self.is_irrefutable_pattern_for_type_inner(
                    module, profile, *right, *inner, tree, symbols, types, visited,
                )
            }
            Pattern::ValueOf { right, .. } => {
                let Type::ValueOf { right: inner, .. } = types.get_type(value_type_id) else {
                    return false;
                };
                self.is_irrefutable_pattern_for_type_inner(
                    module, profile, *right, *inner, tree, symbols, types, visited,
                )
            }
            Pattern::Union { patterns } => patterns.iter().any(|inner| {
                self.is_irrefutable_pattern_for_type_inner(
                    module,
                    profile,
                    *inner,
                    value_type_id,
                    tree,
                    symbols,
                    types,
                    visited,
                )
            }),
            Pattern::Tuple { fields } => self.is_irrefutable_sequence_pattern_for_type(
                module,
                profile,
                fields,
                value_type_id,
                pattern_id.into_any(),
                |rest_types| Type::Tuple {
                    elements: rest_types.into_iter().map(TypeElement::new).collect(),
                    is_readonly: false,
                },
                tree,
                symbols,
                types,
                visited,
            ),
            Pattern::Array { fields } => self.is_irrefutable_sequence_pattern_for_type(
                module,
                profile,
                fields,
                value_type_id,
                pattern_id.into_any(),
                |rest_types| Type::Array {
                    element: rest_types.first().cloned(),
                    is_readonly: false,
                },
                tree,
                symbols,
                types,
                visited,
            ),
            Pattern::TaggedTuple { ty, fields } => self.is_irrefutable_tagged_tuple_pattern(
                module,
                profile,
                *ty,
                fields,
                value_type_id,
                tree,
                symbols,
                types,
                visited,
            ),
            Pattern::Object { fields } => self.is_irrefutable_object_pattern_for_type(
                module,
                profile,
                fields,
                value_type_id,
                pattern_id.into_any(),
                tree,
                symbols,
                types,
                visited,
            ),
            Pattern::TaggedObject { ty, fields } => self.is_irrefutable_tagged_object_pattern(
                module,
                profile,
                *ty,
                fields,
                value_type_id,
                tree,
                symbols,
                types,
                visited,
            ),
            Pattern::Must(_) | Pattern::Expression { .. } | Pattern::Range { .. } => false,
        }
    }

    /// Check if a sequence pattern matches all values of a fixed-size sequence type.
    fn is_irrefutable_sequence_pattern_for_type<F>(
        &self,
        module: &Module,
        profile: ProfileId,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        to_rest_type: F,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool
    where
        F: Fn(Vec<LocalTypeId>) -> Type,
    {
        // resolve fixed element types for the matched value
        let Some(element_types) = self.fixed_sequence_element_types(
            module,
            profile,
            value_type_id,
            source_id,
            tree,
            symbols,
            types,
        ) else {
            return false;
        };

        // locate a rest field when present
        let mut rest_index = None;
        for (index, field_id) in fields.iter().enumerate() {
            if matches!(tree.get(*field_id), PatternField::Spread { .. }) {
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
            let field = tree.get(*field_id);
            match field {
                PatternField::Positional { pattern } => {
                    if !self.is_irrefutable_pattern_for_type_inner(
                        module,
                        profile,
                        *pattern,
                        *element_type_id,
                        tree,
                        symbols,
                        types,
                        visited,
                    ) {
                        return false;
                    }
                }
                PatternField::Named { pattern, .. } => {
                    if let Some(pattern_id) = pattern
                        && !self.is_irrefutable_pattern_for_type_inner(
                            module,
                            profile,
                            *pattern_id,
                            *element_type_id,
                            tree,
                            symbols,
                            types,
                            visited,
                        )
                    {
                        return false;
                    }
                }
                PatternField::Alias { .. } | PatternField::Elision => {}
                PatternField::Computed { .. } | PatternField::Spread { .. } => {
                    return false;
                }
            }
        }

        // validate the rest binding when present
        if let Some(rest_index) = rest_index {
            // build the rest type from remaining elements
            let rest_types = element_types[rest_index..].to_vec();
            let rest_type = to_rest_type(rest_types);
            let rest_type_id = types.insert_type_from_any(rest_type, source_id);

            // validate the rest pattern when provided
            let PatternField::Spread { pattern, .. } = tree.get(fields[rest_index]) else {
                return false;
            };
            if let Some(pattern_id) = pattern
                && !self.is_irrefutable_pattern_for_type_inner(
                    module,
                    profile,
                    *pattern_id,
                    rest_type_id,
                    tree,
                    symbols,
                    types,
                    visited,
                )
            {
                return false;
            }
        }

        true
    }

    /// Check whether a tagged tuple pattern is irrefutable for a nominal type.
    fn is_irrefutable_tagged_tuple_pattern(
        &self,
        module: &Module,
        profile: ProfileId,
        ty: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(tag_symbol) =
            self.reference_symbol_for_expression(module, ty, profile, tree, symbols)
        else {
            return false;
        };

        if !self.value_type_matches_tag_symbol(
            module,
            profile,
            value_type_id,
            tag_symbol,
            ty.into_any(),
            tree,
            symbols,
            types,
        ) {
            return false;
        }

        if tag_symbol.ty() != SymbolType::Newtype {
            return false;
        }

        let Some(target_type_id) = self.alias_target_type_id_for_symbol(
            module,
            profile,
            tag_symbol,
            ty.into_any(),
            symbols,
            types,
        ) else {
            return false;
        };

        let target_type_id = types.unwrap_value_type_id(target_type_id);

        // scalar newtypes use a single field
        if fields.len() == 1 {
            let field = tree.get(fields[0]);
            let nested_pattern = match field {
                PatternField::Positional { pattern } => Some(*pattern),
                PatternField::Named { pattern, .. } => *pattern,
                PatternField::Alias { .. } | PatternField::Elision => None,
                PatternField::Computed { .. } | PatternField::Spread { .. } => return false,
            };

            if let Some(pattern_id) = nested_pattern {
                return self.is_irrefutable_pattern_for_type_inner(
                    module,
                    profile,
                    pattern_id,
                    target_type_id,
                    tree,
                    symbols,
                    types,
                    visited,
                );
            }

            return true;
        }

        self.is_irrefutable_sequence_pattern_for_type(
            module,
            profile,
            fields,
            target_type_id,
            ty.into_any(),
            |rest_types| Type::Tuple {
                elements: rest_types.into_iter().map(TypeElement::new).collect(),
                is_readonly: false,
            },
            tree,
            symbols,
            types,
            visited,
        )
    }

    /// Check whether an object pattern is irrefutable for a structural object type.
    fn is_irrefutable_object_pattern_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(field_map) = self.object_field_map_for_type(
            module,
            profile,
            value_type_id,
            source_id,
            tree,
            symbols,
            types,
        ) else {
            return false;
        };

        self.object_pattern_is_irrefutable(
            module, profile, fields, &field_map, tree, symbols, types, visited,
        )
    }

    /// Check whether a tagged object pattern is irrefutable for a nominal object type.
    fn is_irrefutable_tagged_object_pattern(
        &self,
        module: &Module,
        profile: ProfileId,
        ty: LocalNodeId<Expression>,
        fields: &[LocalNodeId<PatternField>],
        value_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        let Some(tag_symbol) =
            self.reference_symbol_for_expression(module, ty, profile, tree, symbols)
        else {
            return false;
        };

        if !self.value_type_matches_tag_symbol(
            module,
            profile,
            value_type_id,
            tag_symbol,
            ty.into_any(),
            tree,
            symbols,
            types,
        ) {
            return false;
        }

        let Some(object_type_id) = self.object_type_id_for_tag_symbol(
            module,
            profile,
            tag_symbol,
            ty.into_any(),
            tree,
            symbols,
            types,
        ) else {
            return false;
        };

        let Some(field_map) = self.object_field_map_for_object_type(types, object_type_id) else {
            return false;
        };

        self.object_pattern_is_irrefutable(
            module, profile, fields, &field_map, tree, symbols, types, visited,
        )
    }

    /// Check whether a pattern field list is irrefutable against a field map.
    fn object_pattern_is_irrefutable(
        &self,
        module: &Module,
        profile: ProfileId,
        fields: &[LocalNodeId<PatternField>],
        field_map: &HashMap<StaticKey, TypeField>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        for field_id in fields {
            let field = tree.get(*field_id);
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
                    if let Some(inner) = pattern {
                        if !self.is_irrefutable_pattern_for_type_inner(
                            module,
                            profile,
                            *inner,
                            field_ty.ty,
                            tree,
                            symbols,
                            types,
                            visited,
                        ) {
                            return false;
                        }
                    }
                }
                PatternField::Alias { name, .. } => {
                    let key = StaticKey::Name(*name);
                    let Some(field_ty) = field_map.get(&key) else {
                        return false;
                    };
                    if field_ty.is_optional {
                        return false;
                    }
                }
                PatternField::Computed { key, pattern, .. } => {
                    let Some(key) = self.static_key_from_dynamic_key(
                        profile,
                        DynamicKey::Expression(*key),
                        tree,
                        symbols,
                        types,
                    ) else {
                        return false;
                    };
                    let Some(field_ty) = field_map.get(&key) else {
                        return false;
                    };
                    if field_ty.is_optional {
                        return false;
                    }
                    if let Some(inner) = pattern {
                        if !self.is_irrefutable_pattern_for_type_inner(
                            module,
                            profile,
                            *inner,
                            field_ty.ty,
                            tree,
                            symbols,
                            types,
                            visited,
                        ) {
                            return false;
                        }
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
        module: &Module,
        profile: ProfileId,
        value_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<Vec<LocalTypeId>> {
        let value_type = types.get_type(value_type_id).clone();
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
                let length =
                    self.fixed_array_count_value(module, profile, count, tree, symbols, types)?;
                let mut elements = Vec::with_capacity(length);
                for _ in 0..length {
                    elements.push(element);
                }
                Some(elements)
            }
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::TypeAlias => {
                let target_id = self.alias_target_type_id_for_symbol(
                    module, profile, symbol, source_id, symbols, types,
                )?;
                let target_id = types.unwrap_value_type_id(target_id);
                self.fixed_sequence_element_types(
                    module, profile, target_id, source_id, tree, symbols, types,
                )
            }
            Type::Value { value } => self.fixed_sequence_element_types(
                module, profile, value, source_id, tree, symbols, types,
            ),
            _ => None,
        }
    }

    /// Resolve a fixed array count to a literal length.
    fn fixed_array_count_value(
        &self,
        module: &Module,
        profile: ProfileId,
        count: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<usize> {
        // prefer static evaluation for fixed array sizes
        if let Ok(Some(static_value)) = self
            .evaluate_static_expression_value(module, profile, count, tree, symbols, types, None)
        {
            if let StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            } = static_value
            {
                return usize::try_from(value).ok();
            }
        }

        // fall back to inferred literal types when static evaluation is missing
        let count_global = count.into_global_any(types.module_id);
        let count_id = types.get_inferred_type_id(count_global)?;
        let count_ty = types.get_type(count_id);
        let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
        } = count_ty
        else {
            return None;
        };

        usize::try_from(*value).ok()
    }

    /// Resolve a structural object field map for a value type.
    fn object_field_map_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        value_type_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<HashMap<StaticKey, TypeField>> {
        match types.get_type(value_type_id) {
            Type::Object { fields, .. } => {
                let mut map = HashMap::new();
                for field in fields {
                    map.insert(field.key, field.clone());
                }
                Some(map)
            }
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::TypeAlias => {
                let target_id = self.alias_target_type_id_for_symbol(
                    module, profile, *symbol, source_id, symbols, types,
                )?;
                let target_id = types.unwrap_value_type_id(target_id);
                self.object_field_map_for_type(
                    module, profile, target_id, source_id, tree, symbols, types,
                )
            }
            Type::Value { value } => self.object_field_map_for_type(
                module, profile, *value, source_id, tree, symbols, types,
            ),
            _ => None,
        }
    }

    /// Resolve an object type id for a tagged pattern symbol.
    fn object_type_id_for_tag_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        tag_symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        _tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        match tag_symbol.ty() {
            SymbolType::Struct | SymbolType::Interface | SymbolType::Class => {
                types.get_instance_type_id(tag_symbol)
            }
            SymbolType::Newtype | SymbolType::TypeAlias => {
                let target_id = self.alias_target_type_id_for_symbol(
                    module, profile, tag_symbol, source_id, symbols, types,
                )?;
                let target_id = types.unwrap_value_type_id(target_id);
                self.object_type_id_for_type(types, target_id)
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
        module: &Module,
        profile: ProfileId,
        value_type_id: LocalTypeId,
        tag_symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
        _tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let mut visited = HashSet::new();
        let mut current_id = value_type_id;

        loop {
            current_id = types.unwrap_value_type_id(current_id);
            if !visited.insert(current_id) {
                return false;
            }

            match types.get_type(current_id) {
                Type::Reference { symbol, .. } => {
                    if *symbol == tag_symbol {
                        return true;
                    }

                    if matches!(symbol.ty(), SymbolType::TypeAlias | SymbolType::Newtype) {
                        let Some(target_id) = self.alias_target_type_id_for_symbol(
                            module, profile, *symbol, source_id, symbols, types,
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
        module: &Module,
        profile: ProfileId,
        value_type_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<MatchExhaustiveTarget> {
        // enum exhaustiveness
        let value_type = types.get_type(value_type_id);
        if let Some(enum_symbol) = self.enum_symbol_for_type(value_type, types) {
            let enum_fields =
                self.enum_field_symbols_for_enum(module, profile, enum_symbol, tree, symbols);
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
        if let Some(values) = self.literal_union_values_for_type(value_type_id, types) {
            return Some(MatchExhaustiveTarget::LiteralUnion { values });
        }

        // discriminated unions
        if let Some((key, values)) = self.discriminant_union_values_for_type(value_type_id, types) {
            return Some(MatchExhaustiveTarget::DiscriminantUnion { key, values });
        }

        None
    }

    /// Extract literal union values when possible.
    fn literal_union_values_for_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<HashSet<MatchLiteral>> {
        let ty = types.get_type(type_id);
        match ty {
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
                    let literal = match element {
                        Type::TypeLiteral { value } => {
                            self.match_literal_from_type_literal(value)?
                        }
                        _ => return None,
                    };
                    set.insert(literal);
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
                        return None;
                    }
                    return self.discriminant_union_values_for_type(instance_id, types);
                }
                return None;
            }
            Type::Value { value } => {
                return self.discriminant_union_values_for_type(*value, types);
            }
            Type::Union { elements } => {
                // continue below
                let elements = elements;
                // collect discriminant maps for each union element
                let mut maps = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let map = self.discriminant_fields_for_type(types, *element_id)?;
                    maps.push(map);
                }

                let Some((first, rest)) = maps.split_first() else {
                    return None;
                };

                // pick the first key shared by every element
                let mut candidate_keys: Vec<StaticKey> = first.keys().copied().collect();
                candidate_keys.retain(|key| rest.iter().all(|map| map.contains_key(key)));
                let Some(discriminant_key) = candidate_keys.into_iter().next() else {
                    return None;
                };

                let mut values = HashSet::new();
                for map in maps {
                    if let Some(value) = map.get(&discriminant_key) {
                        values.insert(*value);
                    } else {
                        return None;
                    }
                }

                if values.is_empty() {
                    return None;
                }

                return Some((discriminant_key, values));
            }
            _ => return None,
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
                    merged.retain(|key, literal| map.get(key) == Some(literal));
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
        module: &Module,
        profile: ProfileId,
        enum_symbol: GlobalSymbolId,
        pattern_id: LocalNodeId<Pattern>,
        enum_fields: &HashSet<GlobalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<MatchPatternCoverage<GlobalSymbolId>> {
        match tree.get(pattern_id) {
            Pattern::Wildcard => Some(MatchPatternCoverage::All),
            Pattern::Binding { name, pattern, .. } => match pattern {
                Some(pattern) => self.enum_pattern_coverage(
                    module,
                    profile,
                    enum_symbol,
                    *pattern,
                    enum_fields,
                    tree,
                    symbols,
                ),
                None => {
                    if let Some(field_symbol) = self.enum_field_symbol_for_name(
                        module,
                        profile,
                        enum_symbol,
                        *name,
                        tree,
                        symbols,
                    ) {
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
                        module,
                        profile,
                        enum_symbol,
                        *pattern,
                        enum_fields,
                        tree,
                        symbols,
                    );
                    let Some(coverage) = coverage else {
                        return None;
                    };
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
                let field_symbol = self.enum_field_symbol_for_pattern_value(
                    module,
                    profile,
                    enum_symbol,
                    *ty,
                    enum_fields,
                    tree,
                    symbols,
                )?;
                Some(MatchPatternCoverage::Values(vec![field_symbol]))
            }
            Pattern::Expression { value } => {
                let field_symbol = self.enum_field_symbol_for_pattern_value(
                    module,
                    profile,
                    enum_symbol,
                    *value,
                    enum_fields,
                    tree,
                    symbols,
                )?;
                Some(MatchPatternCoverage::Values(vec![field_symbol]))
            }
            _ => None,
        }
    }

    /// Summarize coverage for a literal union pattern using known literals.
    fn literal_pattern_coverage_for_values(
        &self,
        values: &HashSet<MatchLiteral>,
        pattern_id: LocalNodeId<Pattern>,
        tree: &NodeTree,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match tree.get(pattern_id) {
            Pattern::Wildcard => Some(MatchPatternCoverage::All),
            Pattern::Binding { pattern, .. } => match pattern {
                Some(pattern) => self.literal_pattern_coverage_for_values(values, *pattern, tree),
                None => Some(MatchPatternCoverage::All),
            },
            Pattern::Union { patterns } => {
                // merge coverage across union patterns
                let mut covered: HashSet<MatchLiteral> = HashSet::new();
                for pattern in patterns {
                    let coverage =
                        self.literal_pattern_coverage_for_values(values, *pattern, tree)?;
                    match coverage {
                        MatchPatternCoverage::All => return Some(MatchPatternCoverage::All),
                        MatchPatternCoverage::Values(values) => covered.extend(values),
                    }
                }
                Some(MatchPatternCoverage::Values(covered.into_iter().collect()))
            }
            Pattern::Expression { value } => {
                // literal expressions cover a single known value
                let literal = self.match_literal_from_expression(*value, tree)?;
                if values.contains(&literal) {
                    Some(MatchPatternCoverage::Values(vec![literal]))
                } else {
                    None
                }
            }
            Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                // filter literal values that fall within the range
                let Some(start_id) = *start else {
                    return None;
                };
                let Some(end_id) = *end else {
                    return None;
                };
                let start_literal = self.match_literal_from_pattern(start_id, tree)?;
                let end_literal = self.match_literal_from_pattern(end_id, tree)?;
                let literals = self.match_literals_in_range(
                    values,
                    &start_literal,
                    &end_literal,
                    *is_inclusive,
                );
                if literals.is_empty() {
                    None
                } else {
                    Some(MatchPatternCoverage::Values(literals))
                }
            }
            _ => None,
        }
    }

    /// Summarize coverage for a discriminated union pattern.
    fn discriminant_pattern_coverage(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        key: StaticKey,
        values: &HashSet<MatchLiteral>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<MatchPatternCoverage<MatchLiteral>> {
        match tree.get(pattern_id) {
            Pattern::Wildcard => Some(MatchPatternCoverage::All),
            Pattern::Binding { pattern, .. } => match pattern {
                Some(pattern) => self.discriminant_pattern_coverage(
                    module, profile, *pattern, key, values, tree, symbols, types,
                ),
                None => Some(MatchPatternCoverage::All),
            },
            Pattern::Union { patterns } => {
                let mut covered: HashSet<MatchLiteral> = HashSet::new();
                for pattern in patterns {
                    let coverage = self.discriminant_pattern_coverage(
                        module, profile, *pattern, key, values, tree, symbols, types,
                    )?;
                    match coverage {
                        MatchPatternCoverage::All => return Some(MatchPatternCoverage::All),
                        MatchPatternCoverage::Values(values) => covered.extend(values),
                    }
                }
                Some(MatchPatternCoverage::Values(covered.into_iter().collect()))
            }
            Pattern::Object { fields } => {
                let field_literal = self.discriminant_value_for_pattern_fields(
                    profile, key, fields, tree, symbols, types,
                )?;
                let coverage = match field_literal {
                    Some(literal) => MatchPatternCoverage::Values(vec![literal]),
                    None => MatchPatternCoverage::All,
                };
                self.filter_literal_coverage(values, coverage)
            }
            Pattern::TaggedObject { ty, fields } => {
                // prefer explicit discriminant fields in the pattern
                let field_literal = self.discriminant_value_for_pattern_fields(
                    profile, key, fields, tree, symbols, types,
                );
                if let Some(field_literal) = field_literal {
                    let coverage = match field_literal {
                        Some(literal) => MatchPatternCoverage::Values(vec![literal]),
                        None => MatchPatternCoverage::All,
                    };
                    return self.filter_literal_coverage(values, coverage);
                }

                // fall back to the discriminant value on the tag type
                let tag_symbol =
                    self.reference_symbol_for_expression(module, *ty, profile, tree, symbols)?;
                let tag_type_id = types.get_instance_type_id(tag_symbol)?;
                let discriminants = self.discriminant_fields_for_type(types, tag_type_id)?;
                let literal = *discriminants.get(&key)?;
                let coverage = MatchPatternCoverage::Values(vec![literal]);
                self.filter_literal_coverage(values, coverage)
            }
            _ => None,
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

    /// Extract a literal discriminant for object patterns.
    fn discriminant_value_for_pattern_fields(
        &self,
        profile: ProfileId,
        key: StaticKey,
        fields: &[LocalNodeId<PatternField>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<Option<MatchLiteral>> {
        // locate the matching field by key
        let mut matching_field = None;
        for field_id in fields {
            let field_key = match tree.get(*field_id) {
                PatternField::Named { name, .. } | PatternField::Alias { name, .. } => {
                    Some(StaticKey::Name(*name))
                }
                PatternField::Computed { key: field_key, .. } => self.static_key_from_dynamic_key(
                    profile,
                    DynamicKey::Expression(*field_key),
                    tree,
                    symbols,
                    types,
                ),
                _ => None,
            };

            if let Some(field_key) = field_key
                && field_key.matches(&key)
            {
                matching_field = Some(*field_id);
                break;
            }
        }

        let Some(field_id) = matching_field else {
            return None;
        };

        // resolve the literal from the field pattern
        match tree.get(field_id) {
            PatternField::Alias { .. } => Some(None),
            PatternField::Named { pattern, .. } | PatternField::Computed { pattern, .. } => {
                let Some(pattern_id) = *pattern else {
                    return Some(None);
                };
                let literal = self.match_literal_from_pattern(pattern_id, tree)?;
                Some(Some(literal))
            }
            _ => None,
        }
    }

    /// Extract a literal value from a pattern.
    fn match_literal_from_pattern(
        &self,
        pattern_id: LocalNodeId<Pattern>,
        tree: &NodeTree,
    ) -> Option<MatchLiteral> {
        match tree.get(pattern_id) {
            Pattern::Expression { value } => self.match_literal_from_expression(*value, tree),
            Pattern::Binding { pattern, .. } => {
                pattern.and_then(|inner| self.match_literal_from_pattern(inner, tree))
            }
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
            ScalarLiteral::Boolean(value) => Some(MatchLiteral::Boolean(*value)),
            ScalarLiteral::Integer(value) => Some(MatchLiteral::Integer(*value)),
            ScalarLiteral::Bigint(value) => Some(MatchLiteral::Bigint(*value)),
            ScalarLiteral::Float(value) => Some(MatchLiteral::Float(value.to_bits())),
            ScalarLiteral::Character(value) => Some(MatchLiteral::Character(*value)),
            ScalarLiteral::String(value) => Some(MatchLiteral::String(*value)),
            ScalarLiteral::RegexString { .. } => None,
        }
    }

    /// Check whether a match literal falls within a scalar range.
    fn match_literal_in_range(
        &self,
        value: &MatchLiteral,
        start: &MatchLiteral,
        end: &MatchLiteral,
        is_inclusive: bool,
    ) -> bool {
        match (value, start, end) {
            (
                MatchLiteral::Integer(value),
                MatchLiteral::Integer(start),
                MatchLiteral::Integer(end),
            ) => {
                // compare integer ranges
                let end_value = if is_inclusive { *end } else { end - 1 };
                *value >= *start && *value <= end_value
            }
            (
                MatchLiteral::Bigint(value),
                MatchLiteral::Bigint(start),
                MatchLiteral::Bigint(end),
            ) => {
                // compare bigint ranges
                let end_value = if is_inclusive { *end } else { end - 1 };
                *value >= *start && *value <= end_value
            }
            (
                MatchLiteral::Character(value),
                MatchLiteral::Character(start),
                MatchLiteral::Character(end),
            ) => {
                // compare character ranges
                let value = *value as u32;
                let start = *start as u32;
                let end = *end as u32;
                let end_value = if is_inclusive {
                    end
                } else if end == 0 {
                    return false;
                } else {
                    end - 1
                };
                value >= start && value <= end_value
            }
            _ => false,
        }
    }

    /// Filter known literals by a scalar range.
    fn match_literals_in_range(
        &self,
        values: &HashSet<MatchLiteral>,
        start: &MatchLiteral,
        end: &MatchLiteral,
        is_inclusive: bool,
    ) -> Vec<MatchLiteral> {
        // collect literal values covered by the range
        let mut filtered = Vec::new();
        for literal in values {
            if self.match_literal_in_range(literal, start, end, is_inclusive) {
                filtered.push(*literal);
            }
        }
        filtered
    }

    /// Resolve enum field symbols from pattern expressions.
    fn enum_field_symbol_for_pattern_value(
        &self,
        module: &Module,
        profile: ProfileId,
        enum_symbol: GlobalSymbolId,
        expression_id: LocalNodeId<Expression>,
        enum_fields: &HashSet<GlobalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let expression_id = self.unwrap_parenthesized_expression(expression_id, tree);
        match tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                let left_id = self.unwrap_parenthesized_expression(*left, tree);
                let left_symbol =
                    self.reference_symbol_for_expression(module, left_id, profile, tree, symbols)?;
                if left_symbol != enum_symbol {
                    return None;
                }
                let field_symbol = self.enum_field_symbol_for_name(
                    module,
                    profile,
                    enum_symbol,
                    *name,
                    tree,
                    symbols,
                )?;
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

    /// Validate object pattern spread placement.
    fn validate_object_pattern_spreads(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        fields: &[LocalNodeId<PatternField>],
    ) {
        // locate the first spread field and report duplicates
        let mut spread_index = None;
        for (index, field_id) in fields.iter().enumerate() {
            if matches!(tree.get(*field_id), PatternField::Spread { .. }) {
                let error_node = field_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                if spread_index.is_some() {
                    self.error(AnalyzeError::ObjectPatternMultipleSpreads { node: error_node });
                } else {
                    spread_index = Some(index);
                }
            }
        }

        // validate rest targets
        if !module.language_type.is_destack() {
            for field_id in fields {
                let PatternField::Spread { pattern, .. } = tree.get(*field_id) else {
                    continue;
                };
                let is_identifier = pattern.is_some_and(|pattern_id| {
                    matches!(tree.get(pattern_id), Pattern::Binding { pattern: None, .. })
                });
                if !is_identifier {
                    let node = field_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile));
                    self.error(AnalyzeError::ObjectPatternRestNotIdentifier { node });
                }
            }
        }

        // spread must be the last field
        if let Some(index) = spread_index
            && index + 1 < fields.len()
        {
            let error_node = fields[index + 1]
                .into_global_any(module.id)
                .into_anchored(Some(profile));
            self.error(AnalyzeError::ObjectPatternSpreadNotLast { node: error_node });
        }
    }

    /// Validate named fields in array and tuple patterns.
    fn validate_sequence_pattern_fields(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        fields: &[LocalNodeId<PatternField>],
    ) {
        // Destack tuple and array patterns may use named fields
        if module.language_type.is_destack() {
            return;
        }

        // reject named or aliased fields in array and tuple patterns
        for field_id in fields {
            if matches!(
                tree.get(*field_id),
                PatternField::Named { .. } | PatternField::Alias { .. }
            ) {
                let node = field_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
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
            Pattern::ReferenceOf { right, .. } | Pattern::ValueOf { right, .. } => {
                self.pattern_has_definite_assignment(tree, *right)
            }
            Pattern::Binding { pattern, .. } => {
                pattern.is_some_and(|inner| self.pattern_has_definite_assignment(tree, inner))
            }
            Pattern::Range { start, end, .. } => {
                start.is_some_and(|inner| self.pattern_has_definite_assignment(tree, inner))
                    || end.is_some_and(|inner| self.pattern_has_definite_assignment(tree, inner))
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
            Pattern::Wildcard | Pattern::Expression { .. } => false,
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
            PatternField::Named { pattern, .. } | PatternField::Computed { pattern, .. } => {
                pattern.is_some_and(|inner| self.pattern_has_definite_assignment(tree, inner))
            }
            PatternField::Positional { pattern } => {
                self.pattern_has_definite_assignment(tree, *pattern)
            }
            PatternField::Alias { .. } | PatternField::Elision => false,
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

impl Compiler {
    /// Validate a single pattern node.
    pub(super) fn validate_pattern(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        pattern: &Pattern,
    ) {
        let analysis = PatternAnalysis::new(self);
        analysis.validate_pattern(module, profile, tree, pattern);
    }

    /// Validate match exhaustiveness for supported value shapes.
    pub(super) fn validate_match_exhaustiveness(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
    ) {
        let analysis = PatternAnalysis::new(self);
        let mut context =
            PatternAnalysisContext::new(analysis, module, profile, tree, symbols, types);
        context.validate_match_exhaustiveness(expression_id, value_id, cases);
    }

    /// Check whether a pattern contains a definite assignment assertion.
    pub(super) fn pattern_has_definite_assignment(
        &self,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
    ) -> bool {
        let analysis = PatternAnalysis::new(self);
        analysis.pattern_has_definite_assignment(tree, pattern_id)
    }

    /// Check whether a pattern is irrefutable for a given value type.
    pub(crate) fn is_irrefutable_pattern_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        value_type: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        let analysis = PatternAnalysis::new(self);
        let mut context =
            PatternAnalysisContext::new(analysis, module, profile, tree, symbols, types);
        context.is_irrefutable_pattern_for_type(pattern_id, value_type)
    }

    /// Check whether a pattern is a destructuring pattern.
    pub(super) fn is_destructuring_pattern(
        &self,
        tree: &NodeTree,
        pattern_id: LocalNodeId<Pattern>,
    ) -> bool {
        let analysis = PatternAnalysis::new(self);
        analysis.is_destructuring_pattern(tree, pattern_id)
    }
}
