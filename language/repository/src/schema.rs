use destack_serde::SchemaRegistry;

use crate::{
    ArtifactAttemptSnapshot, Revision, TraceCounterSnapshot, TraceSnapshot, TraceSpanSnapshot,
    TraceStageSnapshot, TraceStats, TraceTimeSnapshot, TraceView,
};

/// Include public repository schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<Revision>();

    registry.register::<TraceSnapshot>();
    registry.register::<TraceView>();
    registry.register::<TraceStats>();
    registry.register::<TraceStageSnapshot>();
    registry.register::<TraceTimeSnapshot>();
    registry.register::<TraceSpanSnapshot>();
    registry.register::<TraceCounterSnapshot>();
    registry.register::<ArtifactAttemptSnapshot>();
}
