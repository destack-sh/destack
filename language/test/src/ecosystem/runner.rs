use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Mutex;

use destack_core::StableHasher;
use destack_source::{FileType, glob, matches as glob_matches};

use crate::core::print::color;
use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Runner, Suite, fixtures_dir, load_expected_failures,
    save_expected_failures,
};

use super::manifest::{
    EcosystemManifest, EcosystemPhase, EcosystemSupportTier, EcosystemTscMode, EcosystemTscTool,
};

use super::{fetch, tier};

const DEFAULT_INCLUDE_PATTERNS: &[&str] = &["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"];
const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &["**/node_modules/**", "**/dist/**", "**/build/**"];

const README_SECTION_SUMMARY_RESULTS: &str = "summary-results";
const PREPARE_STAMP_FILE: &str = ".destack_prepare_stamp";

/// Configuration options for running ecosystem tests.
#[derive(Debug, Clone, Default)]
pub struct EcosystemRunOptions {
    /// Phase tiers to run.
    /// Empty means parse only.
    pub phases: Vec<EcosystemPhase>,
    /// Override include patterns for all phases.
    pub include: Vec<String>,
    /// Additional exclude patterns for all phases.
    pub exclude: Vec<String>,
    /// Override max discovered files per package and phase.
    pub max_files: Option<usize>,
    /// Control tsc execution strategy for selected packages and phases.
    pub tsc_mode: EcosystemTscMode,
    /// Select tsc binary strategy.
    pub tsc_tool: EcosystemTscTool,
    /// Print loaded module and line totals in the phase summary table.
    pub read_stats: bool,
    /// Print discovered source file paths for each package phase run.
    pub dump_read_paths: bool,
}

impl EcosystemRunOptions {
    /// Return selected phases in stable execution order.
    fn selected_phases(&self) -> Vec<EcosystemPhase> {
        if self.phases.is_empty() {
            return vec![EcosystemPhase::Parse];
        }

        EcosystemPhase::all()
            .iter()
            .copied()
            .filter(|phase| self.phases.contains(phase))
            .collect()
    }
}

/// Configuration options for fetching ecosystem packages.
#[derive(Debug, Clone, Copy, Default)]
pub struct FetchOptions {
    /// Whether to re-clone packages even if they already exist.
    pub refresh: bool,
    /// Whether to install package dependencies after fetch.
    pub install: bool,
}

/// Read stats captured for one package phase case.
#[derive(Debug, Clone, Copy, Default)]
struct CaseReadStats {
    /// Number of loaded modules in the compiler graph for this phase.
    modules: usize,
    /// Number of source lines across loaded modules.
    lines: usize,
}

/// A suite implementation that runs ecosystem phase tests.
#[derive(Debug)]
pub struct EcosystemSuite {
    phases: Vec<EcosystemPhase>,
    include: Vec<String>,
    exclude: Vec<String>,
    max_files: Option<usize>,
    tsc_mode: EcosystemTscMode,
    tsc_tool: EcosystemTscTool,
    read_stats: bool,
    dump_read_paths: bool,
    read_stats_by_case: Mutex<HashMap<String, CaseReadStats>>,
    manifests: Vec<EcosystemManifest>,
    manifests_by_name: HashMap<String, EcosystemManifest>,
    checkouts_dir: PathBuf,
    patches_dir: PathBuf,
    known_failures_path: PathBuf,
    ignored_path: PathBuf,
    raw_known_failures: HashSet<String>,
    expected_failures: HashSet<String>,
    ignored_cases: HashSet<String>,
    stale_known_entries: Vec<String>,
    stale_ignored_entries: Vec<String>,
    fetch_failures: HashMap<String, String>,
    prepare_failures: HashMap<String, String>,
}

impl EcosystemSuite {
    /// Load suite metadata, manifests, and status files.
    pub fn load(options: &EcosystemRunOptions, test_options: &RunOptions) -> Result<Self, String> {
        // resolve suite paths
        let ecosystem_dir = fixtures_dir().join("ecosystem");
        let packages_dir = ecosystem_dir.join("packages");
        let checkouts_dir = ecosystem_dir.join("checkouts");
        let patches_dir = ecosystem_dir.join("patches");
        let known_failures_path = ecosystem_dir.join("known-failures.txt");
        let ignored_path = ecosystem_dir.join("ignored.txt");

        // load all manifests and keep lookup maps in sync
        let manifest_paths = EcosystemManifest::discover_all(&packages_dir)?;
        if manifest_paths.is_empty() {
            return Err(format!(
                "no ecosystem manifests found under {}",
                packages_dir.display()
            ));
        }

        let mut manifests = Vec::new();
        let mut manifests_by_name = HashMap::new();
        for path in manifest_paths {
            let manifest = EcosystemManifest::load(&path)?;
            manifests_by_name.insert(manifest.package.name.clone(), manifest.clone());
            manifests.push(manifest);
        }

        manifests.sort_by(|left, right| left.package.name.cmp(&right.package.name));

        // derive selected phases and valid case identifiers
        let phases = options.selected_phases();
        let valid_case_ids = build_valid_case_ids(&manifests_by_name);

        // load known failure and ignored status sets
        let status = load_status_sets(
            known_failures_path.as_path(),
            ignored_path.as_path(),
            &valid_case_ids,
        );

        // auto fetch missing checkouts for selected packages
        let fetch_failures = fetch::auto_fetch_missing_checkouts(
            &manifests,
            &phases,
            test_options.filter.as_deref(),
            &checkouts_dir,
            &patches_dir,
            test_options.list,
        );

        // auto prepare selected packages after fetch
        let prepare_failures = auto_prepare_selected_packages(
            &manifests,
            &phases,
            test_options.filter.as_deref(),
            &checkouts_dir,
            &fetch_failures,
            test_options.list,
        );

        // construct the suite state
        Ok(Self {
            phases,
            include: options.include.clone(),
            exclude: options.exclude.clone(),
            max_files: options.max_files,
            tsc_mode: options.tsc_mode,
            tsc_tool: options.tsc_tool,
            read_stats: options.read_stats,
            dump_read_paths: options.dump_read_paths,
            read_stats_by_case: Mutex::new(HashMap::new()),
            manifests,
            manifests_by_name,
            checkouts_dir,
            patches_dir,
            known_failures_path,
            ignored_path,
            raw_known_failures: status.raw_known_failures,
            expected_failures: status.expected_failures,
            ignored_cases: status.ignored_cases,
            stale_known_entries: status.stale_known_entries,
            stale_ignored_entries: status.stale_ignored_entries,
            fetch_failures,
            prepare_failures,
        })
    }

    /// Run one package for one phase.
    fn run_package_phase(&self, manifest: &EcosystemManifest, phase: EcosystemPhase) -> CaseResult {
        // fail fast on setup failures captured during suite load
        if let Some(preflight_failure) = self.preflight_failure_result(manifest) {
            return preflight_failure;
        }

        let package_dir = self.checkouts_dir.join(&manifest.package.name);

        // make sure checkout and overlays are ready before discovery
        if let Err(error) = self.ensure_package_checkout_ready(manifest, package_dir.as_path()) {
            return CaseResult::Failed { message: error };
        }

        // discover candidate source files for this package phase
        let files = discover_package_output(
            &package_dir,
            manifest,
            phase,
            &self.include,
            &self.exclude,
            self.max_files,
        );
        if files.is_empty() {
            return CaseResult::Failed {
                message: format!(
                    "no files found for phase '{}' after filtering",
                    phase.name(),
                ),
            };
        }

        // print discovered read paths when requested by the caller
        self.print_discovered_paths_maybe(manifest, phase, package_dir.as_path(), &files);

        // run the selected phase and optionally collect graph stats
        let collect_read_stats = self.should_collect_case_read_stats(phase);
        let phase_result = tier::run_phase_tier(
            &package_dir,
            manifest,
            phase,
            &files,
            self.tsc_mode,
            self.tsc_tool,
            collect_read_stats,
        );

        // store collected stats under stable case id
        self.store_case_read_stats_maybe(manifest, phase, &phase_result);

        phase_result.result
    }

