#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(feature = "affinity")]
use std::{panic, thread};

#[cfg(feature = "affinity")]
use crate::host::apple::message as apple_host_message;
#[cfg(feature = "affinity")]
use crate::tests::registry as affinity_registry;

/// Environment marker for subprocess affinity execution.
#[cfg(target_os = "macos")]
const AFFINITY_CHILD_ENV: &str = "DESTACK_RUNTIME_AFFINITY_CHILD";
/// Environment key for one explicit affinity helper executable path.
#[cfg(target_os = "macos")]
const AFFINITY_HELPER_ENV: &str = "DESTACK_RUNTIME_AFFINITY_HELPER";

/// Run one main-thread-sensitive test case through one child helper process when needed.
#[cfg(target_os = "macos")]
pub(crate) fn run_main_thread_case_or_return(case_name: &str) -> bool {
    // the child process already owns the correct thread
    if std::env::var_os(AFFINITY_CHILD_ENV).is_some() {
        return false;
    }

    // resolve the helper path from the outer runner
    let helper = affinity_helper_executable();
    let mut command = Command::new(helper);
    command
        .arg("--case")
        .arg(case_name)
        .env(AFFINITY_CHILD_ENV, "1");

    let status = command.status().unwrap_or_else(|error| {
        panic!("failed to launch affinity helper for {case_name}: {error}")
    });

    assert!(
        status.success(),
        "affinity helper failed for {case_name} with status {status}",
    );

    true
}

/// Run one registered affinity-sensitive test case on the correct process thread.
#[cfg(feature = "affinity")]
pub fn run_affinity_case(case_name: &str) {
    let case_name = case_name.to_string();
    run_with_apple_main_thread_service(move || {
        let handled = affinity_registry::run_affinity_case(case_name.as_str());

        assert!(handled, "unknown affinity case: {case_name}");
    });
}

/// Resolve the runtime affinity helper from one explicit runner-provided path.
#[cfg(target_os = "macos")]
fn affinity_helper_executable() -> PathBuf {
    // prefer the exact helper path provided by the outer test runner
    if let Some(path) = std::env::var_os(AFFINITY_HELPER_ENV) {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }

        panic!(
            "{AFFINITY_HELPER_ENV} points to one missing runtime affinity helper: {}",
            path.display()
        );
    }

    // allow direct cargo wiring when it is available
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_runtime_affinity") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }

        panic!(
            "CARGO_BIN_EXE_runtime_affinity points to one missing runtime affinity helper: {}",
            path.display()
        );
    }

    panic!(
        "missing runtime affinity helper path: set {AFFINITY_HELPER_ENV} or provide CARGO_BIN_EXE_runtime_affinity"
    );
}

/// Run one Apple-affine case while the main thread continuously services the run loop.
#[cfg(feature = "affinity")]
pub(crate) fn run_with_apple_main_thread_service(run: impl FnOnce() + Send + 'static) {
    let worker = thread::spawn(run);

    // keep the process main thread free to service AppKit callbacks
    apple_host_message::service_registered_runtimes_until(true, || worker.is_finished()).unwrap();

    // propagate the worker result after the main-thread service loop exits
    if let Err(payload) = worker.join() {
        panic::resume_unwind(payload);
    }
}
