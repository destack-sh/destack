use std::collections::{BTreeMap, VecDeque};

use tspp_program as program;
use tspp_vm as vm;

use super::fiber::{FiberTable, Resumer};
use super::timer::TimerQueue;
use super::{Call, Invocation, Runnable, RunnableId, Wake, WakeKey};
use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Event loop for tasks, microtasks, timers, waiters, and wakes.
#[derive(Debug, Default)]
pub(crate) struct EventLoop {
    /// Pending tasks.
    pub(super) tasks: VecDeque<Runnable>,
    /// The runnable a resume transfer hands the worker to, run before any queued microtask.
    pub(super) next: Option<Runnable>,
    /// Pending microtasks that drain before tasks.
    pub(super) microtasks: VecDeque<Runnable>,
    /// Timer queue for scheduled timer fires.
    pub(super) timers: TimerQueue,
    /// Pending external wakes.
    pub(super) wakes: VecDeque<Wake>,
    /// Repeatable callbacks keyed by their external wake source.
    pub(super) wake_waiters: BTreeMap<WakeKey, Call>,
    /// Live fibers with their parked executions.
    pub(super) fibers: FiberTable,
    /// Values released by scheduler transitions and awaiting destruction.
    pub(super) drops: Vec<program::Value>,
    /// Next runnable identifier to issue.
    pub(super) next_runnable_id: u64,
}

impl EventLoop {
    /// Enqueue a task for execution.
    pub(crate) fn enqueue_task(&mut self, invocation: Invocation) -> RunnableId {
        let runnable = self.identify(invocation);
        let id = runnable.id;
        self.tasks.push_back(runnable);

        id
    }

    /// Enqueue a microtask for execution.
    pub(crate) fn enqueue_microtask(&mut self, invocation: Invocation) -> RunnableId {
        let runnable = self.identify(invocation);
        let id = runnable.id;
        self.microtasks.push_back(runnable);

        id
    }

    /// Hand the worker to one runnable before any queued microtask.
    fn hand_off(&mut self, invocation: Invocation) -> RuntimeResult<()> {
        if self.next.is_some() {
            return Err(RuntimeError::Internal {
                message: "a resume transfer while another is pending".to_string(),
            }
            .boxed());
        }
        self.next = Some(self.identify(invocation));

        Ok(())
    }

    /// Enqueue one wake.
    pub(crate) fn enqueue_wake(&mut self, wake: Wake) {
        self.wakes.push_back(wake);
    }

    /// Pop the handed-off runnable, else the next microtask.
    pub(crate) fn pop_microtask(&mut self) -> Option<Runnable> {
        self.next.take().or_else(|| self.microtasks.pop_front())
    }

    /// Pop the next task if available.
    pub(crate) fn pop_task(&mut self) -> Option<Runnable> {
        self.tasks.pop_front()
    }

    /// Insert one running fiber and return its identity.
    pub(crate) fn insert_fiber(&mut self) -> program::FiberId {
        self.fibers.insert()
    }

    /// Create one fiber holding its call.
    pub(crate) fn create_fiber(&mut self, call: Call, is_joined: bool) -> program::FiberId {
        self.fibers.create(call, is_joined)
    }

    /// Hand the worker from one resumer to one fiber until that fiber parks or finishes.
    pub(crate) fn resume(
        &mut self,
        fiber_id: program::FiberId,
        resumer: Resumer,
        value: program::Value,
    ) -> RuntimeResult<()> {
        self.fibers.mark_resuming(resumer.fiber_id)?;
        self.fibers.set_resumer(fiber_id, resumer)?;

        self.hand_off_fiber(fiber_id, value)
    }

    /// Park one running fiber with its retained execution, waking its resumer unless it resumes.
    pub(crate) fn park_fiber(
        &mut self,
        fiber_id: program::FiberId,
        execution: vm::Fiber,
    ) -> RuntimeResult<()> {
        // a wake that raced the park settles the fiber immediately
        if let Some(value) = self.fibers.park(fiber_id, execution)? {
            self.enqueue_microtask(Invocation::wake(fiber_id, value));
        }

        // keep the resumer waiting while the fiber runs one it resumed
        if self.fibers.take_resuming(fiber_id)? {
            return Ok(());
        }

        self.wake_resumer(fiber_id)
    }

    /// Finish one fiber and wake its resumer.
    pub(crate) fn finish_fiber(
        &mut self,
        fiber_id: program::FiberId,
        value: program::Value,
    ) -> RuntimeResult<()> {
        if let Some(value) = self.fibers.finish(fiber_id, value)? {
            self.release(value);

            return self.retire_fiber(fiber_id);
        }

        self.wake_resumer(fiber_id)
    }