    /// Return a failed result when precomputed setup failed for this package.
    fn preflight_failure_result(&self, manifest: &EcosystemManifest) -> Option<CaseResult> {
        // surface auto-fetch failure from suite load
        if let Some(error) = self.fetch_failures.get(&manifest.package.name) {
            return Some(CaseResult::Failed {
                message: format!(
                    "auto-fetch failed for package '{}': {error}",
                    manifest.package.name
                ),
            });
        }

        // surface auto-prepare failure from suite load
        if let Some(error) = self.prepare_failures.get(&manifest.package.name) {
            return Some(CaseResult::Failed {
                message: format!(
                    "auto-prepare failed for package '{}': {error}",
                    manifest.package.name
                ),
            });
        }

        None
    }

    /// Ensure checkout existence and patch overlays for one package.
    fn ensure_package_checkout_ready(
        &self,
        manifest: &EcosystemManifest,
        package_dir: &Path,
    ) -> Result<(), String> {
        // fail loudly when checkout is still missing
        if !package_dir.exists() {
            return Err(format!(
                "package checkout missing after auto-fetch: {}",
                package_dir.display()
            ));
        }

        // keep overlay patches in sync with checkout
        let package_patches_dir = self.patches_dir.join(&manifest.package.name);
        fetch::ensure_patches_applied(&package_patches_dir, package_dir)
            .map_err(|error| format!("failed to apply patches: {error}"))?;

        Ok(())
    }

    /// Print discovered files when read path dumping is enabled.
    fn print_discovered_paths_maybe(
        &self,
        manifest: &EcosystemManifest,
        phase: EcosystemPhase,
        package_dir: &Path,
        files: &[PathBuf],
    ) {
        if !self.dump_read_paths {
            return;
        }

        let line_count = count_source_lines(files);

        println!();
        println!(
            "read files for {}-{} ({} files, {} lines):",
            manifest.package.name,
            phase.name(),
            files.len(),
            line_count,
        );

        for path in files {
            let relative = path.strip_prefix(package_dir).unwrap_or(path.as_path());
            println!("  {}", relative.display());
        }

        println!();
    }

    /// Return whether this phase should capture read stats.
    fn should_collect_case_read_stats(&self, phase: EcosystemPhase) -> bool {
        self.read_stats || phase == EcosystemPhase::Resolve
    }

    /// Store one phase run stats snapshot under package phase case id.
    fn store_case_read_stats_maybe(
        &self,
        manifest: &EcosystemManifest,
        phase: EcosystemPhase,
        phase_result: &tier::PhaseTierResult,
    ) {
        if !self.should_collect_case_read_stats(phase) {
            return;
        }

        let Some(case_stats) = phase_result.stats else {
            return;
        };

        let Ok(mut read_stats_by_case) = self.read_stats_by_case.lock() else {
            return;
        };

        let case_id = case_id_for(manifest.package.name.as_str(), phase);
        read_stats_by_case.insert(
            case_id,
            CaseReadStats {
                modules: case_stats.modules,
                lines: case_stats.lines,
            },
        );
    }

    /// Update known failure file from current run results.
    fn update_known_failures(&self, results: &[(Case, CaseResult)]) {
        let next_known_failures = compute_updated_known_failures(&self.raw_known_failures, results);

        if let Err(error) = save_expected_failures(&self.known_failures_path, &next_known_failures)
        {
            eprintln!(
                "failed to write known failures {}: {error}",
                self.known_failures_path.display()
            );
            return;
        }

        println!(
            "updated known failures: {} entries at {}",
            next_known_failures.len(),
            self.known_failures_path.display(),
        );
    }
}

impl Suite for EcosystemSuite {
    fn name(&self) -> &'static str {
        "ecosystem"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.manifests
            .iter()
            .flat_map(|manifest| {
                self.phases.iter().map(move |phase| {
                    let case_id = case_id_for(manifest.package.name.as_str(), *phase);
                    let is_ignored = self.ignored_cases.contains(&case_id);
                    Case::directory(
                        &case_id,
                        self.checkouts_dir.join(&manifest.package.name),
                        "destack_test::ecosystem",
                    )
                    .with_skipped(is_ignored)
                })
            })
            .collect()
    }

    fn expected_failures(&self, _options: &RunOptions) -> Option<&HashSet<String>> {
        if self.expected_failures.is_empty() {
            None
        } else {
            Some(&self.expected_failures)
        }
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        let Some((package_name, phase)) = parse_case_id(case.name.as_str()) else {
            return CaseResult::Failed {
                message: format!("invalid ecosystem case id: {}", case.name),
            };
        };

        let Some(manifest) = self.manifests_by_name.get(package_name) else {
            return CaseResult::Failed {
                message: format!("manifest not found: {package_name}"),
            };
        };

        self.run_package_phase(manifest, phase)
    }

    fn report(&self, results: &[(Case, CaseResult)], context: &RunContext<'_>) {
        if results.is_empty() {
            return;
        }

        // surface stale entries first so status files can be repaired
        report_stale_entries(
            "known",
            &self.stale_known_entries,
            &self.known_failures_path,
        );
        report_stale_entries("ignored", &self.stale_ignored_entries, &self.ignored_path);

        // collect read stats snapshots captured during case execution
        let read_stats_by_case = self
            .read_stats_by_case
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default();

        // aggregate package rows, phase rows, and regression deltas
        let mut report_state = SuiteReportState::for_phases(&self.phases);
        for (case, result) in results {
            report_state.record_case(
                case,
                result,
                &self.expected_failures,
                self.read_stats,
                &read_stats_by_case,
            );
        }

        // print one summary row per selected phase
        print_phase_summary_table(&self.phases, &report_state.phase_rows);

        // update readme only for full status runs
        if should_update_readme_for_context(context.options) {
            let is_partial = context.options.filter.is_some()
                || self.phases.len() != EcosystemPhase::all().len();
            update_ecosystem_readme(
                &self.manifests,
                &self.phases,
                &report_state.package_rows,
                &report_state.package_read_stats,
                &self.ignored_cases,
                &self.patches_dir,
                is_partial,
            );
        }

        // print per phase regressions and fixed cases
        print_regression_report(&report_state.regressions, &report_state.fixed);

        // write known failure updates when requested
        if context.options.update_known_failures {
            self.update_known_failures(results);
        }
    }
}

/// Aggregated report rows across all executed ecosystem cases.
#[derive(Debug, Default)]
struct SuiteReportState {
    /// Regressed cases per phase.
    regressions: BTreeMap<EcosystemPhase, Vec<String>>,
    /// Fixed cases per phase.
    fixed: BTreeMap<EcosystemPhase, Vec<String>>,
    /// Per phase summary row values.
    phase_rows: BTreeMap<EcosystemPhase, PhaseSummary>,
    /// Per package phase status cells.
    package_rows: BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
    /// Per package read stats loaded from resolve runs.
    package_read_stats: BTreeMap<String, ReadmePackageReadStats>,
}

impl SuiteReportState {
    /// Initialize report state for selected phases.
    fn for_phases(phases: &[EcosystemPhase]) -> Self {
        let mut phase_rows = BTreeMap::new();
        for phase in phases {
            phase_rows.insert(*phase, PhaseSummary::default());
        }

        Self {
            regressions: BTreeMap::new(),
            fixed: BTreeMap::new(),
            phase_rows,
            package_rows: BTreeMap::new(),
            package_read_stats: BTreeMap::new(),
        }
    }

