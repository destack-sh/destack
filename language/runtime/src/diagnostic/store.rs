use parking_lot::Mutex;

use super::RuntimeError;

/// Stable identifier for stored runtime errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeErrorId(u64);

impl RuntimeErrorId {
    /// Create a runtime error identifier from slot and generation values.
    pub fn new(slot: u32, generation: u32) -> Self {
        let value = (u64::from(generation) << 32) | u64::from(slot);
        Self(value)
    }

    /// Create a runtime error identifier from a raw value.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Return the raw identifier value.
    pub fn get(self) -> u64 {
        self.0
    }

    /// Return the slot index.
    pub fn slot(self) -> u32 {
        (self.0 & 0xffff_ffff) as u32
    }

    /// Return the generation counter.
    pub fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }
}

#[derive(Debug, Default)]
struct RuntimeErrorStoreState {
    /// Recorded runtime errors for diagnostic lookup.
    entries: Vec<Option<Box<RuntimeError>>>,
    /// Generation counters for each slot.
    generations: Vec<u32>,
    /// Free slots for reuse.
    free: Vec<u32>,
}

/// Storage for runtime errors captured across the platform boundary.
#[derive(Debug, Default)]
pub struct RuntimeErrorStore {
    /// Recorded runtime errors for diagnostic lookup.
    state: Mutex<RuntimeErrorStoreState>,
}

impl RuntimeErrorStore {
    /// Record a runtime error and return its identifier.
    pub fn record(&self, error: Box<RuntimeError>) -> RuntimeErrorId {
        // lock the store for mutation
        let mut state = self.state.lock();

        // reuse the most recent free slot if possible
        if let Some(slot) = state.free.pop() {
            let slot_index = slot as usize;
            let generation = state.generations[slot_index];
            state.entries[slot_index] = Some(error);

            return RuntimeErrorId::new(slot, generation);
        }

        // allocate a new slot
        let slot = state.entries.len() as u32;
        state.entries.push(Some(error));
        state.generations.push(0);

        RuntimeErrorId::new(slot, 0)
    }

    /// Take a stored runtime error by id.
    pub fn take(&self, id: RuntimeErrorId) -> Option<Box<RuntimeError>> {
        // lock the store for mutation
        let mut state = self.state.lock();

        // validate the slot bounds
        let slot = id.slot() as usize;
        if slot >= state.entries.len() {
            return None;
        }

        // validate generation for slot reuse
        let generation = state.generations[slot];
        if generation != id.generation() {
            return None;
        }

        // take the error and recycle the slot
        let error = state.entries[slot].take()?;
        state.generations[slot] = generation.wrapping_add(1);
        state.free.push(id.slot());

        Some(error)
    }
}
