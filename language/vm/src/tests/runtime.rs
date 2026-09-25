use std::collections::HashMap;
use std::mem;

use tspp_program as program;

use crate::{Error, Result};

/// One binding implementation used by bytecode execution tests.
pub(crate) type TestBinding =
    for<'a> fn(program::Memory<'a>, &[program::Word], &mut [program::Word]) -> Result<()>;

/// One runtime boundary call made by bytecode execution.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RuntimeCall {
    /// Service one runtime poll.
    Poll {
        /// Action returned to the machine.
        action: program::Poll,
    },
    /// Call one linked runtime binding.
    Binding {
        /// Stable binding identity.
        binding: program::BindingId,
        /// Flattened binding arguments.
        arguments: Vec<program::Word>,
    },
    /// Park one logical fiber.
    Park {
        /// The parked fiber identity.
        fiber_id: program::FiberId,
    },
}

/// Strict runtime used by bytecode execution tests.
#[derive(Debug, Default)]
pub(crate) struct TestRuntime {
    /// Binding implementations keyed by stable identity.
    bindings: HashMap<program::BindingId, TestBinding>,
    /// Wake values delivered to the next parks.
    wakes: Vec<program::Value>,
    /// Action returned by the next requested poll.
    poll: Option<program::Poll>,
    /// Runtime calls in execution order.
    calls: Vec<RuntimeCall>,
    /// Program event categories selected for execution.
    selected_events: program::EventSet,
    /// Program execution events in order.
    events: Vec<program::Event>,
}

impl TestRuntime {
    /// Register one binding implementation.
    pub(crate) fn bind(&mut self, name: &'static str, binding: TestBinding) {
        let id = program::BindingId::from_static_name(name);
        self.bindings.insert(id, binding);
    }

    /// Return and clear recorded runtime calls.
    pub(crate) fn take_calls(&mut self) -> Vec<RuntimeCall> {
        mem::take(&mut self.calls)
    }

    /// Select Program execution event categories.
    pub(crate) fn select_events(&mut self, kinds: impl IntoIterator<Item = program::EventKind>) {
        for kind in kinds {
            self.selected_events.insert(kind);
        }
    }

    /// Return and clear recorded Program execution events.
    pub(crate) fn take_events(&mut self) -> Vec<program::Event> {
        mem::take(&mut self.events)
    }

    /// Request one runtime poll action.
    pub(crate) fn request_poll(&mut self, action: program::Poll) {
        self.poll = Some(action);
    }
}

impl program::Runtime for TestRuntime {
    type Error = Error;

    /// Return whether execution must yield at the current runtime poll.
    fn is_poll_requested(&self) -> bool {
        self.poll.is_some()
    }

    /// Service one requested poll.
    fn poll(&mut self, _memory: program::Memory<'_>) -> Result<program::Poll> {
        let action = self.poll.take().ok_or_else(Error::invalid_instruction)?;
        self.calls.push(RuntimeCall::Poll { action });

        Ok(action)
    }

    /// Return Program event categories selected for observation.
    fn events(&self) -> program::EventSet {
        self.selected_events
    }

    /// Record one selected Program execution event.
    fn observe(
        &mut self,
        _fiber_id: Option<program::FiberId>,
        event: program::Event,
    ) -> Result<()> {
        self.events.push(event);

        Ok(())
    }

    /// Call one registered binding implementation.
    fn call_binding(
        &mut self,
        memory: program::Memory<'_>,
        _context: program::Context,
        _fiber_id: Option<program::FiberId>,
        binding: &program::Binding,
        arguments: &[program::Word],
        result: &mut [program::Word],
    ) -> Result<()> {
        let Some(invoke) = self.bindings.get(&binding.id).copied() else {
            return Err(Error::invalid_instruction());
        };
        self.calls.push(RuntimeCall::Binding {
            binding: binding.id,
            arguments: arguments.to_vec(),
        });

        invoke(memory, arguments, result)
    }

    /// Park one logical fiber or deliver one queued wake.
    fn park(&mut self, fiber_id: program::FiberId) -> Result<program::Park> {
        self.calls.push(RuntimeCall::Park { fiber_id });

        // deliver one queued wake immediately when present
        if self.wakes.is_empty() {
            Ok(program::Park::Parked)
        } else {
            Ok(program::Park::Ready(self.wakes.remove(0)))
        }
    }
}
