use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use destack_source::{FileType, glob};

use crate::harness::print::color;
use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, fixtures_dir,
    load_expected_failures, save_expected_failures,
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

/// A suite implementation that runs ecosystem phase tests.
#[derive(Debug)]
pub struct EcosystemSuite {
    phases: Vec<EcosystemPhase>,
    include: Vec<String>,
    exclude: Vec<String>,
    max_files: Option<usize>,
    tsc_mode: EcosystemTscMode,
    tsc_tool: EcosystemTscTool,
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
    pub fn load(options: &EcosystemRunOptions, test_options: &TestOptions) -> Self {
        let ecosystem_dir = fixtures_dir().join("ecosystem");
        let packages_dir = ecosystem_dir.join("packages");
        let checkouts_dir = ecosystem_dir.join("checkouts");
        let patches_dir = ecosystem_dir.join("patches");
        let known_failures_path = ecosystem_dir.join("known-failures.txt");
        let ignored_path = ecosystem_dir.join("ignored.txt");

        let mut manifests = Vec::new();
        let mut manifests_by_name = HashMap::new();

        for path in EcosystemManifest::discover_all(&packages_dir) {
            match EcosystemManifest::load(&path) {
                Ok(manifest) => {
                    manifests_by_name.insert(manifest.package.name.clone(), manifest.clone());
                    manifests.push(manifest);
                }
                Err(error) => {
                    eprintln!("warning: {error}");
                }
            }
        }

        manifests.sort_by(|left, right| left.package.name.cmp(&right.package.name));

        let phases = options.selected_phases();
        let valid_case_ids = build_valid_case_ids(&manifests_by_name);

        let status = load_status_sets(
            known_failures_path.as_path(),
            ignored_path.as_path(),
            &valid_case_ids,
        );

        let fetch_failures = fetch::auto_fetch_missing_checkouts(
            &manifests,
            &phases,
            test_options.filter.as_deref(),
            &checkouts_dir,
            &patches_dir,
            test_options.list,
        );

        let prepare_failures = auto_prepare_selected_packages(
            &manifests,
            &phases,
            test_options.filter.as_deref(),
            &checkouts_dir,
            &fetch_failures,
            test_options.list,
        );

        Self {
            phases,
            include: options.include.clone(),
            exclude: options.exclude.clone(),
            max_files: options.max_files,
            tsc_mode: options.tsc_mode,
            tsc_tool: options.tsc_tool,
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
        }
    }

    /// Run one package for one phase.
    fn run_package_phase(&self, manifest: &EcosystemManifest, phase: EcosystemPhase) -> TestResult {
        if let Some(error) = self.fetch_failures.get(&manifest.package.name) {
            return TestResult::Failed {
                message: format!(
                    "auto-fetch failed for package '{}': {error}",
                    manifest.package.name
                ),
            };
        }

        if let Some(error) = self.prepare_failures.get(&manifest.package.name) {
            return TestResult::Failed {
                message: format!(
                    "auto-prepare failed for package '{}': {error}",
                    manifest.package.name
                ),
            };
        }

        let package_dir = self.checkouts_dir.join(&manifest.package.name);

        // fail loudly when checkout is still missing
        if !package_dir.exists() {
            return TestResult::Failed {
                message: format!(
                    "package checkout missing after auto-fetch: {}",
                    package_dir.display()
                ),
            };
        }

        // keep overlay patches in sync with checkout
        let package_patches_dir = self.patches_dir.join(&manifest.package.name);
        if let Err(error) = fetch::ensure_patches_applied(&package_patches_dir, &package_dir) {
            return TestResult::Failed {
                message: format!("failed to apply patches: {error}"),
            };
        }

        let files = discover_package_files(
            &package_dir,
            manifest,
            phase,
            &self.include,
            &self.exclude,
            self.max_files,
        );
        if files.is_empty() {
            return TestResult::Failed {
                message: format!(
                    "no files found for phase '{}' after filtering",
                    phase.name(),
                ),
            };
        }

        let phase_result = tier::run_phase_tier(
            &package_dir,
            manifest,
            phase,
            &files,
            self.tsc_mode,
            self.tsc_tool,
        );
        phase_result
    }

