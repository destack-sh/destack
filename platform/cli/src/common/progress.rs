use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use destack_compiler::{CompilerEvent, CompilerEventHandler, TaskId, TaskPhase};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Progress display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressMode {
    /// No progress display.
    #[default]
    None,
    /// Simple spinner with current task.
    Spinner,
    /// Detailed progress with task counts.
    Detailed,
}

/// Tracks an active task for progress display.
#[derive(Debug, Clone)]
struct ActiveTask {
    phase: TaskPhase,
    module_path: String,
    bar: ProgressBar,
}

/// Progress state shared between the handler and the display.
#[derive(Debug, Default)]
pub struct ProgressState {
    /// Number of tasks completed.
    pub tasks_completed: AtomicUsize,
    /// Number of tasks failed.
    pub tasks_failed: AtomicUsize,
    /// Currently active tasks by task_id.
    active_tasks: Mutex<HashMap<TaskId, ActiveTask>>,
}

/// Progress reporter that handles compiler events and updates the display.
pub struct ProgressReporter {
    multi: MultiProgress,
    state: Arc<ProgressState>,
    detailed: bool,
}

impl std::fmt::Debug for ProgressReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressReporter")
            .field("detailed", &self.detailed)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl ProgressReporter {
    /// Create a new progress reporter with the given mode.
    pub fn new(mode: ProgressMode) -> Option<Self> {
        match mode {
            ProgressMode::None => None,
            ProgressMode::Spinner => Some(Self::create(false)),
            ProgressMode::Detailed => Some(Self::create(true)),
        }
    }

    fn create(detailed: bool) -> Self {
        let multi = MultiProgress::new();

        Self {
            multi,
            state: Arc::new(ProgressState::default()),
            detailed,
        }
    }

    /// Create a spinner style for tasks.
    fn spinner_style() -> ProgressStyle {
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {elapsed:.dim} {wide_msg}")
            .unwrap()
    }

    /// Get the event handler to pass to the compiler.
    pub fn handler(&self) -> CompilerEventHandler {
        let state = self.state.clone();
        let multi = self.multi.clone();
        let detailed = self.detailed;

        Arc::new(move |event: CompilerEvent| match &event {
            CompilerEvent::CompilationStarted { .. } => {
                // no-op: we'll create bars as tasks start
            }
            CompilerEvent::TaskStarted {
                task_id,
                phase,
                description,
                ..
            } => {
                let module_path = extract_module_path(description);
                let display_name = path_to_display(&module_path);
                let verb = phase_to_verb(*phase);

                // create a new progress bar for this task
                let bar = multi.add(ProgressBar::new_spinner());
                bar.set_style(Self::spinner_style());
                bar.enable_steady_tick(Duration::from_millis(80));

                let completed = state.tasks_completed.load(Ordering::Relaxed);
                let message = if detailed {
                    format!("[{completed}] {verb} {display_name}")
                } else {
                    format!("{verb} {display_name}")
                };
                bar.set_message(message);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.insert(
                        *task_id,
                        ActiveTask {
                            phase: *phase,
                            module_path,
                            bar,
                        },
                    );
                }
            }
            CompilerEvent::TaskCompleted { task_id, .. } => {
                state.tasks_completed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    if let Some(task) = active.remove(task_id) {
                        task.bar.finish_and_clear();
                    }
                }

                // update remaining task messages with new completion count
                update_all_messages(&state, detailed);
            }
            CompilerEvent::TaskFailed { task_id, .. } => {
                state.tasks_failed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    if let Some(task) = active.remove(task_id) {
                        task.bar.finish_and_clear();
                    }
                }

                update_all_messages(&state, detailed);
            }
            CompilerEvent::TaskYielded { task_id, .. } => {
                // task yielded, remove its progress bar temporarily
                if let Ok(mut active) = state.active_tasks.lock() {
                    if let Some(task) = active.remove(task_id) {
                        task.bar.finish_and_clear();
                    }
                }
            }
            CompilerEvent::TaskSlow {
                phase,
                elapsed,
                description,
                ..
            } => {
                if detailed {
                    let path = extract_module_path(description);
                    let name = path_to_display(&path);
                    let elapsed_str = format_duration(*elapsed);
                    multi
                        .println(format!(
                            "    \x1b[33m{} {} is slow ({})\x1b[0m",
                            phase_to_verb(*phase),
                            name,
                            elapsed_str
                        ))
                        .ok();
                }
            }
            CompilerEvent::CompilationFinished { .. } => {
                // clear all remaining bars
                if let Ok(mut active) = state.active_tasks.lock() {
                    for (_, task) in active.drain() {
                        task.bar.finish_and_clear();
                    }
                }
            }
        })
    }

    /// Finish the progress display.
    pub fn finish(&self) {
        if let Ok(mut active) = self.state.active_tasks.lock() {
            for (_, task) in active.drain() {
                task.bar.finish_and_clear();
            }
        }
    }

    /// Finish with a custom message.
    pub fn finish_with_message(&self, msg: &str) {
        // print the message through the multi-progress
        self.multi.println(msg).ok();
        self.finish();
    }

    /// Clear the progress display without a message.
    pub fn clear(&self) {
        self.finish();
    }
}

