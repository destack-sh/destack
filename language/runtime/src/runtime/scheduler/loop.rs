use std::collections::VecDeque;
use std::sync::OnceLock;

use destack_workspace::{SchedulerOptions, SchedulerPolicy};
use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_heap as heap};

use super::{Microtask, MicrotaskId, Task, TaskId, Timer, TimerHandle, TimerQueue};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostEvent, HostEventKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::engine::{ContinuationImage, Engine};
use crate::runtime::heap::RootSink;
use crate::runtime::poller::{PollerEvent, PollerToken};
use crate::runtime::{DropCounts, ExecutionContext, ExecutionContextId};

/// Watch payload that can be dispatched as one event loop task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLoopWatch {
    /// Runnable continuation image to restore when dispatched.
    pub runnable: ContinuationImage,
    /// Resume value passed into the continuation.
    pub resume_value: engine::Value,
    /// Task priority used when queueing watched tasks.
    pub priority: u8,
}

/// Event loop for task queues, microtasks, timers, and platform events.
#[derive(Debug, Default)]
pub struct EventLoop {
    /// Pending macrotasks.
    pub(super) tasks: VecDeque<Task>,
    /// Pending microtasks that drain before macrotasks.
    pub(super) microtasks: VecDeque<Microtask>,
    /// Pending platform events.
    pub(super) events: VecDeque<PollerEvent>,
    /// Pending host semantic events.
    pub(super) host_events: VecDeque<HostEvent>,
    /// Ready timers waiting for dispatch.
    pub(super) ready_timers: Mutex<VecDeque<Timer>>,
    /// Timer queue for scheduled timer fires.
    pub(super) timers: Mutex<TimerQueue>,
    /// Timer handles canceled after scheduling and before dispatch.
    pub(super) canceled_timers: Mutex<FxHashSet<TimerHandle>>,

    /// Timer watch dispatch table keyed by timer handle.
    pub(super) timer_watches: FxHashMap<ResourceId, EventLoopWatch>,
    /// External event watch dispatch table keyed by poller token.
    pub(super) poller_event_watches: FxHashMap<PollerToken, EventLoopWatch>,
    /// Host event watch dispatch table keyed by host event kind.
    pub(super) host_event_watches: FxHashMap<HostEventKind, EventLoopWatch>,
    /// Configured event loop options.
    pub(super) options: SchedulerOptions,

    /// Next task identifier to issue.
    pub(super) next_task_id: u64,
    /// Next microtask identifier to issue.
    pub(super) next_microtask_id: u64,
    /// Next task queue sequence identifier to issue.
    pub(super) next_sequence: u64,
    /// Drop accounting at the event-loop boundary.
    pub(super) drop_counts: DropCounts,
    /// Number of host semantic events dispatched since the last poller event.
    pub(super) host_events_since_poller: u64,
    /// Canonical execution context identifier for this event loop.
    pub(super) execution_context_id: OnceLock<ExecutionContextId>,
}

impl EventLoop {
    /// Configure event loop options.
    pub fn configure(&mut self, options: SchedulerOptions) -> RuntimeResult<()> {
        // validate options before applying them
        self.validate_scheduler_options(&options)?;
        self.options = options;

        Ok(())
    }

    /// Borrow the configured scheduler options.
    pub fn options(&self) -> &SchedulerOptions {
        &self.options
    }

    /// Return the canonical execution context identifier for this event loop.
    pub fn execution_context_id(&self) -> ExecutionContextId {
        let Some(execution_context_id) = self.execution_context_id.get().copied() else {
            panic!("event loop execution context was not initialized");
        };

        execution_context_id
    }

    /// Initialize and return the canonical execution context identifier for this event loop.
    pub fn initialize_execution_context(
        &self,
        execution_context_id: ExecutionContextId,
    ) -> ExecutionContextId {
        *self
            .execution_context_id
            .get_or_init(|| execution_context_id)
    }

