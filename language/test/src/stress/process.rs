use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::core::CaseResult;

use super::StressCase;

/// Child stress worker kind.
#[derive(Debug, Clone, Copy)]
pub(super) enum StressWorker {
    /// Parser stress worker.
    Parser,
    /// Formatter stress worker.
    Formatter,
}

impl StressWorker {
    /// Return the hidden command flag for this worker.
    fn flag(self) -> &'static str {
        match self {
            Self::Parser => "--run-parser-case",
            Self::Formatter => "--run-formatter-case",
        }
    }
}

/// Run one stress case in a subprocess.
pub(super) fn run_stress_child(
    worker: StressWorker,
    test: &StressCase,
    timeout: Duration,
) -> CaseResult {
    let mut child = match Command::new(current_executable())
        .arg(worker.flag())
        .arg(&test.path)
        .stdout(Stdio::null())
        .stderr(stress_log(test))
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return CaseResult::Failed {
                message: format!("failed to spawn stress worker for {}: {error}", test.name),
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
                return CaseResult::Failed {
                    message: format!("failed to poll stress worker for {}: {error}", test.name),
                };
            }
        }

        // kill timed out worker
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();

            return CaseResult::Failed {
                message: format!("timeout after {timeout:?}: {}", test.path.display()),
            };
        }

        thread::sleep(Duration::from_millis(10));
    }

    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            return CaseResult::Failed {
                message: format!("failed to collect stress worker for {}: {error}", test.name),
            };
        }
    };

    let stderr = match fs::read_to_string(stress_log_path(test)) {
        Ok(stderr) => stderr,
        Err(error) => format!("failed to read stress worker log: {error}"),
    };

    if status.success() {
        if !stderr.is_empty() {
            eprintln!("  {}", tail_output(&stderr));
        }

        return CaseResult::Passed;
    }

    CaseResult::Failed {
        message: format!(
            "{} failed with {}\npath: {}\n\nstderr:\n{}",
            test.name,
            status,
            test.path.display(),
            tail_output(&stderr),
        ),
    }
}

/// Return the current test executable path.
fn current_executable() -> PathBuf {
    std::env::current_exe().expect("stress test executable is unavailable")
}

/// Open the stderr log for one stress child.
fn stress_log(test: &StressCase) -> Stdio {
    File::create(stress_log_path(test))
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null())
}

/// Return the stderr log path for one stress child.
fn stress_log_path(test: &StressCase) -> PathBuf {
    test.path.with_extension("stress.log")
}

/// Return the final lines from one child output.
fn tail_output(output: &str) -> String {
    const MAX_OUTPUT_LINES: usize = 160;

    let lines = output.lines().collect::<Vec<_>>();
    if lines.len() <= MAX_OUTPUT_LINES {
        return output.to_string();
    }

    let start = lines.len() - MAX_OUTPUT_LINES;
    format!("...\n{}", lines[start..].join("\n"))
}
