use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::diagnostic::RuntimeResult;
use crate::host::app::media::roots::desktop_media_roots;
use crate::host::core::{HostRequestContext, HostRuntimeId, HostRuntimeRegistry};
use crate::platform::core::invalid_argument;
use crate::platform::os::abi_generated::MediaWatchOptionsValue;
use crate::runtime::process::{ExecutionMode, ExecutionPolicy, GlobalService, WorkerLoop};

use super::worker::{
    DesktopMediaRuntimeState, initialize_media_watch_state, initialize_media_watch_worker,
    media_watch_worker,
};

/// One active desktop media runtime.
struct DesktopMediaRuntime {
    /// Shared runtime watch state.
    state: Arc<Mutex<DesktopMediaRuntimeState>>,
    /// Worker loop that owns the native watcher.
    _worker: WorkerLoop,
}

/// Process-global desktop media watch service.
struct DesktopMediaWatchService {
    /// Active runtimes keyed by host runtime id.
    runtimes: Mutex<FxHashMap<HostRuntimeId, DesktopMediaRuntime>>,
}

impl DesktopMediaWatchService {
    /// Create one empty desktop media watch service.
    fn new() -> Self {
        Self {
            runtimes: Mutex::new(FxHashMap::default()),
        }
    }

    /// Open one real desktop media watch for one runtime.
    fn open_watch(
        &self,
        context: &HostRequestContext,
        watch_id: &str,
        options: &MediaWatchOptionsValue,
    ) -> RuntimeResult<()> {
        let mut runtimes = self.runtimes.lock();

        // attach one new filtered watch to the active runtime when it already exists
        if let Some(runtime) = runtimes.get_mut(&context.host_runtime_id) {
            let mut state = runtime.state.lock();

            if state.watches.contains_key(watch_id) {
                return Err(invalid_argument(
                    "handle",
                    format!("media watch `{watch_id}` is already open"),
                ));
            }

            let watch = initialize_media_watch_state(context.platform, options)?;
            state.watches.insert(watch_id.to_string(), watch);

            return Ok(());
        }

        let roots = desktop_media_roots(context.platform)?;
        let queue =
            HostRuntimeRegistry::queue_for_runtime(context.host_runtime_id, context.platform)?;
        let stop = Arc::new(AtomicBool::new(false));
        let runtime_state = Arc::new(Mutex::new(DesktopMediaRuntimeState::default()));

        {
            let mut state = runtime_state.lock();
            let watch = initialize_media_watch_state(context.platform, options)?;
            state.watches.insert(watch_id.to_string(), watch);
        }

        let worker_state = Arc::clone(&runtime_state);
        let worker_stop = Arc::clone(&stop);
        let worker_platform = context.platform;
        let worker = WorkerLoop::open(
            &format!("destack-media-runtime-{}", context.host_runtime_id.0),
            "destack.os.media.watchOpen",
            ExecutionPolicy::global(ExecutionMode::Loop),
            move || {
                let worker = initialize_media_watch_worker(roots)?;

                Ok((
                    Box::new(move || {
                        worker_stop.store(true, Ordering::Relaxed);
                    }),
                    Box::new(move || {
                        media_watch_worker(worker, worker_platform, queue, worker_state, stop)
                    }),
                ))
            },
        )?;

        runtimes.insert(
            context.host_runtime_id,
            DesktopMediaRuntime {
                state: runtime_state,
                _worker: worker,
            },
        );

        Ok(())
    }

    /// Close one active desktop media watch for one runtime.
    fn close_watch(&self, host_runtime_id: HostRuntimeId, watch_id: &str) -> RuntimeResult<()> {
        let runtime = {
            let mut runtimes = self.runtimes.lock();
            let Some(runtime) = runtimes.get_mut(&host_runtime_id) else {
                return Err(invalid_argument(
                    "handle",
                    "unknown media watch stream handle",
                ));
            };

            let mut state = runtime.state.lock();
            let removed = state.watches.remove(watch_id);

            if removed.is_none() {
                return Err(invalid_argument(
                    "handle",
                    "unknown media watch stream handle",
                ));
            }

            if !state.watches.is_empty() {
                return Ok(());
            }

            drop(state);
            runtimes.remove(&host_runtime_id)
        };

        drop(runtime);

        Ok(())
    }

    /// Remove every active media watch for one runtime.
    fn unregister_runtime(&self, host_runtime_id: HostRuntimeId) {
        let runtime = {
            let mut runtimes = self.runtimes.lock();
            runtimes.remove(&host_runtime_id)
        };

        drop(runtime);
    }
}

impl GlobalService for DesktopMediaWatchService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::global(ExecutionMode::Inline);
}

/// Open one real desktop media watch for one runtime.
pub(crate) fn open_media_watch(
    context: &HostRequestContext,
    watch_id: &str,
    options: &MediaWatchOptionsValue,
) -> RuntimeResult<()> {
    let service = media_watch_service()?;

    service.open_watch(context, watch_id, options)
}

/// Close one active desktop media watch for one runtime.
pub(crate) fn close_media_watch(
    host_runtime_id: HostRuntimeId,
    watch_id: &str,
) -> RuntimeResult<()> {
    let service = media_watch_service()?;

    service.close_watch(host_runtime_id, watch_id)
}

/// Remove every active media watch for one runtime.
pub(crate) fn unregister_media_runtime(host_runtime_id: HostRuntimeId) {
    if let Some(service) = DesktopMediaWatchService::active() {
        service.unregister_runtime(host_runtime_id);
    }
}

/// Return the shared desktop media watch service.
fn media_watch_service() -> RuntimeResult<Arc<DesktopMediaWatchService>> {
    DesktopMediaWatchService::global(|| Ok(DesktopMediaWatchService::new()))
}
