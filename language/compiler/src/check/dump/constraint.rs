use destack_artifact::ArtifactEvent;

use crate::check::{Condition, Constraint, ConstraintId, DumpContext};

impl Constraint {
    /// Render this constraint as one trace event.
    pub(in crate::check) fn render_event(
        &self,
        id: ConstraintId,
        finished: bool,
        context: &DumpContext<'_, '_>,
    ) -> ArtifactEvent {
        ArtifactEvent::new("relation.checked")
            .debug()
            .text("id", context.constraint_label(id))
            .text("relation", context.relation_label(self.relation))
            .text("left", context.type_label(self.left))
            .text("right", context.type_label(self.right))
            .text("origin", context.origin_label(self.origin))
            .text("at", context.origin_source_label(self.origin))
            .text("role", context.role_label(self.role))
            .text("condition", condition_label(&self.condition, context))
            .bool("finished", finished)
    }
}

/// Render one condition compactly.
pub(super) fn condition_label(condition: &Condition, context: &DumpContext<'_, '_>) -> String {
    match condition {
        Condition::Always => "always".to_string(),
        Condition::When(predicates) => format!("when({})", context.type_list_label(predicates)),
    }
}
