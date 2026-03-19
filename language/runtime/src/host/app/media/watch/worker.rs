use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::host::app::media::catalog::{media_summary_for_watch_path, snapshot_media_assets};
use crate::host::app::media::roots::DesktopMediaRoots;
use crate::host::core::{HostEvent, HostMediaEvent, HostMediaEventKind, HostQueue};
use crate::platform::core::{io_operation_error, pathbuf_from_file_uri};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{MediaAssetSummaryValue, MediaWatchOptionsValue};

/// Interval used while idle media watcher threads wait for stop signals.
const MEDIA_WATCH_IDLE_SLICE: Duration = Duration::from_millis(250);

/// One initialized media watch worker state.
pub(super) struct InitializedMediaWatchWorker {
    /// Native filesystem watcher.
    watcher: RecommendedWatcher,
    /// Native watcher event receiver.
    event_recv: Receiver<Result<Event, notify::Error>>,
}

/// Shared desktop media runtime state.
#[derive(Default)]
pub(super) struct DesktopMediaRuntimeState {
    /// Active filtered watches keyed by watch id.
    pub(super) watches: FxHashMap<String, DesktopMediaWatchState>,
}

/// One active media watch state within one desktop runtime.
pub(super) struct DesktopMediaWatchState {
    /// Filter options for this watch.
    options: MediaWatchOptionsValue,
    /// Current visible assets keyed by stable identifier.
    assets: HashMap<String, MediaAssetSummaryValue>,
    /// Reverse normalized path index for remove events.
    paths: HashMap<PathBuf, String>,
}

/// Initialize one media watch state for one filtered watch.
pub(super) fn initialize_media_watch_state(
    platform: Platform,
    options: &MediaWatchOptionsValue,
) -> RuntimeResult<DesktopMediaWatchState> {
    let assets = snapshot_media_assets(platform, &options.kinds, options.include_hidden)?;
    let mut paths = HashMap::with_capacity(assets.len());

    // seed one reverse path index for remove events
    for asset in assets.values() {
        if let Ok(path) = pathbuf_from_file_uri(asset.id.as_str(), "id") {
            paths.insert(normalized_watch_path(&path), asset.id.clone());
        }
    }

    Ok(DesktopMediaWatchState {
        options: options.clone(),
        assets,
        paths,
    })
}

/// Initialize one media watch worker before the thread enters its event loop.
pub(super) fn initialize_media_watch_worker(
    roots: DesktopMediaRoots,
) -> RuntimeResult<InitializedMediaWatchWorker> {
    let (event_send, event_recv) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            if event_send.send(event).is_err() {
                return;
            }
        },
        Config::default(),
    )
    .map_err(|error| {
        io_operation_error(
            "destack.os.media.watchOpen",
            Some(PlatformErrorCode::IoInvalidData),
            format!("media watcher initialization failed: {error}"),
        )
    })?;

    let roots = [roots.pictures, roots.videos, roots.music];

    // create missing roots so native watchers can bind to the library directories directly
    for root in &roots {
        std::fs::create_dir_all(root).map_err(|error| {
            io_operation_error(
                "destack.os.media.watchOpen",
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "media root creation failed for `{}`: {error}",
                    root.display()
                ),
            )
        })?;
    }

    // watch each root recursively because the desktop media backend is directory based
    for root in &roots {
        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|error| {
                io_operation_error(
                    "destack.os.media.watchOpen",
                    Some(PlatformErrorCode::IoInvalidData),
                    format!(
                        "media watcher registration failed for `{}`: {error}",
                        root.display()
                    ),
                )
            })?;
    }

    Ok(InitializedMediaWatchWorker {
        watcher,
        event_recv,
    })
}

/// Run one media watch worker and keep publishing host events until closed.
pub(super) fn media_watch_worker(
    mut worker: InitializedMediaWatchWorker,
    platform: Platform,
    queue: Arc<HostQueue>,
    state: Arc<Mutex<DesktopMediaRuntimeState>>,
    stop: Arc<AtomicBool>,
) -> RuntimeResult<()> {
    let _watcher = &mut worker.watcher;

    loop {
        // stop once the owning runtime has been closed
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }

        match worker.event_recv.recv_timeout(MEDIA_WATCH_IDLE_SLICE) {
            Ok(Ok(event)) => {
                process_media_watch_event(&queue, platform, &state, event)?;
            }
            Ok(Err(error)) => {
                return Err(io_operation_error(
                    "destack.os.media.watchRead",
                    Some(PlatformErrorCode::IoInvalidData),
                    format!("media watcher failed: {error}"),
                ));
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io_operation_error(
                    "destack.os.media.watchRead",
                    Some(PlatformErrorCode::IoInvalidData),
                    "media watcher channel disconnected",
                ));
            }
        }
    }
}

