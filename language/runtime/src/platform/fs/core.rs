#[cfg(any(unix, windows))]
use std::collections::VecDeque;
#[cfg(any(unix, windows))]
use std::path::{Path, PathBuf};
#[cfg(any(unix, windows))]
use std::sync::Arc;

#[cfg(any(unix, windows))]
use notify::event::{ModifyKind, RenameMode};
#[cfg(any(unix, windows))]
use notify::{Event, EventKind, RecursiveMode, Watcher};
#[cfg(any(unix, windows))]
use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{
    OsPath, PathBytes, PathBytesAbi, PathEncoding, PathUtf16, PathUtf16Abi, WatchBatch, WatchEvent,
    WatchEventKind, WatchMask, WatchOptions,
};
use crate::platform::resource::{ResourceEntry, ResourceKind, WatchHandle};
use crate::platform::{NativeArray, PlatformError, ResourceId};
use crate::runtime::BindingCallContext;

/// Resolve a file or directory handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    context: &BindingCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(id, |entry| {
            if entry.kind != kind {
                return None;
            }
            Some(with_entry(entry))
        })
        .flatten();

    match resolved {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(error),
        None => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            format!("unknown {label} handle"),
        ))
        .boxed()),
    }
}

/// Build an empty byte path payload.
pub(crate) fn empty_path_bytes() -> PathBytes {
    PathBytesAbi::<NativeAbi>(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build an empty UTF-16 path payload.
pub(crate) fn empty_path_utf16() -> PathUtf16 {
    PathUtf16Abi::<NativeAbi>(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build an `OsPath` from raw byte data.
pub(crate) fn path_ref_from_bytes(bytes: PathBytes) -> OsPath {
    OsPath {
        encoding: PathEncoding::Bytes,
        bytes,
        utf16: empty_path_utf16(),
    }
}

/// Build an `OsPath` from UTF-16 data.
pub(crate) fn path_ref_from_utf16(utf16: PathUtf16) -> OsPath {
    OsPath {
        encoding: PathEncoding::Utf16,
        bytes: empty_path_bytes(),
        utf16,
    }
}

/// Decode an `OsPath` into a UTF-8 string.
pub(crate) fn os_path_to_utf8_string(path: OsPath, label: &str) -> RuntimeResult<String> {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            String::from_utf8(bytes.to_vec()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path bytes are not valid utf8",
                ))
                .boxed()
            })
        }
        PathEncoding::Utf16 => {
            let utf16 = unsafe { path.utf16.0.as_slice()? };
            String::from_utf16(utf16).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path utf16 is not valid",
                ))
                .boxed()
            })
        }
    }
}

/// Encode a UTF-8 path string into an `OsPath`.
pub(crate) fn os_path_from_utf8_string(context: &BindingCallContext, value: String) -> OsPath {
    #[cfg(unix)]
    {
        let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.into_bytes()));
        path_ref_from_bytes(bytes)
    }

    #[cfg(windows)]
    {
        let utf16_values = value.encode_utf16().collect::<Vec<_>>();
        let utf16 = PathUtf16Abi::<NativeAbi>(context.store_array(utf16_values));
        path_ref_from_utf16(utf16)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.into_bytes()));
        path_ref_from_bytes(bytes)
    }
}

/// Convert a UTF-16 path into a UTF-8 byte vector.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn utf16_path_to_utf8_bytes(path: PathUtf16, label: &str) -> RuntimeResult<Vec<u8>> {
    let units = unsafe { path.0.as_slice()? };
    let decoded = String::from_utf16(units).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf16",
        ))
        .boxed()
    })?;

    let bytes = decoded.into_bytes();
    if bytes.len() > u32::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path is too large",
        ))
        .boxed());
    }

    Ok(bytes)
}

/// Return a mismatch error for a pair of paths.
pub(crate) fn path_ref_mismatch<T>(label: &str) -> RuntimeResult<T> {
    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        label,
        "mismatched path encoding",
    ))
    .boxed())
}

/// Convert one UTF-16 path into a byte path for Unix handlers.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn with_utf16_as_bytes<T>(
    path: PathUtf16,
    label: &str,
    on_bytes: impl FnOnce(PathBytes) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut bytes = utf16_path_to_utf8_bytes(path, label)?;

    let path_bytes = PathBytesAbi::<NativeAbi>(NativeArray {
        data: bytes.as_mut_ptr(),
        len: bytes.len() as u32,
        capacity: bytes.len() as u32,
    });
    let result = on_bytes(path_bytes);
    drop(bytes);

    result
}

