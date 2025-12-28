use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use destack_compiler::{CompilerEvent, CompilerEventHandler, CompilerStats, TaskId, TaskPhase};
use destack_workspace::Program;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::console;

const MAX_VISIBLE_TASKS: usize = 3;
const MAX_VISIBLE_TASKS_DETAILED: usize = 6;
const HEADER_TICK_RATE: Duration = Duration::from_millis(120);

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
    bar: Option<ProgressBar>,
    started_at: Instant,
}

#[derive(Clone, Debug)]
struct StatsSource {
    stats: Arc<CompilerStats>,
    program: Option<Arc<Program>>,
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
    /// Optional stats source for incremental progress.
    stats_source: Mutex<Option<StatsSource>>,
}

/// Progress reporter that handles compiler events and updates the display.
pub struct ProgressReporter {
    multi: MultiProgress,
    state: Arc<ProgressState>,
    detailed: bool,
    label: String,
    header: ProgressBar,
    max_visible_tasks: usize,
    started_at: Instant,
    stop_ticker: Arc<AtomicBool>,
}

impl std::fmt::Debug for ProgressReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressReporter")
            .field("detailed", &self.detailed)
            .field("label", &self.label)
            .field("max_visible_tasks", &self.max_visible_tasks)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl ProgressReporter {
    /// Create a new progress reporter with the given mode.
    pub fn new(mode: ProgressMode) -> Option<Self> {
        Self::with_label(mode, "Working")
    }

    /// Create a new progress reporter with a custom label.
    pub fn with_label(mode: ProgressMode, label: &str) -> Option<Self> {
        match mode {
            ProgressMode::None => None,
            ProgressMode::Spinner => Some(Self::create(false, label)),
            ProgressMode::Detailed => Some(Self::create(true, label)),
        }
    }

    fn create(detailed: bool, label: &str) -> Self {
        let multi = MultiProgress::new();
        let header = multi.add(ProgressBar::new_spinner());
        header.set_style(Self::spinner_style());
        header.enable_steady_tick(Duration::from_millis(80));

        let stop_ticker = Arc::new(AtomicBool::new(false));
        let started_at = Instant::now();
        let max_visible_tasks = if detailed {
            MAX_VISIBLE_TASKS_DETAILED
        } else {
            MAX_VISIBLE_TASKS
        };

        let state = Arc::new(ProgressState::default());
        let label_text = label.to_string();
        let header_clone = header.clone();
        let ticker_state = state.clone();
        let ticker_stop = stop_ticker.clone();
        let ticker_label = label_text.clone();
        thread::spawn(move || {
            while !ticker_stop.load(Ordering::Relaxed) {
                update_header(
                    &header_clone,
                    &ticker_state,
                    &ticker_label,
                    detailed,
                    started_at,
                );
                thread::sleep(HEADER_TICK_RATE);
            }
        });

        Self {
            multi,
            state,
            detailed,
            label: label_text,
            header,
            max_visible_tasks,
            started_at,
            stop_ticker,
        }
    }

    /// Attach compiler stats for incremental progress reporting.
    pub fn set_stats_source(&self, stats: Arc<CompilerStats>, program: Option<Arc<Program>>) {
        if let Ok(mut source) = self.state.stats_source.lock() {
            *source = Some(StatsSource { stats, program });
        }
    }

    /// Provide a line writer that keeps progress output visible.
    pub fn line_writer(&self) -> Arc<dyn Fn(&str) + Send + Sync> {
        let multi = self.multi.clone();
        Arc::new(move |line: &str| {
            let _ = multi.println(line.to_string());
        })
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
        let header = self.header.clone();
        let label = self.label.clone();
        let max_visible_tasks = self.max_visible_tasks;
        let started_at = self.started_at;
        let stop_ticker = self.stop_ticker.clone();

        Arc::new(move |event: CompilerEvent| match &event {
            CompilerEvent::CompilationStarted { .. } => {
                update_header(&header, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskStarted {
                task_id,
                phase,
                description,
                ..
            } => {
                let module_path = extract_module_path(description);
                let display_name = path_to_display(&module_path);
                let message = format_task_message(*phase, &display_name);

                if let Ok(mut active) = state.active_tasks.lock() {
                    let visible_count = active.values().filter(|task| task.bar.is_some()).count();
                    let bar = if visible_count < max_visible_tasks {
                        let bar = multi.add(ProgressBar::new_spinner());
                        bar.set_style(Self::spinner_style());
                        bar.enable_steady_tick(Duration::from_millis(80));
                        bar.set_message(message);
                        Some(bar)
                    } else {
                        None
                    };
                    active.insert(
                        *task_id,
                        ActiveTask {
                            phase: *phase,
                            module_path,
                            bar,
                            started_at: Instant::now(),
                        },
                    );
                }

                update_header(&header, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskCompleted { task_id, .. } => {
                state.tasks_completed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock()
                    && let Some(mut task) = active.remove(task_id)
                {
                    if let Some(bar) = task.bar.take() {
                        bar.finish_and_clear();
                    }
                    promote_hidden_tasks(&mut active, &multi, max_visible_tasks);
                }

                update_header(&header, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskFailed { task_id, .. } => {
                state.tasks_failed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock()
                    && let Some(mut task) = active.remove(task_id)
                {
                    if let Some(bar) = task.bar.take() {
                        bar.finish_and_clear();
                    }
                    promote_hidden_tasks(&mut active, &multi, max_visible_tasks);
                }

                update_header(&header, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskYielded { task_id, .. } => {
                // task yielded, remove its progress bar temporarily
                if let Ok(mut active) = state.active_tasks.lock()
                    && let Some(mut task) = active.remove(task_id)
                {
                    if let Some(bar) = task.bar.take() {
                        bar.finish_and_clear();
                    }
                    promote_hidden_tasks(&mut active, &multi, max_visible_tasks);
                }

                update_header(&header, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskSlow {
                phase,
                elapsed,
                description,
                ..
            } => {
                let path = extract_module_path(description);
                let name = path_to_display(&path);
                let elapsed_str = console::format_duration(*elapsed);
                let elapsed_label = console::dim(&elapsed_str);
                let label = console::yellow("slow");
                multi
                    .println(format!(
                        "    {elapsed_label} {label} {} {}",
                        phase_to_verb(*phase),
                        name
                    ))
                    .ok();
            }
            CompilerEvent::CompilationFinished { .. } => {
                // clear all remaining task bars
                if let Ok(mut active) = state.active_tasks.lock() {
                    for (_, task) in active.drain() {
                        if let Some(bar) = task.bar {
                            bar.finish_and_clear();
                        }
                    }
                }
                update_header(&header, &state, &label, detailed, started_at);
                header.disable_steady_tick();
                stop_ticker.store(true, Ordering::Relaxed);
            }
        })
    }

    /// Finish the progress display.
    pub fn finish(&self) {
        if let Ok(mut active) = self.state.active_tasks.lock() {
            for (_, task) in active.drain() {
                if let Some(bar) = task.bar {
                    bar.finish_and_clear();
                }
            }
        }
        self.header.finish_and_clear();
        self.stop_ticker.store(true, Ordering::Relaxed);
    }

    /// Finish with a custom message.
    pub fn finish_with_message(&self, msg: &str) {
        // print the message through the multi progress
        self.multi.println(msg).ok();
        self.finish();
    }

    /// Clear the progress display without a message.
    pub fn clear(&self) {
        self.finish();
    }
}

fn update_header(
    header: &ProgressBar,
    state: &ProgressState,
    label: &str,
    detailed: bool,
    started_at: Instant,
) {
    let completed = state.tasks_completed.load(Ordering::Relaxed);
    let failed = state.tasks_failed.load(Ordering::Relaxed);
    let (active_count, visible_count) = if let Ok(active) = state.active_tasks.lock() {
        let visible = active.values().filter(|task| task.bar.is_some()).count();
        (active.len(), visible)
    } else {
        (0, 0)
    };
    let elapsed = started_at.elapsed();
    let progress_stats = read_progress_stats(state);

    let mut parts = Vec::new();
    if let Some(stats) = progress_stats {
        if stats.packages > 0 {
            parts.push(format!("{} packages", format_number(stats.packages)));
        }
        if stats.modules > 0 {
            parts.push(format!("{} modules", format_number(stats.modules)));
        }
        if stats.lines > 0 {
            let lines = format_compact(stats.lines);
            let lines_per_second = if elapsed.as_secs_f64() > 0.2 {
                let throughput = stats.lines as f64 / elapsed.as_secs_f64();
                Some(format_compact(throughput.round() as usize))
            } else {
                None
            };
            if let Some(lines_per_second) = lines_per_second {
                parts.push(format!("{lines} lines ({lines_per_second}/s)"));
            } else {
                parts.push(format!("{lines} lines"));
            }
        }
    }

    let task_status = format_task_status(completed, failed, active_count, visible_count, detailed);
    if let Some(task_status) = task_status {
        parts.push(task_status);
    }

    let sep = console::dim(" · ");
    let status = if parts.is_empty() {
        console::dim("starting")
    } else {
        parts.join(&sep)
    };
    let label = style_label(label);
    header.set_message(format!("{label} {status}"));
}

fn promote_hidden_tasks(
    active: &mut HashMap<TaskId, ActiveTask>,
    multi: &MultiProgress,
    max_visible_tasks: usize,
) {
    let visible_count = active.values().filter(|task| task.bar.is_some()).count();
    if visible_count >= max_visible_tasks {
        return;
    }
    let mut hidden_tasks: Vec<(TaskId, Instant)> = active
        .iter()
        .filter(|(_, task)| task.bar.is_none())
        .map(|(task_id, task)| (*task_id, task.started_at))
        .collect();
    hidden_tasks.sort_by(|a, b| b.1.cmp(&a.1));

    let mut remaining = max_visible_tasks - visible_count;
    for (task_id, _) in hidden_tasks {
        if remaining == 0 {
            break;
        }
        if let Some(task) = active.get_mut(&task_id) {
            let display_name = path_to_display(&task.module_path);
            let message = format_task_message(task.phase, &display_name);
            let bar = multi.add(ProgressBar::new_spinner());
            bar.set_style(ProgressReporter::spinner_style());
            bar.enable_steady_tick(Duration::from_millis(80));
            bar.set_message(message);
            task.bar = Some(bar);
            remaining -= 1;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ProgressStats {
    packages: usize,
    modules: usize,
    lines: usize,
}

fn read_progress_stats(state: &ProgressState) -> Option<ProgressStats> {
    let source = if let Ok(source) = state.stats_source.lock() {
        source.clone()
    } else {
        None
    }?;
    let module_count = source
        .program
        .as_ref()
        .map(|program| program.modules.len())
        .unwrap_or(0);
    let snapshot = source
        .stats
        .snapshot_with_program(module_count, source.program.as_deref());

    let mut packages = 0;
    let mut modules = 0;
    let mut lines = 0;
    for package in &snapshot.packages {
        if package.lines == 0 {
            continue;
        }
        if let Some(name) = package.name.as_deref()
            && name.starts_with('<')
        {
            continue;
        }
        packages += 1;
        modules += package.modules;
        lines += package.lines;
    }

    Some(ProgressStats {
        packages,
        modules,
        lines,
    })
}

fn format_task_status(
    completed: usize,
    failed: usize,
    active: usize,
    visible: usize,
    detailed: bool,
) -> Option<String> {
    if completed == 0 && failed == 0 && active == 0 {
        return None;
    }

    let mut parts = Vec::new();
    if completed > 0 {
        parts.push(console::green(&format!("{completed} completed")));
    }
    if failed > 0 {
        parts.push(console::red(&format!("{failed} failed")));
    }
    if active > 0 {
        let hidden = active.saturating_sub(visible);
        if detailed && hidden > 0 {
            parts.push(format!("{active} active (+{hidden} more)"));
        } else {
            parts.push(format!("{active} active"));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("tasks {}", parts.join(" ")))
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

/// Format a number in compact form (e.g., 1.2k, 3.5M).
fn format_compact(n: usize) -> String {
    if n >= 1_000_000 {
        let m = n as f64 / 1_000_000.0;
        if m >= 10.0 {
            format!("{m:.0}M")
        } else {
            format!("{m:.1}M")
        }
    } else if n >= 1_000 {
        let k = n as f64 / 1_000.0;
        if k >= 10.0 {
            format!("{k:.0}k")
        } else {
            format!("{k:.1}k")
        }
    } else {
        n.to_string()
    }
}

/// Extract module path from trace description like "module=path/to/file.ds profile=xyz".
fn extract_module_path(description: &str) -> String {
    // look for module= or just use the whole thing
    if let Some(rest) = description.strip_prefix("module=") {
        // take until next space or end
        rest.split_whitespace().next().unwrap_or(rest).to_string()
    } else if description.contains('=') {
        // other format, try to extract path like values
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

    const PATH_HEAD_COMPONENTS: usize = 2;
    const PATH_TAIL_COMPONENTS: usize = 4;

    if parts.len() <= PATH_HEAD_COMPONENTS + PATH_TAIL_COMPONENTS {
        // short path, show as is
        path.to_string()
    } else {
        // long path, show head and tail components
        let head = parts[..PATH_HEAD_COMPONENTS].join("/");
        let tail = parts[parts.len() - PATH_TAIL_COMPONENTS..].join("/");
        format!("{head}/.../{tail}")
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

fn format_task_message(phase: TaskPhase, display_name: &str) -> String {
    let verb = phase_to_verb(phase);
    format!("{verb:<12} {display_name}")
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
        self.stop_ticker.store(true, Ordering::Relaxed);
    }
}
