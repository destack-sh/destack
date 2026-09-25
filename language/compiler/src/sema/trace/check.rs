use tspp_repository::TraceEvent;

use crate::sema::{Check, CheckId, EventFormatter};

impl EventFormatter<'_, '_> {
    /// Format this check as one trace event.
    pub(in crate::sema) fn format_check(
        &self,
        check: &Check,
        id: CheckId,
        finished: bool,
    ) -> TraceEvent {
        // label by the check's own kind
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
            Check::Conversion(conversion) => TraceEvent::new("conversion.checked")
                .text("id", self.check_label(id))
                .text("source", self.type_label(conversion.source.ty))
                .text("target", self.type_label(conversion.expectation.target))
                .bool("finished", finished),
            Check::Body(body) => TraceEvent::new("body.checked")
                .text("node", self.node_label(body.node))
                .text("id", self.check_label(id))
                .bool("finished", finished),
            Check::Pattern(pattern) => TraceEvent::new("pattern.checked")
                .text("node", self.node_label(pattern.site.node))
                .text("target", self.type_label(pattern.target))
                .text("id", self.check_label(id))
                .bool("finished", finished),
            Check::Place(place) => TraceEvent::new("place.selected")
                .text("node", self.node_label(place.site.node))
                .text("type", self.type_label(place.ty))
                .text("id", self.check_label(id))
                .bool("finished", finished),
            Check::Declared(entry) => self.format_obligation(&entry.obligation, id, finished),
        }
    }
}
