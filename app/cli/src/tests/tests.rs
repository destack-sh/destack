#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use destack_artifact::MemoryCacheStore;
use destack_daemon::WatchPolicy;
use destack_resolver::{Resolver, ResolverOptions};
use destack_source::{FileSystem, MemoryFileSystem, MemoryFileWatcher};
use destack_workspace::{Edit, HostEnvironment, Ref, Repository, Revision};
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
    /// The repository for resolver state.
    pub repository: Arc<Repository>,
    /// Resolver for workspace lookups.
    pub resolver: Resolver,
}

impl TestProgram {
    /// Create a new test program with an in memory file system.
    pub(super) fn new(prefix: &str) -> Self {
        // build the test root
        let root = temp_path(prefix);

        // initialize the file system and repository
        let fs = Arc::new(MemoryFileSystem::new());
        let repository = Arc::new(Repository::new(
            root.clone(),
            Arc::new(MemoryCacheStore::new()),
            fs.clone(),
            HostEnvironment::capture_process(),
        ));

        // create a resolver for workspace lookups
        let resolver = Resolver::from_repository(repository.clone(), ResolverOptions::default());

        // return the test harness
        Self {
            root,
            fs,
            repository,
            resolver,
        }
    }

    /// Return the primary test root directory.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a file path relative to the test root.
    pub(super) fn path_for(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Return the current workspace revision.
    pub(super) fn current_revision(&self) -> Revision {
        let reference = Ref::for_workspace_root(&self.root);
        self.repository
            .current(&reference)
            .expect("expected current workspace revision")
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
        let _ = self.write_text(relative, contents);
    }

    /// Write text and return the absolute path.
    pub(super) fn write_text(&self, relative: &str, contents: &str) -> PathBuf {
        // resolve the absolute path
        let path = self.path_for(relative);

        // write the file contents
        write_file(self.fs.as_ref(), &path, contents);

        let logical_path = self.repository.logical_path(&path);
        let reference = Ref::for_workspace_root(&self.root);

        // current repository state
        let revision = self.repository.current(&reference).unwrap_or_else(|error| {
            panic!(
                "failed to read repository revision for '{}' after write: {error}",
                path.display()
            )
        });

        // edited repository state
        let revision = self
            .repository
            .fork_with_edits(revision, [Edit::set_text(logical_path, contents)])
            .unwrap_or_else(|error| {
                panic!(
                    "failed to fork repository for '{}' after write: {error}",
                    path.display()
                )
            });

        // publish the new state
        self.repository
            .set_ref(&reference, revision)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to publish repository for '{}' after write: {error}",
                    path.display()
                )
            });

        path
    }

    /// Return the current revision scoped file for a path.
    pub(super) fn file_for_path(&self, path: &Path) -> Arc<destack_source::File> {
        let file_id = self.repository.file_id(path);
        self.repository
            .file(self.current_revision(), file_id)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to read file '{}' from revision: {error}",
                    path.display()
                )
            })
            .unwrap_or_else(|| panic!("missing file for {}", path.display()))
    }

    /// Return whether the current revision still contains a path.
    pub(super) fn has_file_for_path(&self, path: &Path) -> bool {
        let file_id = self.repository.file_id(path);

        self.repository
            .file(self.current_revision(), file_id)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to read file '{}' from revision: {error}",
                    path.display()
                )
            })
            .is_some()
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

    /// Write a destack.json file with the provided json value.
    pub(super) fn write_destack_config(&self, value: Value) {
        // write the config file
        self.write_json("destack.json", value);
    }

    /// Write a destack.json file merged with base compiler options.
    pub(super) fn write_destack_config_with_base(&self, extra: Value) {
        // build the merged config payload
        let value = merge_destack_config_base(extra);

        // write the config file
        self.write_destack_config(value);
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

/// Merge compiler options into a base destack.json payload.
pub(super) fn merge_destack_config_base(extra: Value) -> Value {
    let base = json!({
        "compiler": {
            "target": "esnext",
            "module": "esnext",
        },
    });

    // merge compiler options without clobbering defaults
    merge_json_object(base, extra, &["compiler"])
}

/// Merge JSON objects with selective deep merge keys.
fn merge_json_object(mut base: Value, extra: Value, merge_keys: &[&str]) -> Value {
    // return early when merge inputs are not objects
    let (Value::Object(base_map), Value::Object(extra_map)) = (&mut base, extra) else {
        return base;
    };

    // merge extra keys into the base map
    for (key, value) in extra_map {
        if merge_keys.contains(&key.as_str())
            && let Value::Object(extra_options) = value
        {
            if let Some(Value::Object(base_options)) = base_map.get_mut(&key) {
                base_options.extend(extra_options);
                continue;
            }
            base_map.insert(key, Value::Object(extra_options));
            continue;
        }
        base_map.insert(key, value);
    }

    base
}