/// Process one native filesystem watch event.
fn process_media_watch_event(
    queue: &HostQueue,
    platform: Platform,
    state: &Arc<Mutex<DesktopMediaRuntimeState>>,
    event: Event,
) -> RuntimeResult<()> {
    let mut events = Vec::new();
    let mut state = state.lock();

    for path in event.paths {
        let normalized_path = normalized_watch_path(&path);

        // update every filtered watch against the same filesystem change
        for (watch_id, watch) in &mut state.watches {
            process_media_watch_path(
                platform,
                watch_id.as_str(),
                watch,
                &path,
                &normalized_path,
                &mut events,
            )?;
        }
    }

    drop(state);

    for (watch_id, kind, asset) in events {
        publish_media_event(queue, watch_id, kind, asset);
    }

    Ok(())
}

/// Process one filesystem path change for one filtered watch.
fn process_media_watch_path(
    platform: Platform,
    watch_id: &str,
    watch: &mut DesktopMediaWatchState,
    path: &Path,
    normalized_path: &Path,
    events: &mut Vec<(String, HostMediaEventKind, MediaAssetSummaryValue)>,
) -> RuntimeResult<()> {
    let summary = media_summary_for_event_path(
        platform,
        path,
        &watch.options.kinds,
        watch.options.include_hidden,
    )?;

    // filesystem removal or a file leaving the visible filter acts like removal
    if summary.is_none() {
        let Some(asset_id) = watch.paths.remove(normalized_path) else {
            return Ok(());
        };
        let Some(asset) = watch.assets.remove(&asset_id) else {
            return Ok(());
        };

        if watch.options.include_removed {
            events.push((watch_id.to_string(), HostMediaEventKind::Removed, asset));
        }

        return Ok(());
    }

    let Some(summary) = summary else {
        return Ok(());
    };
    let previous = watch.assets.insert(summary.id.clone(), summary.clone());
    watch
        .paths
        .insert(normalized_path.to_path_buf(), summary.id.clone());

    // a new summary means one added asset
    if previous.is_none() {
        if watch.options.include_added {
            events.push((watch_id.to_string(), HostMediaEventKind::Added, summary));
        }

        return Ok(());
    }

    // unchanged summaries do not emit updates
    if previous.as_ref() == Some(&summary) {
        return Ok(());
    }

    // otherwise this is one asset update
    if watch.options.include_updated {
        events.push((watch_id.to_string(), HostMediaEventKind::Updated, summary));
    }

    Ok(())
}

/// Resolve one media watch event path into one current asset summary.
fn media_summary_for_event_path(
    platform: Platform,
    path: &Path,
    kinds: &[crate::platform::os::abi_generated::MediaAssetKind],
    include_hidden: bool,
) -> RuntimeResult<Option<MediaAssetSummaryValue>> {
    match media_summary_for_watch_path(platform, path, kinds, include_hidden) {
        Ok(summary) => Ok(summary),
        Err(error) => {
            let platform_error = error.platform_error();
            let is_missing_path = platform_error
                .as_ref()
                .is_some_and(|platform_error| platform_error.code == PlatformErrorCode::IoNotFound);

            if is_missing_path {
                return Ok(None);
            }

            Err(error)
        }
    }
}

/// Publish one media host event into the runtime queue.
fn publish_media_event(
    queue: &HostQueue,
    watch_id: String,
    kind: HostMediaEventKind,
    asset: MediaAssetSummaryValue,
) {
    queue.enqueue(HostEvent::Media(Box::new(HostMediaEvent {
        watch_id,
        kind,
        asset,
    })));
}

/// Normalize one watch path for stable remove-event matching.
fn normalized_watch_path(path: &Path) -> PathBuf {
    path.components().collect::<PathBuf>()
}
