use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tspp_repository::{RuntimeDiagnosticLevel, RuntimeDiagnosticOptions};

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
    pub timestamp_nanos: i128,
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

/// Captured diagnostics store state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticImage {
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

impl DiagnosticState {
    /// Return whether this state can be captured or forked.
    fn is_quiescent(&self) -> bool {
        self.errors.iter().all(Option::is_none) && self.diagnostic_entries.is_empty()
    }

    /// Capture this quiescent state.
    fn image(&self) -> DiagnosticImage {
        DiagnosticImage {
            error_slot_count: self.errors.len(),
            error_generations: self.error_generations.clone(),
            free_error_slots: self.free_error_slots.clone(),
            dropped_since_drain: self.dropped_since_drain,
            next_diagnostic_sequence: self.next_diagnostic_sequence,
        }
    }
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
        state.next_diagnostic_sequence = state.next_diagnostic_sequence.wrapping_add(1);

        let record = DiagnosticEntry {
            sequence,
            timestamp_nanos: unix_now_nanos(),
            level,
            module: module.into(),
            operation: operation.into(),
            message: message.into(),
            os_code,
        };

        // keep the ring buffer bounded and track drops
        if state.diagnostic_entries.len() == self.capacity {
            state.diagnostic_entries.pop_front();
            state.dropped_since_drain = state.dropped_since_drain.wrapping_add(1);
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
        // capture only quiescent diagnostic state
        let image = {
            let state = self.state.lock();
            if !state.is_quiescent() {
                return Ok(None);
            }

            state.image()
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
        forked.restore(&image)?;

        Ok(Some(forked))
    }

    /// Capture one diagnostics image.
    pub(crate) fn image(&self) -> RuntimeResult<DiagnosticImage> {
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

        Ok(state.image())
    }

    /// Restore one diagnostics image.
    pub(crate) fn restore(&self, image: &DiagnosticImage) -> RuntimeResult<()> {
        // require one internally consistent slot table
        if image.error_generations.len() != image.error_slot_count {
            return Err(RuntimeError::Internal {
                message: "diagnostics image generation count does not match its slot count"
                    .to_string(),
            }
            .boxed());
        }
        let mut free_slots = vec![false; image.error_slot_count];
        for &slot in &image.free_error_slots {
            let slot = slot as usize;
            let Some(is_free) = free_slots.get_mut(slot) else {
                return Err(RuntimeError::Internal {
                    message: "diagnostics image contains an out-of-range free slot".to_string(),
                }
                .boxed());
            };
            if *is_free {
                return Err(RuntimeError::Internal {
                    message: "diagnostics image contains a duplicate free slot".to_string(),
                }
                .boxed());
            }
            *is_free = true;
        }

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
            .take(image.error_slot_count)
            .collect();
        state.error_generations = image.error_generations.clone();
        state.free_error_slots = image.free_error_slots.clone();
        state.diagnostic_entries.clear();
        state.dropped_since_drain = image.dropped_since_drain;
        state.next_diagnostic_sequence = image.next_diagnostic_sequence;

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
fn unix_now_nanos() -> i128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as i128,
        Err(error) => -(error.duration().as_nanos() as i128),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
