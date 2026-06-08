use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use destack_repository::{RuntimeDiagnosticLevel, RuntimeDiagnosticOptions};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::{RuntimeError, RuntimeResult};

/// Default diagnostics ring-buffer capacity when runtime options do not provide one.
const DEFAULT_DIAGNOSTIC_CAPACITY: usize = 1024;
/// Minimum valid diagnostics ring-buffer capacity.
const MIN_DIAGNOSTIC_CAPACITY: usize = 1;

/// Stable identifier for one stored diagnostic error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticId {
    /// The error slot index.
    slot: u32,
    /// The slot generation counter.
    generation: u32,
}

impl DiagnosticId {
    /// Create one diagnostic identifier from slot and generation values.
    pub const fn new(slot: u32, generation: u32) -> Self {
        Self { slot, generation }
    }

    /// Create one diagnostic identifier from the raw ABI value.
    pub const fn from_raw(raw: u64) -> Self {
        let slot = (raw & 0xffff_ffff) as u32;
        let generation = (raw >> 32) as u32;

        Self::new(slot, generation)
    }

    /// Return the raw ABI value for this identifier.
    pub const fn to_raw(self) -> u64 {
        ((self.generation as u64) << 32) | (self.slot as u64)
    }

    /// Return the slot index.
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// Return the generation counter.
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// Diagnostic entry emitted by host and binding integration paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEntry {
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

/// Durable diagnostics store state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    /// The number of allocated error slots.
    pub error_slot_count: usize,
    /// Generation counters for each allocated error slot.
    pub error_generations: Vec<u32>,
    /// Free error slots for reuse.
    pub free_error_slots: Vec<u32>,
    /// Dropped diagnostics count since the previous drain point.
    pub dropped_since_drain: u64,
    /// The next diagnostic sequence to assign.
    pub next_diagnostic_sequence: u64,
}

/// Shared storage state for diagnostics and runtime errors.
#[derive(Debug)]
struct DiagnosticState {
    /// Recorded runtime errors for diagnostic lookup.
    errors: Vec<Option<Box<RuntimeError>>>,
    /// Generation counters for each error slot.
    error_generations: Vec<u32>,
    /// Free error slots for reuse.
    free_error_slots: Vec<u32>,
    /// Ring buffer of runtime diagnostics.
    diagnostic_entries: VecDeque<DiagnosticEntry>,
    /// Number of diagnostic entries dropped since the previous drain call.
    dropped_since_drain: u64,
    /// Monotonic sequence number generator for diagnostics.
    next_diagnostic_sequence: u64,
}

/// Diagnostics and runtime-error storage.
#[derive(Debug)]
pub struct DiagnosticStore {
    /// Minimum severity required for recording diagnostics.
    minimum_level: RuntimeDiagnosticLevel,
    /// Ring-buffer capacity limit.
    capacity: usize,
    /// Shared mutable store state.
    state: Mutex<DiagnosticState>,
}

impl Default for DiagnosticStore {
    /// Create one diagnostics store with default runtime options.
    fn default() -> Self {
        Self::from_options(&RuntimeDiagnosticOptions::default())
    }
}

impl DiagnosticStore {
    /// Create one diagnostics store from runtime diagnostic options.
    pub fn from_options(options: &RuntimeDiagnosticOptions) -> Self {
        let capacity = match options.capacity {
            Some(capacity) => capacity,
            None => DEFAULT_DIAGNOSTIC_CAPACITY as u64,
        };
        let capacity = capacity.min(usize::MAX as u64) as usize;
        let capacity = capacity.max(MIN_DIAGNOSTIC_CAPACITY);

        Self {
            minimum_level: options.level,
            capacity,
            state: Mutex::new(DiagnosticState {
                errors: Vec::new(),
                error_generations: Vec::new(),
                free_error_slots: Vec::new(),
                diagnostic_entries: VecDeque::with_capacity(capacity),
                dropped_since_drain: 0,
                next_diagnostic_sequence: 1,
            }),
        }
    }

    /// Record one runtime error and return its stable identifier.
    pub fn record_error(&self, error: Box<RuntimeError>) -> DiagnosticId {
        // lock the store for mutation
        let mut state = self.state.lock();

        // reuse the most recent free slot if possible
        if let Some(slot) = state.free_error_slots.pop() {
            let slot_index = slot as usize;
            let generation = state.error_generations[slot_index];
            state.errors[slot_index] = Some(error);

            return DiagnosticId::new(slot, generation);
        }

        // allocate a new slot
        let slot = state.errors.len() as u32;
        state.errors.push(Some(error));
        state.error_generations.push(0);

        DiagnosticId::new(slot, 0)
    }