    /// Record one case result into all report aggregates.
    fn record_case(
        &mut self,
        case: &Case,
        result: &CaseResult,
        expected_failures: &HashSet<String>,
        read_stats_enabled: bool,
        read_stats_by_case: &HashMap<String, CaseReadStats>,
    ) {
        let Some((package_name, phase)) = parse_case_id(case.name.as_str()) else {
            return;
        };

        // update the package phase status matrix
        let package_row = self
            .package_rows
            .entry(package_name.to_string())
            .or_default();
        package_row.insert(phase, readme_cell_from_result(result));

        let case_stats = read_stats_by_case.get(&case.name).copied();
        let is_known_failure = expected_failures.contains(&case.name);
        let is_failed = result.is_failed();
        let is_passed = result.is_passed();

        // update per phase counters and optional read stats
        let phase_summary = self.phase_rows.entry(phase).or_default();
        phase_summary.total += 1;

        if read_stats_enabled && let Some(case_stats) = case_stats {
            phase_summary.read_modules += case_stats.modules;
            phase_summary.read_lines += case_stats.lines;
        }

        update_phase_summary_outcome(phase_summary, result);

        // update fixed and regression counters on phase summary
        if !is_known_failure && is_failed {
            phase_summary.regressions += 1;
        }

        if is_known_failure && is_passed {
            phase_summary.fixed += 1;
        }

        // update resolve read stats used by readme modules and lines columns
        if phase == EcosystemPhase::Resolve
            && let Some(case_stats) = case_stats
        {
            self.package_read_stats.insert(
                package_name.to_string(),
                ReadmePackageReadStats {
                    modules: Some(case_stats.modules),
                    lines: Some(case_stats.lines),
                },
            );
        }

        // track case names for regression and fixed report output
        if !is_known_failure && is_failed {
            self.regressions
                .entry(phase)
                .or_default()
                .push(case.name.clone());
        }

        if is_known_failure && is_passed {
            self.fixed.entry(phase).or_default().push(case.name.clone());
        }
    }
}

/// Update one phase summary row from one case result.
fn update_phase_summary_outcome(phase_summary: &mut PhaseSummary, result: &CaseResult) {
    // count explicit pass and fail outcomes
    if result.is_passed() {
        phase_summary.passed += 1;
        return;
    }

    if result.is_failed() {
        phase_summary.failed += 1;
        return;
    }

    // track skipped reasons in dedicated buckets
    if let CaseResult::Skipped { reason } = result {
        phase_summary.skipped += 1;

        if reason == "known failure" {
            phase_summary.known_failures += 1;
        } else if reason == "marked as skipped" {
            phase_summary.ignored += 1;
        }
    }
}

/// Return whether this run should rewrite readme status tables.
fn should_update_readme_for_context(options: &RunOptions) -> bool {
    !options.run_skipped && !options.run_known_failures && !options.run_ignored
}

/// Print regressions and fixed cases grouped by phase.
fn print_regression_report(
    regressions: &BTreeMap<EcosystemPhase, Vec<String>>,
    fixed: &BTreeMap<EcosystemPhase, Vec<String>>,
) {
    if regressions.is_empty() && fixed.is_empty() {
        return;
    }

    println!();
    println!("ecosystem report");

    for phase in EcosystemPhase::all() {
        let phase_regressions = regressions.get(&phase).cloned().unwrap_or_default();
        let phase_fixed = fixed.get(&phase).cloned().unwrap_or_default();

        if phase_regressions.is_empty() && phase_fixed.is_empty() {
            continue;
        }

        println!("  phase: {}", phase.name());

        if !phase_regressions.is_empty() {
            println!("    regressions (expected pass, got fail):");
            for name in phase_regressions {
                println!("      {name}");
            }
        }

        if !phase_fixed.is_empty() {
            println!("    fixed (expected fail, got pass):");
            for name in phase_fixed {
                println!("      {name}");
            }
        }
    }

    println!();
}

/// Run the ecosystem suite with shared test harness options.
pub fn run_ecosystem_tests(options: &RunOptions, run_options: &EcosystemRunOptions) -> ExitCode {
    let suite = match EcosystemSuite::load(run_options, options) {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!("{}: {error}", color::red("error"));
            return ExitCode::FAILURE;
        }
    };

    Runner::run_suite(suite, options)
}

#[derive(Debug)]
struct StatusLoadResult {
    raw_known_failures: HashSet<String>,
    expected_failures: HashSet<String>,
    ignored_cases: HashSet<String>,
    stale_known_entries: Vec<String>,
    stale_ignored_entries: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
struct PhaseSummary {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    known_failures: usize,
    ignored: usize,
    fixed: usize,
    regressions: usize,
    read_modules: usize,
    read_lines: usize,
}

impl PhaseSummary {
    fn run_total(&self) -> usize {
        self.passed + self.failed
    }

    fn pass_rate(&self) -> Option<f64> {
        let run_total = self.run_total();
        if run_total == 0 {
            return None;
        }

        Some(self.passed as f64 / run_total as f64 * 100.0)
    }

    fn add(&mut self, other: &Self) {
        self.total += other.total;
        self.passed += other.passed;
        self.failed += other.failed;
        self.skipped += other.skipped;
        self.known_failures += other.known_failures;
        self.ignored += other.ignored;
        self.fixed += other.fixed;
        self.regressions += other.regressions;
        self.read_modules += other.read_modules;
        self.read_lines += other.read_lines;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ReadmeCellStatus {
    Pass,
    Fail,
    Ignored,
    #[default]
    Unknown,
}

impl ReadmeCellStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "✓",
            Self::Fail => "x",
            Self::Ignored => "-/-",
            Self::Unknown => "-?-",
        }
    }

    fn parse(value: &str) -> Self {
        match value.trim().trim_end_matches('*') {
            "✓" => Self::Pass,
            "x" => Self::Fail,
            "-/-" => Self::Ignored,
            "-?-" => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

/// Patch marker shown next to one package name in README rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ReadmePatchMarker {
    #[default]
    None,
    Local,
    Dependency,
}

impl ReadmePatchMarker {
    fn as_str(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Local => "*",
            Self::Dependency => "**",
        }
    }

    fn from_flags(has_local_patch: bool, has_dependency_replacement: bool) -> Self {
        if has_dependency_replacement {
            return Self::Dependency;
        }

        if has_local_patch {
            return Self::Local;
        }

        Self::None
    }
}

/// Metadata used when rendering one package row in README output.
#[derive(Debug, Clone, Copy, Default)]
struct ReadmePackageMetadata {
    patch_marker: ReadmePatchMarker,
    tier: Option<EcosystemSupportTier>,
}

/// Loaded read stats rendered in README rows.
#[derive(Debug, Clone, Copy, Default)]
struct ReadmePackageReadStats {
    /// Loaded module count from resolve phase.
    modules: Option<usize>,
    /// Loaded source line count from resolve phase.
    lines: Option<usize>,
}

/// Support tier derived from one package phase status row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ReadmeSupportTier {
    T0,
    T1,
    T2,
    T3,
    T4,
    T5,
    #[default]
    None,
}

impl ReadmeSupportTier {
    fn as_str(&self) -> &'static str {
        match self {
            Self::T0 => "T0",
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::T3 => "T3",
            Self::T4 => "T4",
            Self::T5 => "T5",
            Self::None => "-?-",
        }
    }

    fn from_tier(tier: EcosystemSupportTier) -> Self {
        match tier {
            EcosystemSupportTier::T0 => Self::T0,
            EcosystemSupportTier::T1 => Self::T1,
            EcosystemSupportTier::T2 => Self::T2,
            EcosystemSupportTier::T3 => Self::T3,
            EcosystemSupportTier::T4 => Self::T4,
            EcosystemSupportTier::T5 => Self::T5,
        }
    }

    fn rank(&self) -> Option<u8> {
        match self {
            Self::T0 => Some(0),
            Self::T1 => Some(1),
            Self::T2 => Some(2),
            Self::T3 => Some(3),
            Self::T4 => Some(4),
            Self::T5 => Some(5),
            Self::None => None,
        }
    }

    fn meets_target(&self, tier: Option<EcosystemSupportTier>) -> Option<bool> {
        let tier = tier?;

        let current_rank = self.rank()?;
        let target_rank = Self::from_tier(tier).rank()?;

        Some(current_rank >= target_rank)
    }
}

