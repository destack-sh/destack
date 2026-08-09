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
        let source = context.type_label(self.source);
        let target = context.type_label(self.target);

        ArtifactEvent::new("relation.checked")
            .debug()
            .text("id", context.constraint_label(id))
            .text("relation", context.relation_label(self.relation()))
            .text("source", source)
            .text("target", target)
            .text(
                "origin",
                context.origin_label(context.check.infer.cause(self.cause()).origin),
            )
            .text(
                "at",
                context.origin_source_label(context.check.infer.cause(self.cause()).origin),
            )
            .bool("finished", finished)
    }
}