    /// Update known failure file from current run results.
    fn update_known_failures(&self, results: &[(TestCase, TestResult)]) {
        let mut current_failures = HashSet::new();

        for (case, result) in results {
            let Some((_, _)) = parse_case_id(case.name.as_str()) else {
                continue;
            };

            if result.is_failed() {
                current_failures.insert(case.name.clone());
            }
        }

        // preserve known failures for phases not included in this run
        let mut next_known_failures = self.raw_known_failures.clone();
        next_known_failures.retain(|case_id| {
            let Some((_, phase)) = parse_case_id(case_id.as_str()) else {
                return true;
            };

            !self.phases.contains(&phase)
        });

        next_known_failures.extend(current_failures);

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

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.manifests
            .iter()
            .flat_map(|manifest| {
                self.phases.iter().map(move |phase| {
                    let case_id = case_id_for(manifest.package.name.as_str(), *phase);
                    let is_ignored = self.ignored_cases.contains(&case_id);
                    TestCase::directory(
                        &case_id,
                        self.checkouts_dir.join(&manifest.package.name),
                        "destack_test::ecosystem",
                    )
                    .with_skipped(is_ignored)
                })
            })
            .collect()
    }

    fn expected_failures(&self, _options: &TestOptions) -> Option<&HashSet<String>> {
        if self.expected_failures.is_empty() {
            None
        } else {
            Some(&self.expected_failures)
        }
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        let Some((package_name, phase)) = parse_case_id(case.name.as_str()) else {
            return TestResult::Failed {
                message: format!("invalid ecosystem case id: {}", case.name),
            };
        };

        let Some(manifest) = self.manifests_by_name.get(package_name) else {
            return TestResult::Failed {
                message: format!("manifest not found: {package_name}"),
            };
        };

        self.run_package_phase(manifest, phase)
    }

