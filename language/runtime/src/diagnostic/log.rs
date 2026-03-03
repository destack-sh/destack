use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use destack_workspace::{RuntimeDiagnosticLevel, RuntimeDiagnosticOptions};
use parking_lot::Mutex;

/// Default diagnostics ring-buffer capacity when runtime options do not provide one.
const DEFAULT_DIAGNOSTIC_CAPACITY: usize = 1024;
/// Minimum valid diagnostics ring-buffer capacity.
const MIN_DIAGNOSTIC_CAPACITY: usize = 1;

/// Runtime diagnostic entry emitted by host and binding integration paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDiagnosticRecord {
    /// Monotonic in-store sequence number.
    pub sequence: u64,
    /// Wall-clock timestamp for this diagnostic, in nanoseconds since unix epoch.
    pub timestamp_ns: u64,
    /// Diagnostic severity level.
    pub level: RuntimeDiagnosticLevel,
    /// Module name for this diagnostic.
    pub module: String,
    /// Operation identifier for this diagnostic.
    pub operation: String,
    /// Human-readable message payload.
    pub message: String,
    /// Optional host error code associated with this diagnostic.
    pub os_code: Option<u32>,
}

/// Runtime diagnostic drain result payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDiagnosticBatch {
    /// Drained diagnostic entries in emission order.
    pub entries: Vec<RuntimeDiagnosticRecord>,
    /// Number of dropped entries since the previous drain operation.
    pub dropped_count: u64,
}

/// Shared storage state for runtime diagnostics.
#[derive(Debug)]
struct AgentDiagnosticStoreState {
    /// Ring buffer of runtime diagnostics.
    entries: VecDeque<RuntimeDiagnosticRecord>,
    /// Number of entries dropped since the previous drain call.
    dropped_since_drain: u64,
    /// Monotonic sequence number generator.
    next_sequence: u64,
}

/// Per-agent runtime diagnostics storage.
#[derive(Debug, Clone)]
pub struct RuntimeDiagnosticStore {
    /// Minimum severity required for recording.
    minimum_level: RuntimeDiagnosticLevel,
    /// Ring-buffer capacity limit.
    capacity: usize,
    /// Shared mutable store state.
    state: Arc<Mutex<AgentDiagnosticStoreState>>,
}

impl Default for RuntimeDiagnosticStore {
    fn default() -> Self {
        Self::from_options(&RuntimeDiagnosticOptions::default())
    }
}

impl RuntimeDiagnosticStore {
    /// Create one diagnostics store from runtime configuration options.
    pub fn from_options(options: &RuntimeDiagnosticOptions) -> Self {
        let capacity = options
            .capacity
            .unwrap_or(DEFAULT_DIAGNOSTIC_CAPACITY as u64)
            .try_into()
            .unwrap_or(usize::MAX)
            .max(MIN_DIAGNOSTIC_CAPACITY);

        Self {
            minimum_level: options.level,
            capacity,
            state: Arc::new(Mutex::new(AgentDiagnosticStoreState {
                entries: VecDeque::with_capacity(capacity),
                dropped_since_drain: 0,
                next_sequence: 1,
            })),
        }
    }

    /// Return the configured minimum diagnostic level.
    pub const fn minimum_level(&self) -> RuntimeDiagnosticLevel {
        self.minimum_level
    }

    /// Record one runtime diagnostic when the configured level allows it.
    pub fn record(
        &self,
        level: RuntimeDiagnosticLevel,
        module: impl Into<String>,
        operation: impl Into<String>,
        message: impl Into<String>,
        os_code: Option<u32>,
    ) {
        // skip diagnostics below the configured threshold
        if !is_level_enabled(self.minimum_level, level) {
            return;
        }

        // allocate one record with stable sequence and timestamp
        let mut state = self.state.lock();
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.saturating_add(1);

        let record = RuntimeDiagnosticRecord {
            sequence,
            timestamp_ns: unix_now_ns(),
            level,
            module: module.into(),
            operation: operation.into(),
            message: message.into(),
            os_code,
        };

        // keep the ring buffer bounded and track drops
        if state.entries.len() == self.capacity {
            state.entries.pop_front();
            state.dropped_since_drain = state.dropped_since_drain.saturating_add(1);
        }
        state.entries.push_back(record);
    }

