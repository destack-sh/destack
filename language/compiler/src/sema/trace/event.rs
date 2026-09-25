use tspp_repository::TraceEvent;

use crate::sema::{CheckEvent, EventFormatter, VariableKind};

impl EventFormatter<'_, '_> {
    /// Format this event for a provider trace.
    pub(in crate::sema) fn format(&self, event: &CheckEvent) -> TraceEvent {
        // label by the event's own kind
        match event {
            CheckEvent::VariableAllocated { variable, kind } => {
                TraceEvent::new("variable.allocated")
                    .text("variable", self.variable_label(*variable))
                    .text("origin", self.variable_origin_label(*variable))
                    .text("at", self.variable_source_label(*variable))
                    .text("kind", self.variable_kind_label(*kind))
            }
            CheckEvent::LowerBoundPushed { variable, bound } => TraceEvent::new("variable.lower")
                .text("variable", self.variable_label(*variable))
                .text("bound", self.type_label(bound.ty))
                .text("relation", self.relation_label(bound.relation))
                .text(
                    "cause",
                    self.origin_label(self.check.infer.cause(bound.cause).origin),
                ),
            CheckEvent::UpperBoundPushed { variable, bound } => TraceEvent::new("variable.upper")
                .text("variable", self.variable_label(*variable))
                .text("bound", self.type_label(bound.ty))
                .text("relation", self.relation_label(bound.relation))
                .text(
                    "cause",
                    self.origin_label(self.check.infer.cause(bound.cause).origin),
                ),
            CheckEvent::NodeDecided { node } => TraceEvent::new("node.decided")
                .text("node", self.node_label(*node))
                .text("at", self.node_source_label(*node)),
            CheckEvent::VariableSolved {
                variable,
                bounds,
                solution,
            } => TraceEvent::new("variable.solved")
                .text("variable", self.variable_label(*variable))
                .text("lower", self.type_bound_list_label(&bounds.lower))
                .text("upper", self.type_bound_list_label(&bounds.upper))
                .text("solution", self.type_label(*solution)),
        }
    }

    /// Return the compact variable kind label.
    fn variable_kind_label(&self, kind: VariableKind) -> &'static str {
        match kind {
            VariableKind::Type => "type",
            VariableKind::Integer => "integer",
            VariableKind::Float => "float",
            VariableKind::Memory(_) => "memory",
        }
    }
}
