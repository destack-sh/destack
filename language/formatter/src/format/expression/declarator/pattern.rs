use super::super::{LocalNodeId, NodeTree, Pattern, PatternField};

/// Return whether one pattern subtree contains at least one default assignment.
pub(super) fn pattern_has_default_assignment(
    tree: &NodeTree,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    match tree.get(pattern_id) {
        Pattern::Wildcard | Pattern::Expression { .. } => false,
        Pattern::Must(inner_pattern_id)
        | Pattern::ReferenceOf {
            right: inner_pattern_id,
            ..
        }
        | Pattern::ValueOf {
            right: inner_pattern_id,
            ..
        } => pattern_has_default_assignment(tree, *inner_pattern_id),
        Pattern::Binding { pattern, .. } => pattern.as_ref().is_some_and(|inner_pattern_id| {
            pattern_has_default_assignment(tree, *inner_pattern_id)
        }),
        Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::Array { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedObject { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| pattern_field_has_default_assignment(tree, field_id)),
        Pattern::Union { patterns } => patterns
            .iter()
            .copied()
            .any(|inner_pattern_id| pattern_has_default_assignment(tree, inner_pattern_id)),
    }
}

/// Return whether one pattern field contains at least one default assignment.
fn pattern_field_has_default_assignment(
    tree: &NodeTree,
    pattern_field_id: LocalNodeId<PatternField>,
) -> bool {
    match tree.get(pattern_field_id) {
        PatternField::Named {
            pattern, default, ..
        }
        | PatternField::Computed {
            pattern, default, ..
        } => {
            default.is_some()
                || pattern
                    .as_ref()
                    .is_some_and(|pattern_id| pattern_has_default_assignment(tree, *pattern_id))
        }
        PatternField::Alias { default, .. } => default.is_some(),
        PatternField::Positional { pattern, default } => {
            default.is_some() || pattern_has_default_assignment(tree, *pattern)
        }
        PatternField::Spread { pattern, .. } => pattern
            .as_ref()
            .is_some_and(|pattern_id| pattern_has_default_assignment(tree, *pattern_id)),
        PatternField::Elision => false,
    }
}