    /// Retire one joined fiber once it finished, releasing its result.
    pub(crate) fn join(&mut self, fiber_id: program::FiberId) -> RuntimeResult<bool> {
        let Some(value) = self.fibers.take_finished(fiber_id)? else {
            return Ok(false);
        };
        self.release(value);

        Ok(true)
    }

    /// Queue one created fiber's start, returning whether the fiber was created.
    pub(crate) fn start_fiber(&mut self, fiber_id: program::FiberId) -> RuntimeResult<bool> {
        let Some(call) = self.fibers.start(fiber_id)? else {
            return Ok(false);
        };
        self.enqueue_microtask(Invocation::Start { fiber_id, call });

        Ok(true)
    }

    /// Hand the worker back to the fiber waiting for one fiber's next park or completion.
    fn wake_resumer(&mut self, fiber_id: program::FiberId) -> RuntimeResult<()> {
        if let Some(resumer) = self.fibers.take_resumer(fiber_id)? {
            self.hand_off_fiber(resumer.fiber_id, resumer.value)?;
        }

        Ok(())
    }

    /// Take one wake buffered before the running fiber parked.
    pub(crate) fn take_pending_wake(
        &mut self,
        fiber_id: program::FiberId,
    ) -> RuntimeResult<Option<program::Value>> {
        self.fibers.take_pending(fiber_id)
    }

    /// Deliver one wake to a fiber, starting it when created.
    pub(crate) fn wake_fiber(
        &mut self,
        fiber_id: program::FiberId,
        value: program::Value,
    ) -> RuntimeResult<()> {
        if let Some(invocation) = self.wake_invocation(fiber_id, value)? {
            self.enqueue_microtask(invocation);
        }

        Ok(())
    }

    /// Hand the worker to one woken fiber, starting it when created.
    fn hand_off_fiber(
        &mut self,
        fiber_id: program::FiberId,
        value: program::Value,
    ) -> RuntimeResult<()> {
        if let Some(invocation) = self.wake_invocation(fiber_id, value)? {
            self.hand_off(invocation)?;
        }

        Ok(())
    }

    /// Return the invocation that runs one woken fiber, else buffer the wake.
    fn wake_invocation(
        &mut self,
        fiber_id: program::FiberId,
        value: program::Value,
    ) -> RuntimeResult<Option<Invocation>> {
        // start a created fiber
        if let Some(call) = self.fibers.start(fiber_id)? {
            self.release(value);

            return Ok(Some(Invocation::Start { fiber_id, call }));
        }

        // run a parked fiber with its wake
        let invocation = self
            .fibers
            .wake(fiber_id, value)?
            .map(|value| Invocation::wake(fiber_id, value));

        Ok(invocation)
    }

    /// Take one woken fiber's execution for resumption.
    pub(crate) fn resume_fiber(&mut self, fiber_id: program::FiberId) -> RuntimeResult<vm::Fiber> {
        self.fibers.resume(fiber_id)
    }

    /// Remove one ended fiber and wake its resumer.
    pub(crate) fn retire_fiber(&mut self, fiber_id: program::FiberId) -> RuntimeResult<()> {
        let resumer = self.fibers.take_resumer(fiber_id)?;
        if let Some(pending) = self.fibers.remove(fiber_id)? {
            self.release(pending);
        }
        if let Some(resumer) = resumer {
            self.hand_off_fiber(resumer.fiber_id, resumer.value)?;
        }

        Ok(())
    }

    /// Iterate every parked or ready fiber execution.
    pub(crate) fn executions_mut(&mut self) -> impl Iterator<Item = &mut vm::Fiber> {
        self.fibers.executions_mut()
    }

    /// Pop one value awaiting generated destruction.
    pub(crate) fn pop_drop(&mut self) -> Option<program::Value> {
        self.drops.pop()
    }

    /// Retain one released value until generated destruction can execute.
    pub(crate) fn release(&mut self, value: program::Value) {
        self.drops.push(value);
    }

    /// Clear scheduler state and release every queued value.
    pub(crate) fn clear(&mut self) -> Vec<program::Value> {
        let tasks = std::mem::take(&mut self.tasks);
        let next = self.next.take();
        let microtasks = std::mem::take(&mut self.microtasks);

        // clear scheduler state before draining its queued values
        self.timers = TimerQueue::default();
        self.wakes.clear();
        self.wake_waiters.clear();
        self.fibers = FiberTable::default();
        self.next_runnable_id = 0;

        // release queued wake values with any values already awaiting destruction
        let mut released = std::mem::take(&mut self.drops);
        released.extend(
            tasks
                .into_iter()
                .chain(next)
                .chain(microtasks)
                .filter_map(Runnable::release),
        );

        released
    }

    /// Identify one function invocation for queue execution.
    fn identify(&mut self, invocation: Invocation) -> Runnable {
        let id = RunnableId::new(self.next_runnable_id);
        self.next_runnable_id += 1;

        Runnable::new(id, invocation)
    }
}
