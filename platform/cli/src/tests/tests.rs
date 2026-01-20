#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use destack_daemon::WatchPolicy;
use destack_resolver::{ResolveOptions, Resolver};
use destack_source::{FileSystem, MemoryFileSystem, MemoryFileWatcher};
use destack_workspace::{MemoryCacheStore, Session};
use serde_json::{Value, json};

use crate::common::{InputArgs, ProgramArgs};
use crate::pipeline::watch::{WatchLoopOptions, build_watch_options};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Test environment for CLI pipeline helpers.
#[derive(Debug)]
pub(super) struct TestProgram {
    /// Root directory for the test.
    pub root: PathBuf,
    /// The in memory file system.
    pub fs: Arc<MemoryFileSystem>,
    /// The session for resolver state.
    pub session: Arc<Session>,
    /// Resolver for workspace lookups.
    pub resolver: Resolver,
}

impl TestProgram {
    /// Create a new test program with an in memory file system.
    pub(super) fn new(prefix: &str) -> Self {
        // build the test root
        let root = temp_path(prefix);

        // initialize the file system and session
        let fs = Arc::new(MemoryFileSystem::new());
        let session = Arc::new(
            Session::new(root.clone())
                .with_fs(fs.clone())
                .with_cache_store(Arc::new(MemoryCacheStore::new())),
        );

        // create a resolver for workspace lookups
        let resolver = Resolver::from_session(&session, ResolveOptions::default());

        // return the test harness
        Self {
            root,
            fs,
            session,
            resolver,
        }
    }

    /// Build program arguments rooted at this test directory.
    pub(super) fn program_args(&self) -> ProgramArgs {
        // build program args with the in memory file system
        ProgramArgs {
            cwd: Some(self.root.clone()),
            workspace: Some(self.root.clone()),
            fs_override: Some(crate::common::FileSystemOverride::new(self.fs.clone())),
            ..ProgramArgs::default()
        }
    }

    /// Write a file relative to the test root.
    pub(super) fn write_file(&self, relative: &str, contents: &str) {
        // resolve the absolute path
        let path = self.root.join(relative);

        // write the file contents
        write_file(self.fs.as_ref(), &path, contents);
    }

    /// Write a file and return its absolute path.
    pub(super) fn write_source(&self, relative: &str, contents: &str) -> PathBuf {
        // write the file to the test file system
        self.write_file(relative, contents);

        // return the resolved path
        self.root.join(relative)
    }

    /// Write a json file relative to the test root.
    pub(super) fn write_json(&self, relative: &str, value: Value) {
        // serialize the json payload
        let mut content = serde_json::to_string_pretty(&value)
            .unwrap_or_else(|error| panic!("failed to serialize json: {error}"));
        content.push('\n');

        // write the json file
        self.write_file(relative, &content);
    }

    /// Write a dsconfig.json file with the provided json value.
    pub(super) fn write_dsconfig(&self, value: Value) {
        // write the config file
        self.write_json("dsconfig.json", value);
    }

    /// Write a dsconfig.json file merged with base compiler options.
    pub(super) fn write_dsconfig_with_base(&self, extra: Value) {
        // build the merged config payload
        let value = merge_dsconfig_base(extra);

        // write the config file
        self.write_dsconfig(value);
    }

    /// Write a package.json file with the provided json value.
    pub(super) fn write_package_json(&self, value: Value) {
        // write the package json file
        self.write_json("package.json", value);
    }
}

/// Build a unique in memory root path for tests.
fn temp_path(prefix: &str) -> PathBuf {
    // generate a stable unique suffix
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("destack_cli_{prefix}_{nanos}_{counter}");

    // return a deterministic test root
    PathBuf::from("/test").join(name)
}

/// Write a file, creating parent directories as needed.
fn write_file(fs: &dyn FileSystem, path: &Path, contents: &str) {
    // create parent directories
    if let Some(parent) = path.parent() {
        let _ = fs.create_dir_all(parent);
    }

    // write the file contents
    fs.write(path, contents.as_bytes())
        .expect("writing fixture file should succeed");
}

/// Build input args for a single file.
pub(super) fn input_args_from_path(path: PathBuf) -> InputArgs {
    // build the input args payload
    InputArgs {
        files: vec![path],
        ..InputArgs::default()
    }
}

/// Assert a command exits with the expected code.
pub(super) fn assert_exit(code: i32, expected: i32) {
    // validate the exit code
    assert_eq!(code, expected);
}

/// Assert a command exits successfully.
pub(super) fn assert_success(code: i32) {
    // assert command success
    assert_exit(code, 0);
}

/// Build watch loop options for tests.
pub(super) fn watch_loop_options_for_test(watcher: MemoryFileWatcher) -> WatchLoopOptions {
    // configure the memory watcher with a short coalesce window
    WatchLoopOptions {
        watcher: Arc::new(watcher),
        options: build_watch_options(),
        policy: WatchPolicy {
            coalesce_window: std::time::Duration::from_millis(5),
            max_batch_size: 32,
        },
    }
}

/// Merge compiler options into a base dsconfig payload.
pub(super) fn merge_dsconfig_base(extra: Value) -> Value {
    let mut base = json!({
        "compilerOptions": {
            "target": "esnext",
            "module": "esnext",
        },
    });

    if let (Value::Object(base_map), Value::Object(extra_map)) = (&mut base, extra) {
        for (key, value) in extra_map {
            if key == "compilerOptions" {
                if let Value::Object(extra_options) = value {
                    if let Some(Value::Object(base_options)) = base_map.get_mut("compilerOptions") {
                        base_options.extend(extra_options);
                        continue;
                    }
                    base_map.insert(key, Value::Object(extra_options));
                    continue;
                }
            }
            base_map.insert(key, value);
        }
    }

    base
}
