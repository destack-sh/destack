use destack_artifact::ArtifactEvent;

use crate::check::{BindSource, DumpContext, Task, TryPropagationTarget};

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
            Self::Propagate(propagation) => event
                .text("task", "propagate")
                .text("source", context.node_label(propagation.source))
                .text("value", context.flow_site_label(propagation.value))
                .text("target", try_target_label(propagation.target, context)),
            Self::Oblige(id) => event
                .text("task", "oblige")
                .text("obligation", context.obligation_label(*id)),
            Self::Solve { variable, mode } => event
                .text("task", "solve")
                .text("variable", context.variable_label(*variable))
                .text("mode", format!("{mode:?}")),
            Self::Bind { symbol, source } => {
                let event = event
                    .text("task", "bind")
                    .text("symbol", context.symbol_label(*symbol));

                match source {
                    BindSource::Type(ty) => event.text("type", context.type_label(*ty)),
                    BindSource::Initializer { site, widening } => event
                        .text("initializer", context.flow_site_label(*site))
                        .text("widening", context.widening_label(*widening)),
                }
            }
            task => {
                let event = event.text("task", task.name());
                if let Some(node) = task.node() {
                    event
                        .text("node", context.node_label(node))
                        .text("at", context.node_source_label(node))
                } else {
                    event
                }
            }
        }
    }
}

/// Render one try propagation target.
fn try_target_label(target: TryPropagationTarget, context: &DumpContext<'_, '_>) -> String {
    match target {
        TryPropagationTarget::Failure { ty } => format!("failure({})", context.type_label(ty)),
        TryPropagationTarget::Return { ty } => ty
            .map(|ty| format!("return({})", context.type_label(ty)))
            .unwrap_or_else(|| "return(none)".to_string()),
    }
}
