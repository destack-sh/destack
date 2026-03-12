use std::path::Path;
use std::time::{Duration, Instant};

use super::{
    FsHarnessContext, assert_platform_error_codes_with_privileged_policy, temp_dir,
    with_harness_context,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{FileMode, WatchMask, WatchOptions};
use crate::platform::resource::WatchHandle;

/// Watch-mask bit for create events.
const WATCH_MASK_CREATE_BIT: u32 = 1 << 0;
/// Watch-mask bit for remove events.
const WATCH_MASK_REMOVE_BIT: u32 = 1 << 1;
/// Poll delay used while waiting for backend watch delivery.
const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(20);
/// Timeout budget for shared watch event assertions.
const WATCH_EVENT_TIMEOUT: Duration = Duration::from_secs(3);

/// One decoded watch event observed by the shared test helper.
#[derive(Clone, Debug)]
struct ObservedWatchEvent {
    /// The normalized event kind.
    kind: &'static str,
    /// The normalized event path.
    path: String,
    /// The normalized related path when present.
    related_path: Option<String>,
}

/// Convert one io error into one runtime error payload.
fn runtime_io_error(error: std::io::Error) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io(error.to_string())).boxed()
}

/// Default watch options used by filesystem watch tests.
fn default_watch_options() -> WatchOptions {
    WatchOptions {
        mask: WatchMask(0),
        recursive: false,
        follow_symlinks: true,
    }
}

/// Poll watch batches until the expected event arrives or the timeout expires.
fn poll_watch_events_until(
    context: &mut FsHarnessContext<'_>,
    handle: WatchHandle,
    timeout: Duration,
    mut predicate: impl FnMut(&ObservedWatchEvent) -> bool,
) -> RuntimeResult<(Vec<ObservedWatchEvent>, bool)> {
    let start = Instant::now();
    let mut observed_events = Vec::new();
    loop {
        // read one watch batch and aggregate all decoded events
        let batch = context.destack_fs_watch_read(handle)?;
        let (events, overflowed) = context.watch_batch_from_value(batch)?;
        for event in events {
            let observed_event = ObservedWatchEvent {
                kind: context.watch_event_kind_name(event),
                path: context.watch_event_path(event),
                related_path: context.watch_event_related_path(event),
            };
            let is_match = predicate(&observed_event);
            observed_events.push(observed_event);
            if is_match {
                return Ok((observed_events, false));
            }
        }

        // surface overflow immediately with the events seen so far
        if overflowed {
            return Ok((observed_events, true));
        }

        // stop polling once the timeout budget is exhausted
        if start.elapsed() >= timeout {
            return Ok((observed_events, false));
        }

        std::thread::sleep(WATCH_POLL_INTERVAL);
    }
}

/// Return true when one watch-event path ends with the expected file name.
fn watch_event_matches_file_name(path: &str, expected_name: &str) -> bool {
    Path::new(path).file_name().and_then(|value| value.to_str()) == Some(expected_name)
}

/// Format one observed event stream for failure messages.
fn format_observed_watch_events(events: &[ObservedWatchEvent]) -> String {
    let mut rendered = String::new();
    for event in events {
        if !rendered.is_empty() {
            rendered.push_str(", ");
        }

        rendered.push_str(event.kind);
        rendered.push(':');
        rendered.push_str(&event.path);
        if let Some(related_path) = &event.related_path {
            rendered.push_str("->");
            rendered.push_str(related_path);
        }
    }

    rendered
}

/// Open and close one watch handle for one directory path.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_watch_open_close() {
    with_harness_context(|mut context| {
        // create one temporary directory to watch
        let root = temp_dir("fs_watch_open_close");
        let root_path = context.path_bytes(&root);
        context.destack_fs_mkdir(root_path, FileMode(0o755))?;

        // open and close one watch handle
        let root_path = context.path_bytes(&root);
        let options = context.watch_options_value(default_watch_options());
        let watch = context.destack_fs_watch(root_path, options)?;
        context.destack_fs_watch_close(watch)?;

        // remove the temporary directory
        let root_path = context.path_bytes(&root);
        context.destack_fs_rmdir(root_path)?;

        Ok(())
    });
}

