use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Expression, GlobalNodeIdAny, LocalNodeId, MatchCase, MatchKind, MatchSelector, NodeTree,
    Pattern, PatternField, Property,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Validate expression level syntax rules that do not require inference.
    pub(super) fn validate_expressions(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
    ) {
        // validate export namespace placement
        for (expression_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            match expression {
                Expression::Match {
                    kind: MatchKind::Switch,
                    cases,
                    ..
                } => {
                    for case_id in cases {
                        let (selector, case_span_id) = match tree.get(*case_id) {
                            MatchCase::Expression { selector, .. }
                            | MatchCase::Block { selector, .. } => (selector, *case_id),
                        };
                        if let MatchSelector::Pattern { pattern, guard } = selector {
                            if guard.is_some() {
                                self.error(AnalyzeError::InvalidSwitchCaseGuard {
                                    node: case_span_id
                                        .into_global_any(module.id)
                                        .into_anchored(Some(profile)),
                                });
                            }
                            match tree.get(*pattern) {
                                Pattern::Expression { value } => {
                                    if self.is_invalid_switch_case_expression(tree, *value) {
                                        self.error(AnalyzeError::InvalidSwitchCasePattern {
                                            node: pattern
                                                .into_global_any(module.id)
                                                .into_anchored(Some(profile)),
                                        });
                                    }
                                }
                                _ => {
                                    self.error(AnalyzeError::InvalidSwitchCasePattern {
                                        node: pattern
                                            .into_global_any(module.id)
                                            .into_anchored(Some(profile)),
                                    });
                                }
                            }
                        }
                    }
                }
                Expression::ExportNamespace { .. } => {
                    if !module.language_type.is_declaration() {
                        self.error(AnalyzeError::ExportNamespaceOutsideDeclaration {
                            node: GlobalNodeIdAny::new(module.id, expression_id.into_any())
                                .into_anchored(Some(profile)),
                        });
                    }
                }
                Expression::ObjectExpression { properties }
                | Expression::TaggedObjectExpression { properties, .. } => {
                    self.validate_object_literal_properties(module, profile, tree, properties);
                }
                _ => {}
            }
        }

        // validate object pattern spread placement
        for (_, pattern) in tree.iter_nodes_of_type::<Pattern>() {
            match pattern {
                Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => {
                    self.validate_object_pattern_spreads(module, profile, tree, fields);
                }
                _ => {}
            }
        }
    }

    /// Validate defaults on object literal properties.
    fn validate_object_literal_properties(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        properties: &[LocalNodeId<Property>],
    ) {
        // reject defaults on object literal properties
        for property_id in properties {
            let property = tree.get(*property_id);
            if matches!(
                property,
                Property::Field {
                    default: Some(_),
                    ..
                }
            ) {
                self.error(AnalyzeError::ObjectLiteralDefault {
                    node: property_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                });
            }
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

    // reject structural or wildcard patterns in switch cases
    pub(crate) fn is_invalid_switch_case_expression(
        &self,
        tree: &NodeTree,
        value: LocalNodeId<Expression>,
    ) -> bool {
        match tree.get(value) {
            Expression::TupleExpression { .. } => true,
            Expression::TaggedTupleExpression { .. } => true,
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => {
                if path.segments.len() != 1 {
                    return false;
                }
                let name = self.program.strings.get(path.segments[0]);
                name == "_"
            }
            _ => false,
        }
    }
}
