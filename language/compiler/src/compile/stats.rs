use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use destack_source::PackageId;
use parking_lot::Mutex;

use crate::TaskPhase;

/// Per-package statistics.
#[derive(Debug, Default)]
pub struct PackageStats {
    /// Number of modules in this package.
    pub modules: AtomicUsize,
    /// Lines of code in this package.
    pub lines: AtomicUsize,
    /// Total time spent processing this package (nanoseconds).
    pub duration_ns: AtomicU64,
}

/// Per-phase timing statistics.
#[derive(Debug, Default)]
pub struct PhaseStats {
    /// Total time spent in this phase (nanoseconds).
    pub duration_ns: AtomicU64,
    /// Number of tasks processed in this phase.
    pub task_count: AtomicUsize,
}

/// Per-task timing statistics keyed by task name.
#[derive(Debug, Default)]
pub struct TaskNameStats {
    /// Total time spent in this task (nanoseconds).
    pub duration_ns: AtomicU64,
    /// Number of tasks processed for this name.
    pub task_count: AtomicUsize,
}

/// Per timing tag statistics.
#[derive(Debug, Default)]
pub struct TimingStats {
    /// Total time spent in this timing tag (nanoseconds).
    pub duration_ns: AtomicU64,
    /// Number of times this timing tag was recorded.
    pub sample_count: AtomicUsize,
}

/// Task statistics.
#[derive(Debug, Default)]
pub struct TaskStats {
    /// Total tasks enqueued.
    pub enqueued: AtomicUsize,
    /// Tasks completed successfully.
    pub completed: AtomicUsize,
    /// Tasks that failed.
    pub failed: AtomicUsize,
    /// Tasks that yielded (waiting on dependencies).
    pub yielded: AtomicUsize,
    /// Tasks that were skipped.
    pub skipped: AtomicUsize,
}

/// Module statistics.
#[derive(Debug, Default)]
pub struct ModuleStats {
    /// Modules parsed (import phase).
    pub parsed: AtomicUsize,
    /// Modules bound (bind phase).
    pub bound: AtomicUsize,
    /// Modules resolved (resolve phase).
    pub resolved: AtomicUsize,
    /// Modules analyzed (analyze phase).
    pub analyzed: AtomicUsize,
    /// Modules elaborated (elaborate phase).
    pub elaborated: AtomicUsize,
    /// Modules generated (generate phase).
    pub generated: AtomicUsize,
    /// Modules executed (execute phase).
    pub executed: AtomicUsize,
    /// Modules lowered (lower phase).
    pub lowered: AtomicUsize,
    /// Modules verified (verify phase).
    pub verified: AtomicUsize,
    /// Modules optimized (optimize phase).
    pub optimized: AtomicUsize,
    /// Total lines of source code processed.
    pub lines_processed: AtomicUsize,
}

/// MIR optimization statistics.
#[derive(Debug, Default)]
pub struct MirOptimizationStats {
    /// Functions optimized.
    pub functions_optimized: AtomicUsize,
    /// MIR instructions before optimization.
    pub instructions_before: AtomicUsize,
    /// MIR instructions after optimization.
    pub instructions_after: AtomicUsize,
    /// MIR blocks before optimization.
    pub blocks_before: AtomicUsize,
    /// MIR blocks after optimization.
    pub blocks_after: AtomicUsize,
}

/// Cache statistics.
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Cache hits for AST entries from memory.
    pub ast_hits_memory: AtomicUsize,
    /// Cache misses for AST entries.
    pub ast_misses: AtomicUsize,
    /// Cache writes for AST entries to memory.
    pub ast_writes_memory: AtomicUsize,
    /// Cache hits for DIR entries from memory.
    pub dir_hits_memory: AtomicUsize,
    /// Cache misses for DIR entries.
    pub dir_misses: AtomicUsize,
    /// Cache writes for DIR entries to memory.
    pub dir_writes_memory: AtomicUsize,
    /// Cache hits for MIR entries from memory.
    pub mir_hits_memory: AtomicUsize,
    /// Cache misses for MIR entries.
    pub mir_misses: AtomicUsize,
    /// Cache writes for MIR entries to memory.
    pub mir_writes_memory: AtomicUsize,
    /// Cache errors.
    pub errors: AtomicUsize,
}