    /// Return one execution context bound to this event loop.
    pub fn execution_context(&self, is_process_main: bool) -> ExecutionContext {
        let execution_context_id = self.execution_context_id();

        ExecutionContext {
            id: execution_context_id,
            is_process_main,
        }
    }

    /// Borrow the timer queue.
    pub fn timers(&self) -> &Mutex<TimerQueue> {
        &self.timers
    }

    /// Enqueue a macrotask for execution.
    pub fn enqueue_task(&mut self, task: Task) {
        // insert higher priority tasks ahead of lower priority tasks
        let insert_at = self
            .tasks
            .iter()
            .position(|queued| queued.priority < task.priority)
            .unwrap_or(self.tasks.len());
        self.tasks.insert(insert_at, task);
    }

    /// Enqueue a microtask for execution.
    pub fn enqueue_microtask(&mut self, microtask: Microtask) {
        self.microtasks.push_back(microtask);
    }

    /// Enqueue external events.
    pub fn enqueue_events(&mut self, events: Vec<PollerEvent>) {
        let mut events = events;
        self.sort_platform_events(&mut events);
        self.events.extend(events);
    }

    /// Enqueue host semantic events.
    pub fn enqueue_host_events(&mut self, events: Vec<HostEvent>) {
        self.host_events.extend(events);
    }

    /// Allocate the next task identifier.
    pub fn next_task_id(&mut self) -> TaskId {
        let id = TaskId::new(self.next_task_id);
        self.next_task_id = self.next_task_id.wrapping_add(1);
        id
    }

    /// Allocate the next microtask identifier.
    pub fn next_microtask_id(&mut self) -> MicrotaskId {
        let id = MicrotaskId::new(self.next_microtask_id);
        self.next_microtask_id = self.next_microtask_id.wrapping_add(1);
        id
    }

