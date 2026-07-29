use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::{self, JoinHandle};

use destack_program as program;
use destack_repository::ExecutionMode;
use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::heap::SharedHeap;

/// World-owned shared heap collector scheduler.
#[derive(Debug)]
pub struct SharedCollector {
    /// Shared heap collector mode.
    mode: SharedCollectorMode,
    /// Dedicated collector thread for concurrent mode.
    thread: Mutex<Option<CollectorThread>>,
}

/// Shared heap collector scheduling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SharedCollectorMode {
    /// Run shared heap collection from world runs and worker assists.
    Cooperative,
    /// Run shared heap collection on one collector thread plus worker assists.
    Concurrent,
}

/// Dedicated collector thread.
#[derive(Debug)]
struct CollectorThread {
    /// Wake sender for the collector thread.
    sender: Sender<CollectorMessage>,
    /// Join handle for the live collector thread.
    join: Mutex<Option<JoinHandle<()>>>,
}

/// Shared collector thread message.
#[derive(Debug)]
enum CollectorMessage {
    /// Run one bounded shared heap collection step.
    Wake {
        /// Shared heap to advance.
        heap: Arc<SharedHeap>,
        /// Program owning the trace table.
        program: Arc<program::Program>,
    },
    /// Shut down the collector thread.
    Stop,
}

impl SharedCollectorMode {
    /// Resolve shared collection mode for one execution mode.
    pub const fn from_execution_mode(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Fast => Self::Concurrent,
            ExecutionMode::Strict | ExecutionMode::Record | ExecutionMode::Replay => {
                Self::Cooperative
            }
        }
    }

    /// Return whether this mode uses one background collector thread.
    pub const fn is_concurrent(self) -> bool {
        matches!(self, Self::Concurrent)
    }
}

impl SharedCollector {
    /// Create one world-owned shared collector.
    pub(crate) fn new(
        mode: SharedCollectorMode,
        name: impl Into<String>,
    ) -> RuntimeResult<Arc<Self>> {
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
    pub const fn mode(&self) -> SharedCollectorMode {
        self.mode
    }

    /// Wake concurrent collection for one shared heap.
    pub(crate) fn wake(self: &Arc<Self>, heap: &Arc<SharedHeap>, program: &Arc<program::Program>) {
        if !self.mode.is_concurrent() || !heap.schedule() {
            return;
        }

        let thread = self.thread.lock();
        let Some(thread) = thread.as_ref() else {
            heap.fail_scheduled("collector thread is missing");

            return;
        };

        if thread.wake(heap.clone(), program.clone()).is_err() {
            heap.fail_scheduled("collector thread is stopped");
        }
    }
}

impl CollectorThread {
    /// Spawn one dedicated collector thread.
    fn spawn(name: impl Into<String>, collector: Arc<SharedCollector>) -> RuntimeResult<Self> {
        let (sender, receiver) = mpsc::channel();
        let join = thread::Builder::new()
            .name(name.into())
            .spawn(move || run_collector(receiver, collector))
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
    fn wake(
        &self,
        heap: Arc<SharedHeap>,
        program: Arc<program::Program>,
    ) -> Result<(), SendError<CollectorMessage>> {
        self.sender.send(CollectorMessage::Wake { heap, program })
    }
}

impl Drop for CollectorThread {
    fn drop(&mut self) {
        let _ = self.sender.send(CollectorMessage::Stop);

        if let Some(join) = self.join.lock().take() {
            let _ = join.join();
        }
    }
}

/// Run the dedicated collector thread.
fn run_collector(receiver: Receiver<CollectorMessage>, collector: Arc<SharedCollector>) {
    while let Ok(message) = receiver.recv() {
        match message {
            CollectorMessage::Wake { heap, program } => {
                heap.run_scheduled(&collector, &program);
            }
            CollectorMessage::Stop => break,
        }
    }
}
