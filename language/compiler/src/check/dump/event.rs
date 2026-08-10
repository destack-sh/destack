use destack_artifact::{ArtifactEvent, ArtifactEventLog};

use crate::check::{CheckEvent, DumpContext};

impl CheckEvent {
    /// Render this event into an artifact event log.
    pub(in crate::check) fn render(
        &self,
        context: &DumpContext<'_, '_>,
        log: &mut ArtifactEventLog,
    ) {
        let event = match self {
            Self::ProbeStarted { variables } => ArtifactEvent::new("probe.started")
                .debug()
                .usize("variables", *variables),
            Self::ProbeFinished { verdict } => {
                let verdict = match verdict {
                    Some(verdict) => format!("{verdict:?}"),
                    None => "none".to_string(),
                };

                ArtifactEvent::new("probe.finished")
                    .debug()
                    .text("verdict", verdict)
            }
            Self::VariableAllocated { variable, widening } => {
                ArtifactEvent::new("variable.allocated")
                    .debug()
                    .text("variable", context.variable_label(*variable))
                    .text("origin", context.variable_origin_label(*variable))
                    .text("at", context.variable_source_label(*variable))
                    .text("widening", context.widening_label(*widening))
            }
            Self::LowerBoundPushed { variable, bound } => ArtifactEvent::new("variable.lower")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("bound", context.type_label(bound.ty))
                .text("relation", context.relation_label(bound.relation))
                .text(
                    "cause",
                    context.origin_label(context.check.infer.cause(bound.cause).origin),
                ),
            Self::UpperBoundPushed { variable, bound } => ArtifactEvent::new("variable.upper")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("bound", context.type_label(bound.ty))
                .text("relation", context.relation_label(bound.relation))
                .text(
                    "cause",
                    context.origin_label(context.check.infer.cause(bound.cause).origin),
                ),
            Self::RelationChecked {
                constraint,
                is_finished,
            } => match context.check.fulfill.constraints.get(*constraint) {
                Ok(relation) => relation.render_event(*constraint, *is_finished, context),
                Err(_) => ArtifactEvent::new("relation.checked")
                    .debug()
                    .text("id", context.constraint_label(*constraint))
                    .bool("finished", *is_finished),
            },
            Self::ObligationChecked {
                obligation,
                is_finished,
            } => match context.check.fulfill.obligations.get(*obligation) {
                Ok(entry) => entry
                    .obligation
                    .render_event(*obligation, *is_finished, context),
                Err(_) => ArtifactEvent::new("obligation.checked")
                    .debug()
                    .text("id", context.obligation_label(*obligation))
                    .bool("finished", *is_finished),
            },
            Self::NodeDecided { node } => ArtifactEvent::new("node.decided")
                .debug()
                .text("node", context.node_label(*node))
                .text("at", context.node_source_label(*node)),
            Self::VariableSolved {
                variable,
                bounds,
                solution,
            } => ArtifactEvent::new("variable.solved")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("lower", context.type_bound_list_label(&bounds.lower))
                .text("upper", context.type_bound_list_label(&bounds.upper))
                .text("solution", context.type_label(*solution)),
        };

        log.push(event);
    }
}
