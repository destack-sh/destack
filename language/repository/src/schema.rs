use destack_serde::SchemaRegistry;

use crate::{
    ArtifactAttemptSnapshot, Destack, DestackLayout, DestackLayoutOverride, DestackLock,
    FormatterOptions, ManifestOverride, Revision, Settings, TraceCounterSnapshot, TraceSnapshot,
    TraceSpanKind, TraceSpanSnapshot, TraceStageSnapshot, TraceStats, TraceTimeSnapshot, TraceView,
};

/// Include public repository schema roots.
pub fn schema(registry: &mut SchemaRegistry) {
    registry.register::<Revision>();

    registry.register::<Destack>();
    registry.register::<Settings>();
    registry.register::<DestackLayout>();
    registry.register::<DestackLayoutOverride>();
    registry.register::<DestackLock>();
    registry.register::<ManifestOverride>();
    registry.register::<FormatterOptions>();

    registry.register::<TraceSnapshot>();
    registry.register::<TraceView>();
    registry.register::<TraceStats>();
    registry.register::<TraceStageSnapshot>();
    registry.register::<TraceTimeSnapshot>();
    registry.register::<TraceSpanKind>();
    registry.register::<TraceSpanSnapshot>();
    registry.register::<TraceCounterSnapshot>();
    registry.register::<ArtifactAttemptSnapshot>();
}
