use super::{Loop, LoopWatch, Task, TaskStatus, Timer};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostEvent, HostEventKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::engine::EngineContinuation;
use crate::runtime::poller::{PollerEvent, PollerToken};

impl Loop {
    /// Register one timer watch.
    pub fn watch_timer(&mut self, handle: ResourceId, watch: LoopWatch) -> RuntimeResult<()> {
        // only native continuations can be cloned for repeated dispatch
        self.validate_watch(&watch)?;
        self.timer_watches.insert(handle, watch);

        Ok(())
    }

    /// Remove the timer watch registered for one timer handle.
    pub fn unwatch_timer(&mut self, handle: ResourceId) -> Option<LoopWatch> {
        self.timer_watches.remove(&handle)
    }

    /// Register one event watch.
    pub fn watch_event(&mut self, token: PollerToken, watch: LoopWatch) -> RuntimeResult<()> {
        // only native continuations can be cloned for repeated dispatch
        self.validate_watch(&watch)?;
        self.poller_event_watches.insert(token, watch);

        Ok(())
    }

    /// Remove the event watch registered for one poller token.
    pub fn unwatch_event(&mut self, token: PollerToken) -> Option<LoopWatch> {
        self.poller_event_watches.remove(&token)
    }

    /// Return whether one event watch is registered for the given token.
    pub fn watches_event(&self, token: PollerToken) -> bool {
        self.poller_event_watches.contains_key(&token)
    }

    /// Register one host semantic event watch.
    pub fn watch_host_event(&mut self, kind: HostEventKind, watch: LoopWatch) -> RuntimeResult<()> {
        // only native continuations can be cloned for repeated dispatch
        self.validate_watch(&watch)?;
        self.host_event_watches.insert(kind, watch);

        Ok(())
    }

    /// Remove the host semantic event watch registered for one kind.
    pub fn unwatch_host_event(&mut self, kind: HostEventKind) -> Option<LoopWatch> {
        self.host_event_watches.remove(&kind)
    }

    /// Return whether one host semantic watch is registered for the given kind.
    pub fn watches_host_event(&self, kind: HostEventKind) -> bool {
        self.host_event_watches.contains_key(&kind)
    }

    /// Build one task for a fired timer watch.
    pub fn task_for_timer(&mut self, timer: Timer) -> Option<Task> {
        let handle = timer.handle.resource_id()?;
        let watch = self.timer_watches.get(&handle)?;
        let EngineContinuation::Native(native) = watch.runnable else {
            return None;
        };
        let watch = LoopWatch {
            runnable: EngineContinuation::Native(native),
            resume_value: watch.resume_value,
            priority: watch.priority,
        };

        Some(self.task_for_watch(watch))
    }

    /// Build one task for one external event watch.
    pub fn task_for_event(&mut self, event: PollerEvent) -> Option<Task> {
        let watch = self.poller_event_watches.get(&event.token)?;
        let EngineContinuation::Native(native) = watch.runnable else {
            return None;
        };
        let watch = LoopWatch {
            runnable: EngineContinuation::Native(native),
            resume_value: watch.resume_value,
            priority: watch.priority,
        };

        Some(self.task_for_watch(watch))
    }

    /// Build one task for one host semantic event watch.
    pub fn task_for_host_event(&mut self, event: HostEvent) -> Option<Task> {
        let kind = event.kind();
        let watch = self.host_event_watches.get(&kind)?;
        let EngineContinuation::Native(native) = watch.runnable else {
            return None;
        };
        let watch = LoopWatch {
            runnable: EngineContinuation::Native(native),
            resume_value: watch.resume_value,
            priority: watch.priority,
        };

        Some(self.task_for_watch(watch))
    }

    /// Build one task from one watch payload.
    fn task_for_watch(&mut self, watch: LoopWatch) -> Task {
        let task_id = self.next_task_id();
        Task {
            id: task_id,
            runnable: watch.runnable,
            resume_value: watch.resume_value,
            status: TaskStatus::Ready,
            priority: watch.priority,
        }
    }

    /// Validate one watch payload for repeatable dispatch.
    fn validate_watch(&self, watch: &LoopWatch) -> RuntimeResult<()> {
        if matches!(watch.runnable, EngineContinuation::Vm(_)) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "watch.runnable",
                "vm continuations are not supported for event loop watches",
            ))
            .boxed());
        }

        Ok(())
    }
}