/// Emit watch events for file creation in one watched directory.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_watch_reports_create_lifecycle() {
    with_harness_context(|mut context| {
        // create one temporary directory to watch
        let root = temp_dir("fs_watch_create");
        let file = root.join("item.txt");
        let root_path = context.path_bytes(&root);
        context.destack_fs_mkdir(root_path, FileMode(0o755))?;

        // open one watch handle for the directory
        let root_path = context.path_bytes(&root);
        let options = context.watch_options_value(default_watch_options());
        let watch = context.destack_fs_watch(root_path, options)?;

        // write one file to trigger watcher notifications
        std::fs::write(&file, b"watch-data").map_err(runtime_io_error)?;

        // read watch data until the created file path appears
        let (events, overflowed) =
            poll_watch_events_until(&mut context, watch, WATCH_EVENT_TIMEOUT, |event| {
                watch_event_matches_file_name(&event.path, "item.txt")
            })?;
        assert!(!overflowed);
        let matched_event = events
            .iter()
            .find(|event| watch_event_matches_file_name(&event.path, "item.txt"));
        assert!(
            matched_event.is_some(),
            "expected one watch event for item.txt, observed: {}",
            format_observed_watch_events(&events)
        );
        assert!(matches!(
            matched_event.map(|event| event.kind),
            Some("create" | "modify" | "metadata" | "rename")
        ));

        // close the watcher and remove filesystem state
        context.destack_fs_watch_close(watch)?;
        std::fs::remove_file(&file).map_err(runtime_io_error)?;

        let root_path = context.path_bytes(&root);
        context.destack_fs_rmdir(root_path)?;

        Ok(())
    });
}

/// Emit watch events for file removal in one watched directory.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_watch_reports_remove_lifecycle() {
    with_harness_context(|mut context| {
        // create one temporary directory and watched file
        let root = temp_dir("fs_watch_remove");
        let file = root.join("item.txt");
        let root_path = context.path_bytes(&root);
        context.destack_fs_mkdir(root_path, FileMode(0o755))?;
        std::fs::write(&file, b"watch-remove").map_err(runtime_io_error)?;

        // open one watch handle for the directory
        let root_path = context.path_bytes(&root);
        let options = context.watch_options_value(WatchOptions {
            mask: WatchMask(WATCH_MASK_REMOVE_BIT),
            recursive: false,
            follow_symlinks: true,
        });
        let watch = context.destack_fs_watch(root_path, options)?;

        // remove the file to trigger watcher notifications
        std::fs::remove_file(&file).map_err(runtime_io_error)?;

        // require one remove event for the removed file
        let (events, overflowed) =
            poll_watch_events_until(&mut context, watch, WATCH_EVENT_TIMEOUT, |event| {
                event.kind == "remove" && watch_event_matches_file_name(&event.path, "item.txt")
            })?;
        assert!(!overflowed);
        assert!(
            events.iter().all(|event| event.kind == "remove"),
            "expected remove-only mask filtering, observed: {}",
            format_observed_watch_events(&events)
        );
        assert!(
            events
                .iter()
                .any(|event| watch_event_matches_file_name(&event.path, "item.txt")),
            "expected one remove event for item.txt, observed: {}",
            format_observed_watch_events(&events)
        );

        // close the watcher and remove filesystem state
        context.destack_fs_watch_close(watch)?;
        let root_path = context.path_bytes(&root);
        context.destack_fs_rmdir(root_path)?;

        Ok(())
    });
}

