use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use destack_compiler::{CompilerEvent, CompilerEventHandler, TaskId, TaskPhase};
use indicatif::{ProgressBar, ProgressStyle};

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
    bar: ProgressBar,
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
        let bar = ProgressBar::new_spinner();

        // cargo-style: green status word, then message
        let style = ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {wide_msg}")
            .unwrap();

        bar.set_style(style);
        bar.enable_steady_tick(Duration::from_millis(80));

        Self {
            bar,
            state: Arc::new(ProgressState::default()),
            detailed,
        }
    }

    /// Get the event handler to pass to the compiler.
    pub fn handler(&self) -> CompilerEventHandler {
        let state = self.state.clone();
        let bar = self.bar.clone();
        let detailed = self.detailed;

        Arc::new(move |event: CompilerEvent| match &event {
            CompilerEvent::CompilationStarted { .. } => {
                bar.set_message("Starting...");
            }
            CompilerEvent::TaskStarted {
                task_id,
                phase,
                description,
                ..
            } => {
                let module_path = extract_module_path(description);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.insert(
                        *task_id,
                        ActiveTask {
                            phase: *phase,
                            module_path,
                        },
                    );
                }

                update_display(&bar, &state, detailed);
            }
            CompilerEvent::TaskCompleted { task_id, .. } => {
                state.tasks_completed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(task_id);
                }

                update_display(&bar, &state, detailed);
            }
            CompilerEvent::TaskFailed { task_id, .. } => {
                state.tasks_failed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(task_id);
                }

                update_display(&bar, &state, detailed);
            }
            CompilerEvent::TaskYielded { task_id, .. } => {
                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(task_id);
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
                    bar.println(format!(
                        "    {} {} is slow ({:.1}s)",
                        phase_to_verb(*phase),
                        name,
                        elapsed.as_secs_f64()
                    ));
                }
            }
            CompilerEvent::CompilationFinished { .. } => {
                bar.finish_and_clear();
            }
        })
    }

    /// Finish the progress display.
    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }

    /// Finish with a custom message.
    pub fn finish_with_message(&self, msg: &str) {
        self.bar.finish_with_message(msg.to_string());
    }

    /// Clear the progress display without a message.
    pub fn clear(&self) {
        self.bar.finish_and_clear();
    }
}

/// Update the progress bar display based on active tasks.
fn update_display(bar: &ProgressBar, state: &ProgressState, detailed: bool) {
    let Ok(active) = state.active_tasks.lock() else {
        return;
    };

    if active.is_empty() {
        bar.set_message("Waiting...");
        return;
    }

    // group by phase
    let mut by_phase: HashMap<TaskPhase, Vec<&str>> = HashMap::new();
    for task in active.values() {
        by_phase
            .entry(task.phase)
            .or_default()
            .push(&task.module_path);
    }

    let completed = state.tasks_completed.load(Ordering::Relaxed);

    // format the message
    let message = if by_phase.len() == 1 {
        let (phase, paths) = by_phase.iter().next().unwrap();
        let verb = phase_to_verb(*phase);
        format_phase_message(verb, paths, completed, detailed)
    } else {
        // multiple phases: summarize
        let total: usize = by_phase.values().map(|v| v.len()).sum();
        let verbs: Vec<_> = by_phase.keys().map(|p| phase_to_verb(*p)).collect();
        if detailed {
            format!("[{}] {} tasks ({} done)", verbs.join("/"), total, completed)
        } else {
            format!("{} tasks", total)
        }
    };

    bar.set_message(message);
}

/// Format message for a single phase with its modules.
fn format_phase_message(verb: &str, paths: &[&str], completed: usize, detailed: bool) -> String {
    let count = paths.len();

    if count == 1 {
        // single module: "Checking src/main.ds"
        let name = path_to_display(paths[0]);
        if detailed {
            format!("[{completed}] {verb} {name}")
        } else {
            format!("{verb} {name}")
        }
    } else if count <= 3 {
        // few modules: "Checking main.ds, util.ds"
        let names: Vec<_> = paths.iter().map(|p| file_name(p)).collect();
        if detailed {
            format!("[{completed}] {verb} {}", names.join(", "))
        } else {
            format!("{verb} {}", names.join(", "))
        }
    } else {
        // many modules: "Checking 5 modules"
        if detailed {
            format!("[{completed}] {verb} {count} modules")
        } else {
            format!("{verb} {count} modules")
        }
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

    if parts.len() <= 3 {
        // short path: show as-is
        path.to_string()
    } else {
        // long path: show "dir/.../file.ds"
        let first = parts.first().unwrap_or(&"");
        let last = parts.last().unwrap_or(&"");
        format!("{first}/.../{last}")
    }
}

/// Get just the file name from a path.
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
