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
            Self::VariableAllocated { variable, widening } => {
                ArtifactEvent::new("variable.allocated")
                    .debug()
                    .text("variable", context.variable_label(*variable))
                    .text("origin", context.variable_origin_label(*variable))
                    .text("at", context.variable_source_label(*variable))
                    .text("widening", context.widening_label(*widening))
            }
            Self::SolveStarted { tasks, variables } => ArtifactEvent::new("solve.started")
                .info()
                .usize("tasks", *tasks)
                .usize("variables", *variables),
            Self::TaskRan { step, task } => {
                let event = ArtifactEvent::new("task.ran").info().usize("step", *step);

                task.render_event(event, context)
            }
            Self::SolveFinished {
                iterations,
                variables,
            } => ArtifactEvent::new("solve.finished")
                .info()
                .usize("iterations", *iterations)
                .usize("variables", *variables),
            Self::RelationChecked {
                constraint,
                is_finished,
            } => match context.check.solver.constraints.get(*constraint) {
                Ok(relation) => relation.render_event(*constraint, *is_finished, context),
                Err(_) => ArtifactEvent::new("relation.checked")
                    .debug()
                    .text("id", context.constraint_label(*constraint))
                    .bool("finished", *is_finished),
            },
            Self::ObligationChecked {
                obligation,
                is_finished,
            } => match context.check.solver.obligations.get(*obligation) {
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
                waiters,
            } => ArtifactEvent::new("variable.solved")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("lower", context.type_bound_list_label(&bounds.lower))
                .text("upper", context.type_bound_list_label(&bounds.upper))
                .text("default", context.optional_type_label(bounds.default))
                .text("solution", context.type_label(*solution))
                .usize("waiters", *waiters),
            Self::VariableBlocked {
                variable,
                bounds,
                blockers,
            } => ArtifactEvent::new("variable.blocked")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("lower", context.type_bound_list_label(&bounds.lower))
                .text("upper", context.type_bound_list_label(&bounds.upper))
                .text("default", context.optional_type_label(bounds.default))
                .text("blockers", context.dependency_list_label(blockers)),
            Self::VariableUnsolved { variable, bounds } => ArtifactEvent::new("variable.unsolved")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("lower", context.type_bound_list_label(&bounds.lower))
                .text("upper", context.type_bound_list_label(&bounds.upper))
                .text("default", context.optional_type_label(bounds.default)),
            Self::VariableAliased {
                variable,
                representative,
            } => ArtifactEvent::new("variable.alias")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("representative", context.variable_label(*representative)),
        };

        log.push(event);
    }
}
