use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{io, thread};

use destack_source::{FileSystem, MemoryFileSystem};
use destack_workspace::Session;

use crate::harness::{TestResult, discover_test_files};

use super::parser::MdTestCase;

/// Per-test timeout in seconds.
pub const TEST_TIMEOUT_SECONDS: u64 = 1;

/// Set up an in-memory test environment from a markdown test case.
pub fn setup_test_environment(
    test: &MdTestCase,
) -> (
    Arc<Session>,
    Arc<destack_workspace::program::Program>,
    PathBuf,
) {
    let memory_fs = Arc::new(MemoryFileSystem::new());
    let cwd = PathBuf::from("/test");

    // populate filesystem with test files
    let mut main_path: Option<PathBuf> = None;
    for file in &test.files {
        let file_path = cwd.join(&file.path);
        memory_fs
            .add_file(&file_path, file.content.as_bytes())
            .expect("failed to add test file");

        if file.path == "main.ds" || main_path.is_none() {
            main_path = Some(file_path);
        }
    }

    let main_path = main_path.expect("test should have at least one file");

    // create session with in-memory filesystem
    let fs: Arc<dyn FileSystem> = memory_fs;
    let session = Arc::new(Session::new(cwd.clone()).with_fs(fs));
    let program = session.add_root(cwd);

    (session, program, main_path)
}

/// Run a test function with a timeout.
/// Returns a failed result if the test times out or panics.
pub fn run_with_timeout<F>(test: MdTestCase, timeout: Duration, f: F) -> TestResult
where
    F: FnOnce(&MdTestCase) -> TestResult + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        // catch panics to prevent thread from hanging during cleanup
        let result = catch_unwind(AssertUnwindSafe(|| f(&test)));

        let test_result = match result {
            Ok(result) => result,
            Err(panic) => {
                let msg = panic
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("unknown panic");

                // if it's a todo!() panic with #Incomplete marker, skip the test
                if msg.contains("#Incomplete") {
                    TestResult::Skipped {
                        reason: msg.to_string(),
                    }
                } else {
                    TestResult::Failed {
                        message: format!("panic: {msg}"),
                    }
                }
            }
        };

        let _ = tx.send(test_result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => TestResult::Failed {
            message: format!(
                "test timed out after {}s (likely deadlock or infinite loop)",
                timeout.as_secs()
            ),
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => TestResult::Failed {
            message: "test thread disconnected unexpectedly".to_string(),
        },
    }
}

/// Discover markdown files recursively in a directory.
/// Skips README files, hidden directories, node_modules, and staging.
pub fn discover_md_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    // direct .md files, excluding README
    let direct_files = discover_test_files(dir, &["md"], "mdtest")?;
    for test in direct_files {
        if test.name.to_lowercase() == "readme" {
            continue;
        }
        files.push(test.path);
    }

    // recurse into subdirectories
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && name != "node_modules" && name != "staging" {
                files.extend(discover_md_files(&path)?);
            }
        }
    }

    Ok(files)
}

/// Convert a test name to a URL-safe slug.
pub fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
