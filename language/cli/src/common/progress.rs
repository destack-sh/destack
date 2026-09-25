use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};
use tspp_artifact::ArtifactKey;
use tspp_session::{ArtifactRunEvent, ArtifactRunId, SessionEvent, SessionEventHandler};

use crate::console;
use crate::diagnostic::{ConsoleError, ConsoleResult};

const HEADER_TICK_RATE: Duration = Duration::from_millis(100);

/// Key for one active session task.
type ActiveTaskKey = (ArtifactRunId, ArtifactKey);

/// Progress display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressMode {
    /// No progress display.
    #[default]
    None,
    /// Simple spinner with current status.
    Spinner,
    /// Detailed progress with package lines.
    Detailed,
}

/// Tracks an active task.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ActiveTask {
    /// The active artifact key.
    artifact_key: ArtifactKey,
    /// The module path.
    module_path: String,
    /// When the task started.
    started_at: Instant,
}

/// Progress state shared between the handler and the display.
#[derive(Debug, Default)]
pub struct ProgressState {
    /// Number of tasks completed.
    pub tasks_completed: AtomicUsize,
    /// Number of tasks failed.
    pub tasks_failed: AtomicUsize,
    /// Currently active tasks by run and artifact key.
    active_tasks: Mutex<HashMap<ActiveTaskKey, ActiveTask>>,
}

/// Progress reporter that handles compiler events and updates the display.
pub struct ProgressReporter {
    /// The single progress bar (status line at bottom).
    status: ProgressBar,
    state: Arc<ProgressState>,
    detailed: bool,
    label: String,
    started_at: Instant,
    stop_ticker: Arc<AtomicBool>,
}

impl std::fmt::Debug for ProgressReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressReporter")
            .field("detailed", &self.detailed)
            .field("label", &self.label)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl ProgressReporter {
    /// Create a new progress reporter with the given mode.
    pub fn new(mode: ProgressMode) -> ConsoleResult<Option<Self>> {
        Self::with_label(mode, "Working")
    }

    /// Create a new progress reporter with a custom label.
    pub fn with_label(mode: ProgressMode, label: &str) -> ConsoleResult<Option<Self>> {
        match mode {
            ProgressMode::None => Ok(None),
            ProgressMode::Spinner => Self::create(false, label).map(Some),
            ProgressMode::Detailed => Self::create(true, label).map(Some),
        }
    }

    fn create(detailed: bool, label: &str) -> ConsoleResult<Self> {
        let status = ProgressBar::new_spinner();
        status.set_style(Self::spinner_style()?);
        status.enable_steady_tick(Duration::from_millis(80));

        let stop_ticker = Arc::new(AtomicBool::new(false));
        let started_at = Instant::now();
        let state = Arc::new(ProgressState::default());
        let label_text = label.to_string();

        // background thread for periodic updates
        let status_clone = status.clone();
        let ticker_state = state.clone();
        let ticker_stop = stop_ticker.clone();
        let ticker_label = label_text.clone();
        thread::spawn(move || {
            while !ticker_stop.load(Ordering::Relaxed) {
                update_status(
                    &status_clone,
                    &ticker_state,
                    &ticker_label,
                    detailed,
                    started_at,
                );
                thread::sleep(HEADER_TICK_RATE);
            }
        });

        Ok(Self {
            status,
            state,
            detailed,
            label: label_text,
            started_at,
            stop_ticker,
        })
    }

    /// Provide a line writer that prints above the status line.
    pub fn line_writer(&self) -> Arc<dyn Fn(&str) + Send + Sync> {
        let status = self.status.clone();
        Arc::new(move |line: &str| {
            status.suspend(|| {
                eprintln!("{line}");
            });
        })
    }

    /// Create a spinner style.
    fn spinner_style() -> ConsoleResult<ProgressStyle> {
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan}  {wide_msg}")
            .map_err(|error| ConsoleError::message(format!("invalid progress style: {error}")))
    }

    /// Get the event handler to pass to the session.
    pub fn handler(&self) -> SessionEventHandler {
        let state = self.state.clone();
        let status = self.status.clone();
        let detailed = self.detailed;
        let label = self.label.clone();
        let started_at = self.started_at;
        let stop_ticker = self.stop_ticker.clone();

        Arc::new(move |event: SessionEvent| match &event {
            SessionEvent::Run(ArtifactRunEvent::Started { .. }) => {
                update_status(&status, &state, &label, detailed, started_at);
            }
            SessionEvent::Run(ArtifactRunEvent::Required { .. }) => {}
            SessionEvent::TaskStarted {
                run_id,
                artifact_key,
            } => {
                let module_path = module_path_for_artifact(*artifact_key, &state);

                if let Ok(mut active) = state.active_tasks.lock() {
                    let task_key = (*run_id, *artifact_key);

                    active.insert(
                        task_key,
                        ActiveTask {
                            artifact_key: *artifact_key,
                            module_path,
                            started_at: Instant::now(),
                        },
                    );
                }

                update_status(&status, &state, &label, detailed, started_at);
            }
            SessionEvent::TaskFinished {
                run_id,
                artifact_key,
            } => {
                state.tasks_completed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(&(*run_id, *artifact_key));
                }

                update_status(&status, &state, &label, detailed, started_at);
            }
            SessionEvent::TaskFailed {
                run_id,
                artifact_key,
            } => {
                state.tasks_failed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(&(*run_id, *artifact_key));
                }

                update_status(&status, &state, &label, detailed, started_at);
            }
            SessionEvent::Run(ArtifactRunEvent::Finished { .. }) => {
                stop_ticker.store(true, Ordering::Relaxed);
                status.disable_steady_tick();
                status.finish_and_clear();
            }
        })
    }

    /// Update the status line from one workspace progress event.
    pub fn update_workspace(&self, task: &str, message: Option<&str>) {
        // workspace updates share the ticker line's shape
        let styled_label = style_label(&self.label);
        let sep = console::dim(" · ");
        let mut parts = Vec::new();
        if !task.is_empty() && task != self.label {
            parts.push(task.to_string());
        }
        if let Some(message) = message {
            parts.push(message.to_string());
        }
        parts.push(console::format_duration(self.started_at.elapsed()));
        self.status
            .set_message(format!("{styled_label}{sep}{}", parts.join(&sep)));
    }

    /// Stop updating the progress display.
    pub fn stop(&self) {
        self.status.disable_steady_tick();
        self.stop_ticker.store(true, Ordering::Relaxed);
    }

    /// Finish the progress display.
    pub fn finish(&self) {
        self.stop_ticker.store(true, Ordering::Relaxed);
        self.status.disable_steady_tick();
        self.status.finish_and_clear();
    }

    /// Finish with a custom message.
    pub fn finish_with_message(&self, msg: &str) {
        self.status.suspend(|| {
            eprintln!("{msg}");
        });
        self.finish();
    }

    /// Clear the progress display without a message.
    pub fn clear(&self) {
        self.finish();
    }
}

