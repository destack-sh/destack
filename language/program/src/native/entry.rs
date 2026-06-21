use destack_serde::Schema;
use serde::{Deserialize, Serialize};

use crate::FunctionId;

/// Native entry table keyed by program ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct EntryTable {
    /// Native function entries keyed by program function id.
    pub(super) function: Vec<Option<Entry>>,
    /// Native resume entries keyed by frame state id.
    pub(super) resume: Vec<Option<Resume>>,
}

impl EntryTable {
    /// Create one native entry table.
    pub fn new(function: Vec<Option<Entry>>, resume: Vec<Option<Resume>>) -> Self {
        Self { function, resume }
    }

    /// Return one native function entry.
    pub fn function(&self, function: FunctionId) -> Option<&Entry> {
        self.function.get(function.index()).and_then(Option::as_ref)
    }

    /// Return one native resume entry.
    pub fn resume(&self, frame_state: destack_mir::FrameStateId) -> Option<&Resume> {
        self.resume
            .get(frame_state.0 as usize)
            .and_then(Option::as_ref)
    }

    /// Return native function entries in dense program function id order.
    pub fn function_entry(&self) -> &[Option<Entry>] {
        &self.function
    }

    /// Return native resume entries in dense frame state id order.
    pub fn resume_entry(&self) -> &[Option<Resume>] {
        &self.resume
    }
}

/// Native function entry resolved by symbol name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct Entry {
    /// The function implemented by this entry.
    pub function: FunctionId,
    /// The native symbol exported by the linked image.
    pub symbol: String,
}

impl Entry {
    /// Create one native function entry.
    pub fn new(function: FunctionId, symbol: String) -> Self {
        Self { function, symbol }
    }
}

/// Native continuation resume entry resolved by symbol name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct Resume {
    /// The frame state resumed by this entry.
    pub frame_state: destack_mir::FrameStateId,
    /// The native symbol exported by the linked image.
    pub symbol: String,
}

impl Resume {
    /// Create one native resume entry.
    pub fn new(frame_state: destack_mir::FrameStateId, symbol: String) -> Self {
        Self {
            frame_state,
            symbol,
        }
    }
}