    fn report(&self, results: &[(TestCase, TestResult)], context: &RunContext<'_>) {
        if results.is_empty() {
            return;
        }

        report_stale_entries(
            "known",
            &self.stale_known_entries,
            &self.known_failures_path,
        );
        report_stale_entries("ignored", &self.stale_ignored_entries, &self.ignored_path);

        let mut regressions: BTreeMap<EcosystemPhase, Vec<String>> = BTreeMap::new();
        let mut fixed: BTreeMap<EcosystemPhase, Vec<String>> = BTreeMap::new();
        let mut phase_rows = BTreeMap::new();
        let mut package_rows: BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>> =
            BTreeMap::new();
        for phase in &self.phases {
            phase_rows.insert(*phase, PhaseSummary::default());
        }

        for (case, result) in results {
            let Some((package_name, phase)) = parse_case_id(case.name.as_str()) else {
                continue;
            };

            let package_row = package_rows.entry(package_name.to_string()).or_default();
            package_row.insert(phase, readme_cell_from_result(result));

            let phase_summary = phase_rows.entry(phase).or_default();
            phase_summary.total += 1;

            match result {
                TestResult::Passed => {
                    phase_summary.passed += 1;
                }
                TestResult::Failed { .. } => {
                    phase_summary.failed += 1;
                }
                TestResult::Skipped { reason } => {
                    phase_summary.skipped += 1;

                    if reason == "known failure" {
                        phase_summary.known_failures += 1;
                    } else if reason == "marked as skipped" {
                        phase_summary.ignored += 1;
                    }
                }
                TestResult::Suite { .. } => {}
            }

            let is_known_failure = self.expected_failures.contains(&case.name);
            if !is_known_failure && result.is_failed() {
                regressions
                    .entry(phase)
                    .or_default()
                    .push(case.name.clone());
                phase_summary.regressions += 1;
            }
            if is_known_failure && result.is_passed() {
                fixed.entry(phase).or_default().push(case.name.clone());
                phase_summary.fixed += 1;
            }
        }

        print_phase_summary_table(&self.phases, &phase_rows);

        let should_update_readme = !context.options.include_skipped
            && !context.options.include_known_failures
            && !context.options.include_ignored;
        if should_update_readme {
            let is_partial = context.options.filter.is_some()
                || self.phases.len() != EcosystemPhase::all().len();
            update_ecosystem_readme(
                &self.manifests,
                &self.phases,
                &package_rows,
                &self.ignored_cases,
                &self.patches_dir,
                is_partial,
            );
        }

        if !regressions.is_empty() || !fixed.is_empty() {
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

        if context.options.update_known_failures {
            self.update_known_failures(results);
        }
    }
}

/// Run the ecosystem suite with shared test harness options.
pub fn run_ecosystem_tests(options: &TestOptions, run_options: &EcosystemRunOptions) -> ExitCode {
    let suite = EcosystemSuite::load(run_options, options);
    Runner::run_suite(&suite, options)
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
            Self::Ignored => "[--]",
            Self::Unknown => "---",
        }
    }

    fn parse(value: &str) -> Self {
        let value = value.trim().trim_end_matches('*').to_ascii_lowercase();
        match value.as_str() {
            "✓" | "✓✓" | "[✓]" | "v" | "pass" | "ok" | "green" => Self::Pass,
            "x" | "xx" | "[x]" | "fail" | "failed" | "red" => Self::Fail,
            "[--]" | "ignored" | "skip" | "skipped" | "watch" => Self::Ignored,
            "---" | "-" => Self::Unknown,
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
    target_tier: Option<EcosystemSupportTier>,
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
            Self::None => "---",
        }
    }

    fn from_target_tier(tier: EcosystemSupportTier) -> Self {
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

    fn meets_target(&self, target_tier: Option<EcosystemSupportTier>) -> Option<bool> {
        let target_tier = target_tier?;

        let current_rank = self.rank()?;
        let target_rank = Self::from_target_tier(target_tier).rank()?;

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
        let target_tier = manifest.package.target_tier;

        package_metadata.insert(
            manifest.package.name.clone(),
            ReadmePackageMetadata {
                patch_marker,
                target_tier,
            },
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

fn readme_cell_from_result(result: &TestResult) -> ReadmeCellStatus {
    match result {
        TestResult::Passed => ReadmeCellStatus::Pass,
        TestResult::Failed { .. } => ReadmeCellStatus::Fail,
        TestResult::Skipped { .. } => ReadmeCellStatus::Ignored,
        TestResult::Suite { .. } => ReadmeCellStatus::Unknown,
    }
}
fn update_ecosystem_readme(
    manifests: &[EcosystemManifest],
    phases: &[EcosystemPhase],
    package_rows: &BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
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

    let old_rows = parse_readme_summary_rows(&content).unwrap_or_default();
    let mut merged_rows = if is_partial {
        old_rows
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

    // keep readme rows aligned to current manifests
    let manifest_names = manifests
        .iter()
        .map(|manifest| manifest.package.name.as_str())
        .collect::<HashSet<_>>();
    merged_rows.retain(|package_name, _| manifest_names.contains(package_name.as_str()));

    // reapply ignored statuses from ignored.txt after parsing old rows
    apply_ignored_case_statuses(&mut merged_rows, ignored_cases);
    let package_metadata = collect_readme_package_metadata(manifests, patches_dir);
    let summary_section = format_readme_summary_rows(&merged_rows, &package_metadata);
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
    package_metadata: &BTreeMap<String, ReadmePackageMetadata>,
) -> String {
    let phases = EcosystemPhase::all();

    let mut header = String::from("| Package ");
    for phase in phases {
        header.push_str(&format!("| {} ", phase.name()));
    }
    header.push_str("| Current | Target | Met | Total |  Rate   | Incl. Rate |");

    let mut separator = String::from("|:--------");
    for _ in phases {
        separator.push_str("|:--------:");
    }
    separator.push_str("|:-------:|:------:|:---:|------:|--------:|-----------:|");

    let mut lines = Vec::new();
    lines.push(header);
    lines.push(separator);

    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut total_ignored = 0usize;
    let mut total_unknown = 0usize;

    for (package_name, row) in rows {
        let statuses = phases
            .iter()
            .map(|phase| row.get(phase).copied().unwrap_or_default())
            .collect::<Vec<_>>();

        let passed = statuses
            .iter()
            .filter(|status| **status == ReadmeCellStatus::Pass)
            .count();
        let failed = statuses
            .iter()
            .filter(|status| **status == ReadmeCellStatus::Fail)
            .count();
        let ignored = statuses
            .iter()
            .filter(|status| **status == ReadmeCellStatus::Ignored)
            .count();
        let unknown = statuses
            .iter()
            .filter(|status| **status == ReadmeCellStatus::Unknown)
            .count();

        // count all phases in the row total, even if a phase is unknown
        let total = statuses.len();
        let rate = format_rate(passed, failed);
        let inclusive_rate = format_inclusive_rate(passed, total);

        total_passed += passed;
        total_failed += failed;
        total_ignored += ignored;
        total_unknown += unknown;

        let package_label =
            format_readme_package_label(package_name, package_metadata.get(package_name).copied());
        let support_tier = readme_support_tier_from_statuses(&statuses);
        let target_tier = package_metadata
            .get(package_name)
            .and_then(|metadata| metadata.target_tier);

        let target_label = target_tier.map_or("---", |tier| tier.as_str());
        let met_label = support_tier
            .meets_target(target_tier)
            .map_or("---", |is_met| if is_met { "✓" } else { "x" });

        let mut row_line = format!("| {package_label:<7} ");
        for status in &statuses {
            row_line.push_str(&format!("| {:^8} ", status.as_str()));
        }
        row_line.push_str(&format!("| {:^7} ", support_tier.as_str()));
        row_line.push_str(&format!("| {:^6} ", target_label));
        row_line.push_str(&format!("| {:^3} ", met_label));
        row_line.push_str(&format!("| {total:>5} | {rate:>7} | {inclusive_rate:>9} |"));
        lines.push(row_line);
    }

    let total_cases = total_passed + total_failed + total_ignored + total_unknown;
    let total_rate = format_rate(total_passed, total_failed);
    let total_inclusive_rate = format_inclusive_rate(total_passed, total_cases);
    let total_phase_cells = phases
        .iter()
        .map(|phase| {
            let mut phase_passed = 0usize;
            let mut phase_failed = 0usize;
            for row in rows.values() {
                let status = row.get(phase).copied().unwrap_or_default();
                match status {
                    ReadmeCellStatus::Pass => {
                        phase_passed += 1;
                    }
                    ReadmeCellStatus::Fail => {
                        phase_failed += 1;
                    }
                    ReadmeCellStatus::Ignored | ReadmeCellStatus::Unknown => {}
                }
            }
            format_phase_total_cell(phase_passed, phase_failed)
        })
        .collect::<Vec<_>>();

    let mut footer_separator = String::from("|---------");
    for _ in phases {
        footer_separator.push_str("|----------");
    }
    footer_separator.push_str("|---------|--------|-----|-------|---------|------------|");
    lines.push(footer_separator);

    let mut total_line = format!("| {:<7} ", "total");
    for phase_cell in total_phase_cells {
        total_line.push_str(&format!("| {:^8} ", phase_cell));
    }
    total_line.push_str(&format!("| {:^7} ", "---"));
    total_line.push_str(&format!("| {:^6} ", "---"));
    total_line.push_str(&format!("| {:^3} ", "---"));
    total_line.push_str(&format!(
        "| {:>5} | {:>7} | {:>9} |",
        total_cases, total_rate, total_inclusive_rate
    ));
    lines.push(total_line);
    lines.push(String::new());
    lines.push(format!(
        "Total Blended Pass Rate: **{total_rate}** ({total_inclusive_rate} incl. ignored)"
    ));

    lines.join("\n")
}

fn format_rate(passed: usize, failed: usize) -> String {
    let total = passed + failed;
    if total == 0 {
        return "-".to_string();
    }

    let rate = passed as f64 / total as f64 * 100.0;
    format!("{rate:.2}%")
}

fn format_phase_total_cell(passed: usize, failed: usize) -> String {
    let run_total = passed + failed;
    if run_total == 0 {
        return "---".to_string();
    }

    format!("{passed}/{run_total}")
}

fn format_inclusive_rate(passed: usize, total: usize) -> String {
    if total == 0 {
        return "-".to_string();
    }

    let rate = passed as f64 / total as f64 * 100.0;
    format!("{rate:.2}%")
}

fn parse_readme_summary_rows(
    content: &str,
) -> Option<BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>> {
    let section = readme_section(content, README_SECTION_SUMMARY_RESULTS)?;
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
            continue;
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

    Some(rows)
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

fn readme_section<'a>(content: &'a str, section_name: &str) -> Option<&'a str> {
    let begin_marker = format!("<!-- begin:{section_name} -->");
    let end_marker = format!("<!-- end:{section_name} -->");

    let begin_index = content.find(&begin_marker)?;
    let section_start = begin_index + begin_marker.len();
    let section_end = content[section_start..].find(&end_marker)? + section_start;
    Some(content[section_start..section_end].trim())
}

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
    if phases.is_empty() {
        return;
    }

    let selected = phases
        .iter()
        .map(EcosystemPhase::name)
        .collect::<Vec<_>>()
        .join(", ");

    println!();
    println!("{}", color::bold("ECOSYSTEM PHASE SUMMARY"));
    println!("  selected phases: {}", color::cyan(&selected));
    println!();

    println!(
        "  {:10}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>8}  {:>8}",
        "Phase", "Total", "Pass", "Fail", "Skip", "Known", "Ign", "Fixed", "Regr", "Rate", "Status",
    );
    println!("  {}", "─".repeat(99));

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

        println!(
            "  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
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
        );
    }

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

    println!("  {}", "─".repeat(99));
    println!(
        "  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
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
    );
    println!();
}

fn format_phase_rate(rate: Option<f64>) -> String {
    match rate {
        Some(value) => format!("{value:>7.2}%"),
        None => format!("{:>8}", "-"),
    }
}

fn colorize_phase_rate(rate: Option<f64>, rate_text: &str) -> String {
    match rate {
        Some(value) if value >= 99.99 => color::green(rate_text),
        Some(value) if value >= 90.0 => color::yellow(rate_text),
        Some(_) => color::red(rate_text),
        None => color::dim(rate_text),
    }
}

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
    let mut hasher = DefaultHasher::new();

    manifest.package.git_ref.hash(&mut hasher);

    for phase in &manifest.prepare.phases {
        phase.name().hash(&mut hasher);
    }

    for command_tokens in &manifest.prepare.commands {
        command_tokens.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
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

/// Discover source files for a package and phase.
fn discover_package_files(
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

    let mut exclude_set = HashSet::new();
    for pattern in &exclude_patterns {
        let full_pattern = package_dir.join(pattern);
        let matches = glob(&full_pattern.to_string_lossy());
        exclude_set.extend(matches);
    }

    let mut files = include_set
        .into_iter()
        .filter(|path| !exclude_set.contains(path))
        .collect::<Vec<_>>();
    files.sort();

    if let Some(limit) = max_files_override.or(workload.max_files) {
        files.truncate(limit);
    }

    files
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readme_cell_status_roundtrips_unknown_and_ignored() {
        // keep unknown cells distinct from ignored cells
        let unknown = ReadmeCellStatus::parse(ReadmeCellStatus::Unknown.as_str());
        let ignored = ReadmeCellStatus::parse(ReadmeCellStatus::Ignored.as_str());

        assert_eq!(unknown, ReadmeCellStatus::Unknown);
        assert_eq!(ignored, ReadmeCellStatus::Ignored);
    }

    #[test]
    fn test_parse_readme_summary_rows_keeps_unknown_cells() {
        let readme = r#"
<!-- begin:summary-results -->
| Package | parse | resolve | analyze | lower | Current | Target | Met | Total |  Rate   | Incl. Rate |
|:--------|:-----:|:-------:|:-------:|:-----:|:-------:|:------:|:---:|------:|--------:|-----------:|
| sample  |  ---  |  [--]   |   ✓     |   x   |   ---   |  ---   | --- |     4 |  50.00% |    25.00% |
<!-- end:summary-results -->
"#;

        let rows = parse_readme_summary_rows(readme).expect("expected summary rows");
        let row = rows.get("sample").expect("expected sample row");

        // preserve unknown and ignored meanings when parsing summary rows
        assert_eq!(
            row.get(&EcosystemPhase::Parse),
            Some(&ReadmeCellStatus::Unknown)
        );
        assert_eq!(
            row.get(&EcosystemPhase::Resolve),
            Some(&ReadmeCellStatus::Ignored)
        );
        assert_eq!(
            row.get(&EcosystemPhase::Analyze),
            Some(&ReadmeCellStatus::Pass)
        );
        assert_eq!(
            row.get(&EcosystemPhase::Lower),
            Some(&ReadmeCellStatus::Fail)
        );
    }
}
