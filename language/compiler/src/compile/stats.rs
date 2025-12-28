use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use destack_source::PackageId;
use parking_lot::Mutex;

/// Per-package statistics.
#[derive(Debug, Default)]
pub struct PackageStats {
    /// Number of modules in this package.
    pub modules: AtomicUsize,
    /// Lines of code in this package.
    pub lines: AtomicUsize,
}

/// Statistics collected during compilation.
#[derive(Debug)]
pub struct CompilerStats {
    /// When compilation started.
    started_at: Mutex<Option<Instant>>,

    // Task counts
    /// Total tasks enqueued.
    pub tasks_enqueued: AtomicUsize,
    /// Tasks completed successfully.
    pub tasks_completed: AtomicUsize,
    /// Tasks that failed.
    pub tasks_failed: AtomicUsize,
    /// Tasks that yielded (waiting on dependencies).
    pub tasks_yielded: AtomicUsize,

    // Module counts per pass
    /// Modules parsed (import phase).
    pub modules_parsed: AtomicUsize,
    /// Modules bound (bind phase).
    pub modules_bound: AtomicUsize,
    /// Modules resolved (resolve phase).
    pub modules_resolved: AtomicUsize,
    /// Modules analyzed (analyze phase).
    pub modules_analyzed: AtomicUsize,
    /// Modules elaborated (elaborate phase).
    pub modules_elaborated: AtomicUsize,
    /// Modules generated (generate phase).
    pub modules_generated: AtomicUsize,
    /// Modules linted (lint phase).
    pub modules_linted: AtomicUsize,

    // Size metrics
    /// Total lines of source code processed.
    pub lines_processed: AtomicUsize,
    /// Per-package statistics.
    package_stats: DashMap<PackageId, PackageStats>,

    // Slow task tracking
    /// Number of slow tasks detected.
    pub slow_tasks: AtomicUsize,
}

impl Default for CompilerStats {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerStats {
    /// Create a new stats tracker.
    pub fn new() -> Self {
        Self {
            started_at: Mutex::new(None),
            tasks_enqueued: AtomicUsize::new(0),
            tasks_completed: AtomicUsize::new(0),
            tasks_failed: AtomicUsize::new(0),
            tasks_yielded: AtomicUsize::new(0),
            modules_parsed: AtomicUsize::new(0),
            modules_bound: AtomicUsize::new(0),
            modules_resolved: AtomicUsize::new(0),
            modules_analyzed: AtomicUsize::new(0),
            modules_elaborated: AtomicUsize::new(0),
            modules_generated: AtomicUsize::new(0),
            modules_linted: AtomicUsize::new(0),
            lines_processed: AtomicUsize::new(0),
            package_stats: DashMap::new(),
            slow_tasks: AtomicUsize::new(0),
        }
    }

    /// Mark compilation as started.
    pub fn start(&self) {
        *self.started_at.lock() = Some(Instant::now());
    }

    /// Get elapsed time since start.
    pub fn elapsed(&self) -> Duration {
        self.started_at
            .lock()
            .map(|start| start.elapsed())
            .unwrap_or_default()
    }

