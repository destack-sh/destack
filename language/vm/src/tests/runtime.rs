use std::collections::HashMap;
use std::mem;

use destack_program as program;

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
        fiber: program::Fiber,
    },
    /// Allocate one detached fiber identity.
    Detach {
        /// The issued identity.
        fiber: program::Fiber,
    },
    /// Retire one detached fiber that never parked.
    Retire {
        /// The retired identity.
        fiber: program::Fiber,
    },
}

/// Strict runtime probe used by bytecode execution tests.
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
    /// Next detached fiber slot to issue.
    next_fiber: u32,
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

    /// Call one registered binding implementation.
    fn call_binding(
        &mut self,
        memory: program::Memory<'_>,
        _context: program::Context,
        _fiber: program::Fiber,
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
    fn park(&mut self, fiber: program::Fiber) -> Result<program::Park> {
        self.calls.push(RuntimeCall::Park { fiber });

        // deliver one queued wake immediately when present
        if self.wakes.is_empty() {
            Ok(program::Park::Parked)
        } else {
            Ok(program::Park::Ready(self.wakes.remove(0)))
        }
    }

    /// Allocate one detached fiber identity.
    fn detach(&mut self) -> Result<program::Fiber> {
        let fiber = program::Fiber::new(self.next_fiber, 1);
        self.next_fiber += 1;
        self.calls.push(RuntimeCall::Detach { fiber });

        Ok(fiber)
    }

    /// Retire one detached fiber that never parked.
    fn retire(&mut self, fiber: program::Fiber) -> Result<()> {
        self.calls.push(RuntimeCall::Retire { fiber });

        Ok(())
    }
}
