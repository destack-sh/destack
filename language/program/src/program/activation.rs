use std::fmt;

use crate::{Binding, Context, Fiber, Memory, Value, Word};

/// Action returned by one runtime poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// Continue execution in the current engine.
    Continue,
    /// Retain execution and return control to the host.
    Pause,
    /// Retain execution for transfer to another engine.
    Deoptimize,
}

/// Runtime services available to executing program code.
pub trait Runtime {
    /// Runtime operation failure.
    type Error;

    /// Return whether runtime work is pending.
    fn is_poll_requested(&self) -> bool;

    /// Service pending runtime work; roots are visited after control returns.
    fn poll(&mut self, memory: Memory<'_>) -> Result<Poll, Self::Error>;

    /// Call one linked runtime binding on one logical fiber.
    fn call_binding(
        &mut self,
        memory: Memory<'_>,
        context: Context,
        fiber: Fiber,
        binding: &Binding,
        arguments: &[Word],
        result: &mut [Word],
    ) -> Result<(), Self::Error>;

    /// Park one logical fiber, or take an already delivered wake value.
    fn park(&mut self, fiber: Fiber) -> Result<Park, Self::Error>;

    /// Allocate one detached fiber identity at a task boundary.
    fn detach(&mut self) -> Result<Fiber, Self::Error>;

    /// Retire one detached fiber that completed without parking.
    fn retire(&mut self, fiber: Fiber) -> Result<(), Self::Error>;
}

/// Decision returned by one fiber park request.
#[derive(Debug, PartialEq, Eq)]
pub enum Park {
    /// The innermost logical fiber is parked; the engine returns control to
    /// its worker, or continues the caller past a detach boundary.
    Parked,
    /// The wake already settled; execution continues immediately.
    Ready(Value),
}

/// One active program execution.
pub struct Activation<'runtime, 'memory, R>
where
    R: Runtime + ?Sized,
{
    /// Runtime operations available to executing code.
    pub runtime: &'runtime mut R,
    /// Memory available to this call.
    pub memory: Memory<'memory>,
    /// Current dynamically scoped execution context.
    pub context: &'runtime mut Context,
}

impl<R> Activation<'_, '_, R>
where
    R: Runtime + ?Sized,
{
    /// Reborrow this activation for one nested machine call.
    pub fn reborrow(&mut self) -> Activation<'_, '_, R> {
        Activation {
            runtime: &mut *self.runtime,
            memory: self.memory.reborrow(),
            context: &mut *self.context,
        }
    }
}

impl<R> fmt::Debug for Activation<'_, '_, R>
where
    R: Runtime + ?Sized,
{
    /// Format one active program execution.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Activation")
            .field("runtime", &true)
            .field("memory", &self.memory)
            .field("context", &self.context)
            .finish()
    }
}