/// Open one relative watch with watchat and receive events from child writes.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_watchat_reports_child_events() {
    with_harness_context(|mut context| {
        // create one parent and child directory for relative watching
        let parent = temp_dir("fs_watchat");
        let child = parent.join("child");
        let file = child.join("entry.txt");

        let parent_path = context.path_bytes(&parent);
        context.destack_fs_mkdir(parent_path, FileMode(0o755))?;

        let child_path = context.path_bytes(&child);
        context.destack_fs_mkdir(child_path, FileMode(0o755))?;

        // open one directory handle and one relative watcher
        let parent_path = context.path_bytes(&parent);
        let directory = context.destack_fs_opendir(parent_path)?;

        let relative = context.path_bytes(Path::new("child"));
        let options = context.watch_options_value(default_watch_options());
        let watch = context.destack_fs_watchat(directory, relative, options)?;

        // write one file in the child directory
        std::fs::write(&file, b"watchat-data").map_err(runtime_io_error)?;

        // read watch data until the child file path appears
        let (events, overflowed) =
            poll_watch_events_until(&mut context, watch, WATCH_EVENT_TIMEOUT, |event| {
                watch_event_matches_file_name(&event.path, "entry.txt")
            })?;
        assert!(!overflowed);
        let matched_event = events
            .iter()
            .find(|event| watch_event_matches_file_name(&event.path, "entry.txt"));
        assert!(
            matched_event.is_some(),
            "expected one watchat event for entry.txt, observed: {}",
            format_observed_watch_events(&events)
        );
        assert!(matches!(
            matched_event.map(|event| event.kind),
            Some("create" | "modify" | "metadata" | "rename")
        ));

        // close handles and remove filesystem state
        context.destack_fs_watch_close(watch)?;
        context.destack_fs_closedir(directory)?;

        std::fs::remove_file(&file).map_err(runtime_io_error)?;

        let child_path = context.path_bytes(&child);
        context.destack_fs_rmdir(child_path)?;

        let parent_path = context.path_bytes(&parent);
        context.destack_fs_rmdir(parent_path)?;

        Ok(())
    });
}

/// Filter watch events through the requested mask.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_watch_respects_create_mask() {
    with_harness_context(|mut context| {
        // create one temporary directory to watch
        let root = temp_dir("fs_watch_mask");
        let file = root.join("masked.txt");
        let root_path = context.path_bytes(&root);
        context.destack_fs_mkdir(root_path, FileMode(0o755))?;

        // open one watch handle filtered to create events
        let root_path = context.path_bytes(&root);
        let options = context.watch_options_value(WatchOptions {
            mask: WatchMask(WATCH_MASK_CREATE_BIT),
            recursive: false,
            follow_symlinks: true,
        });
        let watch = context.destack_fs_watch(root_path, options)?;

        // write one file to trigger watcher notifications
        std::fs::write(&file, b"mask-data").map_err(runtime_io_error)?;

        // read watch data until the created file path appears
        let (events, overflowed) =
            poll_watch_events_until(&mut context, watch, WATCH_EVENT_TIMEOUT, |event| {
                watch_event_matches_file_name(&event.path, "masked.txt")
            })?;
        assert!(!overflowed);
        assert!(
            !events.is_empty(),
            "expected at least one masked watch event"
        );

        // require create-only filtering and the exact created file path
        for event in &events {
            assert_eq!(
                event.kind,
                "create",
                "expected create-only mask filtering, observed: {}",
                format_observed_watch_events(&events)
            );
        }
        assert!(
            events
                .iter()
                .any(|event| watch_event_matches_file_name(&event.path, "masked.txt")),
            "expected one create event for masked.txt, observed: {}",
            format_observed_watch_events(&events)
        );

        // close the watcher and remove filesystem state
        context.destack_fs_watch_close(watch)?;
        std::fs::remove_file(&file).map_err(runtime_io_error)?;

        let root_path = context.path_bytes(&root);
        context.destack_fs_rmdir(root_path)?;

        Ok(())
    });
}

/// Reject unknown watch-mask bits instead of silently dropping them.
#[cfg(any(unix, windows))]
#[test]
fn test_fs_watch_rejects_unknown_mask_bits() {
    with_harness_context(|mut context| {
        // create one temporary directory to watch
        let root = temp_dir("fs_watch_invalid_mask");
        let root_path = context.path_bytes(&root);
        context.destack_fs_mkdir(root_path, FileMode(0o755))?;

        // reject unsupported mask bits at watch creation time
        let root_path = context.path_bytes(&root);
        let options = context.watch_options_value(WatchOptions {
            mask: WatchMask(0x40),
            recursive: false,
            follow_symlinks: true,
        });
        assert_platform_error_codes_with_privileged_policy(
            context.destack_fs_watch(root_path, options),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // cleanup
        let root_path = context.path_bytes(&root);
        context.destack_fs_rmdir(root_path)?;

        Ok(())
    });
}