/// Statistics collected during compilation.
#[derive(Debug)]
pub struct CompilerStats {
    /// When compilation started.
    started_at: Mutex<Option<Instant>>,
    /// Task statistics.
    pub tasks: TaskStats,
    /// Module statistics.
    pub modules: ModuleStats,
    /// MIR optimization statistics.
    pub mir: MirOptimizationStats,
    /// Cache statistics.
    pub cache: CacheStats,
    /// Per-package statistics.
    package_stats: DashMap<PackageId, PackageStats>,
    /// Per-phase timing statistics.
    phase_stats: DashMap<TaskPhase, PhaseStats>,
    /// Per-task timing statistics.
    task_name_stats: DashMap<String, TaskNameStats>,
    /// Per timing tag statistics.
    timing_stats: DashMap<&'static str, TimingStats>,
    /// Whether timing tags are enabled.
    timings_enabled: bool,
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
        Self::new_with_timings(false)
    }

    /// Create a new stats tracker with explicit timing enablement.
    pub fn new_with_timings(timings_enabled: bool) -> Self {
        Self {
            started_at: Mutex::new(None),
            tasks: TaskStats::default(),
            modules: ModuleStats::default(),
            mir: MirOptimizationStats::default(),
            cache: CacheStats::default(),
            package_stats: DashMap::new(),
            phase_stats: DashMap::new(),
            task_name_stats: DashMap::new(),
            timing_stats: DashMap::new(),
            timings_enabled: timings_enabled || timings_enabled_from_env(),
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
        self.tasks.enqueued.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task completing successfully.
    #[inline]
    pub fn record_complete(&self) {
        self.tasks.completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task failing.
    #[inline]
    pub fn record_fail(&self) {
        self.tasks.failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task being skipped.
    #[inline]
    pub fn record_skip(&self) {
        self.tasks.skipped.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a task yielding.
    #[inline]
    pub fn record_yield(&self) {
        self.tasks.yielded.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being parsed.
    #[inline]
    pub fn record_parse(&self) {
        self.modules.parsed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record lines of source code processed for a package.
    #[inline]
    pub fn record_lines(&self, package_id: PackageId, count: usize) {
        self.modules
            .lines_processed
            .fetch_add(count, Ordering::Relaxed);

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

    /// Record time spent processing a package.
    #[inline]
    pub fn record_package_time(&self, package_id: PackageId, duration: Duration) {
        self.package_stats
            .entry(package_id)
            .or_default()
            .duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Record a module being bound.
    #[inline]
    pub fn record_bind(&self) {
        self.modules.bound.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being resolved.
    #[inline]
    pub fn record_resolve(&self) {
        self.modules.resolved.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being analyzed.
    #[inline]
    pub fn record_analyze(&self) {
        self.modules.analyzed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being elaborated.
    #[inline]
    pub fn record_elaborate(&self) {
        self.modules.elaborated.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being generated.
    #[inline]
    pub fn record_generate(&self) {
        self.modules.generated.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being executed (comptime).
    #[inline]
    pub fn record_execute(&self) {
        self.modules.executed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being lowered to MIR.
    #[inline]
    pub fn record_lower(&self) {
        self.modules.lowered.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being verified.
    #[inline]
    pub fn record_verify(&self) {
        self.modules.verified.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a module being optimized.
    #[inline]
    pub fn record_optimize(&self) {
        self.modules.optimized.fetch_add(1, Ordering::Relaxed);
    }

    /// Record MIR optimization metrics for a module.
    #[inline]
    pub fn record_optimize_mir(
        &self,
        functions: usize,
        instructions_before: usize,
        instructions_after: usize,
        blocks_before: usize,
        blocks_after: usize,
    ) {
        self.mir
            .functions_optimized
            .fetch_add(functions, Ordering::Relaxed);
        self.mir
            .instructions_before
            .fetch_add(instructions_before, Ordering::Relaxed);
        self.mir
            .instructions_after
            .fetch_add(instructions_after, Ordering::Relaxed);
        self.mir
            .blocks_before
            .fetch_add(blocks_before, Ordering::Relaxed);
        self.mir
            .blocks_after
            .fetch_add(blocks_after, Ordering::Relaxed);
    }

    /// Record a slow task.
    #[inline]
    pub fn record_slow_task(&self) {
        self.slow_tasks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an AST cache hit from memory.
    #[inline]
    pub fn record_cache_ast_hit_memory(&self) {
        self.cache.ast_hits_memory.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an AST cache miss.
    #[inline]
    pub fn record_cache_ast_miss(&self) {
        self.cache.ast_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an AST cache write to memory.
    #[inline]
    pub fn record_cache_ast_write_memory(&self) {
        self.cache.ast_writes_memory.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a DIR cache hit from memory.
    #[inline]
    pub fn record_cache_dir_hit_memory(&self) {
        self.cache.dir_hits_memory.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a DIR cache miss.
    #[inline]
    pub fn record_cache_dir_miss(&self) {
        self.cache.dir_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a DIR cache write to memory.
    #[inline]
    pub fn record_cache_dir_write_memory(&self) {
        self.cache.dir_writes_memory.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a MIR cache hit from memory.
    #[inline]
    pub fn record_cache_mir_hit_memory(&self) {
        self.cache.mir_hits_memory.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a MIR cache miss.
    #[inline]
    pub fn record_cache_mir_miss(&self) {
        self.cache.mir_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a MIR cache write to memory.
    #[inline]
    pub fn record_cache_mir_write_memory(&self) {
        self.cache.mir_writes_memory.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache error.
    #[inline]
    pub fn record_cache_error(&self) {
        self.cache.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record time spent in a phase.
    #[inline]
    pub fn record_phase_time(&self, phase: TaskPhase, duration: Duration) {
        let entry = self.phase_stats.entry(phase).or_default();
        entry
            .duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
        entry.task_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record time spent in a named task.
    #[inline]
    pub fn record_task_name_time(&self, name: &str, duration: Duration) {
        let entry = self.task_name_stats.entry(name.to_string()).or_default();
        entry
            .duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
        entry.task_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record time spent in a timing tag.
    #[inline]
    pub fn record_timing(&self, name: &'static str, duration: Duration) {
        let entry = self.timing_stats.entry(name).or_default();
        entry
            .duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
        entry.sample_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record time spent in a timing tag with an explicit sample count.
    #[inline]
    pub fn record_timing_samples(
        &self,
        name: &'static str,
        duration: Duration,
        sample_count: usize,
    ) {
        let entry = self.timing_stats.entry(name).or_default();
        entry
            .duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
        entry
            .sample_count
            .fetch_add(sample_count, Ordering::Relaxed);
    }

    /// Check whether timing tags are enabled.
    #[inline]
    pub fn timings_enabled(&self) -> bool {
        self.timings_enabled
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
                let duration_ns = entry.value().duration_ns.load(Ordering::Relaxed);
                PackageStatsSnapshot {
                    package_id,
                    name,
                    modules: entry.value().modules.load(Ordering::Relaxed),
                    lines: entry.value().lines.load(Ordering::Relaxed),
                    duration: Duration::from_nanos(duration_ns),
                }
            })
            .collect();

        // collect per-phase stats, sorted by phase code
        let mut phases: Vec<PhaseStatsSnapshot> = self
            .phase_stats
            .iter()
            .map(|entry| {
                let phase = *entry.key();
                let duration_ns = entry.value().duration_ns.load(Ordering::Relaxed);
                PhaseStatsSnapshot {
                    phase,
                    duration: Duration::from_nanos(duration_ns),
                    task_count: entry.value().task_count.load(Ordering::Relaxed),
                }
            })
            .collect();
        phases.sort_by_key(|p| p.phase.code());

        // collect per-task stats, sorted by duration descending
        let mut task_names: Vec<TaskNameStatsSnapshot> = self
            .task_name_stats
            .iter()
            .map(|entry| {
                let duration_ns = entry.value().duration_ns.load(Ordering::Relaxed);
                TaskNameStatsSnapshot {
                    name: entry.key().clone(),
                    duration: Duration::from_nanos(duration_ns),
                    task_count: entry.value().task_count.load(Ordering::Relaxed),
                }
            })
            .collect();
        task_names.sort_by_key(|entry| std::cmp::Reverse(entry.duration));

        // collect per-timing stats, sorted by duration descending
        let mut timings: Vec<TimingStatsSnapshot> = self
            .timing_stats
            .iter()
            .map(|entry| {
                let duration_ns = entry.value().duration_ns.load(Ordering::Relaxed);
                TimingStatsSnapshot {
                    name: (*entry.key()).to_string(),
                    duration: Duration::from_nanos(duration_ns),
                    sample_count: entry.value().sample_count.load(Ordering::Relaxed),
                }
            })
            .collect();
        timings.sort_by_key(|entry| std::cmp::Reverse(entry.duration));

        StatsSnapshot {
            elapsed: self.elapsed(),
            tasks: TaskStatsSnapshot {
                enqueued: self.tasks.enqueued.load(Ordering::Relaxed),
                completed: self.tasks.completed.load(Ordering::Relaxed),
                failed: self.tasks.failed.load(Ordering::Relaxed),
                yielded: self.tasks.yielded.load(Ordering::Relaxed),
                skipped: self.tasks.skipped.load(Ordering::Relaxed),
            },
            modules: ModuleStatsSnapshot {
                parsed: self.modules.parsed.load(Ordering::Relaxed),
                bound: self.modules.bound.load(Ordering::Relaxed),
                resolved: self.modules.resolved.load(Ordering::Relaxed),
                analyzed: self.modules.analyzed.load(Ordering::Relaxed),
                elaborated: self.modules.elaborated.load(Ordering::Relaxed),
                generated: self.modules.generated.load(Ordering::Relaxed),
                executed: self.modules.executed.load(Ordering::Relaxed),
                lowered: self.modules.lowered.load(Ordering::Relaxed),
                verified: self.modules.verified.load(Ordering::Relaxed),
                optimized: self.modules.optimized.load(Ordering::Relaxed),
                lines_processed: self.modules.lines_processed.load(Ordering::Relaxed),
            },
            cache: CacheStatsSnapshot {
                ast_hits_memory: self.cache.ast_hits_memory.load(Ordering::Relaxed),
                ast_misses: self.cache.ast_misses.load(Ordering::Relaxed),
                ast_writes_memory: self.cache.ast_writes_memory.load(Ordering::Relaxed),
                dir_hits_memory: self.cache.dir_hits_memory.load(Ordering::Relaxed),
                dir_misses: self.cache.dir_misses.load(Ordering::Relaxed),
                dir_writes_memory: self.cache.dir_writes_memory.load(Ordering::Relaxed),
                mir_hits_memory: self.cache.mir_hits_memory.load(Ordering::Relaxed),
                mir_misses: self.cache.mir_misses.load(Ordering::Relaxed),
                mir_writes_memory: self.cache.mir_writes_memory.load(Ordering::Relaxed),
                errors: self.cache.errors.load(Ordering::Relaxed),
            },
            mir: MirOptimizationStatsSnapshot {
                functions_optimized: self.mir.functions_optimized.load(Ordering::Relaxed),
                instructions_before: self.mir.instructions_before.load(Ordering::Relaxed),
                instructions_after: self.mir.instructions_after.load(Ordering::Relaxed),
                blocks_before: self.mir.blocks_before.load(Ordering::Relaxed),
                blocks_after: self.mir.blocks_after.load(Ordering::Relaxed),
            },
            module_count,
            packages,
            phases,
            task_names,
            timings,
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
    /// Total time spent processing this package.
    pub duration: Duration,
}

/// Snapshot of per-phase timing statistics.
#[derive(Debug, Clone)]
pub struct PhaseStatsSnapshot {
    /// The phase.
    pub phase: TaskPhase,
    /// Total time spent in this phase.
    pub duration: Duration,
    /// Number of tasks completed in this phase.
    pub task_count: usize,
}

/// Snapshot of per-task timing statistics.
#[derive(Debug, Clone)]
pub struct TaskNameStatsSnapshot {
    /// The task name.
    pub name: String,
    /// Total time spent in this task.
    pub duration: Duration,
    /// Number of tasks processed for this name.
    pub task_count: usize,
}

/// Snapshot of timing tag statistics.
#[derive(Debug, Clone)]
pub struct TimingStatsSnapshot {
    /// The timing tag name.
    pub name: String,
    /// Total time spent in this timing tag.
    pub duration: Duration,
    /// Number of samples recorded for this timing tag.
    pub sample_count: usize,
}

/// Snapshot of task statistics.
#[derive(Debug, Clone)]
pub struct TaskStatsSnapshot {
    /// Total tasks enqueued.
    pub enqueued: usize,
    /// Tasks completed successfully.
    pub completed: usize,
    /// Tasks that failed.
    pub failed: usize,
    /// Tasks that yielded.
    pub yielded: usize,
    /// Tasks that were skipped.
    pub skipped: usize,
}

/// Snapshot of module statistics.
#[derive(Debug, Clone)]
pub struct ModuleStatsSnapshot {
    /// Modules parsed.
    pub parsed: usize,
    /// Modules bound.
    pub bound: usize,
    /// Modules resolved.
    pub resolved: usize,
    /// Modules analyzed.
    pub analyzed: usize,
    /// Modules elaborated.
    pub elaborated: usize,
    /// Modules generated.
    pub generated: usize,
    /// Modules executed.
    pub executed: usize,
    /// Modules lowered.
    pub lowered: usize,
    /// Modules verified.
    pub verified: usize,
    /// Modules optimized.
    pub optimized: usize,
    /// Total lines of source code processed.
    pub lines_processed: usize,
}

/// Snapshot of cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStatsSnapshot {
    /// Cache hits for AST entries from memory.
    pub ast_hits_memory: usize,
    /// Cache misses for AST entries.
    pub ast_misses: usize,
    /// Cache writes for AST entries to memory.
    pub ast_writes_memory: usize,
    /// Cache hits for DIR entries from memory.
    pub dir_hits_memory: usize,
    /// Cache misses for DIR entries.
    pub dir_misses: usize,
    /// Cache writes for DIR entries to memory.
    pub dir_writes_memory: usize,
    /// Cache hits for MIR entries from memory.
    pub mir_hits_memory: usize,
    /// Cache misses for MIR entries.
    pub mir_misses: usize,
    /// Cache writes for MIR entries to memory.
    pub mir_writes_memory: usize,
    /// Cache errors.
    pub errors: usize,
}

/// Snapshot of MIR optimization statistics.
#[derive(Debug, Clone)]
pub struct MirOptimizationStatsSnapshot {
    /// Functions optimized.
    pub functions_optimized: usize,
    /// MIR instructions before optimization.
    pub instructions_before: usize,
    /// MIR instructions after optimization.
    pub instructions_after: usize,
    /// MIR blocks before optimization.
    pub blocks_before: usize,
    /// MIR blocks after optimization.
    pub blocks_after: usize,
}

/// A point-in-time snapshot of compiler statistics.
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    /// Time elapsed since compilation started.
    pub elapsed: Duration,
    /// Task statistics.
    pub tasks: TaskStatsSnapshot,
    /// Module statistics.
    pub modules: ModuleStatsSnapshot,
    /// Cache statistics.
    pub cache: CacheStatsSnapshot,
    /// MIR statistics.
    pub mir: MirOptimizationStatsSnapshot,
    /// Total modules in program.
    pub module_count: usize,
    /// Per-package statistics.
    pub packages: Vec<PackageStatsSnapshot>,
    /// Per-phase timing statistics.
    pub phases: Vec<PhaseStatsSnapshot>,
    /// Per-task timing statistics.
    pub task_names: Vec<TaskNameStatsSnapshot>,
    /// Per timing tag statistics.
    pub timings: Vec<TimingStatsSnapshot>,
    /// Slow tasks detected.
    pub slow_tasks: usize,
}

impl StatsSnapshot {
    /// Whether compilation had any failures.
    pub fn has_failures(&self) -> bool {
        self.tasks.failed > 0
    }

    /// Total modules processed.
    pub fn modules_processed(&self) -> usize {
        // Use module_count if available, otherwise fall back to tracked counts
        if self.module_count > 0 {
            self.module_count
        } else {
            self.modules.parsed.max(self.modules.analyzed)
        }
    }

    /// Aggregate cache totals across all IR kinds.
    pub fn cache_totals(&self) -> CacheTotals {
        // aggregate totals across cache kinds
        let hits_memory =
            self.cache.ast_hits_memory + self.cache.dir_hits_memory + self.cache.mir_hits_memory;
        let misses = self.cache.ast_misses + self.cache.dir_misses + self.cache.mir_misses;
        let writes_memory = self.cache.ast_writes_memory
            + self.cache.dir_writes_memory
            + self.cache.mir_writes_memory;

        CacheTotals {
            hits_memory,
            misses,
            writes_memory,
            errors: self.cache.errors,
        }
    }

    /// Return the cache hit rate across all IR kinds.
    pub fn cache_hit_rate(&self) -> f32 {
        let totals = self.cache_totals();
        let hits = totals.hits_memory;
        let total = hits + totals.misses;
        if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        }
    }

    /// Return the cache hit rate for AST entries.
    pub fn cache_hit_rate_ast(&self) -> f32 {
        let hits = self.cache.ast_hits_memory;
        let total = hits + self.cache.ast_misses;
        if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        }
    }

    /// Return the cache hit rate for DIR entries.
    pub fn cache_hit_rate_dir(&self) -> f32 {
        let hits = self.cache.dir_hits_memory;
        let total = hits + self.cache.dir_misses;
        if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        }
    }

    /// Return the cache hit rate for MIR entries.
    pub fn cache_hit_rate_mir(&self) -> f32 {
        let hits = self.cache.mir_hits_memory;
        let total = hits + self.cache.mir_misses;
        if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        }
    }
}

fn timings_enabled_from_env() -> bool {
    // honor DESTACK_TIMINGS when set to a truthy value
    std::env::var("DESTACK_TIMINGS")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value > 0)
        .unwrap_or(false)
}

/// Aggregated cache totals.
#[derive(Debug, Clone, Copy)]
pub struct CacheTotals {
    /// Cache hits from memory.
    pub hits_memory: usize,
    /// Cache misses.
    pub misses: usize,
    /// Cache writes to memory.
    pub writes_memory: usize,
    /// Cache errors.
    pub errors: usize,
}
