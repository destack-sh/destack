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
            .text("source", context.type_label(self.source()))
            .text("target", context.type_label(self.target()))
            .text(
                "origin",
                context.origin_label(context.check.solver.origin(self.origin())),
            )
            .text(
                "at",
                context.origin_source_label(context.check.solver.origin(self.origin())),
            )
            .text("use", context.value_use_label(self.value_use()))
            .bool("finished", finished)
    }
}
