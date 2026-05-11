use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::{self, JoinHandle};

use destack_heap::{SharedGcPhase, SharedHeap};
use destack_workspace::SchedulerMode;
use parking_lot::{Condvar, Mutex};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::heap::SharedRootSet;

/// Lineage-owned shared heap collection scheduler.
#[derive(Debug)]
pub struct Collector {
    /// Shared heap collector mode.
    mode: CollectorMode,
    /// Dedicated collector thread for concurrent mode.
    thread: Mutex<Option<CollectorThread>>,
}

/// Runtime-owned shared heap collection state.
#[derive(Debug)]
pub struct CollectorWork {
    /// Shared heap driven by this collector work.
    heap: Arc<SharedHeap>,
    /// Shared roots consumed by mark steps.
    roots: Arc<SharedRootSet>,
    /// Collector work state changed by world and collector threads.
    state: Mutex<CollectorWorkState>,
    /// Wake quiescence waiters when pending collection work drains.
    quiesce: Condvar,
    /// Terminal collection failure recorded by one collector run.
    failure: Mutex<Option<Box<RuntimeError>>>,
}

/// Shared heap collection scheduling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectorMode {
    /// Run shared heap collection from world ticks and worker assists.
    Cooperative,
    /// Run shared heap collection on one collector thread plus worker assists.
    Concurrent,
}

impl CollectorMode {
    /// Convert workspace scheduler mode into shared heap collection mode.
    pub const fn from_scheduler_mode(mode: SchedulerMode) -> Self {
        match mode {
            SchedulerMode::Cooperative => Self::Cooperative,
            SchedulerMode::Parallel => Self::Concurrent,
        }
    }

    /// Return whether this mode uses one background collector thread.
    pub const fn is_concurrent(self) -> bool {
        matches!(self, Self::Concurrent)
    }

    /// Convert this collector mode into workspace scheduler mode.
    pub const fn to_scheduler_mode(self) -> SchedulerMode {
        match self {
            Self::Cooperative => SchedulerMode::Cooperative,
            Self::Concurrent => SchedulerMode::Parallel,
        }
    }
}

/// Shared collection lifecycle state.
#[derive(Debug, Default)]
struct CollectorWorkState {
    /// Pending scheduler state.
    pending: CollectorWorkPending,
    /// Whether one collector thread is currently running a step.
    is_running: bool,
}

/// Pending collection scheduling state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CollectorWorkPending {
    /// No collection work is currently queued.
    #[default]
    Idle,
    /// One collector wake is queued.
    Scheduled,
    /// Collector work is suspended for capture or restore.
    Suspended,
}

/// Dedicated collector thread.
#[derive(Debug)]
struct CollectorThread {
    /// Wake sender for the collector thread.
    sender: Sender<CollectorThreadMessage>,
    /// Join handle for the live collector thread.
    join: Mutex<Option<JoinHandle<()>>>,
}

/// Collector thread message.
#[derive(Debug)]
enum CollectorThreadMessage {
    /// Run one bounded shared heap collection step.
    Wake(Arc<CollectorWork>),
    /// Shut down the collector thread.
    Stop,
}

impl Collector {
    /// Create one lineage-owned collector.
    pub(crate) fn new(mode: CollectorMode, name: impl Into<String>) -> RuntimeResult<Arc<Self>> {
        let collector = Arc::new(Self {
            mode,
            thread: Mutex::new(None),
        });

        if mode.is_concurrent() {
            let thread = CollectorThread::spawn(name, collector.clone())?;
            *collector.thread.lock() = Some(thread);
        }

        Ok(collector)
    }

    /// Return this collector's scheduling mode.
    pub const fn mode(&self) -> CollectorMode {
        self.mode
    }

    /// Wake concurrent collection for one world.
    pub(crate) fn wake(self: &Arc<Self>, work: &Arc<CollectorWork>) {
        if !self.mode.is_concurrent() || !work.schedule() {
            return;
        }

        let thread = self.thread.lock();
        let Some(thread) = thread.as_ref() else {
            work.fail_scheduled("collector thread is missing");

            return;
        };

        if thread.wake(work.clone()).is_err() {
            work.fail_scheduled("collector thread is stopped");
        }
    }
}

impl CollectorWork {
    /// Create one runtime-owned shared collection state.
    pub(crate) fn new(heap: Arc<SharedHeap>, roots: Arc<SharedRootSet>) -> Arc<Self> {
        Arc::new(Self {
            heap,
            roots,
            state: Mutex::new(CollectorWorkState::default()),
            quiesce: Condvar::new(),
            failure: Mutex::new(None),
        })
    }

    /// Return one pending collection failure when background collection failed.
    pub(crate) fn take_failure(&self) -> Option<Box<RuntimeError>> {
        self.failure.lock().take()
    }

    /// Suspend collection and wait for in-flight collection work to drain.
    pub(crate) fn quiesce(&self) {
        let mut state = self.state.lock();
        state.pending = CollectorWorkPending::Suspended;

        while state.is_running || state.pending == CollectorWorkPending::Scheduled {
            self.quiesce.wait(&mut state);
        }
    }

