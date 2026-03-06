use std::path::PathBuf;
use std::process::Command;
#[cfg(target_os = "macos")]
use std::{panic, thread};

#[cfg(target_os = "macos")]
use crate::host::apple::message as apple_host_message;
use crate::platform::display::tests::affinity as display_affinity_tests;

/// Environment marker for subprocess affinity execution.
const AFFINITY_CHILD_ENV: &str = "DESTACK_RUNTIME_AFFINITY_CHILD";

/// Run one main-thread-sensitive test case through one child helper process when needed.
pub(crate) fn run_main_thread_case_or_return(
    case_name: &str,
    explicit_helper_path: Option<&str>,
) -> bool {
    // the child process already owns the correct thread
    if std::env::var_os(AFFINITY_CHILD_ENV).is_some() {
        return false;
    }

    let helper = affinity_helper_executable(explicit_helper_path);
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

/// Run one affinity-sensitive display case on the correct process thread.
pub(crate) fn run_display_main_thread_case(case_name: &str) {
    #[cfg(target_os = "macos")]
    {
        let case_name = case_name.to_string();
        run_with_apple_main_thread_service(move || {
            display_affinity_tests::run_case(case_name.as_str());
        });
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        display_affinity_tests::run_case(case_name);
    }
}

/// Resolve the harness-free affinity helper executable.
fn affinity_helper_executable(explicit_helper_path: Option<&str>) -> PathBuf {
    // prefer the exact helper path passed from the current test binary
    if let Some(path) = explicit_helper_path {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }

    // cargo may also expose the helper path at runtime for some layouts
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_runtime_affinity") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }

    panic!(
        "failed to resolve runtime affinity helper: expected one explicit helper path or CARGO_BIN_EXE_runtime_affinity"
    );
}

/// Run one Apple-affine case while the main thread continuously services the run loop.
#[cfg(target_os = "macos")]
pub(crate) fn run_with_apple_main_thread_service(run: impl FnOnce() + Send + 'static) {
    let worker = thread::spawn(run);

    // keep the process main thread free to service AppKit callbacks
    apple_host_message::service_registered_runtimes_until(true, || worker.is_finished());

    // propagate the worker result after the main-thread service loop exits
    if let Err(payload) = worker.join() {
        panic::resume_unwind(payload);
    }
}
