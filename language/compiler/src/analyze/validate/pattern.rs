use crate::{AnalyzeError, Compiler};
use destack_dir::{LocalNodeId, NodeTree, Pattern, PatternField};
use destack_workspace::{Module, ProfileId};

impl Compiler {
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
            PatternField::Alias { .. } | PatternField::Spread { .. } | PatternField::Elision => {
                false
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