/// Derive one support tier from ordered phase statuses.
fn readme_support_tier_from_statuses(statuses: &[ReadmeCellStatus]) -> ReadmeSupportTier {
    let phase_passes = |index: usize| statuses.get(index).copied() == Some(ReadmeCellStatus::Pass);

    // parse must pass before any higher support tier can apply
    if !phase_passes(0) {
        return ReadmeSupportTier::None;
    }

    // parse passes, resolve does not
    if !phase_passes(1) {
        return ReadmeSupportTier::T0;
    }

    // parse and resolve pass, analyze does not
    if !phase_passes(2) {
        return ReadmeSupportTier::T1;
    }

    // parse, resolve, and analyze pass, lower does not
    if !phase_passes(3) {
        return ReadmeSupportTier::T2;
    }

    // parse through lower pass, run does not
    if !phase_passes(4) {
        return ReadmeSupportTier::T3;
    }

    // parse through run pass, tests do not
    if !phase_passes(5) {
        return ReadmeSupportTier::T4;
    }

    ReadmeSupportTier::T5
}

/// Render one package label with patch markers.
fn format_readme_package_label(
    package_name: &str,
    package_metadata: Option<ReadmePackageMetadata>,
) -> String {
    let marker = package_metadata
        .map(|metadata| metadata.patch_marker.as_str())
        .unwrap_or_default();

    format!("{package_name}{marker}")
}

/// Build README package metadata from manifests and patch directories.
fn collect_readme_package_metadata(
    manifests: &[EcosystemManifest],
    patches_dir: &Path,
) -> BTreeMap<String, ReadmePackageMetadata> {
    let mut package_metadata = BTreeMap::new();
    for manifest in manifests {
        let package_patches_dir = patches_dir.join(&manifest.package.name);
        let has_local_patch = patch_directory_has_files(&package_patches_dir);
        let has_dependency_replacement = manifest.patch.dependency_replacement();

        let patch_marker =
            ReadmePatchMarker::from_flags(has_local_patch, has_dependency_replacement);
        let tier = manifest.package.tier;

        package_metadata.insert(
            manifest.package.name.clone(),
            ReadmePackageMetadata { patch_marker, tier },
        );
    }

    package_metadata
}

/// Return whether one patch directory tree contains files.
fn patch_directory_has_files(dir: &Path) -> bool {
    if !dir.exists() {
        return false;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            return true;
        }

        if path.is_dir() && patch_directory_has_files(&path) {
            return true;
        }
    }

    false
}

/// Normalize README package names by removing patch markers.
fn normalize_readme_package_name(package_name: &str) -> String {
    package_name.trim_end_matches('*').to_string()
}

/// Map one test result to a readme table status cell.
fn readme_cell_from_result(result: &CaseResult) -> ReadmeCellStatus {
    match result {
        CaseResult::Passed => ReadmeCellStatus::Pass,
        CaseResult::Failed { .. } => ReadmeCellStatus::Fail,
        CaseResult::Skipped { reason } => {
            // known failures should count as failures in status tables
            if reason == "known failure" {
                ReadmeCellStatus::Fail
            }
            // explicit ignored entries stay in the ignored bucket
            else {
                ReadmeCellStatus::Ignored
            }
        }
        CaseResult::Suite { .. } => ReadmeCellStatus::Unknown,
    }
}
fn update_ecosystem_readme(
    manifests: &[EcosystemManifest],
    phases: &[EcosystemPhase],
    package_rows: &BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
    package_read_stats: &BTreeMap<String, ReadmePackageReadStats>,
    ignored_cases: &HashSet<String>,
    patches_dir: &Path,
    is_partial: bool,
) {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("ecosystem")
        .join("README.md");
    let content = match fs::read_to_string(&readme_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("{}: failed to read README.md: {error}", color::red("error"));
            return;
        }
    };

    let old_rows = match parse_readme_summary_rows(&content) {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!(
                "{}: failed to parse ecosystem README rows: {error}",
                color::red("error")
            );
            return;
        }
    };
    let old_read_stats = match parse_readme_summary_read_stats(&content) {
        Ok(read_stats) => read_stats,
        Err(error) => {
            eprintln!(
                "{}: failed to parse ecosystem README read stats: {error}",
                color::red("error")
            );
            return;
        }
    };

    let mut merged_rows = if is_partial {
        old_rows
    } else {
        BTreeMap::new()
    };
    let mut merged_read_stats = if is_partial {
        old_read_stats
    } else {
        BTreeMap::new()
    };

    for manifest in manifests {
        merged_rows
            .entry(manifest.package.name.clone())
            .or_insert_with(empty_readme_phase_row);
    }

    for (package_name, row) in package_rows {
        let merged_row = merged_rows
            .entry(package_name.clone())
            .or_insert_with(empty_readme_phase_row);
        for (phase, status) in row {
            merged_row.insert(*phase, *status);
        }
    }

    for (package_name, read_stats) in package_read_stats {
        merged_read_stats.insert(package_name.clone(), *read_stats);
    }

    // keep readme rows aligned to current manifests
    let manifest_names = manifests
        .iter()
        .map(|manifest| manifest.package.name.as_str())
        .collect::<HashSet<_>>();
    merged_rows.retain(|package_name, _| manifest_names.contains(package_name.as_str()));
    merged_read_stats.retain(|package_name, _| manifest_names.contains(package_name.as_str()));

    // reapply ignored statuses from ignored.txt after parsing old rows
    apply_ignored_case_statuses(&mut merged_rows, ignored_cases);
    let package_metadata = collect_readme_package_metadata(manifests, patches_dir);
    let summary_section =
        format_readme_summary_rows(&merged_rows, &merged_read_stats, &package_metadata);
    let Some(new_content) =
        replace_readme_section(&content, README_SECTION_SUMMARY_RESULTS, &summary_section)
    else {
        eprintln!(
            "{}: README.md missing summary-results markers",
            color::red("error")
        );
        return;
    };

    if new_content == content {
        return;
    }

    if let Err(error) = fs::write(&readme_path, &new_content) {
        eprintln!(
            "{}: failed to write README.md: {error}",
            color::red("error")
        );
        return;
    }

    let selected = phases
        .iter()
        .map(EcosystemPhase::name)
        .collect::<Vec<_>>()
        .join(", ");
    println!();
    println!(
        "  {} ecosystem README updated (phases: {})",
        color::green("UPDATED"),
        selected
    );
}

/// Create one default readme row with unknown status in all phases.
fn empty_readme_phase_row() -> BTreeMap<EcosystemPhase, ReadmeCellStatus> {
    let mut row = BTreeMap::new();
    for phase in EcosystemPhase::all() {
        row.insert(phase, ReadmeCellStatus::Unknown);
    }
    row
}

