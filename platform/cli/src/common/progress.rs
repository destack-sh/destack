use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use destack_compiler::{CompilerEvent, CompilerEventHandler, CompilerStats, TaskId, TaskPhase};
use destack_workspace::Program;
use indicatif::{ProgressBar, ProgressStyle};

use crate::console;

const HEADER_TICK_RATE: Duration = Duration::from_millis(100);

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
    /// The phase of the task.
    phase: TaskPhase,
    /// The module path.
    module_path: String,
    /// When the task started.
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
    /// Packages that have been printed (to avoid duplicates).
    printed_packages: Mutex<HashSet<String>>,
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
        let status = ProgressBar::new_spinner();
        status.set_style(Self::spinner_style());
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

        Self {
            status,
            state,
            detailed,
            label: label_text,
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
    fn spinner_style() -> ProgressStyle {
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan}  {wide_msg}")
            .unwrap()
    }

    /// Get the event handler to pass to the compiler.
    pub fn handler(&self) -> CompilerEventHandler {
        let state = self.state.clone();
        let status = self.status.clone();
        let detailed = self.detailed;
        let label = self.label.clone();
        let started_at = self.started_at;
        let stop_ticker = self.stop_ticker.clone();

        Arc::new(move |event: CompilerEvent| match &event {
            CompilerEvent::CompilationStarted { .. } => {
                update_status(&status, &state, &label, detailed, started_at);
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
                            started_at: Instant::now(),
                        },
                    );
                }

                update_status(&status, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskCompleted { task_id, .. } => {
                state.tasks_completed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(task_id);
                }

                update_status(&status, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskFailed { task_id, .. } => {
                state.tasks_failed.fetch_add(1, Ordering::Relaxed);

                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(task_id);
                }

                update_status(&status, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskYielded { task_id, .. } => {
                if let Ok(mut active) = state.active_tasks.lock() {
                    active.remove(task_id);
                }

                update_status(&status, &state, &label, detailed, started_at);
            }
            CompilerEvent::TaskSlow {
                phase,
                elapsed,
                description,
                ..
            } => {
                // print slow task warning as permanent line
                let path = extract_module_path(description);
                let name = path_to_display(&path);
                let elapsed_str = console::format_duration(*elapsed);
                let warn_label = console::yellow("slow");
                let phase_verb = phase_to_verb(*phase).to_lowercase();
                status.suspend(|| {
                    eprintln!(
                        "    {warn_label} {phase_verb} {} ({})",
                        console::dim(&name),
                        console::dim(&elapsed_str)
                    );
                });
            }
            CompilerEvent::CompilationFinished { .. } => {
                status.disable_steady_tick();
                stop_ticker.store(true, Ordering::Relaxed);
            }
        })
    }

    /// Finish the progress display.
    pub fn finish(&self) {
        self.status.finish_and_clear();
        self.stop_ticker.store(true, Ordering::Relaxed);
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
    let progress_stats = read_progress_stats(state);

    // in detailed mode, print newly completed packages
    if detailed && let Some(ref stats) = progress_stats {
        print_new_packages(status, state, stats, label);
    }

    // get active module for display
    let active_module = get_active_module(state);

    // build status message
    let mut parts = Vec::new();
    if let Some(stats) = progress_stats {
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
            if let Some(lps) = lines_per_second {
                parts.push(format!("{lines} lines ({lps}/s)"));
            } else {
                parts.push(format!("{lines} lines"));
            }
        }
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
        format!("{styled_label} {status_text} {module_display}")
    } else {
        format!("{styled_label} {status_text}")
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
        .strip_suffix(".ds")
        .or_else(|| name.strip_suffix(".ts"))
        .or_else(|| name.strip_suffix(".d.ts"))
    {
        stem.to_string()
    } else {
        name.to_string()
    }
}

/// Print newly completed packages as permanent lines.
fn print_new_packages(
    status: &ProgressBar,
    state: &ProgressState,
    stats: &ProgressStats,
    label: &str,
) {
    let mut printed = match state.printed_packages.lock() {
        Ok(p) => p,
        Err(_) => return,
    };

    for pkg in &stats.packages {
        // skip if already printed or no duration (not started)
        if printed.contains(&pkg.name) || pkg.duration.as_nanos() == 0 {
            continue;
        }

        // print package line
        let duration_str = console::format_duration(pkg.duration);
        let lines_str = format_compact(pkg.lines);
        let throughput = if pkg.duration.as_secs_f64() > 0.001 {
            let lps = pkg.lines as f64 / pkg.duration.as_secs_f64();
            format!(" ({}/s)", format_compact(lps.round() as usize))
        } else {
            String::new()
        };

        let modules_word = if pkg.modules == 1 {
            "module"
        } else {
            "modules"
        };
        status.suspend(|| {
            eprintln!(
                "    {} {} {} · {} {} · {} lines{}",
                console::cyan(label),
                pkg.name,
                console::dim(&duration_str),
                pkg.modules,
                modules_word,
                lines_str,
                throughput
            );
        });

        printed.insert(pkg.name.clone());
    }
}

#[derive(Debug, Clone)]
struct PackageInfo {
    name: String,
    modules: usize,
    lines: usize,
    duration: Duration,
}

#[derive(Debug, Clone)]
struct ProgressStats {
    packages: Vec<PackageInfo>,
    modules: usize,
    lines: usize,
}

fn read_progress_stats(state: &ProgressState) -> Option<ProgressStats> {
    let source = state.stats_source.lock().ok()?.clone()?;
    let module_count = source
        .program
        .as_ref()
        .map(|program| program.modules.len())
        .unwrap_or(0);
    let snapshot = source
        .stats
        .snapshot_with_program(module_count, source.program.as_deref());

    let mut packages = Vec::new();
    let mut total_modules = 0;
    let mut total_lines = 0;

    for package in &snapshot.packages {
        // skip internal packages
        if let Some(name) = package.name.as_deref() {
            if name.starts_with('<') {
                continue;
            }
        }
        if package.lines == 0 {
            continue;
        }

        let name = package.name.clone().unwrap_or_default();
        packages.push(PackageInfo {
            name,
            modules: package.modules,
            lines: package.lines,
            duration: package.duration,
        });
        total_modules += package.modules;
        total_lines += package.lines;
    }

    Some(ProgressStats {
        packages,
        modules: total_modules,
        lines: total_lines,
    })
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
    if let Some(rest) = description.strip_prefix("module=") {
        rest.split_whitespace().next().unwrap_or(rest).to_string()
    } else if description.contains('=') {
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
        path.to_string()
    } else {
        let head = parts[..PATH_HEAD_COMPONENTS].join("/");
        let tail = parts[parts.len() - PATH_TAIL_COMPONENTS..].join("/");
        format!("{head}/.../{tail}")
    }
}

fn phase_to_verb(phase: TaskPhase) -> &'static str {
    match phase {
        TaskPhase::Import => "Parsing",
        TaskPhase::Resolve => "Resolving",
        TaskPhase::Analyze => "Checking",
        TaskPhase::Elaborate => "Elaborating",
        TaskPhase::Execute => "Executing",
        TaskPhase::Lower => "Lowering",
        TaskPhase::Optimize => "Optimizing",
        TaskPhase::Generate => "Generating",
        TaskPhase::Link => "Linking",
        TaskPhase::Emit => "Writing",
        TaskPhase::Lint => "Linting",
    }
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