/// Update the status line with current progress.
fn update_status(
    status: &ProgressBar,
    state: &ProgressState,
    label: &str,
    detailed: bool,
    started_at: Instant,
) {
    let elapsed = started_at.elapsed();
    let _ = detailed;

    // get active module for display
    let active_module = get_active_module(state);

    // build status message
    let mut parts = Vec::new();
    let completed = state.tasks_completed.load(Ordering::Relaxed);
    let failed = state.tasks_failed.load(Ordering::Relaxed);
    if completed > 0 {
        parts.push(format!("{} done", format_number(completed)));
    }
    if failed > 0 {
        parts.push(format!("{} failed", format_number(failed)));
    }

    // add elapsed time
    let elapsed_str = console::format_duration(elapsed);
    parts.push(elapsed_str);

    let sep = console::dim(" · ");
    let status_text = if parts.len() == 1 {
        // only elapsed time, still starting
        console::dim("starting")
    } else {
        parts.join(&sep)
    };

    // build final message with optional active module
    let styled_label = style_label(label);
    let message = if let Some(module) = active_module {
        let module_display = console::dim(&module);
        format!("{styled_label}{sep}{status_text} {module_display}")
    } else {
        format!("{styled_label}{sep}{status_text}")
    };
    status.set_message(message);
}

/// Get the most recently started active module for display.
fn get_active_module(state: &ProgressState) -> Option<String> {
    let active = state.active_tasks.lock().ok()?;
    if active.is_empty() {
        return None;
    }

    // find the most recently started task
    let newest = active.values().max_by_key(|task| task.started_at)?;

    // shorten the path for display
    let short_name = module_to_short_name(&newest.module_path);
    Some(short_name)
}

/// Convert a module path to a short display name.
fn module_to_short_name(path: &str) -> String {
    // get just the filename or last path component
    let name = path.rsplit('/').next().unwrap_or(path);

    // strip extension if present
    if let Some(stem) = name
        .strip_suffix(".d.tspp")
        .or_else(|| name.strip_suffix(".tspp"))
    {
        stem.to_string()
    } else {
        name.to_string()
    }
}

/// Format a number with thousands separators.
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Resolve the best display path for one artifact key.
fn module_path_for_artifact(artifact_key: ArtifactKey, state: &ProgressState) -> String {
    let _ = state;

    artifact_key.name().to_string()
}

fn style_label(label: &str) -> String {
    console::style_for_stream(label, &["1", "36"], console::Stream::Stderr)
}

/// Check if stderr is a TTY (terminal).
pub fn is_tty() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stderr())
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        self.finish();
    }
}
