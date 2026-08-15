use destack_artifact::ArtifactEvent;

use crate::sema::{Check, CheckId, DumpContext};

impl Check {
    /// Render this check as one trace event.
    pub(in crate::sema) fn render_event(
        &self,
        id: CheckId,
        finished: bool,
        context: &DumpContext<'_, '_>,
    ) -> ArtifactEvent {
        match self {
            Self::Relation(relation) => {
                let source = context.type_label(relation.source);
                let target = context.type_label(relation.target);

                ArtifactEvent::new("relation.checked")
                    .debug()
                    .text("id", context.check_label(id))
                    .text("relation", context.relation_label(relation.relation))
                    .text("source", source)
                    .text("target", target)
                    .text(
                        "origin",
                        context.origin_label(context.check.infer.cause(relation.cause).origin),
                    )
                    .text(
                        "at",
                        context
                            .origin_source_label(context.check.infer.cause(relation.cause).origin),
                    )
                    .bool("finished", finished)
            }
            Self::Node(node) => ArtifactEvent::new("node.checked")
                .debug()
                .text("id", context.check_label(id))
                .text("target", context.type_label(node.expectation.target))
                .bool("finished", finished),
            Self::Conversion(conversion) => ArtifactEvent::new("conversion.checked")
                .debug()
                .text("id", context.check_label(id))
                .text("source", context.type_label(conversion.source.ty))
                .text("target", context.type_label(conversion.expectation.target))
                .bool("finished", finished),
            Self::Narrowing(narrowing) => ArtifactEvent::new("narrowing.checked")
                .debug()
                .text("id", context.check_label(id))
                .text("source", context.type_label(narrowing.source))
                .text("operation", context.node_label(narrowing.operation))
                .bool("finished", finished),
            Self::Selection(selection) => ArtifactEvent::new("selection.checked")
                .debug()
                .text("node", context.node_label(selection.site.node))
                .text("id", context.check_label(id))
                .bool("finished", finished),
            Self::Declared(entry) => entry.obligation.render_event(id, finished, context),
        }
    }
}