/// Convert a pair of UTF-16 paths into byte paths for Unix handlers.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn with_utf16_pair_as_bytes<T>(
    left: PathUtf16,
    right: PathUtf16,
    label: &str,
    on_bytes: impl FnOnce(PathBytes, PathBytes) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut left_bytes = utf16_path_to_utf8_bytes(left, label)?;
    let mut right_bytes = utf16_path_to_utf8_bytes(right, label)?;

    let left_path = PathBytesAbi::<NativeAbi>(NativeArray {
        data: left_bytes.as_mut_ptr(),
        len: left_bytes.len() as u32,
        capacity: left_bytes.len() as u32,
    });
    let right_path = PathBytesAbi::<NativeAbi>(NativeArray {
        data: right_bytes.as_mut_ptr(),
        len: right_bytes.len() as u32,
        capacity: right_bytes.len() as u32,
    });

    let result = on_bytes(left_path, right_path);
    drop((left_bytes, right_bytes));

    result
}

/// Convert one byte path into UTF-16 data for UTF-16 handlers.
#[allow(dead_code)]
pub(crate) fn path_utf16_from_bytes(
    context: &BindingCallContext,
    path: PathBytes,
    label: &str,
) -> RuntimeResult<PathUtf16> {
    let bytes = unsafe { path.0.as_slice()? };
    let text = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;
    let utf16: Vec<u16> = text.encode_utf16().collect();

    Ok(PathUtf16Abi::<NativeAbi>(context.store_array(utf16)))
}

/// Dispatch one OsPath through byte or UTF-16 handlers.
pub(crate) fn with_path_ref<T>(
    path: OsPath,
    label: &str,
    on_bytes: impl FnOnce(PathBytes) -> RuntimeResult<T>,
    _on_utf16: impl FnOnce(PathUtf16) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    #[cfg(not(unix))]
    let _ = label;

    match path.encoding {
        PathEncoding::Bytes => on_bytes(path.bytes),
        PathEncoding::Utf16 => {
            #[cfg(unix)]
            {
                with_utf16_as_bytes(path.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(path.utf16)
            }
        }
    }
}

/// Dispatch two OsPath values through byte or UTF-16 handlers.
pub(crate) fn with_path_ref_pair<T>(
    left: OsPath,
    right: OsPath,
    label: &str,
    on_bytes: impl FnOnce(PathBytes, PathBytes) -> RuntimeResult<T>,
    _on_utf16: impl FnOnce(PathUtf16, PathUtf16) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    match (left.encoding, right.encoding) {
        (PathEncoding::Bytes, PathEncoding::Bytes) => on_bytes(left.bytes, right.bytes),
        (PathEncoding::Utf16, PathEncoding::Utf16) => {
            #[cfg(unix)]
            {
                with_utf16_pair_as_bytes(left.utf16, right.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(left.utf16, right.utf16)
            }
        }
        _ => path_ref_mismatch(label),
    }
}

/// Build an `OsPath` from a host path.
#[cfg(any(unix, windows))]
pub(crate) fn os_path_from_path(context: &BindingCallContext, path: &Path) -> OsPath {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes = path.as_os_str().as_bytes().to_vec();
        let bytes = PathBytesAbi::<NativeAbi>(context.store_array(bytes));
        path_ref_from_bytes(bytes)
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let utf16 = path.as_os_str().encode_wide().collect::<Vec<_>>();
        let utf16 = PathUtf16Abi::<NativeAbi>(context.store_array(utf16));
        path_ref_from_utf16(utf16)
    }
}

/// Return an empty `OsPath` payload.
#[cfg(any(unix, windows))]
pub(crate) fn empty_os_path() -> OsPath {
    #[cfg(unix)]
    {
        path_ref_from_bytes(empty_path_bytes())
    }

    #[cfg(windows)]
    {
        path_ref_from_utf16(empty_path_utf16())
    }
}

/// Create one invalid-watch-handle error.
#[cfg(any(unix, windows))]
fn invalid_watch_handle_error() -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(
        "handle",
        "unknown watch handle",
    ))
    .boxed()
}

/// Create one watcher backend error.
#[cfg(any(unix, windows))]
fn watch_backend_error(operation: &str, error: notify::Error) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io(format!("{operation} failed: {error}"))).boxed()
}

