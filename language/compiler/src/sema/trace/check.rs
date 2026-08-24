use destack_repository::TraceEvent;

use crate::sema::{Check, CheckId, EventFormatter};

impl EventFormatter<'_, '_> {
    /// Format this check as one trace event.
    pub(in crate::sema) fn format_check(
        &self,
        check: &Check,
        id: CheckId,
        finished: bool,
    ) -> TraceEvent {
        match check {
            Check::Relation(relation) => {
                let source = self.type_label(relation.source);
                let target = self.type_label(relation.target);

                TraceEvent::new("relation.checked")
                    .text("id", self.check_label(id))
                    .text("relation", self.relation_label(relation.relation))
                    .text("source", source)
                    .text("target", target)
                    .text(
                        "origin",
                        self.origin_label(self.check.infer.cause(relation.cause).origin),
                    )
                    .text(
                        "at",
                        self.origin_source_label(self.check.infer.cause(relation.cause).origin),
                    )
                    .bool("finished", finished)
            }
            Check::Node(node) => TraceEvent::new("node.checked")
                .text("id", self.check_label(id))
                .text("target", self.type_label(node.expectation.target))
                .bool("finished", finished),
            Check::Conversion(conversion) => TraceEvent::new("conversion.checked")
                .text("id", self.check_label(id))
                .text("source", self.type_label(conversion.source.ty))
                .text("target", self.type_label(conversion.expectation.target))
                .bool("finished", finished),
            Check::Narrowing(narrowing) => TraceEvent::new("narrowing.checked")
                .text("id", self.check_label(id))
                .text("source", self.type_label(narrowing.source))
                .text("operation", self.node_label(narrowing.operation))
                .bool("finished", finished),
            Check::Selection(selection) => TraceEvent::new("selection.checked")
                .text("node", self.node_label(selection.site.node))
                .text("id", self.check_label(id))
                .bool("finished", finished),
            Check::Equality(equality) => TraceEvent::new("equality.checked")
                .text("node", self.node_label(equality.value))
                .text("id", self.check_label(id))
                .bool("finished", finished),
            Check::Declared(entry) => self.format_obligation(&entry.obligation, id, finished),
        }
    }
}