/// Apply ignored case statuses from ignored.txt to one merged README row set.
fn apply_ignored_case_statuses(
    rows: &mut BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
    ignored_cases: &HashSet<String>,
) {
    for case_id in ignored_cases {
        let Some((package_name, phase)) = parse_case_id(case_id.as_str()) else {
            continue;
        };

        let package_row = rows
            .entry(package_name.to_string())
            .or_insert_with(empty_readme_phase_row);
        package_row.insert(phase, ReadmeCellStatus::Ignored);
    }
}
fn format_readme_summary_rows(
    rows: &BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
    package_read_stats: &BTreeMap<String, ReadmePackageReadStats>,
    package_metadata: &BTreeMap<String, ReadmePackageMetadata>,
) -> String {
    let phases = EcosystemPhase::all();

    // build header rows first
    let mut lines = Vec::new();
    lines.push(format_readme_summary_header(&phases));
    lines.push(format_readme_summary_separator(&phases));

    // append one package row per sorted package key
    for (package_name, row) in rows {
        lines.push(format_readme_summary_package_row(
            package_name,
            row,
            &phases,
            package_read_stats,
            package_metadata,
        ));
    }

    // append footer summary rows
    let total_phase_cells = format_readme_summary_total_phase_cells(rows, &phases);
    let (total_modules, total_lines) =
        format_readme_summary_total_read_stats(rows, package_read_stats);
    lines.push(format_readme_summary_footer_separator(&phases));
    lines.push(format_readme_summary_total_row(
        &total_phase_cells,
        total_modules,
        total_lines,
    ));

    lines.join(
        "
",
    )
}

/// Format the markdown header row for readme summary table.
fn format_readme_summary_header(phases: &[EcosystemPhase]) -> String {
    let mut header = String::from("| Package ");
    for phase in phases {
        header.push_str(&format!("| {} ", phase.name()));
    }
    header.push_str("| Modules | Lines | Current | Target | Met |");

    header
}

/// Format the markdown separator row for readme summary table.
fn format_readme_summary_separator(phases: &[EcosystemPhase]) -> String {
    let mut separator = String::from("|:--------");
    for _ in phases {
        separator.push_str("|:--------:");
    }
    separator.push_str("|:--------:|:-----:|:-------:|:------:|:---:|");

    separator
}

/// Format one package row in the readme summary table.
fn format_readme_summary_package_row(
    package_name: &str,
    row: &BTreeMap<EcosystemPhase, ReadmeCellStatus>,
    phases: &[EcosystemPhase],
    package_read_stats: &BTreeMap<String, ReadmePackageReadStats>,
    package_metadata: &BTreeMap<String, ReadmePackageMetadata>,
) -> String {
    // gather phase statuses and support tier labels
    let statuses = phases
        .iter()
        .map(|phase| row.get(phase).copied().unwrap_or_default())
        .collect::<Vec<_>>();
    let package_label =
        format_readme_package_label(package_name, package_metadata.get(package_name).copied());
    let support_tier = readme_support_tier_from_statuses(&statuses);
    let target_tier = package_metadata
        .get(package_name)
        .and_then(|metadata| metadata.tier);

    // gather read stats and target met status labels
    let read_stats = package_read_stats
        .get(package_name)
        .copied()
        .unwrap_or_default();
    let modules_label = format_readme_count_cell(read_stats.modules);
    let lines_label = format_readme_count_cell(read_stats.lines);
    let target_label = target_tier.map_or("-?-", |tier| tier.as_str());
    let met_label = support_tier
        .meets_target(target_tier)
        .map_or("-?-", |is_met| if is_met { "✓" } else { "x" });

    // render one fixed width markdown table row
    let mut row_line = format!("| {package_label:<7} ");
    for status in &statuses {
        row_line.push_str(&format!("| {:^8} ", status.as_str()));
    }
    row_line.push_str(&format!("| {modules_label:^8} "));
    row_line.push_str(&format!("| {lines_label:^5} "));
    row_line.push_str(&format!("| {:^7} ", support_tier.as_str()));
    row_line.push_str(&format!("| {target_label:^6} "));
    row_line.push_str(&format!("| {met_label:^3} "));
    row_line.push('|');

    row_line
}

/// Format per phase totals for readme summary footer row.
fn format_readme_summary_total_phase_cells(
    rows: &BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
    phases: &[EcosystemPhase],
) -> Vec<String> {
    phases
        .iter()
        .map(|phase| {
            let mut phase_passed = 0usize;
            let mut phase_failed = 0usize;
            let mut phase_ignored = 0usize;
            let mut phase_unknown = 0usize;

            for row in rows.values() {
                let status = row.get(phase).copied().unwrap_or_default();

                // aggregate all status kinds: unknown contributes to denominator, ignored is reported separately
                match status {
                    ReadmeCellStatus::Pass => {
                        phase_passed += 1;
                    }
                    ReadmeCellStatus::Fail => {
                        phase_failed += 1;
                    }
                    ReadmeCellStatus::Ignored => {
                        phase_ignored += 1;
                    }
                    ReadmeCellStatus::Unknown => {
                        phase_unknown += 1;
                    }
                }
            }

            format_phase_total_cell(phase_passed, phase_failed, phase_unknown, phase_ignored)
        })
        .collect::<Vec<_>>()
}

/// Sum loaded module and line counts for packages that have resolve stats.
fn format_readme_summary_total_read_stats(
    rows: &BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
    package_read_stats: &BTreeMap<String, ReadmePackageReadStats>,
) -> (Option<usize>, Option<usize>) {
    let mut total_modules = 0usize;
    let mut total_modules_count = 0usize;
    let mut total_lines = 0usize;
    let mut total_lines_count = 0usize;

    for package_name in rows.keys() {
        let read_stats = package_read_stats
            .get(package_name)
            .copied()
            .unwrap_or_default();

        if let Some(modules) = read_stats.modules {
            total_modules += modules;
            total_modules_count += 1;
        }

        if let Some(lines_count) = read_stats.lines {
            total_lines += lines_count;
            total_lines_count += 1;
        }
    }

    let total_modules = if total_modules_count == 0 {
        None
    } else {
        Some(total_modules)
    };
    let total_lines = if total_lines_count == 0 {
        None
    } else {
        Some(total_lines)
    };

    (total_modules, total_lines)
}

/// Format the markdown footer separator row for readme summary table.
fn format_readme_summary_footer_separator(phases: &[EcosystemPhase]) -> String {
    let mut footer_separator = String::from("|---------");
    for _ in phases {
        footer_separator.push_str("|----------");
    }
    footer_separator.push_str("|----------|-------|---------|--------|-----|");

    footer_separator
}

/// Format the markdown total row for readme summary table.
fn format_readme_summary_total_row(
    total_phase_cells: &[String],
    total_modules: Option<usize>,
    total_lines: Option<usize>,
) -> String {
    let mut total_line = format!("| {:<7} ", "total");
    for phase_cell in total_phase_cells {
        total_line.push_str(&format!("| {phase_cell:^8} "));
    }
    total_line.push_str(&format!(
        "| {:^8} ",
        format_readme_count_cell(total_modules)
    ));
    total_line.push_str(&format!("| {:^5} ", format_readme_count_cell(total_lines)));
    total_line.push_str(&format!("| {:^7} ", "-?-"));
    total_line.push_str(&format!("| {:^6} ", "-?-"));
    total_line.push_str(&format!("| {:^3} ", "-?-"));
    total_line.push('|');

    total_line
}

/// Format one optional numeric count for readme table output.
fn format_readme_count_cell(value: Option<usize>) -> String {
    value.map_or_else(|| "-?-".to_string(), |value| value.to_string())
}

fn format_phase_total_cell(passed: usize, failed: usize, unknown: usize, ignored: usize) -> String {
    // unknown cells still represent one package and should count toward coverage denominator
    let run_total = passed + failed + unknown;
    let base = if run_total == 0 {
        "-?-".to_string()
    } else {
        format!("{passed}/{run_total}")
    };

    // ignored cells are displayed separately, they are intentionally excluded from denominator
    if ignored == 0 {
        base
    } else {
        format!("{base} (+{ignored})")
    }
}