/// Watch-mask bit for create events.
#[cfg(any(unix, windows))]
const WATCH_MASK_CREATE: u32 = 1 << 0;
/// Watch-mask bit for remove events.
#[cfg(any(unix, windows))]
const WATCH_MASK_REMOVE: u32 = 1 << 1;
/// Watch-mask bit for modify events.
#[cfg(any(unix, windows))]
const WATCH_MASK_MODIFY: u32 = 1 << 2;
/// Watch-mask bit for rename events.
#[cfg(any(unix, windows))]
const WATCH_MASK_RENAME: u32 = 1 << 3;
/// Watch-mask bit for metadata events.
#[cfg(any(unix, windows))]
const WATCH_MASK_METADATA: u32 = 1 << 4;
/// Watch-mask bit for overflow events.
#[cfg(any(unix, windows))]
const WATCH_MASK_OVERFLOW: u32 = 1 << 5;

/// One queued watch event record.
#[cfg(any(unix, windows))]
#[derive(Debug, Clone)]
struct WatchQueuedEvent {
    /// Event kind for this record.
    kind: WatchEventKind,
    /// Primary path payload when present.
    path: Option<PathBuf>,
    /// Related path payload when present.
    related_path: Option<PathBuf>,
    /// Backend cookie value when present.
    cookie: u64,
}

/// Mutable watch-resource payload stored in the resource table.
#[cfg(any(unix, windows))]
#[derive(Debug)]
struct WatchResource {
    /// Native watcher backend handle.
    watcher: notify::RecommendedWatcher,
    /// Event receiver for backend callbacks.
    receiver: std::sync::mpsc::Receiver<notify::Result<Event>>,
    /// Pending queued events not yet consumed by `watchRead`.
    queued_events: VecDeque<WatchQueuedEvent>,
    /// Overflow marker for dropped or invalidated backend state.
    overflowed: bool,
    /// Requested event-mask filter.
    mask: WatchMask,
}

/// Resolve one watch resource payload from the runtime table.
#[cfg(any(unix, windows))]
fn watch_resource(
    context: &BindingCallContext,
    handle: WatchHandle,
) -> RuntimeResult<Arc<Mutex<WatchResource>>> {
    require_resource(context, handle.0, ResourceKind::Watch, "watch", |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<WatchResource>>>())
            .cloned()
            .ok_or_else(invalid_watch_handle_error)
    })
}

/// Return true when one watch-kind passes one mask.
#[cfg(any(unix, windows))]
fn watch_mask_allows(mask: WatchMask, kind: WatchEventKind) -> bool {
    if mask.0 == 0 {
        return true;
    }

    let bit = match kind {
        WatchEventKind::Create => WATCH_MASK_CREATE,
        WatchEventKind::Remove => WATCH_MASK_REMOVE,
        WatchEventKind::Modify => WATCH_MASK_MODIFY,
        WatchEventKind::Rename => WATCH_MASK_RENAME,
        WatchEventKind::Metadata => WATCH_MASK_METADATA,
        WatchEventKind::Overflow => WATCH_MASK_OVERFLOW,
    };
    mask.0 & bit != 0
}

/// Enqueue one watch record when it passes the mask filter.
#[cfg(any(unix, windows))]
fn push_watch_record(
    resource: &mut WatchResource,
    kind: WatchEventKind,
    path: Option<PathBuf>,
    related_path: Option<PathBuf>,
    cookie: u64,
) {
    if !watch_mask_allows(resource.mask, kind) {
        return;
    }

    resource.queued_events.push_back(WatchQueuedEvent {
        kind,
        path,
        related_path,
        cookie,
    });
}

/// Enqueue one overflow marker.
#[cfg(any(unix, windows))]
fn push_overflow_record(resource: &mut WatchResource, cookie: u64) {
    resource.overflowed = true;
    push_watch_record(resource, WatchEventKind::Overflow, None, None, cookie);
}

/// Map one notify event into queued watch records.
#[cfg(any(unix, windows))]
fn push_notify_event(resource: &mut WatchResource, event: Event) {
    let cookie = event.tracker().unwrap_or(0) as u64;
    if event.need_rescan() {
        push_overflow_record(resource, cookie);
    }

    match event.kind {
        EventKind::Create(_) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchEventKind::Create, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Create, Some(path), None, cookie);
            }
        }
        EventKind::Remove(_) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchEventKind::Remove, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Remove, Some(path), None, cookie);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            if event.paths.len() >= 2 {
                let source = event.paths[0].clone();
                let target = event.paths[1].clone();
                push_watch_record(
                    resource,
                    WatchEventKind::Rename,
                    Some(source),
                    Some(target),
                    cookie,
                );
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Rename, Some(path), None, cookie);
            }
        }
        EventKind::Modify(ModifyKind::Name(_)) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchEventKind::Rename, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Rename, Some(path), None, cookie);
            }
        }
        EventKind::Modify(ModifyKind::Metadata(_)) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchEventKind::Metadata, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Metadata, Some(path), None, cookie);
            }
        }
        EventKind::Modify(_) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchEventKind::Modify, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Modify, Some(path), None, cookie);
            }
        }
        EventKind::Access(_) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchEventKind::Metadata, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Metadata, Some(path), None, cookie);
            }
        }
        EventKind::Any | EventKind::Other => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchEventKind::Metadata, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchEventKind::Metadata, Some(path), None, cookie);
            }
        }
    }
}

