#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::os::unix::fs::PermissionsExt;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::sync::{Mutex, OnceLock};
#[cfg(feature = "execution")]
use std::{panic, thread};

#[cfg(feature = "execution")]
use crate::host::apple::message as apple_host_message;
#[cfg(feature = "execution")]
use crate::tests::registry as execution_registry;

/// Environment marker for subprocess execution dispatch.
#[cfg(target_os = "macos")]
const EXECUTION_CHILD_ENV: &str = "DESTACK_RUNTIME_EXECUTION_CHILD";
/// Environment key for one explicit execution helper path.
#[cfg(target_os = "macos")]
const EXECUTION_HELPER_ENV: &str = "DESTACK_RUNTIME_EXECUTION_HELPER";

/// Process-wide gate that serializes helper child processes.
#[cfg(target_os = "macos")]
static EXECUTION_CHILD_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Run one execution-sensitive test case through one child helper process when needed.
#[cfg(target_os = "macos")]
pub(crate) fn run_execution_case_or_return(case_name: &str) -> bool {
    // the child process already owns the correct thread
    if std::env::var_os(EXECUTION_CHILD_ENV).is_some() {
        return false;
    }

    // serialize helper children because some host services, like the macOS pasteboard,
    // are process-global and become flaky when multiple execution cases race them
    let _guard = EXECUTION_CHILD_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // resolve the helper path from the outer runner
    let helper = execution_helper_executable();
    let mut command = Command::new(helper);
    command
        .arg("--case")
        .arg(case_name)
        .env(EXECUTION_CHILD_ENV, "1");

    let status = command.status().unwrap_or_else(|error| {
        panic!("failed to launch execution helper for {case_name}: {error}")
    });

    assert!(
        status.success(),
        "execution helper failed for {case_name} with status {status}",
    );

    true
}

/// Return whether the current execution case was delegated to one helper process.
#[cfg(not(target_os = "macos"))]
pub(crate) fn run_execution_case_or_return(_case_name: &str) -> bool {
    false
}

/// Run one registered execution-sensitive test case on the correct process thread.
#[cfg(feature = "execution")]
pub fn run_execution_case(case_name: &str) {
    let case_name = case_name.to_string();
    run_with_apple_main_thread_service(move || {
        let handled = execution_registry::run_execution_case(case_name.as_str());

        assert!(handled, "unknown execution case: {case_name}");
    });
}

/// Resolve the runtime execution helper from one explicit runner-provided path.
#[cfg(target_os = "macos")]
fn execution_helper_executable() -> PathBuf {
    // prefer the exact helper path provided by the outer runner
    if let Some(path) = std::env::var_os(EXECUTION_HELPER_ENV) {
        let path = PathBuf::from(path);
        if path_is_executable_file(&path) {
            return path;
        }

        panic!(
            "{EXECUTION_HELPER_ENV} points to one missing runtime execution helper: {}",
            path.display()
        );
    }

    // allow direct cargo wiring when it is available
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_runtime_execution") {
        let path = PathBuf::from(path);
        if path_is_executable_file(&path) {
            return path;
        }

        panic!(
            "CARGO_BIN_EXE_runtime_execution points to one missing runtime execution helper: {}",
            path.display()
        );
    }

    // fall back to the current target deps directory when cargo test runs directly
    if let Some(path) = discover_execution_helper_beside_current_executable() {
        return path;
    }

    panic!(
        "missing runtime execution helper path: set {EXECUTION_HELPER_ENV} or provide CARGO_BIN_EXE_runtime_execution"
    );
}

/// Discover the helper executable beside the current test binary.
#[cfg(target_os = "macos")]
fn discover_execution_helper_beside_current_executable() -> Option<PathBuf> {
    let current_executable = std::env::current_exe().ok()?;
    let current_name = current_executable.file_name()?.to_os_string();
    let directory = current_executable.parent()?;
    let entries = fs::read_dir(directory).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name() else {
            continue;
        };

        if name == current_name {
            continue;
        }

        let Some(name) = name.to_str() else {
            continue;
        };

        if !name.starts_with("runtime_execution-") {
            continue;
        }

        if !path_is_executable_file(&path) {
            continue;
        }

        return Some(path);
    }

    None
}

/// Return whether one path points to one executable file.
#[cfg(target_os = "macos")]
fn path_is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };

    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

/// Run one Apple-constrained case while the main thread continuously services the run loop.
#[cfg(feature = "execution")]
pub(crate) fn run_with_apple_main_thread_service(run: impl FnOnce() + Send + 'static) {
    let worker = thread::spawn(run);

    // keep the process main thread free to service AppKit callbacks
    apple_host_message::service_registered_runtimes_until(true, || worker.is_finished()).unwrap();

    // propagate the worker result after the main-thread service loop exits
    if let Err(payload) = worker.join() {
        panic::resume_unwind(payload);
    }
}
