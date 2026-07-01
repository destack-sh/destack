use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::core::CaseResult;

use super::StressFixture;
use super::file::unique_sibling_path;
use super::target::StressTarget;

/// Run one stress fixture in a worker subprocess.
pub(super) fn run_stress_worker(
    target: StressTarget,
    fixture: &StressFixture,
    timeout: Duration,
) -> CaseResult {
    let executable = match current_executable() {
        Ok(executable) => executable,
        Err(message) => return CaseResult::Failed { message },
    };

    let log_path = match stress_log_path(fixture) {
        Ok(path) => path,
        Err(message) => return CaseResult::Failed { message },
    };

    let stderr_log = match stress_log(&log_path) {
        Ok(stderr_log) => stderr_log,
        Err(message) => return CaseResult::Failed { message },
    };

    let mut child = match Command::new(executable)
        .arg(target.worker_flag())
        .arg(&fixture.path)
        .stdout(Stdio::null())
        .stderr(stderr_log)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            remove_stress_log(&log_path);

            return CaseResult::Failed {
                message: format!(
                    "failed to spawn stress worker for {}: {error}",
                    fixture.name
                ),
            };
        }
    };

    let start = Instant::now();
    loop {
        // check worker completion
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {}
            Err(error) => {
                remove_stress_log(&log_path);

                return CaseResult::Failed {
                    message: format!("failed to poll stress worker for {}: {error}", fixture.name),
                };
            }
        }

        // kill timed out worker
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            remove_stress_log(&log_path);

            return CaseResult::Failed {
                message: format!("timeout after {timeout:?}: {}", fixture.path.display()),
            };
        }

        thread::sleep(Duration::from_millis(10));
    }

    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            remove_stress_log(&log_path);

            return CaseResult::Failed {
                message: format!(
                    "failed to collect stress worker for {}: {error}",
                    fixture.name
                ),
            };
        }
    };

    let stderr = match fs::read_to_string(&log_path) {
        Ok(stderr) => stderr,
        Err(error) => format!("failed to read stress worker log: {error}"),
    };
    remove_stress_log(&log_path);

    if status.success() {
        if !stderr.is_empty() {
            eprintln!("  {}", tail_output(&stderr));
        }

        return CaseResult::Passed;
    }

    CaseResult::Failed {
        message: format!(
            "{} failed with {}\npath: {}\n\nstderr:\n{}",
            fixture.name,
            status,
            fixture.path.display(),
            tail_output(&stderr),
        ),
    }
}

/// Return the current test executable path.
fn current_executable() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("stress test executable is unavailable: {error}"))
}

/// Open the stderr log for one stress worker.
fn stress_log(path: &Path) -> Result<Stdio, String> {
    File::create(path).map(Stdio::from).map_err(|error| {
        format!(
            "failed to create stress worker log {}: {error}",
            path.display()
        )
    })
}

/// Return the stderr log path for one stress worker.
fn stress_log_path(fixture: &StressFixture) -> Result<PathBuf, String> {
    unique_sibling_path(&fixture.path, "stress.log")
}

/// Remove one worker stderr log after its process has finished.
fn remove_stress_log(path: &Path) {
    let _ = fs::remove_file(path);
}

/// Return the final lines from one worker output.
fn tail_output(output: &str) -> String {
    const MAX_OUTPUT_LINES: usize = 160;

    let lines = output.lines().collect::<Vec<_>>();
    if lines.len() <= MAX_OUTPUT_LINES {
        return output.to_string();
    }

    let start = lines.len() - MAX_OUTPUT_LINES;
    format!("...\n{}", lines[start..].join("\n"))
}