/// Drain queued backend events into the watch resource queue.
#[cfg(any(unix, windows))]
fn drain_watch_backend_events(resource: &mut WatchResource) {
    loop {
        let received = resource.receiver.try_recv();
        match received {
            Ok(Ok(event)) => {
                push_notify_event(resource, event);
            }
            Ok(Err(_error)) => {
                push_overflow_record(resource, 0);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                push_overflow_record(resource, 0);
                break;
            }
        }
    }
}

/// Build one native watch event from one queued record.
#[cfg(any(unix, windows))]
fn watch_event_from_queued(context: &BindingCallContext, record: WatchQueuedEvent) -> WatchEvent {
    let path = if let Some(path) = record.path.as_deref() {
        os_path_from_path(context, path)
    } else {
        empty_os_path()
    };
    let related_path = if let Some(related_path) = record.related_path.as_deref() {
        os_path_from_path(context, related_path)
    } else {
        empty_os_path()
    };

    WatchEvent {
        kind: record.kind,
        path,
        related_path,
        cookie: record.cookie,
    }
}

/// Open one filesystem watch handle.
#[cfg(any(unix, windows))]
pub(crate) fn open_watch(
    context: &BindingCallContext,
    path: &Path,
    options: WatchOptions,
) -> RuntimeResult<WatchHandle> {
    // create one callback channel for backend events
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |result| {
        let _ = sender.send(result);
    })
    .map_err(|error| watch_backend_error("watcher", error))?;

    // apply follow-symlink behavior when the backend supports it
    let config = notify::Config::default().with_follow_symlinks(options.follow_symlinks);
    watcher
        .configure(config)
        .map_err(|error| watch_backend_error("watcher.configure", error))?;

    // register the watch path
    let recursive_mode = if options.recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    watcher
        .watch(path, recursive_mode)
        .map_err(|error| watch_backend_error("watch", error))?;

    // register one watch resource in the runtime table
    let resource = WatchResource {
        watcher,
        receiver,
        queued_events: VecDeque::new(),
        overflowed: false,
        mask: options.mask,
    };
    let entry =
        ResourceEntry::new(ResourceKind::Watch).with_payload(Arc::new(Mutex::new(resource)));
    let resource_id = context.runtime().resources.insert(entry);

    Ok(WatchHandle(resource_id))
}

/// Close one filesystem watch handle.
#[cfg(any(unix, windows))]
pub(crate) fn close_watch(context: &BindingCallContext, handle: WatchHandle) -> RuntimeResult<()> {
    // ensure the handle still points to one watch resource
    let _resource = watch_resource(context, handle)?;

    // remove the watch resource and drop the backend watcher
    let removed = context.runtime().resources.remove_and_finalize(handle.0);
    if !removed {
        return Err(invalid_watch_handle_error());
    }

    Ok(())
}

/// Read one filesystem watch batch from a watch handle.
#[cfg(any(unix, windows))]
pub(crate) fn read_watch(
    context: &BindingCallContext,
    handle: WatchHandle,
) -> RuntimeResult<WatchBatch> {
    // resolve the watch resource and drain pending backend events
    let resource = watch_resource(context, handle)?;
    let mut resource = resource.lock();
    let _watcher = &resource.watcher;
    drain_watch_backend_events(&mut resource);

    // move queued events out and reset overflow state
    let queued_events = resource.queued_events.drain(..).collect::<Vec<_>>();
    let overflowed = resource.overflowed;
    resource.overflowed = false;
    drop(resource);

    // encode queued events into ABI payloads
    let mut events = Vec::with_capacity(queued_events.len());
    for queued_event in queued_events {
        let event = watch_event_from_queued(context, queued_event);
        events.push(event);
    }

    Ok(WatchBatch {
        events: context.store_array(events),
        overflowed,
    })
}
