#![cfg_attr(not(any(unix, windows)), allow(unused_imports))]

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
    FileLockFlags, FileSize, MmapFlags, OsPath, OsPathBytes, OsPathUtf16, PathBytes, PathBytesAbi,
    PathUtf16, PathUtf16Abi, WatchBatch, WatchCreateEvent, WatchEvent, WatchEventMetadata,
    WatchMask, WatchMetadataEvent, WatchModifyEvent, WatchOptions, WatchOverflowEvent,
    WatchRemoveEvent, WatchRenameEvent, XattrFlags,
};
use crate::platform::resource::{ResourceEntry, ResourceKind, WatchHandle};
use crate::platform::{NativeArray, PlatformError, ResourceId};
use crate::runtime::BindingCallContext;

/// Maximum staging buffer used by fallback copy loops.
const COPY_FALLBACK_BUFFER_CAPACITY: usize = 256 * 1024;
/// Shared lock flag value.
const FILE_LOCK_SHARED: u32 = 0x1;
/// Exclusive lock flag value.
const FILE_LOCK_EXCLUSIVE: u32 = 0x2;
/// Non-blocking lock flag value.
const FILE_LOCK_NONBLOCK: u32 = 0x4;
/// Unlock flag value.
const FILE_LOCK_UNLOCK: u32 = 0x8;
/// Bitmask of all supported file-lock flag bits.
const FILE_LOCK_SUPPORTED_FLAGS: u32 =
    FILE_LOCK_SHARED | FILE_LOCK_EXCLUSIVE | FILE_LOCK_NONBLOCK | FILE_LOCK_UNLOCK;
/// Shared mmap flag value.
const MMAP_FLAG_SHARED: u32 = 0x1;
/// Private mmap flag value.
const MMAP_FLAG_PRIVATE: u32 = 0x2;
/// Fixed-address mmap flag value.
const MMAP_FLAG_FIXED: u32 = 0x10;
/// Anonymous mmap flag value.
const MMAP_FLAG_ANONYMOUS: u32 = 0x20;
/// Bitmask of all supported mmap flag bits.
const MMAP_SUPPORTED_FLAGS: u32 =
    MMAP_FLAG_SHARED | MMAP_FLAG_PRIVATE | MMAP_FLAG_FIXED | MMAP_FLAG_ANONYMOUS;
/// Create one extended attribute, failing if it already exists.
const XATTR_FLAG_CREATE: u32 = 0x1;
/// Replace one extended attribute, failing if it does not exist.
const XATTR_FLAG_REPLACE: u32 = 0x2;
/// Bitmask of all supported xattr flag bits.
const XATTR_SUPPORTED_FLAGS: u32 = XATTR_FLAG_CREATE | XATTR_FLAG_REPLACE;

/// Parsed file-lock operation.
#[derive(Clone, Copy)]
pub(crate) enum FileLockOperation {
    /// Acquire one shared lock.
    Shared {
        /// Whether the operation should fail instead of blocking.
        nonblocking: bool,
    },
    /// Acquire one exclusive lock.
    Exclusive {
        /// Whether the operation should fail instead of blocking.
        nonblocking: bool,
    },
    /// Release an existing lock.
    Unlock,
}

/// Parsed mmap flag payload.
#[derive(Clone, Copy)]
pub(crate) struct DecodedMmapFlags {
    /// Whether the mapping is shared.
    pub(crate) is_shared: bool,
    /// Whether the mapping is fixed-address.
    pub(crate) is_fixed: bool,
}

/// Resolve a file or directory handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    binding: &BindingCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = binding
        .worker()
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

/// Choose one bounded staging buffer length for fallback copy loops.
pub(crate) fn copy_fallback_buffer_length(length: u64) -> usize {
    let requested = usize::try_from(length).unwrap_or(COPY_FALLBACK_BUFFER_CAPACITY);
    requested.clamp(1, COPY_FALLBACK_BUFFER_CAPACITY)
}

