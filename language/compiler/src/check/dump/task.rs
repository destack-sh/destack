use destack_artifact::ArtifactEvent;

use crate::check::{DumpContext, Task};

impl Task {
    /// Render this task as fields on one event.
    pub(in crate::check) fn render_event(
        &self,
        event: ArtifactEvent,
        context: &DumpContext<'_, '_>,
    ) -> ArtifactEvent {
        match self {
            Self::Relate(id) => event
                .text("task", "relate")
                .text("constraint", context.constraint_label(*id)),
            Self::Check(id) => event
                .text("task", "check")
                .text("constraint", context.constraint_label(*id)),
            Self::Infer { site, use_ } => event
                .text("task", "infer")
                .text("site", context.flow_site_label(*site))
                .text("use", format!("{use_:?}")),
            Self::CheckBody(body) => event
                .text("task", "check-body")
                .text("site", context.flow_site_label(body.site)),
            Self::Oblige(id) => event
                .text("task", "oblige")
                .text("obligation", context.obligation_label(*id)),
        }
    }
}