/// Update all active task messages with the current completion count.
fn update_all_messages(state: &ProgressState, detailed: bool) {
    let completed = state.tasks_completed.load(Ordering::Relaxed);

    if let Ok(active) = state.active_tasks.lock() {
        for task in active.values() {
            let display_name = path_to_display(&task.module_path);
            let verb = phase_to_verb(task.phase);

            let message = if detailed {
                format!("[{completed}] {verb} {display_name}")
            } else {
                format!("{verb} {display_name}")
            };
            task.bar.set_message(message);
        }
    }
}

/// Format a duration in a human-friendly way.
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if total_secs == 0 {
        let secs_f = duration.as_secs_f64();
        if secs_f < 0.01 {
            format!("{secs_f:.3}s")
        } else if secs_f < 0.1 {
            format!("{secs_f:.2}s")
        } else {
            format!("{secs_f:.1}s")
        }
    } else if total_secs < 60 {
        let secs_f = duration.as_secs_f64();
        if millis == 0 {
            format!("{total_secs}s")
        } else {
            format!("{secs_f:.1}s")
        }
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{mins}m {secs}s")
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        format!("{hours}h {mins}m")
    }
}

/// Extract module path from trace description like "module=path/to/file.ds profile=xyz".
fn extract_module_path(description: &str) -> String {
    // look for module= or just use the whole thing
    if let Some(rest) = description.strip_prefix("module=") {
        // take until next space or end
        rest.split_whitespace().next().unwrap_or(rest).to_string()
    } else if description.contains('=') {
        // other format, try to extract path-like thing
        description
            .split_whitespace()
            .find(|s| s.contains('/') || s.ends_with(".ds") || s.ends_with(".ts"))
            .unwrap_or(description)
            .to_string()
    } else {
        description.to_string()
    }
}

/// Convert a path to a display name, shortening if needed.
fn path_to_display(path: &str) -> String {
    let parts: Vec<_> = path.split('/').collect();

    if parts.len() <= 4 {
        // short-ish path: show as-is
        path.to_string()
    } else {
        // long path: show last 3 components with leading ...
        let tail: Vec<_> = parts.iter().rev().take(3).collect();
        let suffix = tail
            .into_iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("/");
        format!(".../{suffix}")
    }
}

/// Get just the file name from a path.
#[allow(dead_code)]
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn phase_to_verb(phase: TaskPhase) -> &'static str {
    match phase {
        TaskPhase::Import => "Parsing",
        TaskPhase::Bind => "Binding",
        TaskPhase::Resolve => "Resolving",
        TaskPhase::Analyze => "Checking",
        TaskPhase::Elaborate => "Elaborating",
        TaskPhase::Lower => "Lowering",
        TaskPhase::Verify => "Verifying",
        TaskPhase::Execute => "Executing",
        TaskPhase::Optimize => "Optimizing",
        TaskPhase::Generate => "Generating",
        TaskPhase::Link => "Linking",
        TaskPhase::Emit => "Writing",
        TaskPhase::Lint => "Linting",
    }
}

/// Check if stdout is a TTY (terminal).
pub fn is_tty() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stdout())
}