fn parse_readme_summary_read_stats(
    content: &str,
) -> Result<BTreeMap<String, ReadmePackageReadStats>, String> {
    let section = readme_section(content, README_SECTION_SUMMARY_RESULTS)
        .ok_or_else(|| "README.md is missing summary-results section".to_string())?;
    let mut rows = BTreeMap::new();

    let mut modules_index = None;
    let mut lines_index = None;

    for line in section.lines() {
        if !line.trim_start().starts_with('|') {
            continue;
        }

        let cells = line.split('|').map(str::trim).collect::<Vec<_>>();
        if is_readme_separator_row(&cells) {
            continue;
        }

        let package_name = cells
            .get(1)
            .copied()
            .ok_or_else(|| format!("malformed ecosystem README row: {line}"))?;

        if package_name.eq_ignore_ascii_case("package") {
            for (index, cell) in cells.iter().enumerate() {
                if cell.eq_ignore_ascii_case("modules") {
                    modules_index = Some(index);
                }

                if cell.eq_ignore_ascii_case("lines") {
                    lines_index = Some(index);
                }
            }

            continue;
        }

        if package_name.eq_ignore_ascii_case("total") || package_name.is_empty() {
            continue;
        }

        let Some(modules_index) = modules_index else {
            return Err("ecosystem README header is missing Modules column".to_string());
        };
        let Some(lines_index) = lines_index else {
            return Err("ecosystem README header is missing Lines column".to_string());
        };

        let modules = parse_readme_count_cell(
            cells
                .get(modules_index)
                .ok_or_else(|| format!("missing Modules cell in ecosystem README row: {line}"))?,
        )?;
        let lines = parse_readme_count_cell(
            cells
                .get(lines_index)
                .ok_or_else(|| format!("missing Lines cell in ecosystem README row: {line}"))?,
        )?;

        let package_name = normalize_readme_package_name(package_name);
        rows.insert(package_name, ReadmePackageReadStats { modules, lines });
    }

    Ok(rows)
}

/// Parse one readme count cell value into an optional number.
fn parse_readme_count_cell(value: &str) -> Result<Option<usize>, String> {
    let normalized = value.trim();
    if normalized.is_empty() || normalized == "-?-" {
        return Ok(None);
    }

    normalized
        .parse::<usize>()
        .map(Some)
        .map_err(|_| format!("invalid ecosystem README count cell: '{value}'"))
}

fn parse_readme_summary_rows(
    content: &str,
) -> Result<BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>, String> {
    let section = readme_section(content, README_SECTION_SUMMARY_RESULTS)
        .ok_or_else(|| "README.md is missing summary-results section".to_string())?;
    let mut rows = BTreeMap::new();
    let phases = EcosystemPhase::all();

    for line in section.lines() {
        if !line.trim_start().starts_with('|') {
            continue;
        }

        let cells = line.split('|').map(str::trim).collect::<Vec<_>>();
        if is_readme_separator_row(&cells) {
            continue;
        }

        if cells.len() < phases.len() + 3 {
            return Err(format!("malformed ecosystem README row: {line}"));
        }

        let package_name = cells[1];
        if package_name.eq_ignore_ascii_case("package")
            || package_name.eq_ignore_ascii_case("total")
            || package_name.is_empty()
        {
            continue;
        }

        let mut row = empty_readme_phase_row();
        for (index, phase) in phases.iter().enumerate() {
            let Some(cell_value) = cells.get(2 + index) else {
                continue;
            };
            row.insert(*phase, ReadmeCellStatus::parse(cell_value));
        }
        let package_name = normalize_readme_package_name(package_name);
        rows.insert(package_name, row);
    }

    Ok(rows)
}

/// Return whether a parsed markdown table row is a separator row.
fn is_readme_separator_row(cells: &[&str]) -> bool {
    let mut has_non_empty_cell = false;

    for cell in cells {
        if cell.is_empty() {
            continue;
        }

        has_non_empty_cell = true;
        if !cell.chars().all(|character| matches!(character, '-' | ':')) {
            return false;
        }
    }

    has_non_empty_cell
}

/// Extract one named readme section body by marker tags.
fn readme_section<'a>(content: &'a str, section_name: &str) -> Option<&'a str> {
    let begin_marker = format!("<!-- begin:{section_name} -->");
    let end_marker = format!("<!-- end:{section_name} -->");

    let begin_index = content.find(&begin_marker)?;
    let section_start = begin_index + begin_marker.len();
    let section_end = content[section_start..].find(&end_marker)? + section_start;
    Some(content[section_start..section_end].trim())
}

/// Replace one named readme section body by marker tags.
fn replace_readme_section(content: &str, section_name: &str, new_section: &str) -> Option<String> {
    let begin_marker = format!("<!-- begin:{section_name} -->");
    let end_marker = format!("<!-- end:{section_name} -->");

    let begin_index = content.find(&begin_marker)?;
    let section_start = begin_index + begin_marker.len();
    let end_index = content[section_start..].find(&end_marker)? + section_start;

    Some(format!(
        "{}\n{}\n{}{}",
        &content[..begin_index + begin_marker.len()],
        new_section,
        end_marker,
        &content[end_index + end_marker.len()..]
    ))
}

/// Load known failures and ignored sets from single status files.
fn load_status_sets(
    known_failures_path: &Path,
    ignored_path: &Path,
    valid_case_ids: &HashSet<String>,
) -> StatusLoadResult {
    let raw_known_failures = load_expected_failures(known_failures_path);
    let raw_ignored = load_expected_failures(ignored_path);

    let mut expected_failures = HashSet::new();
    let mut ignored_cases = HashSet::new();
    let mut stale_known_entries = Vec::new();
    let mut stale_ignored_entries = Vec::new();

    for case_id in raw_known_failures.iter().cloned() {
        if valid_case_ids.contains(&case_id) {
            expected_failures.insert(case_id);
        } else {
            stale_known_entries.push(case_id);
        }
    }

    for case_id in raw_ignored {
        if valid_case_ids.contains(&case_id) {
            ignored_cases.insert(case_id);
        } else {
            stale_ignored_entries.push(case_id);
        }
    }

    stale_known_entries.sort();
    stale_ignored_entries.sort();

    StatusLoadResult {
        raw_known_failures,
        expected_failures,
        ignored_cases,
        stale_known_entries,
        stale_ignored_entries,
    }
}

/// Report stale status entries that no longer map to discovered cases.
fn report_stale_entries(kind: &str, stale_entries: &[String], path: &Path) {
    if stale_entries.is_empty() {
        return;
    }

    println!();
    println!("ecosystem: stale {kind} entries in {}", path.display(),);

    let show_count = stale_entries.len().min(20);
    for entry in stale_entries.iter().take(show_count) {
        println!("  {entry}");
    }

    if stale_entries.len() > show_count {
        println!("  ... and {} more", stale_entries.len() - show_count);
    }

    println!();
}

