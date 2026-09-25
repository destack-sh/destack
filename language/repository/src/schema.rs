use tspp_serde::Schema;

use crate::{
    ArtifactAttemptSnapshot, Revision, TraceCounterSnapshot, TraceEvent, TraceEventField,
    TraceEventValue, TraceSnapshot, TraceSpanKind, TraceSpanSnapshot, TraceStageSnapshot,
    TraceStats, TraceTimeSnapshot, TraceView,
};

/// Include public repository schema roots.
pub fn schema(schema: &mut Schema) {
    schema.register::<Revision>();

    schema.register::<TraceSnapshot>();
    schema.register::<TraceView>();
    schema.register::<TraceStats>();
    schema.register::<TraceStageSnapshot>();
    schema.register::<TraceTimeSnapshot>();
    schema.register::<TraceSpanKind>();
    schema.register::<TraceSpanSnapshot>();
    schema.register::<TraceCounterSnapshot>();
    schema.register::<TraceEvent>();
    schema.register::<TraceEventField>();
    schema.register::<TraceEventValue>();
    schema.register::<ArtifactAttemptSnapshot>();
}