    /// Resume collection after a quiescent world operation.
    pub(crate) fn resume(&self) {
        let mut state = self.state.lock();
        if state.pending == CollectorWorkPending::Suspended {
            state.pending = CollectorWorkPending::Idle;
        }
        self.quiesce.notify_all();
    }

    /// Return whether concurrent collection work is queued or running.
    pub(crate) fn is_busy(&self) -> bool {
        let state = self.state.lock();

        state.is_running || state.pending == CollectorWorkPending::Scheduled
    }

    /// Run one scheduled collection step on the collector thread.
    fn run_scheduled(self: &Arc<Self>, collector: &Arc<Collector>) {
        if !self.begin_run() {
            return;
        }

        let should_continue = self.collect_with_failure();
        self.finish_run();

        if should_continue {
            collector.wake(self);
        }
    }

    /// Mark one scheduled collection step as running.
    fn begin_run(&self) -> bool {
        let mut state = self.state.lock();
        if state.pending != CollectorWorkPending::Scheduled {
            self.quiesce.notify_all();

            return false;
        }

        state.pending = CollectorWorkPending::Idle;
        state.is_running = true;

        true
    }

    /// Mark the running collection step as finished.
    fn finish_run(&self) {
        let mut state = self.state.lock();
        state.is_running = false;
        self.quiesce.notify_all();
    }

    /// Schedule one collection step.
    fn schedule(&self) -> bool {
        let mut state = self.state.lock();
        if state.pending != CollectorWorkPending::Idle {
            return false;
        }

        state.pending = CollectorWorkPending::Scheduled;

        true
    }

    /// Clear scheduled work after one collector scheduling failure.
    fn fail_scheduled(&self, message: impl Into<String>) {
        let mut state = self.state.lock();
        if state.pending == CollectorWorkPending::Scheduled {
            state.pending = CollectorWorkPending::Idle;
        }
        self.quiesce.notify_all();
        drop(state);

        *self.failure.lock() = Some(
            RuntimeError::Internal {
                message: message.into(),
            }
            .boxed(),
        );
    }

    /// Run one bounded shared collection increment and retain one failure.
    fn collect_with_failure(&self) -> bool {
        match self.collect() {
            Ok(should_continue) => should_continue,
            Err(error) => {
                *self.failure.lock() = Some(error);

                false
            }
        }
    }

    /// Run one bounded shared collection increment.
    fn collect(&self) -> RuntimeResult<bool> {
        if self.heap.gc_phase() == SharedGcPhase::Idle {
            return Ok(false);
        }

        let roots = self.roots.roots_snapshot();
        let roots_complete = self.roots.roots_complete();
        let budget_bytes = self.heap.take_collection_budget_bytes(1);
        let progress = self
            .heap
            .collect_step(roots.as_ref(), roots_complete, budget_bytes)
            .map_err(Box::<RuntimeError>::from)?;

        if self.heap.gc_phase() != SharedGcPhase::Mark || roots_complete {
            self.roots.clear_termination();
        } else if self.heap.mark_idle() {
            self.roots.request_termination();
        }

        let is_active = self.heap.gc_phase() != SharedGcPhase::Idle;
        let is_waiting_on_roots =
            self.heap.gc_phase() == SharedGcPhase::Mark && self.heap.mark_idle() && !roots_complete;

        Ok(progress.made_progress() || (is_active && !is_waiting_on_roots))
    }
}

impl CollectorThread {
    /// Spawn one dedicated collector thread.
    fn spawn(name: impl Into<String>, collector: Arc<Collector>) -> RuntimeResult<Self> {
        let (sender, receiver) = mpsc::channel();
        let join = thread::Builder::new()
            .name(name.into())
            .spawn(move || run_collector_thread(receiver, collector))
            .map_err(|error| {
                RuntimeError::Internal {
                    message: format!("collector thread failed to spawn: {error}"),
                }
                .boxed()
            })?;

        Ok(Self {
            sender,
            join: Mutex::new(Some(join)),
        })
    }

    /// Wake the collector thread.
    fn wake(&self, work: Arc<CollectorWork>) -> Result<(), SendError<CollectorThreadMessage>> {
        self.sender.send(CollectorThreadMessage::Wake(work))
    }
}

impl Drop for CollectorThread {
    fn drop(&mut self) {
        let _ = self.sender.send(CollectorThreadMessage::Stop);

        if let Some(join) = self.join.lock().take() {
            let _ = join.join();
        }
    }
}

/// Run the dedicated collector thread loop.
fn run_collector_thread(receiver: Receiver<CollectorThreadMessage>, collector: Arc<Collector>) {
    while let Ok(message) = receiver.recv() {
        match message {
            CollectorThreadMessage::Wake(work) => work.run_scheduled(&collector),
            CollectorThreadMessage::Stop => break,
        }
    }
}