    /// Record a task being enqueued.
    #[inline]
    pub fn record_enqueue(&self) {
        self.tasks_enqueued.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task completing successfully.
    #[inline]
    pub fn record_complete(&self) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task failing.
    #[inline]
    pub fn record_fail(&self) {
        self.tasks_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task yielding.
    #[inline]
    pub fn record_yield(&self) {
        self.tasks_yielded.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being parsed.
    #[inline]
    pub fn record_parse(&self) {
        self.modules_parsed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record lines of source code processed for a package.
    #[inline]
    pub fn record_lines(&self, package_id: PackageId, count: usize) {
        self.lines_processed.fetch_add(count, Ordering::Relaxed);

        // update per-package stats
        self.package_stats
            .entry(package_id)
            .or_default()
            .lines
            .fetch_add(count, Ordering::Relaxed);
    }

    /// Record a module parsed for a package.
    #[inline]
    pub fn record_module_for_package(&self, package_id: PackageId) {
        self.package_stats
            .entry(package_id)
            .or_default()
            .modules
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being bound.
    #[inline]
    pub fn record_bind(&self) {
        self.modules_bound.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being resolved.
    #[inline]
    pub fn record_resolve(&self) {
        self.modules_resolved.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being analyzed.
    #[inline]
    pub fn record_analyze(&self) {
        self.modules_analyzed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being elaborated.
    #[inline]
    pub fn record_elaborate(&self) {
        self.modules_elaborated.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being generated.
    #[inline]
    pub fn record_generate(&self) {
        self.modules_generated.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being linted.
    #[inline]
    pub fn record_lint(&self) {
        self.modules_linted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a slow task.
    #[inline]
    pub fn record_slow_task(&self) {
        self.slow_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of current statistics.
    pub fn snapshot(&self) -> StatsSnapshot {
        self.snapshot_with_modules(0)
    }

    /// Get a snapshot with an explicit module count (from program.modules.len()).
    pub fn snapshot_with_modules(&self, module_count: usize) -> StatsSnapshot {
        self.snapshot_with_program(module_count, None)
    }

    /// Get a snapshot with module count and optional program for package name resolution.
    pub fn snapshot_with_program(
        &self,
        module_count: usize,
        program: Option<&destack_workspace::Program>,
    ) -> StatsSnapshot {
        // collect per-package stats
        let packages: Vec<PackageStatsSnapshot> = self
            .package_stats
            .iter()
            .map(|entry| {
                let package_id = *entry.key();
                let name = program.and_then(|p| {
                    let pkg = p.packages.get(package_id);
                    let pkg = pkg.read();
                    pkg.name.clone()
                });
                PackageStatsSnapshot {
                    package_id,
                    name,
                    modules: entry.value().modules.load(Ordering::Relaxed),
                    lines: entry.value().lines.load(Ordering::Relaxed),
                }
            })
            .collect();

        StatsSnapshot {
            elapsed: self.elapsed(),
            tasks_enqueued: self.tasks_enqueued.load(Ordering::Relaxed),
            tasks_completed: self.tasks_completed.load(Ordering::Relaxed),
            tasks_failed: self.tasks_failed.load(Ordering::Relaxed),
            tasks_yielded: self.tasks_yielded.load(Ordering::Relaxed),
            modules_parsed: self.modules_parsed.load(Ordering::Relaxed),
            modules_bound: self.modules_bound.load(Ordering::Relaxed),
            modules_resolved: self.modules_resolved.load(Ordering::Relaxed),
            modules_analyzed: self.modules_analyzed.load(Ordering::Relaxed),
            modules_elaborated: self.modules_elaborated.load(Ordering::Relaxed),
            modules_generated: self.modules_generated.load(Ordering::Relaxed),
            modules_linted: self.modules_linted.load(Ordering::Relaxed),
            module_count,
            lines_processed: self.lines_processed.load(Ordering::Relaxed),
            packages,
            slow_tasks: self.slow_tasks.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of per-package statistics.
#[derive(Debug, Clone)]
pub struct PackageStatsSnapshot {
    /// Package id.
    pub package_id: PackageId,
    /// Package name (if available).
    pub name: Option<String>,
    /// Number of modules in this package.
    pub modules: usize,
    /// Lines of code in this package.
    pub lines: usize,
}

/// A point-in-time snapshot of compiler statistics.
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    /// Time elapsed since compilation started.
    pub elapsed: Duration,
    /// Total tasks enqueued.
    pub tasks_enqueued: usize,
    /// Tasks completed successfully.
    pub tasks_completed: usize,
    /// Tasks that failed.
    pub tasks_failed: usize,
    /// Tasks that yielded.
    pub tasks_yielded: usize,
    /// Modules parsed.
    pub modules_parsed: usize,
    /// Modules bound.
    pub modules_bound: usize,
    /// Modules resolved.
    pub modules_resolved: usize,
    /// Modules analyzed.
    pub modules_analyzed: usize,
    /// Modules elaborated.
    pub modules_elaborated: usize,
    /// Modules generated.
    pub modules_generated: usize,
    /// Modules linted.
    pub modules_linted: usize,
    /// Total modules in program.
    pub module_count: usize,
    /// Total lines of source code processed.
    pub lines_processed: usize,
    /// Per-package statistics.
    pub packages: Vec<PackageStatsSnapshot>,
    /// Slow tasks detected.
    pub slow_tasks: usize,
}

impl StatsSnapshot {
    /// Whether compilation had any failures.
    pub fn has_failures(&self) -> bool {
        self.tasks_failed > 0
    }

    /// Total modules processed.
    pub fn modules_processed(&self) -> usize {
        // Use module_count if available, otherwise fall back to tracked counts
        if self.module_count > 0 {
            self.module_count
        } else {
            self.modules_parsed.max(self.modules_analyzed)
        }
    }
}

impl std::fmt::Display for StatsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let modules = self.modules_processed();
        let elapsed_secs = self.elapsed.as_secs_f64();
        let module_word = if modules == 1 { "module" } else { "modules" };

        if self.tasks_failed > 0 {
            write!(
                f,
                "Checked {modules} {module_word} ({} failed) in {elapsed_secs:.2}s",
                self.tasks_failed
            )
        } else {
            write!(f, "Checked {modules} {module_word} in {elapsed_secs:.2}s")
        }
    }
}
