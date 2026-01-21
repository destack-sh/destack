use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use destack_daemon::{Daemon, WatchPolicy};
use destack_source::{File, FileType, FileWatchEvent, FileWatchEventKind, MemoryFileWatcher, Uri};
use destack_workspace::Program;

use crate::common::WatchCompileReason;
use crate::pipeline::watch::{
    WatchLoopAction, WatchLoopOptions, build_watch_options, is_watchable_path, run_watch_loop,
};

use super::tests::TestProgram;

/// Register a file in the program registry.
fn register_file(program: &Program, path: &Path, contents: &str, ty: FileType) {
    // allocate a file id
    let file_id = program.files.next_id();

    // build the file metadata
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let uri = Uri::from_path(path);

    // insert the file record
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        ty,
        contents.to_string(),
    );
    program.files.insert(file);
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
    fn new(session: Arc<destack_workspace::Session>) -> Self {
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
        let daemon = Daemon::new(session);

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
fn test_is_watchable_path_filters_extensions() {
    let code = PathBuf::from("/test/main.ds");
    let data = PathBuf::from("/test/config.json");
    let text = PathBuf::from("/test/readme.md");
    let image = PathBuf::from("/test/logo.png");

    // assert watchable extensions
    assert!(is_watchable_path(&code));
    assert!(is_watchable_path(&data));
    assert!(is_watchable_path(&text));
    assert!(!is_watchable_path(&image));
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
    let program = test.session.get_or_create_program(test.root.clone());

    let path = test.write_source("src/main.ds", "export const value = 1;\n");
    register_file(
        &program,
        &path,
        "export const value = 1;\n",
        FileType::Destack,
    );

    let daemon = Daemon::new(test.session.clone());
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };

    let result = daemon.apply_watch_event(&event);

    // assert the update is applied without rescan
    assert!(result.updated());
    assert!(!result.rescan);
}

/// Apply delete watch events to a tracked file.
#[test]
fn test_apply_watch_event_deletes_file() {
    let test = TestProgram::new("watch_delete");
    let program = test.session.get_or_create_program(test.root.clone());

    let path = test.write_source("src/main.ds", "export const value = 1;\n");
    register_file(
        &program,
        &path,
        "export const value = 1;\n",
        FileType::Destack,
    );

    let daemon = Daemon::new(test.session.clone());
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Deleted,
    };

    let result = daemon.apply_watch_event(&event);
    let file = program
        .files
        .get_by_path(&path)
        .expect("file should be tracked");

    // check that the deleted files are marked missing
    assert!(result.updated());
    assert!(!result.rescan);
    assert!(file.is_missing());
}

/// Apply rename watch events to tracked files.
#[test]
fn test_apply_watch_event_renames_file() {
    let test = TestProgram::new("watch_rename");
    let program = test.session.get_or_create_program(test.root.clone());

    let old_path = test.write_source("src/old.ds", "export const value = 1;\n");
    register_file(
        &program,
        &old_path,
        "export const value = 1;\n",
        FileType::Destack,
    );

    let new_path = test.write_source("src/new.ds", "export const value = 1;\n");
    register_file(
        &program,
        &new_path,
        "export const value = 1;\n",
        FileType::Destack,
    );

    let daemon = Daemon::new(test.session.clone());
    let event = FileWatchEvent {
        path: new_path.clone(),
        previous_path: Some(old_path.clone()),
        kind: FileWatchEventKind::Renamed,
    };

    let result = daemon.apply_watch_event(&event);
    let old_file = program
        .files
        .get_by_path(&old_path)
        .expect("old file should be tracked");
    let new_file = program
        .files
        .get_by_path(&new_path)
        .expect("new file should be tracked");

    // check that the rename removes the old file and updates the new one
    assert!(result.updated());
    assert!(!result.rescan);
    assert!(old_file.is_missing());
    assert!(!new_file.is_missing());
}

/// Rescan when config files change.
#[test]
fn test_apply_watch_event_requests_rescan_for_config() {
    let test = TestProgram::new("watch_config");
    let program = test.session.get_or_create_program(test.root.clone());

    let path = test.write_source("dsconfig.json", "{ \"compilerOptions\": {} }\n");
    register_file(
        &program,
        &path,
        "{ \"compilerOptions\": {} }\n",
        FileType::Json,
    );

    let daemon = Daemon::new(test.session.clone());
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };

    let result = daemon.apply_watch_event(&event);

    // check that the rescan is requested
    assert!(!result.updated());
    assert!(result.rescan);
}

/// Exercise the shared watch loop for config updates.
#[test]
fn test_run_watch_loop_handles_config_update() {
    let test = TestProgram::new("watch_loop_config");
    let program = test.session.get_or_create_program(test.root.clone());

    let path = test.write_source("dsconfig.json", "{ \"compilerOptions\": {} }\n");
    register_file(
        &program,
        &path,
        "{ \"compilerOptions\": {} }\n",
        FileType::Json,
    );

    let harness = WatchLoopHarness::new(test.session.clone());
    let event = FileWatchEvent {
        path: path.clone(),
        previous_path: None,
        kind: FileWatchEventKind::Modified,
    };
    let (exit_code, state) = harness.run_once(test.root.clone(), event);

    // check that the config updates trigger rescan compiles
    assert_eq!(exit_code, 0);
    assert_eq!(state.rescan_calls, 1);
    assert_eq!(state.compile_calls, 1);
    assert_eq!(state.last_reason, Some(WatchCompileReason::UpdateRescan));
    assert_eq!(state.last_updated, Some(true));
    assert_eq!(state.last_rescan, Some(true));
}

/// Exercise the shared watch loop for source updates.
#[test]
fn test_run_watch_loop_handles_source_update() {
    let test = TestProgram::new("watch_loop_source");
    let program = test.session.get_or_create_program(test.root.clone());

    let path = test.write_source("src/main.ds", "export const value = 1;\n");
    register_file(
        &program,
        &path,
        "export const value = 1;\n",
        FileType::Destack,
    );

    let harness = WatchLoopHarness::new(test.session.clone());
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
