use destack_workspace::{RuntimeDiagnosticLevel, RuntimeDiagnosticOptions};

use super::{
    AgentErrorStore, RuntimeDiagnosticBatch, RuntimeDiagnosticStore, RuntimeError, RuntimeErrorId,
};

/// Agent-scoped diagnostics store for runtime errors and warning events.
#[derive(Debug, Default)]
pub struct AgentDiagnosticStore {
    /// Runtime error slot store used by ABI status error ids.
    errors: AgentErrorStore,
    /// Runtime diagnostics event store used by callback and best-effort lanes.
    events: RuntimeDiagnosticStore,
}

impl AgentDiagnosticStore {
    /// Create one agent diagnostics store from runtime diagnostic options.
    pub fn from_options(options: &RuntimeDiagnosticOptions) -> Self {
        Self {
            errors: AgentErrorStore::default(),
            events: RuntimeDiagnosticStore::from_options(options),
        }
    }

    /// Record one runtime error and return its stable identifier.
    pub fn record_error(&self, error: Box<RuntimeError>) -> RuntimeErrorId {
        self.errors.record(error)
    }

    /// Take one runtime error by identifier.
    pub fn take_error(&self, id: RuntimeErrorId) -> Option<Box<RuntimeError>> {
        self.errors.take(id)
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
        self.events
            .record(level, module, operation, message, os_code);
    }

    /// Record one runtime warning diagnostic event.
    pub fn warn(
        &self,
        module: impl Into<String>,
        operation: impl Into<String>,
        message: impl Into<String>,
        os_code: Option<u32>,
    ) {
        self.events.warn(module, operation, message, os_code);
    }

    /// Drain runtime diagnostic events and reset dropped counters.
    pub fn drain(&self) -> RuntimeDiagnosticBatch {
        self.events.drain()
    }

    /// Return the current queued runtime diagnostic count.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Return whether no runtime diagnostics are currently queued.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