    /// Allocate the next task queue sequence identifier.
    pub fn next_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        sequence
    }

    /// Pop the next microtask if available.
    pub fn pop_microtask(&mut self) -> Option<Microtask> {
        self.microtasks.pop_front()
    }

    /// Pop the next macrotask if available.
    pub fn pop_task(&mut self) -> Option<Task> {
        self.tasks.pop_front()
    }

    /// Report whether any microtasks are pending.
    pub fn has_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    /// Return whether this event loop currently retains any queued or watched work.
    pub(super) fn is_quiescent(&self) -> bool {
        if !self.tasks.is_empty()
            || !self.microtasks.is_empty()
            || !self.events.is_empty()
            || !self.host_events.is_empty()
            || !self.ready_timers.lock().is_empty()
        {
            return false;
        }

        let timers = self.timers.lock();
        if timers.has_pending_timers() || !self.canceled_timers.lock().is_empty() {
            return false;
        }
        drop(timers);

        self.timer_watches.is_empty()
            && self.poller_event_watches.is_empty()
            && self.host_event_watches.is_empty()
    }

    /// Visit GC roots retained by queued and watched event-loop state.
    pub(crate) fn visit_roots(
        &mut self,
        engine: &mut Engine,
        roots: &mut RootSink<'_>,
    ) -> RuntimeResult<()> {
        // queued tasks
        for task in &self.tasks {
            engine.visit_continuation_roots(&task.runnable, roots)?;
            visit_value_roots(&task.resume_value, roots)?;
        }

        // queued microtasks
        for microtask in &self.microtasks {
            engine.visit_continuation_roots(&microtask.continuation, roots)?;
            visit_value_roots(&microtask.resume_value, roots)?;
        }

        // watched continuations
        for watch in self.timer_watches.values() {
            engine.visit_continuation_image_roots(&watch.runnable, roots)?;
            visit_value_roots(&watch.resume_value, roots)?;
        }
        for watch in self.poller_event_watches.values() {
            engine.visit_continuation_image_roots(&watch.runnable, roots)?;
            visit_value_roots(&watch.resume_value, roots)?;
        }
        for watch in self.host_event_watches.values() {
            engine.visit_continuation_image_roots(&watch.runnable, roots)?;
            visit_value_roots(&watch.resume_value, roots)?;
        }

        Ok(())
    }

    /// Visit mutable local root slots retained by queued scheduler state.
    pub(crate) fn visit_root_slots(
        &mut self,
        engine: &mut Engine,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        // queued work
        for task in &mut self.tasks {
            engine.visit_continuation_root_slots(&mut task.runnable, visit)?;
            visit_value_root_slot(&mut task.resume_value, visit)?;
        }

        for microtask in &mut self.microtasks {
            engine.visit_continuation_root_slots(&mut microtask.continuation, visit)?;
            visit_value_root_slot(&mut microtask.resume_value, visit)?;
        }

        // watched work
        for watch in self.timer_watches.values_mut() {
            engine.visit_continuation_image_root_slots(&mut watch.runnable, visit)?;
            visit_value_root_slot(&mut watch.resume_value, visit)?;
        }

        for watch in self.poller_event_watches.values_mut() {
            engine.visit_continuation_image_root_slots(&mut watch.runnable, visit)?;
            visit_value_root_slot(&mut watch.resume_value, visit)?;
        }

        for watch in self.host_event_watches.values_mut() {
            engine.visit_continuation_image_root_slots(&mut watch.runnable, visit)?;
            visit_value_root_slot(&mut watch.resume_value, visit)?;
        }

        Ok(())
    }

    /// Validate one scheduler options payload.
    fn validate_scheduler_options(&self, options: &SchedulerOptions) -> RuntimeResult<()> {
        // enforce the currently implemented queue policy
        if options.policy != SchedulerPolicy::Fifo {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.scheduler.policy",
                "only fifo scheduling policy is currently supported",
            ))
            .boxed());
        }

        // reject unsupported access limits until task throttling lands
        if options.task_limit.is_some() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.scheduler.task_limit",
                "task limit is not implemented yet",
            ))
            .boxed());
        }

        // reject unsupported preemption until engine preempt points land
        if options.preempt_interval_ns.is_some() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.scheduler.preempt_interval_ns",
                "preempt interval is not implemented yet",
            ))
            .boxed());
        }

        // reject invalid budget and timing values
        if matches!(options.tick_budget_ns, Some(0))
            || matches!(options.microtask_budget, Some(0))
            || matches!(options.host_event_budget, Some(0))
            || matches!(options.max_microtask_depth, Some(0))
            || matches!(options.timer_resolution_ns, Some(0))
            || matches!(options.max_timer_coalesce_ns, Some(0))
        {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.scheduler",
                "scheduler budget and timer values must be greater than zero",
            ))
            .boxed());
        }

        Ok(())
    }
}

/// Visit heap roots embedded in one resume value.
fn visit_value_roots(value: &engine::Value, roots: &mut RootSink<'_>) -> RuntimeResult<()> {
    // direct heap roots
    match value {
        engine::Value::HeapReference(reference) => {
            roots.push_heap(*reference);
        }
        engine::Value::SharedHeapReference(reference) => {
            roots.push_shared_heap(*reference);
        }

        // non root payloads
        engine::Value::Void
        | engine::Value::Bool(_)
        | engine::Value::Int { .. }
        | engine::Value::UInt { .. }
        | engine::Value::Float32 { .. }
        | engine::Value::Float64 { .. }
        | engine::Value::Char(_)
        | engine::Value::RawPointer(_)
        | engine::Value::SharedRawPointer(_) => {}
    }

    Ok(())
}

/// Visit the mutable local root slot in one value.
fn visit_value_root_slot(
    value: &mut engine::Value,
    visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
) -> RuntimeResult<()> {
    let engine::Value::HeapReference(reference) = value else {
        return Ok(());
    };

    visit(heap::RootSlot::Reference(reference)).map_err(Box::<RuntimeError>::from)
}
