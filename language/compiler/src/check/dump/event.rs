use destack_artifact::{ArtifactEvent, ArtifactEventLog};

use crate::check::{CheckEvent, DumpContext};

impl CheckEvent {
    /// Render this event as one timestamped plain text line.
    pub(in crate::check) fn render_plain_at(
        &self,
        context: &DumpContext<'_, '_>,
        timestamp: usize,
    ) -> String {
        let mut log = ArtifactEventLog::new();
        self.render(context, &mut log);

        let Some(mut event) = log.events.into_iter().next() else {
            return String::new();
        };
        event.timestamp = timestamp;

        event.render_plain()
    }

    /// Render this event into an artifact event log.
    pub(in crate::check) fn render(
        &self,
        context: &DumpContext<'_, '_>,
        log: &mut ArtifactEventLog,
    ) {
        let event = match self {
            Self::SolveStart { tasks, variables } => ArtifactEvent::new("solve.start")
                .info()
                .usize("tasks", *tasks)
                .usize("variables", *variables),
            Self::SolveStep { step, task } => {
                let event = ArtifactEvent::new("solve.step").info().usize("step", *step);

                task.render_event(event, context)
            }
            Self::SolveFinish {
                iterations,
                variables,
            } => ArtifactEvent::new("solve.finish")
                .info()
                .usize("iterations", *iterations)
                .usize("variables", *variables),
            Self::RelationCheck {
                constraint,
                finished,
            } => match context.check.constraints.get(*constraint) {
                Ok(relation) => relation.render_event(*constraint, *finished, context),
                Err(_) => ArtifactEvent::new("relation.check")
                    .debug()
                    .text("id", context.constraint_label(*constraint))
                    .bool("finished", *finished),
            },
            Self::ObligationCheck {
                obligation,
                finished,
            } => match context.check.obligations.get(*obligation) {
                Ok(obligation_state) => {
                    obligation_state.render_event(*obligation, *finished, context)
                }
                Err(_) => ArtifactEvent::new("obligation.check")
                    .debug()
                    .text("id", context.obligation_label(*obligation))
                    .bool("finished", *finished),
            },
            Self::Decision { node } => ArtifactEvent::new("decision.set")
                .debug()
                .text("node", context.node_label(*node))
                .text("at", context.node_source_label(*node)),
            Self::VariableSolution {
                variable,
                solution,
                waiters,
            } => ArtifactEvent::new("variable.solution")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("solution", context.type_label(*solution))
                .text("waiters", context.task_list_label(waiters)),
            Self::VariableBlocked {
                variable,
                lower,
                upper,
                default,
                blockers,
            } => ArtifactEvent::new("variable.blocked")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("lower", context.type_list_label(lower))
                .text("upper", context.type_list_label(upper))
                .text("default", context.optional_type_label(*default))
                .text("blockers", context.dependency_list_label(blockers)),
            Self::VariableUnsolved {
                variable,
                lower,
                upper,
                default,
            } => ArtifactEvent::new("variable.unsolved")
                .debug()
                .text("variable", context.variable_label(*variable))
                .text("lower", context.type_list_label(lower))
                .text("upper", context.type_list_label(upper))
                .text("default", context.optional_type_label(*default)),
            Self::VariableAlias {
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
