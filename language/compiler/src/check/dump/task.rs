use destack_artifact::ArtifactEvent;

use crate::check::{DumpContext, Task};

impl Task {
    /// Render this task as fields on one event.
    pub(in crate::check) fn render_event(
        self,
        event: ArtifactEvent,
        context: &DumpContext<'_, '_>,
    ) -> ArtifactEvent {
        match self {
            Self::Relate(id) => event
                .text("task", "relate")
                .text("constraint", context.constraint_label(id)),
            Self::Decide(node) => event
                .text("task", "decide")
                .text("node", context.node_label(node))
                .text("at", context.node_source_label(node)),
            Self::Solve(variable) => event
                .text("task", "solve")
                .text("variable", context.variable_label(variable)),
        }
    }
}
