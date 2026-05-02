use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use destack_daemon::{Daemon, WatchPolicy};
use destack_source::{FileSystem, FileWatchEvent, FileWatchEventKind, MemoryFileWatcher};
use destack_workspace::Repository;

use crate::common::WatchCompileReason;
use crate::pipeline::watch::{
    WatchLoopAction, WatchLoopOptions, build_watch_options, is_source_path, run_watch_loop,
};

use super::tests::TestProgram;

/// Build a daemon configured for deterministic test execution.
fn daemon_with_single_worker(repository: Arc<Repository>) -> Daemon {
    Daemon::new(repository, 1, None)
}

/// State captured by watch loop callbacks.
#[derive(Debug, Default)]
struct WatchLoopState {
    /// Number of rescan hooks executed.
    rescan_calls: usize,
    /// Number of compile hooks executed.
    compile_calls: usize,
    /// Last watch reason observed.
    last_reason: Option<WatchCompileReason>,
    /// Last updated flag observed.
    last_updated: Option<bool>,
    /// Last rescan flag observed.
    last_rescan: Option<bool>,
}

/// Harness for running a single watch loop cycle.
struct WatchLoopHarness {
    /// The daemon backing watch updates.
    daemon: Daemon,
    /// The watcher used to emit events.
    watcher: MemoryFileWatcher,
    /// Watch loop options for the watcher.
    options: WatchLoopOptions,
}

impl WatchLoopHarness {
    /// Build a harness for a test program.
    fn new(repository: Arc<Repository>) -> Self {
        // configure the memory watcher with a short coalesce window
        let watcher = MemoryFileWatcher::new();
        let options = WatchLoopOptions {
            watcher: Arc::new(watcher.clone()),
            options: build_watch_options(),
            policy: WatchPolicy {
                coalesce_window: Duration::from_millis(5),
                max_batch_size: 32,
            },
        };

        // create the daemon used for updates
        let daemon = daemon_with_single_worker(repository);

        Self {
            daemon,
            watcher,
            options,
        }
    }

    /// Run a watch loop cycle while emitting a single event.
    fn run_once(&self, root: PathBuf, event: FileWatchEvent) -> (i32, WatchLoopState) {
        // set up watch state
        let mut state = WatchLoopState::default();
        let mut reporter = None;

        // execute the loop and emit the event immediately after startup
        let exit_code = run_watch_loop(
            &self.daemon,
            vec![root],
            &mut reporter,
            self.options.clone(),
            &mut state,
            |_| {
                self.watcher.emit(event);
            },
            |state| {
                // track rescan invocations
                state.rescan_calls += 1;
                Ok(())
            },
            |state, _reporter, reason, _batch_id, updated, rescan| {
                // track compile invocations
                state.compile_calls += 1;
                state.last_reason = Some(reason);
                state.last_updated = Some(updated);
                state.last_rescan = Some(rescan);

                WatchLoopAction::stop_with(Some(0))
            },
            0,
        );

        (exit_code, state)
    }
}

/// Validate the watchable path filter.
#[test]
fn test_is_source_path_filters_extensions() {
    let code = PathBuf::from("/test/main.ds");
    let data = PathBuf::from("/test/config.json");
    let text = PathBuf::from("/test/readme.md");
    let image = PathBuf::from("/test/logo.png");

    // assert watchable extensions
    assert!(is_source_path(&code));
    assert!(is_source_path(&data));
    assert!(is_source_path(&text));
    assert!(!is_source_path(&image));
}

/// Ensure watch options include a usable filter.
#[test]
fn test_build_watch_options_filters_paths() {
    let options = build_watch_options();
    let filter = options.filter.expect("watch filter should be defined");

    let code = PathBuf::from("/test/main.ts");
    let binary = PathBuf::from("/test/asset.wasm");

    // assert filter behavior
    assert!(filter(&code));
    assert!(!filter(&binary));
}

/// Apply watch events to a tracked file.
#[test]
fn test_apply_watch_event_updates_file() {
    let test = TestProgram::new("watch_update");

    let path = test.write_text("src/main.ds", "export const value = 1;\n");
    let daemon = daemon_with_single_worker(test.repository.clone());
    let _ = daemon
        .update_file(&path, "export const value = 1;\n".to_string())
        .expect("initial source update should succeed");
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };

    let result = daemon.apply_watch_event(&event);

    // assert the update is applied
    assert!(result.updated());
}