fn print_phase_summary_table(
    phases: &[EcosystemPhase],
    phase_rows: &BTreeMap<EcosystemPhase, PhaseSummary>,
) {
    // skip table rendering when no phases were selected
    if phases.is_empty() {
        return;
    }

    // build selected phase label
    let selected = phases
        .iter()
        .map(EcosystemPhase::name)
        .collect::<Vec<_>>()
        .join(", ");

    // print table header
    println!();
    println!("{}", color::bold("ECOSYSTEM PHASE SUMMARY"));
    println!("  selected phases: {}", color::cyan(&selected));
    println!();
    println!(
        "  {:10}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>8}  {:>8}  {:>10}  {:>10}",
        "Phase",
        "Total",
        "Pass",
        "Fail",
        "Skip",
        "Known",
        "Ign",
        "Fixed",
        "Regr",
        "Rate",
        "Status",
        "Modules",
        "Lines",
    );
    println!("  {}", "─".repeat(123));

    // print per phase rows and accumulate totals
    let mut total = PhaseSummary::default();
    for phase in phases {
        let row = phase_rows.get(phase).copied().unwrap_or_default();
        total.add(&row);

        let phase_name = format!("{:10}", phase.name());
        let total_text = format!("{:>6}", row.total);
        let passed_text = format!("{:>6}", row.passed);
        let failed_text = format!("{:>6}", row.failed);
        let skipped_text = if row.skipped > 0 {
            format!("{:>6}", row.skipped)
        } else {
            format!("{:>6}", "-")
        };
        let known_text = if row.known_failures > 0 {
            format!("{:>6}", row.known_failures)
        } else {
            format!("{:>6}", "-")
        };
        let ignored_text = if row.ignored > 0 {
            format!("{:>6}", row.ignored)
        } else {
            format!("{:>6}", "-")
        };
        let fixed_text = if row.fixed > 0 {
            format!("{:>6}", row.fixed)
        } else {
            format!("{:>6}", "-")
        };
        let regressions_text = if row.regressions > 0 {
            format!("{:>6}", row.regressions)
        } else {
            format!("{:>6}", "-")
        };
        let rate_text = format_phase_rate(row.pass_rate());
        let status_text = format_phase_status(&row);
        let modules_text = format_stats_value(row.read_modules, 10);
        let lines_text = format_stats_value(row.read_lines, 10);

        println!(
            "  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            color::cyan(&phase_name),
            color::bold(&total_text),
            color::green(&passed_text),
            color::red(&failed_text),
            color::dim(&skipped_text),
            color::yellow(&known_text),
            color::dim(&ignored_text),
            color::green(&fixed_text),
            color::red(&regressions_text),
            colorize_phase_rate(row.pass_rate(), &rate_text),
            status_text,
            color::cyan(&modules_text),
            color::cyan(&lines_text),
        );
    }

    // format total row values
    let total_rate = total.pass_rate();
    let total_rate_text = format_phase_rate(total_rate);
    let total_status_text = format_phase_status(&total);
    let total_label = format!("{:10}", "TOTAL");
    let total_total_text = format!("{:>6}", total.total);
    let total_passed_text = format!("{:>6}", total.passed);
    let total_failed_text = format!("{:>6}", total.failed);
    let total_skipped_text = if total.skipped > 0 {
        format!("{:>6}", total.skipped)
    } else {
        format!("{:>6}", "-")
    };
    let total_known_text = if total.known_failures > 0 {
        format!("{:>6}", total.known_failures)
    } else {
        format!("{:>6}", "-")
    };
    let total_ignored_text = if total.ignored > 0 {
        format!("{:>6}", total.ignored)
    } else {
        format!("{:>6}", "-")
    };
    let total_fixed_text = if total.fixed > 0 {
        format!("{:>6}", total.fixed)
    } else {
        format!("{:>6}", "-")
    };
    let total_regressions_text = if total.regressions > 0 {
        format!("{:>6}", total.regressions)
    } else {
        format!("{:>6}", "-")
    };
    let total_modules_text = format_stats_value(total.read_modules, 10);
    let total_lines_text = format_stats_value(total.read_lines, 10);

    // print total row
    println!("  {}", "─".repeat(123));
    println!(
        "  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
        color::bold(&total_label),
        color::bold(&total_total_text),
        color::green(&total_passed_text),
        color::red(&total_failed_text),
        color::dim(&total_skipped_text),
        color::yellow(&total_known_text),
        color::dim(&total_ignored_text),
        color::green(&total_fixed_text),
        color::red(&total_regressions_text),
        colorize_phase_rate(total_rate, &total_rate_text),
        total_status_text,
        color::cyan(&total_modules_text),
        color::cyan(&total_lines_text),
    );
    println!();
}

/// Format one optional phase pass rate percentage label.
fn format_phase_rate(rate: Option<f64>) -> String {
    match rate {
        Some(value) => format!("{value:>7.2}%"),
        None => format!("{:>8}", "-"),
    }
}

/// Format one numeric summary value with fallback unknown marker.
fn format_stats_value(value: usize, width: usize) -> String {
    if value == 0 {
        return format!("{:>width$}", "-?-");
    }

    format!("{value:>width$}")
}

/// Colorize one preformatted phase rate text value.
fn colorize_phase_rate(rate: Option<f64>, rate_text: &str) -> String {
    match rate {
        Some(value) if value >= 99.99 => color::green(rate_text),
        Some(value) if value >= 90.0 => color::yellow(rate_text),
        Some(_) => color::red(rate_text),
        None => color::dim(rate_text),
    }
}

/// Format one phase status label from summary counters.
fn format_phase_status(row: &PhaseSummary) -> String {
    let status_text = if row.failed > 0 || row.regressions > 0 {
        format!("{:>8}", "red")
    } else if row.known_failures > 0 || row.ignored > 0 {
        format!("{:>8}", "watch")
    } else {
        format!("{:>8}", "green")
    };

    if row.failed > 0 || row.regressions > 0 {
        color::red(&status_text)
    } else if row.known_failures > 0 || row.ignored > 0 {
        color::yellow(&status_text)
    } else {
        color::green(&status_text)
    }
}

/// Build all valid case ids across all phases and known manifests.
fn build_valid_case_ids(manifests_by_name: &HashMap<String, EcosystemManifest>) -> HashSet<String> {
    let mut ids = HashSet::new();

    for package_name in manifests_by_name.keys() {
        for phase in EcosystemPhase::all() {
            ids.insert(case_id_for(package_name, phase));
        }
    }

    ids
}

/// Run manifest configured prepare commands before selected phases.
fn auto_prepare_selected_packages(
    manifests: &[EcosystemManifest],
    phases: &[EcosystemPhase],
    filter: Option<&str>,
    checkouts_dir: &Path,
    fetch_failures: &HashMap<String, String>,
    is_list_mode: bool,
) -> HashMap<String, String> {
    if is_list_mode {
        return HashMap::new();
    }

    // collect manifests that need one prepare run for selected phases
    let selected = manifests
        .iter()
        .filter(|manifest| manifest_matches_filter(manifest, phases, filter))
        .filter(|manifest| manifest_requires_prepare_for_selected_phases(manifest, phases))
        .collect::<Vec<_>>();

    if selected.is_empty() {
        return HashMap::new();
    }

    println!();
    println!("auto-preparing {} ecosystem checkouts", selected.len());

    let mut failures = HashMap::new();

    for manifest in selected {
        if fetch_failures.contains_key(&manifest.package.name) {
            continue;
        }

        let package_dir = checkouts_dir.join(&manifest.package.name);
        if !package_dir.exists() {
            continue;
        }

        print!("  {} ... ", manifest.package.name);
        match ensure_prepare_commands_applied(manifest, package_dir.as_path()) {
            Ok(()) => println!("ok"),
            Err(error) => {
                println!("FAILED: {error}");
                failures.insert(manifest.package.name.clone(), error);
            }
        }
    }

    println!();

    failures
}

/// Return whether one manifest should run for the current filter and phases.
fn manifest_matches_filter(
    manifest: &EcosystemManifest,
    phases: &[EcosystemPhase],
    filter: Option<&str>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    // allow direct package name filters
    if manifest.package.name.contains(filter) {
        return true;
    }

    // allow package-phase case id filters
    phases
        .iter()
        .copied()
        .any(|phase| case_id_for(manifest.package.name.as_str(), phase).contains(filter))
}

/// Return whether this manifest needs prepare for any selected phase.
fn manifest_requires_prepare_for_selected_phases(
    manifest: &EcosystemManifest,
    phases: &[EcosystemPhase],
) -> bool {
    if !manifest.prepare.has_commands() {
        return false;
    }

    phases
        .iter()
        .copied()
        .any(|phase| manifest.prepare.includes_phase(phase))
}

