use destack_artifact::ArtifactEvent;

use crate::check::{Constraint, ConstraintId, DumpContext};

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
            .text("relation", context.relation_label(self.relation()))
            .text("left", context.type_label(self.left()))
            .text("right", context.type_label(self.right()))
            .text("origin", context.origin_label(self.origin()))
            .text("at", context.origin_source_label(self.origin()))
            .text("use", context.value_use_label(self.value_use()))
            .bool("finished", finished)
    }
}