/// Validate and decode one file-lock flag payload.
pub(crate) fn decode_file_lock_flags(flags: FileLockFlags) -> RuntimeResult<FileLockOperation> {
    // reject unknown flag bits
    let unsupported_flags = flags.0 & !FILE_LOCK_SUPPORTED_FLAGS;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported file lock flags: {unsupported_flags:#x}"),
        ))
        .boxed());
    }

    // require unlock to stand alone
    if flags.0 & FILE_LOCK_UNLOCK != 0 {
        if flags.0 != FILE_LOCK_UNLOCK {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "flags",
                "unlock cannot be combined with other lock flags",
            ))
            .boxed());
        }

        return Ok(FileLockOperation::Unlock);
    }

    // require exactly one lock mode
    let shared = flags.0 & FILE_LOCK_SHARED != 0;
    let exclusive = flags.0 & FILE_LOCK_EXCLUSIVE != 0;
    if shared == exclusive {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "exactly one of shared or exclusive lock must be requested",
        ))
        .boxed());
    }

    let nonblocking = flags.0 & FILE_LOCK_NONBLOCK != 0;
    if shared {
        return Ok(FileLockOperation::Shared { nonblocking });
    }

    Ok(FileLockOperation::Exclusive { nonblocking })
}

/// Validate and decode one mmap flag payload.
pub(crate) fn decode_mmap_flags(
    flags: MmapFlags,
    allow_anonymous: bool,
) -> RuntimeResult<DecodedMmapFlags> {
    // reject unknown flag bits
    let unsupported_flags = flags.0 & !MMAP_SUPPORTED_FLAGS;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported mmap flags: {unsupported_flags:#x}"),
        ))
        .boxed());
    }

    // require exactly one sharing mode
    let is_shared = flags.0 & MMAP_FLAG_SHARED != 0;
    let is_private = flags.0 & MMAP_FLAG_PRIVATE != 0;
    if is_shared == is_private {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "exactly one of MAP_SHARED or MAP_PRIVATE must be requested",
        ))
        .boxed());
    }

    // require or reject anonymous mapping mode explicitly
    let is_anonymous = flags.0 & MMAP_FLAG_ANONYMOUS != 0;
    if allow_anonymous {
        if !is_anonymous {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "flags",
                "MAP_ANON is required for anonymous mappings",
            ))
            .boxed());
        }
    } else if is_anonymous {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "MAP_ANON is invalid for file mappings",
        ))
        .boxed());
    }

    Ok(DecodedMmapFlags {
        is_shared,
        is_fixed: flags.0 & MMAP_FLAG_FIXED != 0,
    })
}