    /// Take one runtime error by identifier.
    pub fn take_error(&self, id: DiagnosticId) -> Option<Box<RuntimeError>> {
        // lock the store for mutation
        let mut state = self.state.lock();

        // validate the slot bounds
        let slot = id.slot() as usize;
        if slot >= state.errors.len() {
            return None;
        }

        // validate generation for slot reuse
        let generation = state.error_generations[slot];
        if generation != id.generation() {
            return None;
        }

        // take the error and recycle the slot
        let error = state.errors[slot].take()?;

        // recycle only while generation ids remain unique
        if let Some(next_generation) = generation.checked_add(1) {
            state.error_generations[slot] = next_generation;
            state.free_error_slots.push(id.slot());
        }

        Some(error)
    }

    /// Record one runtime diagnostic event.
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
        let sequence = state.next_diagnostic_sequence;
        state.next_diagnostic_sequence = state.next_diagnostic_sequence.saturating_add(1);

        let record = DiagnosticEntry {
            sequence,
            timestamp_ns: unix_now_ns(),
            level,
            module: module.into(),
            operation: operation.into(),
            message: message.into(),
            os_code,
        };

        // keep the ring buffer bounded and track drops
        if state.diagnostic_entries.len() == self.capacity {
            state.diagnostic_entries.pop_front();
            state.dropped_since_drain = state.dropped_since_drain.saturating_add(1);
        }
        state.diagnostic_entries.push_back(record);
    }

    /// Record one runtime warning diagnostic event.
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

    /// Return the current queued runtime diagnostic count.
    pub fn len(&self) -> usize {
        let state = self.state.lock();
        state.diagnostic_entries.len()
    }

    /// Return whether no runtime diagnostics are currently queued.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fork one quiescent diagnostics store.
    pub(crate) fn try_fork(&self) -> RuntimeResult<Option<Self>> {
        // require one quiescent diagnostics state
        let snapshot = match self.snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => return Ok(None),
        };

        // rebuild the same storage policy on one fresh store
        let forked = Self {
            minimum_level: self.minimum_level,
            capacity: self.capacity,
            state: Mutex::new(DiagnosticState {
                errors: Vec::new(),
                error_generations: Vec::new(),
                free_error_slots: Vec::new(),
                diagnostic_entries: VecDeque::with_capacity(self.capacity),
                dropped_since_drain: 0,
                next_diagnostic_sequence: 1,
            }),
        };
        forked.restore_snapshot(&snapshot)?;

        Ok(Some(forked))
    }

    /// Capture one durable diagnostics snapshot.
    pub(crate) fn snapshot(&self) -> RuntimeResult<DiagnosticSnapshot> {
        let state = self.state.lock();

        // require no live stored errors
        if state.errors.iter().any(Option::is_some) {
            return Err(RuntimeError::Internal {
                message: "diagnostics cannot capture: stored errors are still present".to_string(),
            }
            .boxed());
        }

        // require no queued diagnostic entries
        if !state.diagnostic_entries.is_empty() {
            return Err(RuntimeError::Internal {
                message: "diagnostics cannot capture: entries are still queued".to_string(),
            }
            .boxed());
        }

        Ok(DiagnosticSnapshot {
            error_slot_count: state.errors.len(),
            error_generations: state.error_generations.clone(),
            free_error_slots: state.free_error_slots.clone(),
            dropped_since_drain: state.dropped_since_drain,
            next_diagnostic_sequence: state.next_diagnostic_sequence,
        })
    }

    /// Restore one durable diagnostics snapshot.
    pub(crate) fn restore_snapshot(&self, snapshot: &DiagnosticSnapshot) -> RuntimeResult<()> {
        let mut state = self.state.lock();

        // require no live stored errors
        if state.errors.iter().any(Option::is_some) {
            return Err(RuntimeError::Internal {
                message: "diagnostics cannot restore: stored errors are still present".to_string(),
            }
            .boxed());
        }

        // require no queued diagnostic entries
        if !state.diagnostic_entries.is_empty() {
            return Err(RuntimeError::Internal {
                message: "diagnostics cannot restore: entries are still queued".to_string(),
            }
            .boxed());
        }

        state.errors = std::iter::repeat_with(|| None)
            .take(snapshot.error_slot_count)
            .collect();
        state.error_generations = snapshot.error_generations.clone();
        state.free_error_slots = snapshot.free_error_slots.clone();
        state.diagnostic_entries.clear();
        state.dropped_since_drain = snapshot.dropped_since_drain;
        state.next_diagnostic_sequence = snapshot.next_diagnostic_sequence;

        Ok(())
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

    now.as_nanos().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