    /// Record one warning diagnostic.
    pub fn warn(
        &self,
        module: impl Into<String>,
        operation: impl Into<String>,
        message: impl Into<String>,
        os_code: Option<u32>,
    ) {
        self.record(
            RuntimeDiagnosticLevel::Warn,
            module,
            operation,
            message,
            os_code,
        );
    }

    /// Drain buffered diagnostics and reset drop counters.
    pub fn drain(&self) -> RuntimeDiagnosticBatch {
        let mut state = self.state.lock();
        let entries = state.entries.drain(..).collect();
        let dropped_count = std::mem::take(&mut state.dropped_since_drain);

        RuntimeDiagnosticBatch {
            entries,
            dropped_count,
        }
    }

    /// Return the current queued diagnostic count.
    pub fn len(&self) -> usize {
        let state = self.state.lock();
        state.entries.len()
    }

    /// Return whether no diagnostics are currently queued.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Return whether one diagnostic level passes one minimum-level threshold.
fn is_level_enabled(minimum_level: RuntimeDiagnosticLevel, level: RuntimeDiagnosticLevel) -> bool {
    if minimum_level == RuntimeDiagnosticLevel::Off {
        return false;
    }

    level_rank(level) <= level_rank(minimum_level)
}

/// Return the numeric severity rank for one diagnostic level.
fn level_rank(level: RuntimeDiagnosticLevel) -> u8 {
    match level {
        RuntimeDiagnosticLevel::Off => 0,
        RuntimeDiagnosticLevel::Error => 1,
        RuntimeDiagnosticLevel::Warn => 2,
        RuntimeDiagnosticLevel::Info => 3,
        RuntimeDiagnosticLevel::Debug => 4,
        RuntimeDiagnosticLevel::Trace => 5,
    }
}

/// Return the current wall-clock timestamp in nanoseconds since unix epoch.
fn unix_now_ns() -> u64 {
    let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration,
        Err(_) => return 0,
    };
    now.as_nanos().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Record diagnostics at or above the configured severity threshold.
    #[test]
    fn test_record_filters_by_level() {
        let store = RuntimeDiagnosticStore::from_options(&RuntimeDiagnosticOptions {
            level: RuntimeDiagnosticLevel::Warn,
            capacity: Some(8),
        });

        store.record(
            RuntimeDiagnosticLevel::Info,
            "display",
            "destack.display.window.open",
            "ignored diagnostic",
            None,
        );
        store.record(
            RuntimeDiagnosticLevel::Warn,
            "display",
            "destack.display.window.open",
            "recorded diagnostic",
            Some(5),
        );

        let batch = store.drain();
        assert_eq!(batch.entries.len(), 1);
        assert_eq!(batch.entries[0].level, RuntimeDiagnosticLevel::Warn);
        assert_eq!(batch.entries[0].module, "display");
        assert_eq!(batch.entries[0].os_code, Some(5));
    }

    /// Keep diagnostics bounded to the configured ring-buffer capacity.
    #[test]
    fn test_record_enforces_capacity_and_drops_oldest() {
        let store = RuntimeDiagnosticStore::from_options(&RuntimeDiagnosticOptions {
            level: RuntimeDiagnosticLevel::Trace,
            capacity: Some(2),
        });

        store.record(
            RuntimeDiagnosticLevel::Error,
            "display",
            "op1",
            "message1",
            None,
        );
        store.record(
            RuntimeDiagnosticLevel::Warn,
            "display",
            "op2",
            "message2",
            None,
        );
        store.record(
            RuntimeDiagnosticLevel::Info,
            "display",
            "op3",
            "message3",
            None,
        );

        let batch = store.drain();
        assert_eq!(batch.entries.len(), 2);
        assert_eq!(batch.entries[0].operation, "op2");
        assert_eq!(batch.entries[1].operation, "op3");
        assert_eq!(batch.dropped_count, 1);
    }

    /// Emit monotonically increasing sequence numbers for recorded diagnostics.
    #[test]
    fn test_record_assigns_monotonic_sequence() {
        let store = RuntimeDiagnosticStore::from_options(&RuntimeDiagnosticOptions {
            level: RuntimeDiagnosticLevel::Trace,
            capacity: Some(4),
        });

        store.warn("display", "op1", "message1", None);
        store.warn("display", "op2", "message2", None);

        let batch = store.drain();
        assert_eq!(batch.entries.len(), 2);
        assert_eq!(batch.entries[0].sequence, 1);
        assert_eq!(batch.entries[1].sequence, 2);
    }
}
