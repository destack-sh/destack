use std::path::Path;
use std::time::{Duration, Instant};

use super::{FsHarnessContext, FsWatchEvent, temp_dir, with_harness_context};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{FileMode, WatchMask, WatchOptions};
use crate::platform::resource::WatchHandle;

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

/// Poll watch batches until one event or overflow marker arrives.
fn poll_watch_events(
    context: &mut FsHarnessContext<'_>,
    handle: WatchHandle,
    timeout: Duration,
) -> RuntimeResult<(Vec<FsWatchEvent>, bool)> {
    let start = Instant::now();
    loop {
        // read one watch batch and return when signal data arrives
        let batch = context.destack_fs_watch_read(handle)?;
        let (events, overflowed) = context.watch_batch_from_value(batch)?;
        if !events.is_empty() || overflowed {
            return Ok((events, overflowed));
        }

        // stop polling once the timeout budget is exhausted
        if start.elapsed() >= timeout {
            return Ok((Vec::new(), false));
        }

        std::thread::sleep(Duration::from_millis(20));
    }
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

        // read watch data until one event is observed
        let (events, overflowed) = poll_watch_events(&mut context, watch, Duration::from_secs(3))?;
        assert!(!overflowed);
        assert!(!events.is_empty(), "expected at least one watch event");

        // verify one create or modify style event was observed
        let mut matched_kind = false;
        for event in events {
            let event_path = context.watch_event_path(event);
            let _related_path = context.watch_event_related_path(event);
            let is_matching_kind = context.watch_event_is_create_modify_metadata_or_rename(event);
            if is_matching_kind {
                assert!(!event_path.is_empty());
                matched_kind = true;
                break;
            }
        }
        assert!(
            matched_kind,
            "expected a create or modify style watch event"
        );

        // close the watcher and remove filesystem state
        context.destack_fs_watch_close(watch)?;
        std::fs::remove_file(&file).map_err(runtime_io_error)?;

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

        // read one event batch and verify signal presence
        let (events, overflowed) = poll_watch_events(&mut context, watch, Duration::from_secs(3))?;
        assert!(!overflowed);
        assert!(!events.is_empty(), "expected at least one watchat event");
        let mut has_path_event = false;
        for event in events {
            let event_path = context.watch_event_path(event);
            let _related_path = context.watch_event_related_path(event);
            if !event_path.is_empty() {
                has_path_event = true;
            }
        }
        assert!(
            has_path_event,
            "expected at least one watchat event with a path"
        );

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
