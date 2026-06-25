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
            Self::Select(id) => event
                .text("task", "select")
                .text("selection", context.selection_label(id)),
            Self::Oblige(id) => event
                .text("task", "oblige")
                .text("obligation", context.obligation_label(id)),
            Self::Solve(variable) => event
                .text("task", "solve")
                .text("variable", context.variable_label(variable)),
        }
    }
}
