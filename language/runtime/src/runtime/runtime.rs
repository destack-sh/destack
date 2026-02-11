use crate::diagnostic::RuntimeResult;
use crate::memory::Heap;
use crate::platform::bindings::{BindingPolicy, BindingRegistry};
use crate::platform::{PlatformContext, PlatformPoller};
use crate::scheduler::Scheduler;
use crate::snapshot::SnapshotStore;
use destack_workspace::RuntimeOptions;

use super::RuntimeContext;
use super::poller::poller_for_options;

/// Primary runtime instance for executing Destack programs.
pub struct Runtime {
    /// External binding registry and policy enforcement.
    pub bindings: BindingRegistry,
    /// Shared runtime state for platform bindings and execution.
    pub context: RuntimeContext,
    /// Managed heap and GC coordination.
    pub heap: Heap,
    /// Scheduler for tasks and microtasks.
    pub scheduler: Box<Scheduler>,
    /// Optional platform poller for external events.
    pub poller: Option<Box<dyn PlatformPoller>>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("bindings", &self.bindings)
            .field("context", &self.context)
            .field("heap", &self.heap)
            .field("scheduler", &self.scheduler)
            .field("poller", &"<platform poller>")
            .finish()
    }
}

impl Runtime {
    /// Create a runtime with explicit context.
    pub fn new(context: RuntimeContext) -> Self {
        let scheduler = Box::new(Scheduler::default());
        let mut bindings = BindingRegistry::new();
        bindings.set_policy(BindingPolicy::new(context.replay().mode()));
        bindings.set_runtime_handle(&context, scheduler.as_ref());
        bindings.install_native_defaults();

        Self {
            bindings,
            context,
            heap: Heap::default(),
            scheduler,
            poller: None,
        }
    }

    /// Create a runtime with explicit runtime options.
    pub fn from_runtime_options(
        platform: PlatformContext,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let context = RuntimeContext::from_runtime_options(platform, options);
        let mut runtime = Self::new(context);
        runtime.bindings.apply_runtime_options(options);
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }
        Ok(runtime)
    }

    /// Create a runtime with the configured platform poller.
    pub fn with_configured_poller(
        context: RuntimeContext,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let mut runtime = Self::new(context);
        runtime.bindings.apply_runtime_options(options);
        if let Some(poller) = poller_for_options(options)? {
            runtime.set_poller(poller);
        }
        Ok(runtime)
    }

    /// Attach a platform poller for external events.
    pub fn set_poller(&mut self, poller: Box<dyn PlatformPoller>) {
        self.poller = Some(poller);
    }

    /// Run the runtime event loop until it becomes idle.
    pub fn run(&mut self) -> RuntimeResult<()> {
        loop {
            // drive a single tick of the runtime
            let progressed = self.tick()?;

            // exit once no work remains
            if !progressed {
                break;
            }
        }

        Ok(())
    }

    /// Execute a single runtime tick.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        // poll timers for ready callbacks
        let now = self.context.time().wall_nanos();
        let ready_timers = self.scheduler.poll_timers(now)?;
        let mut progressed = !ready_timers.is_empty();

        // NOTE #Incomplete: wire timer callbacks into tasks

        // poll platform events if a poller is installed
        if let Some(poller) = self.poller.as_mut() {
            let event_count = self.scheduler.poll_poller(poller.as_mut(), Some(0))?;
            if event_count > 0 {
                progressed = true;
            }
        }

        // NOTE #Incomplete: wire scheduler runnables into tasks

        Ok(progressed)
    }

    /// Capture a runtime snapshot and record a checkpoint in the replay log.
    pub fn snapshot(&mut self, store: &SnapshotStore) -> RuntimeResult<()> {
        // allocate a new checkpoint id
        let checkpoint_id = store.allocate_checkpoint_id();

        // capture replay metadata
        let branch_id = self.context.replay().log().branch_id();
        let sequence = self.context.replay().log().next_sequence();

        // NOTE #Incomplete: snapshot payload capture is not implemented yet
        let payload = Vec::new();

        // write snapshot payload and register in the replay log
        let metadata = store.write_snapshot(checkpoint_id, branch_id, sequence, &payload)?;
        self.context
            .replay()
            .log()
            .record_checkpoint(metadata.into_checkpoint_index())?;

        Ok(())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new(RuntimeContext::default())
    }
}
