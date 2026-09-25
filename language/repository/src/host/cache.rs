use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};
use tspp_artifact::{ArtifactCache, ArtifactCacheError};

use crate::{ArtifactCacheWrite, Revision};

/// Asynchronous persistent artifact writer shared by one host.
pub struct ArtifactCacheWriter {
    /// Persistent cache receiving completed writes.
    cache: Arc<ArtifactCache>,
    /// Queued writes and writer lifecycle.
    queue: Arc<ArtifactCacheWriteQueue>,
    /// Maximum changed packs encoded concurrently.
    worker_count: usize,
    /// Dedicated native writer thread.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    thread: Option<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for ArtifactCacheWriter {
    /// Format the visible writer state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.queue.state.lock();

        formatter
            .debug_struct("ArtifactCacheWriter")
            .field("cache", &self.cache)
            .field("pending", &state.pending.len())
            .field("active_write", &state.active_write)
            .field("worker_count", &self.worker_count)
            .field("is_closed", &state.is_closed)
            .finish()
    }
}

impl ArtifactCacheWriter {
    /// Start one writer for a shared persistent artifact cache.
    pub fn new(cache: Arc<ArtifactCache>, worker_count: usize) -> Self {
        let queue = Arc::new(ArtifactCacheWriteQueue::default());
        let worker_count = worker_count.max(1);

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        let thread = {
            let cache = cache.clone();
            let queue = queue.clone();
            Some(std::thread::spawn(move || {
                Self::run(cache, queue, worker_count)
            }))
        };

        Self {
            cache,
            queue,
            worker_count,
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            thread,
        }
    }

    /// Return the persistent artifact cache owned by this writer.
    pub fn cache(&self) -> &ArtifactCache {
        self.cache.as_ref()
    }

    /// Enqueue the newest artifact write for one repository.
    pub(crate) fn enqueue(&self, write: ArtifactCacheWrite) -> bool {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        return self.queue.push(write);

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            if !self.queue.push(write) {
                return false;
            }
            let Some(write) = self.queue.next() else {
                return false;
            };
            let result = Self::persist(&self.cache, &write, self.worker_count);
            let persisted_generation = result.as_ref().ok().copied();
            self.queue
                .finish(&write, persisted_generation, result.err());

            true
        }
    }

    /// Wait until every write queued before this call has completed.
    pub fn flush(&self) -> Result<(), ArtifactCacheError> {
        self.queue.flush()
    }

    /// Process coalesced writes until the writer closes.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn run(cache: Arc<ArtifactCache>, queue: Arc<ArtifactCacheWriteQueue>, worker_count: usize) {
        while let Some(write) = queue.next() {
            // persist outside the queue lock
            let result = Self::persist(&cache, &write, worker_count);

            // publish completion to waiters
            let persisted_generation = result.as_ref().ok().copied();
            queue.finish(&write, persisted_generation, result.err());
        }
    }

    /// Persist one artifact selection and perform due cache collection.
    fn persist(
        cache: &ArtifactCache,
        write: &ArtifactCacheWrite,
        worker_count: usize,
    ) -> Result<u64, ArtifactCacheError> {
        let generation = write.write(cache, worker_count)?;
        write.mark_persisted(generation);

        // collect obsolete cache records outside artifact publication
        cache.collect_if_due::<Revision>(write.repository())?;

        Ok(generation)
    }
}

impl Drop for ArtifactCacheWriter {
    /// Drain pending writes and stop the dedicated writer thread.
    fn drop(&mut self) {
        self.queue.close();

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        if let Some(thread) = self.thread.take()
            && let Err(error) = thread.join()
        {
            std::panic::resume_unwind(error);
        }
    }
}

/// Coalesced artifact writes consumed by one writer.
#[derive(Debug, Default)]
struct ArtifactCacheWriteQueue {
    /// Mutable queue state protected from writer and caller threads.
    state: Mutex<ArtifactCacheWriterState>,
    /// Writer wakeup and flush completion signal.
    changed: Condvar,
}

/// Mutable artifact writer state.
#[derive(Debug, Default)]
struct ArtifactCacheWriterState {
    /// Newest pending write for each repository path.
    pending: BTreeMap<PathBuf, ArtifactCacheWrite>,
    /// Repository revision and selection generation currently being written.
    active_write: Option<(PathBuf, Revision, u64)>,
    /// Whether the writer is draining for shutdown.
    is_closed: bool,
    /// First write failure not yet returned by a flush.
    failure: Option<ArtifactCacheError>,
}

impl ArtifactCacheWriteQueue {
    /// Replace the pending write for one repository.
    fn push(&self, write: ArtifactCacheWrite) -> bool {
        let mut state = self.state.lock();
        let repository = write.repository().to_path_buf();
        let generation = (write.revision(), write.generation());
        let is_active = state
            .active_write
            .as_ref()
            .is_some_and(|(path, revision, selection)| {
                path == &repository && (*revision, *selection) == generation
            });
        let is_pending = state.pending.get(&repository).is_some_and(|pending| {
            pending.revision() == write.revision() && pending.generation() == write.generation()
        });
        if is_active || is_pending {
            return false;
        }
        state.pending.insert(repository, write);
        self.changed.notify_one();

        true
    }

    /// Wait for and remove the next queued write.
    fn next(&self) -> Option<ArtifactCacheWrite> {
        let mut state = self.state.lock();

        // wait for work or shutdown
        while state.pending.is_empty() && !state.is_closed {
            self.changed.wait(&mut state);
        }
        let (_, write) = state.pending.pop_first()?;
        state.active_write = Some((
            write.repository().to_path_buf(),
            write.revision(),
            write.generation(),
        ));

        Some(write)
    }

    /// Publish one completed write to flush waiters.
    fn finish(
        &self,
        write: &ArtifactCacheWrite,
        persisted_generation: Option<u64>,
        failure: Option<ArtifactCacheError>,
    ) {
        let mut state = self.state.lock();

        // record this completed write and its first pending failure
        state.active_write = None;
        if state.failure.is_none() {
            state.failure = failure;
        }

        // remove a queued generation already covered by this write
        if let Some(persisted_generation) = persisted_generation
            && state
                .pending
                .get(write.repository())
                .is_some_and(|pending| {
                    pending.revision() == write.revision()
                        && pending.generation() <= persisted_generation
                })
        {
            state.pending.remove(write.repository());
        }

        // wake flush waiters after publishing the completed state
        self.changed.notify_all();
    }

    /// Wait until all queued and active writes finish.
    fn flush(&self) -> Result<(), ArtifactCacheError> {
        let mut state = self.state.lock();

        // wait for both queued and active work
        while !state.pending.is_empty() || state.active_write.is_some() {
            self.changed.wait(&mut state);
        }

        // return the first failure recorded since the preceding flush
        match state.failure.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Close the queue after every pending write.
    fn close(&self) {
        let mut state = self.state.lock();
        state.is_closed = true;
        self.changed.notify_all();
    }
}
