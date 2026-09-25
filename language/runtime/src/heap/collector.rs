use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use std::thread::{self, JoinHandle};

use parking_lot::Mutex;
use tspp_heap as heap;
use tspp_program as program;
use tspp_repository::ExecutionMode;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::heap::SharedCollectionState;

/// World-owned shared heap collector.
#[derive(Debug)]
pub struct WorldCollector {
    /// Shared heap collector mode.
    mode: WorldCollectorMode,
    /// Dedicated collector thread for concurrent mode.
    thread: Mutex<Option<CollectorThread>>,
}

/// World collector execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldCollectorMode {
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

/// World collector thread message.
#[derive(Debug)]
enum CollectorMessage {
    /// Run one bounded shared heap collection step.
    Wake {
        /// Collection state to advance.
        collection: Arc<SharedCollectionState>,
        /// Shared heap to advance.
        heap: Arc<heap::SharedHeap>,
        /// Program owning the trace table.
        program: Arc<program::Program>,
    },
    /// Shut down the collector thread.
    Stop,
}

impl WorldCollectorMode {
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

impl WorldCollector {
    /// Create one world collector.
    pub(crate) fn new(
        mode: WorldCollectorMode,
        name: impl Into<String>,
    ) -> RuntimeResult<Arc<Self>> {
        let collector = Arc::new(Self {
            mode,
            thread: Mutex::new(None),
        });

        if mode.is_concurrent() {
            let thread = CollectorThread::spawn(name)?;
            *collector.thread.lock() = Some(thread);
        }

        Ok(collector)
    }

    /// Return this collector's scheduling mode.
    pub const fn mode(&self) -> WorldCollectorMode {
        self.mode
    }

    /// Wake concurrent collection for one shared heap.
    pub(crate) fn wake(
        &self,
        collection: &Arc<SharedCollectionState>,
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
    ) {
        if !collection.schedule() {
            return;
        }

        let thread = self.thread.lock();
        let Some(thread) = thread.as_ref() else {
            collection.fail_scheduled("collector thread is missing");

            return;
        };

        if thread
            .wake(collection.clone(), heap.clone(), program.clone())
            .is_err()
        {
            collection.fail_scheduled("collector thread is stopped");
        }
    }
}

impl CollectorThread {
    /// Spawn one dedicated collector thread.
    fn spawn(name: impl Into<String>) -> RuntimeResult<Self> {
        let (sender, receiver) = mpsc::channel();
        let join = thread::Builder::new()
            .name(name.into())
            .spawn(move || run_collector(receiver))
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
        collection: Arc<SharedCollectionState>,
        heap: Arc<heap::SharedHeap>,
        program: Arc<program::Program>,
    ) -> Result<(), SendError<CollectorMessage>> {
        self.sender.send(CollectorMessage::Wake {
            collection,
            heap,
            program,
        })
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
fn run_collector(receiver: Receiver<CollectorMessage>) {
    while let Ok(message) = receiver.recv() {
        match message {
            CollectorMessage::Wake {
                collection,
                heap,
                program,
            } => {
                collection.run_scheduled(&heap, &program);
            }
            CollectorMessage::Stop => break,
        }
    }
}
