#![allow(dead_code)]

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use futures::executor::block_on;
use serde_json::{Value, json};
use tspp_artifact::BuildId;
use tspp_repository::{
    DestackLayout, DestackLayoutOverride, Edit, Environment, Host, Repository, Revision,
    RevisionPin, Settings,
};
use tspp_source::{File, FileSystem, MemoryFileSystem};

use crate::common::{FileSystemOverride, InputArgs, ProgramArgs};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Test environment for CLI helpers.
#[derive(Debug)]
pub(super) struct TestProgram {
    /// Root directory for the test.
    pub root: PathBuf,
    /// The in memory file system.
    pub fs: Arc<MemoryFileSystem>,
    /// The repository under test.
    pub repository: Arc<Repository>,
    /// Current retained repository revision.
    pub revision: Mutex<RevisionPin>,
}

impl TestProgram {
    /// Create a new test program with an in memory file system.
    pub(super) fn new(prefix: &str) -> Self {
        // build the test root
        let root = temp_path(prefix);

        // initialize the file system and repository
        let fs = Arc::new(MemoryFileSystem::new());
        let environment = Environment::capture_process();
        let layout = DestackLayout::resolve(
            &root,
            &root,
            &environment,
            &Settings::default(),
            &DestackLayoutOverride::default(),
            None,
        );
        let host = Host::new(BuildId::test(), environment, fs.clone());
        let (repository, revision) =
            Repository::new(root.clone(), host, Settings::default(), layout);
        let repository = Arc::new(repository);
        let revision = repository
            .pin(revision)
            .expect("initial CLI test revision should pin");

        Self {
            root,
            fs,
            repository,
            revision: Mutex::new(revision),
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
        self.revision
            .lock()
            .expect("CLI test revision lock should remain available")
            .revision()
    }

    /// Build program arguments rooted at this test directory.
    pub(super) fn program_args(&self) -> ProgramArgs {
        // build program args with the in memory file system
        ProgramArgs {
            cwd: Some(self.root.clone()),
            workspace: Some(self.root.clone()),
            file_system_override: Some(FileSystemOverride::new(self.fs.clone())),
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

        // commit the same contents to semantic state
        let logical_path = self.repository.logical_path(&path);
        let revision = self.current_revision();
        let blob = self
            .repository
            .retain_blob(contents.as_bytes())
            .expect("CLI test Blob should store");
        let revision = self
            .repository
            .edit(revision, [Edit::set_file(logical_path, blob)])
            .unwrap_or_else(|error| {
                panic!(
                    "failed to fork repository for '{}' after write: {error}",
                    path.display()
                )
            })
            .after;

        // retain the new state
        let revision = self
            .repository
            .pin(revision)
            .expect("edited CLI test revision should pin");
        *self
            .revision
            .lock()
            .expect("CLI test revision lock should remain available") = revision;

        path
    }

    /// Return the current revision scoped file for a path.
    pub(super) fn file_for_path(&self, path: &Path) -> Arc<File> {
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

    /// Write a package.json file with the provided json value.
    pub(super) fn write_manifest(&self, value: Value) {
        // write the config file
        self.write_json("package.json", value);
    }

    /// Write a package.json file merged with base compiler options.
    pub(super) fn write_manifest_with_base(&self, extra: Value) {
        // build the merged config payload
        let value = merge_manifest_base(extra);

        // write the config file
        self.write_manifest(value);
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
    let name = format!("tspp_cli_{prefix}_{nanos}_{counter}");

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

/// Execute one asynchronous CLI operation.
pub(super) fn execute<T>(operation: impl Future<Output = T>) -> T {
    block_on(operation)
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

/// Merge compiler options into a base package.json payload.
pub(super) fn merge_manifest_base(extra: Value) -> Value {
    let base = json!({
        "packageManager": "tspp@2026.9.0",
        "name": "test",
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
