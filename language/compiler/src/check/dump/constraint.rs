use destack_artifact::ArtifactEvent;

use crate::check::{Constraint, ConstraintId, DumpContext, ValueSource};

impl Constraint {
    /// Render this constraint as one trace event.
    pub(in crate::check) fn render_event(
        &self,
        id: ConstraintId,
        finished: bool,
        context: &DumpContext<'_, '_>,
    ) -> ArtifactEvent {
        let (source, target) = match self {
            Self::Type(constraint) => (
                context.type_label(constraint.source),
                context.type_label(constraint.target),
            ),
            Self::Value(constraint) => {
                let source = match constraint.source {
                    ValueSource::Node(node) => context.node_label(node),
                    ValueSource::Type(ty) => context.type_label(ty),
                };
                let target = context.type_label(constraint.target);

                (source, target)
            }
        };

        ArtifactEvent::new("relation.checked")
            .debug()
            .text("id", context.constraint_label(id))
            .text("relation", context.relation_label(self.relation()))
            .text("source", source)
            .text("target", target)
            .text(
                "origin",
                context.origin_label(context.check.solver.cause(self.cause()).origin),
            )
            .text(
                "at",
                context.origin_source_label(context.check.solver.cause(self.cause()).origin),
            )
            .text("use", context.value_use_label(self.value_use()))
            .bool("finished", finished)
    }
}