/// Ensure prepare commands have run for this package checkout snapshot.
fn ensure_prepare_commands_applied(
    manifest: &EcosystemManifest,
    package_dir: &Path,
) -> Result<(), String> {
    // skip manifests without prepare commands
    if !manifest.prepare.has_commands() {
        return Ok(());
    }

    let expected_stamp = prepare_stamp_for_manifest(manifest);
    let stamp_path = package_dir.join(PREPARE_STAMP_FILE);

    // skip prepare when the current manifest stamp already ran
    if let Ok(existing_stamp) = fs::read_to_string(&stamp_path)
        && existing_stamp.trim() == expected_stamp
    {
        return Ok(());
    }

    for command_tokens in &manifest.prepare.commands {
        // reject empty commands loudly so the manifest is explicit
        if command_tokens.is_empty() {
            return Err("prepare command cannot be empty".to_string());
        }

        let program = &command_tokens[0];
        let args = &command_tokens[1..];
        let display_command = format_prepare_command(command_tokens);

        // run one prepare command in checkout root with deterministic ci env
        let status = Command::new(program)
            .args(args)
            .current_dir(package_dir)
            .env("CI", "1")
            .status()
            .map_err(|error| {
                format!(
                    "prepare command failed to start in {}: {} ({error})",
                    package_dir.display(),
                    display_command
                )
            })?;

        // fail fast on any non-zero prepare command status
        if !status.success() {
            return Err(format!(
                "prepare command failed in {}: {}",
                package_dir.display(),
                display_command
            ));
        }
    }

    fs::write(&stamp_path, format!("{expected_stamp}\n"))
        .map_err(|error| format!("failed writing {}: {error}", stamp_path.display()))?;

    Ok(())
}

/// Build one stable prepare stamp from manifest config and checkout ref.
fn prepare_stamp_for_manifest(manifest: &EcosystemManifest) -> String {
    let mut hasher = StableHasher::new();

    manifest.package.git_ref.hash(&mut hasher);

    for phase in &manifest.prepare.phases {
        phase.name().hash(&mut hasher);
    }

    for command_tokens in &manifest.prepare.commands {
        command_tokens.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish_u64())
}

/// Format one tokenized command for diagnostics.
fn format_prepare_command(command_tokens: &[String]) -> String {
    command_tokens.join(" ")
}

/// Build a stable case id from package and phase.
fn case_id_for(package_name: &str, phase: EcosystemPhase) -> String {
    format!("{package_name}-{}", phase.name())
}

/// Parse case ids in the form package-phase.
fn parse_case_id(case_id: &str) -> Option<(&str, EcosystemPhase)> {
    for phase in EcosystemPhase::all() {
        let suffix = format!("-{}", phase.name());
        if let Some(package_name) = case_id.strip_suffix(&suffix)
            && !package_name.is_empty()
        {
            return Some((package_name, phase));
        }
    }

    None
}

/// Compute the next known failure set from current run results.
fn compute_updated_known_failures(
    raw_known_failures: &HashSet<String>,
    results: &[(Case, CaseResult)],
) -> HashSet<String> {
    let mut observed_case_ids = HashSet::new();
    let mut current_failures = HashSet::new();

    for (case, result) in results {
        // only ecosystem phase case ids participate in known failure updates
        if parse_case_id(case.name.as_str()).is_none() {
            continue;
        }

        // preserve existing known failures when they were skipped as known failures
        if let CaseResult::Skipped { reason } = result
            && reason == "known failure"
        {
            continue;
        }

        observed_case_ids.insert(case.name.clone());

        // keep failing observed cases in the next known failure set
        if result.is_failed() {
            current_failures.insert(case.name.clone());
        }
    }

    // keep stale or unobserved entries and replace only observed case ids
    let mut next_known_failures = raw_known_failures.clone();
    next_known_failures.retain(|case_id| !observed_case_ids.contains(case_id));
    next_known_failures.extend(current_failures);

    next_known_failures
}

/// Count normalized source lines across selected files.
fn count_source_lines(files: &[PathBuf]) -> usize {
    let mut line_count = 0usize;

    for path in files {
        if let Ok(content) = fs::read_to_string(path) {
            line_count += content.lines().count().max(1);
        }
    }

    line_count
}

/// Discover source files for a package and phase.
fn discover_package_output(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    phase: EcosystemPhase,
    include_override: &[String],
    exclude_override: &[String],
    max_files_override: Option<usize>,
) -> Vec<PathBuf> {
    let workload = manifest.workloads.for_phase(phase);

    let include_patterns: Vec<String> = if !include_override.is_empty() {
        include_override.to_vec()
    } else if !workload.include.is_empty() {
        workload.include.clone()
    } else if !manifest.discovery.include.is_empty() {
        manifest.discovery.include.clone()
    } else {
        DEFAULT_INCLUDE_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect()
    };

    let mut include_set = HashSet::new();
    for pattern in &include_patterns {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());

        for path in matches {
            if path.is_file() && path_is_supported_source(path.as_path()) {
                include_set.insert(path);
            }
        }
    }

    let mut exclude_patterns = Vec::new();
    exclude_patterns.extend(DEFAULT_EXCLUDE_PATTERNS.iter().map(ToString::to_string));
    exclude_patterns.extend(manifest.discovery.exclude.clone());
    exclude_patterns.extend(workload.exclude.clone());
    exclude_patterns.extend(exclude_override.to_vec());

    let mut files = include_set
        .into_iter()
        .filter(|path| !path_matches_any_exclude_pattern(path, package_dir, &exclude_patterns))
        .collect::<Vec<_>>();
    files.sort();

    if let Some(limit) = max_files_override.or(workload.max_files) {
        files.truncate(limit);
    }

    files
}

/// Return whether a path matches any configured exclude pattern.
fn path_matches_any_exclude_pattern(
    path: &Path,
    package_dir: &Path,
    exclude_patterns: &[String],
) -> bool {
    let relative_path = match path.strip_prefix(package_dir) {
        Ok(relative_path) => relative_path.to_string_lossy().replace('\\', "/"),
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    };
    let file_name = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy())
        .unwrap_or_default();

    for pattern in exclude_patterns {
        let normalized_pattern = pattern.trim_start_matches("./");
        if normalized_pattern.is_empty() {
            continue;
        }

        // first: try matching against package relative path
        let matches_relative_path = glob_matches(
            normalized_pattern.as_bytes(),
            0,
            relative_path.as_bytes(),
            0,
        );
        if matches_relative_path {
            return true;
        }

        // second: support basename style excludes for simple patterns
        let has_path_separator = normalized_pattern.contains('/');
        if !has_path_separator {
            let matches_file_name =
                glob_matches(normalized_pattern.as_bytes(), 0, file_name.as_bytes(), 0);
            if matches_file_name {
                return true;
            }
        }
    }

    false
}

/// Return whether this path is a source file supported by ecosystem phases.
fn path_is_supported_source(path: &Path) -> bool {
    matches!(
        FileType::from_path(path),
        Some(
            FileType::TypeScript
                | FileType::TypeScriptXml
                | FileType::TypeScriptDeclaration
                | FileType::JavaScript
                | FileType::JavaScriptXml
        )
    )
}

/// Fetch one package checkout into checkouts directory.
pub fn fetch_package(
    manifest: &EcosystemManifest,
    checkouts_dir: &Path,
    options: FetchOptions,
) -> Result<PathBuf, String> {
    fetch::fetch_package(manifest, checkouts_dir, options)
}

/// Fetch all package checkouts declared by ecosystem manifests.
pub fn fetch_all_packages(options: FetchOptions) -> ExitCode {
    fetch::fetch_all_packages(options)
}