/// Apply delete watch events to a tracked file.
#[test]
fn test_apply_watch_event_deletes_file() {
    let test = TestProgram::new("watch_delete");

    let path = test.write_text("src/main.ds", "export const value = 1;\n");
    let daemon = daemon_with_single_worker(test.repository.clone());
    let _ = daemon
        .update_file(&path, "export const value = 1;\n".to_string())
        .expect("initial source update should succeed");
    test.fs
        .remove_file(&path)
        .expect("deleted file should be removed from the filesystem");
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Deleted,
    };

    let result = daemon.apply_watch_event(&event);
    // check that the deleted file is removed from the revision
    assert!(result.updated());
    assert!(!test.has_file_for_path(&path));
}

/// Apply rename watch events to tracked files.
#[test]
fn test_apply_watch_event_renames_file() {
    let test = TestProgram::new("watch_rename");

    let old_path = test.write_text("src/old.ds", "export const value = 1;\n");
    let new_path = test.write_text("src/new.ds", "export const value = 1;\n");
    let daemon = daemon_with_single_worker(test.repository.clone());
    let _ = daemon
        .update_file(&old_path, "export const value = 1;\n".to_string())
        .expect("initial old source update should succeed");
    let _ = daemon
        .update_file(&new_path, "export const value = 1;\n".to_string())
        .expect("initial new source update should succeed");
    test.fs
        .remove_file(&old_path)
        .expect("renamed file should be removed from the old path");
    let event = FileWatchEvent {
        path: new_path.clone(),
        previous_path: Some(old_path.clone()),
        kind: FileWatchEventKind::Renamed,
    };

    let result = daemon.apply_watch_event(&event);
    // check that the rename removes the old file and updates the new one
    assert!(result.updated());
    assert!(!test.has_file_for_path(&old_path));
    assert!(test.has_file_for_path(&new_path));
}

/// Apply config watch events through the daemon.
#[test]
fn test_apply_watch_event_requests_rescan_for_config() {
    let test = TestProgram::new("watch_config");

    let path = test.write_text("destack.json", "{ \"compiler\": {} }\n");
    let daemon = daemon_with_single_worker(test.repository.clone());
    let _ = daemon
        .update_file(&path, "{ \"compiler\": {} }\n".to_string())
        .expect("initial config update should succeed");
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };

    let result = daemon.apply_watch_event(&event);

    // check that the config update is applied
    assert!(result.updated());
}

/// Exercise the shared watch loop for config updates.
#[test]
fn test_run_watch_loop_handles_config_update() {
    let test = TestProgram::new("watch_loop_config");

    let path = test.write_text("destack.json", "{ \"compiler\": {} }\n");
    let daemon = daemon_with_single_worker(test.repository.clone());
    let _ = daemon
        .update_file(&path, "{ \"compiler\": {} }\n".to_string())
        .expect("initial config update should succeed");
    let harness = WatchLoopHarness::new(test.repository.clone());
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };
    let (exit_code, state) = harness.run_once(test.root.clone(), event);

    // check that config updates trigger rescan compiles
    assert_eq!(exit_code, 0);
    assert_eq!(state.rescan_calls, 1);
    assert_eq!(state.compile_calls, 1);
    assert!(matches!(
        state.last_reason,
        Some(WatchCompileReason::Rescan) | Some(WatchCompileReason::UpdateRescan)
    ));
    assert_eq!(state.last_rescan, Some(true));
}

/// Exercise the shared watch loop for source updates.
#[test]
fn test_run_watch_loop_handles_source_update() {
    let test = TestProgram::new("watch_loop_source");

    let path = test.write_text("src/main.ds", "export const value = 1;\n");
    let daemon = daemon_with_single_worker(test.repository.clone());
    let _ = daemon
        .update_file(&path, "export const value = 1;\n".to_string())
        .expect("initial source update should succeed");
    let harness = WatchLoopHarness::new(test.repository.clone());
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };
    let (exit_code, state) = harness.run_once(test.root.clone(), event);

    // check that the source updates skip rescan
    assert_eq!(exit_code, 0);
    assert_eq!(state.rescan_calls, 0);
    assert_eq!(state.compile_calls, 1);
    assert_eq!(state.last_reason, Some(WatchCompileReason::Update));
    assert_eq!(state.last_updated, Some(true));
    assert_eq!(state.last_rescan, Some(false));
}