/// Validate one xattr flag payload.
pub(crate) fn validate_xattr_flags(flags: XattrFlags) -> RuntimeResult<()> {
    // reject unknown flag bits
    let unsupported_flags = flags.0 & !XATTR_SUPPORTED_FLAGS;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported xattr flags: {unsupported_flags:#x}"),
        ))
        .boxed());
    }

    // reject mutually exclusive create and replace requests
    let create = flags.0 & XATTR_FLAG_CREATE != 0;
    let replace = flags.0 & XATTR_FLAG_REPLACE != 0;
    if create && replace {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "XATTR_CREATE and XATTR_REPLACE are mutually exclusive",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one native or vm mapping length against slice limits.
pub(crate) fn validate_mapping_length(length: FileSize) -> RuntimeResult<usize> {
    if length.0 == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "length",
            "mapping length must be non-zero",
        ))
        .boxed());
    }

    if length.0 > u64::from(u32::MAX) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "length",
            "mapping length exceeds slice limits",
        ))
        .boxed());
    }

    Ok(length.0 as usize)
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
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn empty_path_utf16() -> PathUtf16 {
    PathUtf16Abi::<NativeAbi>(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build an `OsPath` from raw byte data.
pub(crate) fn path_ref_from_bytes(bytes: PathBytes) -> OsPath {
    OsPath::OsPathBytes(OsPathBytes {
        kind: "bytes".into(),
        bytes,
    })
}

/// Build an `OsPath` from UTF-16 data.
pub(crate) fn path_ref_from_utf16(utf16: PathUtf16) -> OsPath {
    OsPath::OsPathUtf16(OsPathUtf16 {
        kind: "utf16".into(),
        utf16,
    })
}

/// Decode an `OsPath` into a UTF-8 string.
pub(crate) fn os_path_to_utf8_string(path: OsPath, label: &str) -> RuntimeResult<String> {
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            let bytes = unsafe { path_bytes.bytes.0.as_slice()? };
            String::from_utf8(bytes.to_vec()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path bytes are not valid utf8",
                ))
                .boxed()
            })
        }
        OsPath::OsPathUtf16(path_utf16) => {
            let utf16 = unsafe { path_utf16.utf16.0.as_slice()? };
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
pub(crate) fn os_path_from_utf8_string(binding: &BindingCallContext, value: String) -> OsPath {
    #[cfg(unix)]
    {
        let bytes = PathBytesAbi::<NativeAbi>(binding.store_array(value.into_bytes()));
        path_ref_from_bytes(bytes)
    }

    #[cfg(windows)]
    {
        let utf16_values = value.encode_utf16().collect::<Vec<_>>();
        let utf16 = PathUtf16Abi::<NativeAbi>(binding.store_array(utf16_values));
        path_ref_from_utf16(utf16)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = PathBytesAbi::<NativeAbi>(binding.store_array(value.into_bytes()));
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
    binding: &BindingCallContext,
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

    Ok(PathUtf16Abi::<NativeAbi>(binding.store_array(utf16)))
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

    match path {
        OsPath::OsPathBytes(path_bytes) => on_bytes(path_bytes.bytes),
        OsPath::OsPathUtf16(path_utf16) => {
            #[cfg(unix)]
            {
                with_utf16_as_bytes(path_utf16.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(path_utf16.utf16)
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
    match (left, right) {
        (OsPath::OsPathBytes(left_bytes), OsPath::OsPathBytes(right_bytes)) => {
            on_bytes(left_bytes.bytes, right_bytes.bytes)
        }
        (OsPath::OsPathUtf16(left_utf16), OsPath::OsPathUtf16(right_utf16)) => {
            #[cfg(unix)]
            {
                with_utf16_pair_as_bytes(left_utf16.utf16, right_utf16.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(left_utf16.utf16, right_utf16.utf16)
            }
        }
        _ => path_ref_mismatch(label),
    }
}

/// Build an `OsPath` from a host path.
#[cfg(any(unix, windows))]
pub(crate) fn os_path_from_path(binding: &BindingCallContext, path: &Path) -> OsPath {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes = path.as_os_str().as_bytes().to_vec();
        let bytes = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
        path_ref_from_bytes(bytes)
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let utf16 = path.as_os_str().encode_wide().collect::<Vec<_>>();
        let utf16 = PathUtf16Abi::<NativeAbi>(binding.store_array(utf16));
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
/// Bitmask of all supported watch-mask bits.
#[cfg(any(unix, windows))]
const WATCH_MASK_SUPPORTED_BITS: u32 = WATCH_MASK_CREATE
    | WATCH_MASK_REMOVE
    | WATCH_MASK_MODIFY
    | WATCH_MASK_RENAME
    | WATCH_MASK_METADATA
    | WATCH_MASK_OVERFLOW;

/// One queued watch event record.
#[cfg(any(unix, windows))]
#[derive(Debug, Clone)]
struct WatchQueuedEvent {
    /// Event kind for this record.
    kind: WatchQueuedKind,
    /// Primary path payload when present.
    path: Option<PathBuf>,
    /// Related path payload when present.
    related_path: Option<PathBuf>,
    /// Backend cookie value when present.
    cookie: u64,
}

/// One internal watch event kind.
#[cfg(any(unix, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchQueuedKind {
    /// Create.
    Create,
    /// Remove.
    Remove,
    /// Modify.
    Modify,
    /// Rename.
    Rename,
    /// Metadata.
    Metadata,
    /// Overflow.
    Overflow,
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
    binding: &BindingCallContext,
    handle: WatchHandle,
) -> RuntimeResult<Arc<Mutex<WatchResource>>> {
    require_resource(binding, handle.0, ResourceKind::Watch, "watch", |entry| {
        entry
            .payload_cloned::<Arc<Mutex<WatchResource>>>()
            .ok_or_else(invalid_watch_handle_error)
    })
}

/// Return true when one watch-kind passes one mask.
#[cfg(any(unix, windows))]
fn watch_mask_allows(mask: WatchMask, kind: WatchQueuedKind) -> bool {
    if mask.0 == 0 {
        return true;
    }

    let bit = match kind {
        WatchQueuedKind::Create => WATCH_MASK_CREATE,
        WatchQueuedKind::Remove => WATCH_MASK_REMOVE,
        WatchQueuedKind::Modify => WATCH_MASK_MODIFY,
        WatchQueuedKind::Rename => WATCH_MASK_RENAME,
        WatchQueuedKind::Metadata => WATCH_MASK_METADATA,
        WatchQueuedKind::Overflow => WATCH_MASK_OVERFLOW,
    };
    mask.0 & bit != 0
}

/// Validate one caller-supplied watch mask.
#[cfg(any(unix, windows))]
fn validate_watch_mask(mask: WatchMask) -> RuntimeResult<()> {
    let unsupported_bits = mask.0 & !WATCH_MASK_SUPPORTED_BITS;
    if unsupported_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.mask",
            format!("unsupported watch mask bits: {unsupported_bits:#x}"),
        ))
        .boxed());
    }

    Ok(())
}

/// Enqueue one watch record when it passes the mask filter.
#[cfg(any(unix, windows))]
fn push_watch_record(
    resource: &mut WatchResource,
    kind: WatchQueuedKind,
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
    push_watch_record(resource, WatchQueuedKind::Overflow, None, None, cookie);
}

/// Map one notify event into queued watch records.
#[cfg(any(unix, windows))]
fn push_notify_event(resource: &mut WatchResource, event: Event) {
    let cookie = event.tracker().unwrap_or(0) as u64;
    let needs_rescan = event.need_rescan();
    if needs_rescan {
        push_overflow_record(resource, cookie);
    }

    match event.kind {
        EventKind::Create(_) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchQueuedKind::Create, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchQueuedKind::Create, Some(path), None, cookie);
            }
        }
        EventKind::Remove(_) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchQueuedKind::Remove, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchQueuedKind::Remove, Some(path), None, cookie);
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            if event.paths.len() >= 2 {
                let source = event.paths[0].clone();
                let target = event.paths[1].clone();
                push_watch_record(
                    resource,
                    WatchQueuedKind::Rename,
                    Some(source),
                    Some(target),
                    cookie,
                );
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchQueuedKind::Rename, Some(path), None, cookie);
            }
        }
        EventKind::Modify(ModifyKind::Name(_)) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchQueuedKind::Rename, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchQueuedKind::Rename, Some(path), None, cookie);
            }
        }
        EventKind::Modify(ModifyKind::Metadata(_)) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchQueuedKind::Metadata, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(
                    resource,
                    WatchQueuedKind::Metadata,
                    Some(path),
                    None,
                    cookie,
                );
            }
        }
        EventKind::Modify(_) => {
            if event.paths.is_empty() {
                push_watch_record(resource, WatchQueuedKind::Modify, None, None, cookie);
                return;
            }

            for path in event.paths {
                push_watch_record(resource, WatchQueuedKind::Modify, Some(path), None, cookie);
            }
        }
        EventKind::Access(_) => {
            // ignore access events: the public watch contract does not expose an access class
        }
        EventKind::Any | EventKind::Other => {
            // surface unknown backend events as overflow unless a rescan already did that
            if !needs_rescan {
                push_overflow_record(resource, cookie);
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
fn watch_event_from_queued(binding: &BindingCallContext, record: WatchQueuedEvent) -> WatchEvent {
    let metadata = WatchEventMetadata {
        cookie: record.cookie,
    };

    match record.kind {
        WatchQueuedKind::Create => {
            let path = if let Some(path) = record.path.as_deref() {
                os_path_from_path(binding, path)
            } else {
                empty_os_path()
            };
            WatchEvent::WatchCreateEvent(WatchCreateEvent {
                kind: "create".into(),
                metadata,
                path,
            })
        }
        WatchQueuedKind::Remove => {
            let path = if let Some(path) = record.path.as_deref() {
                os_path_from_path(binding, path)
            } else {
                empty_os_path()
            };
            WatchEvent::WatchRemoveEvent(WatchRemoveEvent {
                kind: "remove".into(),
                metadata,
                path,
            })
        }
        WatchQueuedKind::Modify => {
            let path = if let Some(path) = record.path.as_deref() {
                os_path_from_path(binding, path)
            } else {
                empty_os_path()
            };
            WatchEvent::WatchModifyEvent(WatchModifyEvent {
                kind: "modify".into(),
                metadata,
                path,
            })
        }
        WatchQueuedKind::Rename => {
            let path = if let Some(path) = record.path.as_deref() {
                os_path_from_path(binding, path)
            } else {
                empty_os_path()
            };
            let related_path = if let Some(related_path) = record.related_path.as_deref() {
                os_path_from_path(binding, related_path)
            } else {
                empty_os_path()
            };
            WatchEvent::WatchRenameEvent(WatchRenameEvent {
                kind: "rename".into(),
                metadata,
                path,
                related_path,
            })
        }
        WatchQueuedKind::Metadata => {
            let path = if let Some(path) = record.path.as_deref() {
                os_path_from_path(binding, path)
            } else {
                empty_os_path()
            };
            WatchEvent::WatchMetadataEvent(WatchMetadataEvent {
                kind: "metadata".into(),
                metadata,
                path,
            })
        }
        WatchQueuedKind::Overflow => WatchEvent::WatchOverflowEvent(WatchOverflowEvent {
            kind: "overflow".into(),
            metadata,
        }),
    }
}

/// Open one filesystem watch handle.
#[cfg(any(unix, windows))]
pub(crate) fn open_watch(
    binding: &BindingCallContext,
    path: &Path,
    options: WatchOptions,
) -> RuntimeResult<WatchHandle> {
    // reject unknown watch mask bits up front
    validate_watch_mask(options.mask)?;

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
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));

    Ok(WatchHandle(resource_id))
}

