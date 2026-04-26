use super::{EventLoop, EventLoopWatch, Task, TaskStatus, Timer};
use crate::diagnostic::RuntimeResult;
use crate::host::{HostEvent, HostEventKind};
use crate::platform::ResourceId;
use crate::runtime::engine::{Continuation, Engine};
use crate::runtime::poller::{PollerEvent, PollerToken};
use destack_core::CaptureMode;
use destack_engine as engine;
impl EventLoop {
    /// Register one timer watch.
    pub fn watch_timer(
        &mut self,
        handle: ResourceId,
        runnable: Continuation,
        resume_value: engine::Value,
        priority: u8,
        engine: &mut Engine,
    ) -> RuntimeResult<()> {
        let watch = self.capture_watch(runnable, resume_value, priority, engine)?;
        self.timer_watches.insert(handle, watch);

        Ok(())
    }

    /// Remove the timer watch registered for one timer handle.
    pub fn unwatch_timer(&mut self, handle: ResourceId) -> Option<EventLoopWatch> {
        self.timer_watches.remove(&handle)
    }

    /// Register one event watch.
    pub fn watch_event(
        &mut self,
        token: PollerToken,
        runnable: Continuation,
        resume_value: engine::Value,
        priority: u8,
        engine: &mut Engine,
    ) -> RuntimeResult<()> {
        let watch = self.capture_watch(runnable, resume_value, priority, engine)?;
        self.poller_event_watches.insert(token, watch);

        Ok(())
    }

    /// Remove the event watch registered for one poller token.
    pub fn unwatch_event(&mut self, token: PollerToken) -> Option<EventLoopWatch> {
        self.poller_event_watches.remove(&token)
    }

    /// Return whether one event watch is registered for the given token.
    pub fn watches_event(&self, token: PollerToken) -> bool {
        self.poller_event_watches.contains_key(&token)
    }

    /// Register one host semantic event watch.
    pub fn watch_host_event(
        &mut self,
        kind: HostEventKind,
        runnable: Continuation,
        resume_value: engine::Value,
        priority: u8,
        engine: &mut Engine,
    ) -> RuntimeResult<()> {
        let watch = self.capture_watch(runnable, resume_value, priority, engine)?;
        self.host_event_watches.insert(kind, watch);

        Ok(())
    }

    /// Remove the host semantic event watch registered for one kind.
    pub fn unwatch_host_event(&mut self, kind: HostEventKind) -> Option<EventLoopWatch> {
        self.host_event_watches.remove(&kind)
    }

    /// Return whether one host semantic watch is registered for the given kind.
    pub fn watches_host_event(&self, kind: HostEventKind) -> bool {
        self.host_event_watches.contains_key(&kind)
    }

    /// Build one task for a fired timer watch.
    pub fn task_for_timer(&mut self, timer: Timer, engine: &mut Engine) -> Option<Task> {
        let handle = timer.handle.resource_id()?;
        let watch = self.timer_watches.get(&handle)?.clone();

        self.task_for_watch(&watch, engine).ok()
    }

    /// Build one task for one external event watch.
    pub fn task_for_event(&mut self, event: PollerEvent, engine: &mut Engine) -> Option<Task> {
        let watch = self.poller_event_watches.get(&event.token)?.clone();

        self.task_for_watch(&watch, engine).ok()
    }

    /// Build one task for one host semantic event watch.
    pub fn task_for_host_event(&mut self, event: HostEvent, engine: &mut Engine) -> Option<Task> {
        let kind = event.kind();
        let watch = self.host_event_watches.get(&kind)?.clone();

        self.task_for_watch(&watch, engine).ok()
    }

    /// Build one task from one watch payload.
    fn task_for_watch(
        &mut self,
        watch: &EventLoopWatch,
        engine: &mut Engine,
    ) -> RuntimeResult<Task> {
        let task_id = self.next_task_id();
        let runnable = engine.restore_continuation_image(&watch.runnable)?;

        Ok(Task {
            id: task_id,
            runnable,
            resume_value: watch.resume_value.clone(),
            status: TaskStatus::Ready,
            priority: watch.priority,
        })
    }

    /// Capture one immutable watch payload for repeatable dispatch.
    fn capture_watch(
        &self,
        runnable: Continuation,
        resume_value: engine::Value,
        priority: u8,
        engine: &mut Engine,
    ) -> RuntimeResult<EventLoopWatch> {
        let runnable = engine.continuation_image(&runnable, CaptureMode::Fork)?;

        Ok(EventLoopWatch {
            runnable,
            resume_value,
            priority,
        })
    }
}