/// Close one filesystem watch handle.
#[cfg(any(unix, windows))]
pub(crate) fn close_watch(binding: &BindingCallContext, handle: WatchHandle) -> RuntimeResult<()> {
    // ensure the handle still points to one watch resource
    let _resource = watch_resource(binding, handle)?;

    // remove the watch resource and drop the backend watcher
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(invalid_watch_handle_error());
    }

    Ok(())
}

/// Read one filesystem watch batch from a watch handle.
#[cfg(any(unix, windows))]
pub(crate) fn read_watch(
    binding: &BindingCallContext,
    handle: WatchHandle,
) -> RuntimeResult<WatchBatch> {
    // resolve the watch resource and drain pending backend events
    let resource = watch_resource(binding, handle)?;
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
        let event = watch_event_from_queued(binding, queued_event);
        events.push(event);
    }

    Ok(WatchBatch {
        events: binding.store_array(events),
        overflowed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one watch resource for internal event-mapping tests.
    fn test_watch_resource() -> WatchResource {
        let (_sender, receiver) = std::sync::mpsc::channel();
        let watcher = notify::recommended_watcher(|_| {}).expect("watcher should initialize");

        WatchResource {
            watcher,
            receiver,
            queued_events: VecDeque::new(),
            overflowed: false,
            mask: WatchMask(0),
        }
    }

    /// Ignore access events because the watch contract does not expose them.
    #[test]
    fn test_watch_mapping_ignores_access_events() {
        let mut resource = test_watch_resource();
        let event = Event::new(EventKind::Access(notify::event::AccessKind::Open(
            notify::event::AccessMode::Read,
        )))
        .add_path(PathBuf::from("alpha.txt"));

        push_notify_event(&mut resource, event);

        assert!(!resource.overflowed);
        assert!(resource.queued_events.is_empty());
    }

    /// Surface unclassified backend events as overflow instead of relabeling them as metadata.
    #[test]
    fn test_watch_mapping_escalates_unknown_events_to_overflow() {
        let mut resource = test_watch_resource();
        let event = Event::new(EventKind::Other).set_tracker(7);

        push_notify_event(&mut resource, event);

        assert!(resource.overflowed);
        assert_eq!(resource.queued_events.len(), 1);
        assert_eq!(resource.queued_events[0].kind, WatchQueuedKind::Overflow);
        assert_eq!(resource.queued_events[0].cookie, 7);
    }

    /// Preserve one rescan overflow without duplicating it for unknown event kinds.
    #[test]
    fn test_watch_mapping_does_not_duplicate_rescan_overflow() {
        let mut resource = test_watch_resource();
        let event = Event::new(EventKind::Any)
            .set_flag(notify::event::Flag::Rescan)
            .set_tracker(9);

        push_notify_event(&mut resource, event);

        assert!(resource.overflowed);
        assert_eq!(resource.queued_events.len(), 1);
        assert_eq!(resource.queued_events[0].kind, WatchQueuedKind::Overflow);
        assert_eq!(resource.queued_events[0].cookie, 9);
    }

    /// Preserve paired rename paths when the backend supplies both sides.
    #[test]
    fn test_watch_mapping_preserves_rename_pair_paths() {
        let mut resource = test_watch_resource();
        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("alpha.txt"))
            .add_path(PathBuf::from("beta.txt"))
            .set_tracker(11);

        push_notify_event(&mut resource, event);

        assert!(!resource.overflowed);
        assert_eq!(resource.queued_events.len(), 1);
        assert_eq!(resource.queued_events[0].kind, WatchQueuedKind::Rename);
        assert_eq!(
            resource.queued_events[0].path,
            Some(PathBuf::from("alpha.txt"))
        );
        assert_eq!(
            resource.queued_events[0].related_path,
            Some(PathBuf::from("beta.txt"))
        );
        assert_eq!(resource.queued_events[0].cookie, 11);
    }
}
