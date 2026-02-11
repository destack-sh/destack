use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, OptimizeTask, ResolveTask};
use destack_parser::{Parser, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, File, FileRegistry, FileSystem, FileType,
    LanguageType, MemoryFileSystem, ModuleId, PhysicalFileSystem, PrintOptions, Uri, glob,
};
use destack_workspace::{
    FormatterOptions, LinterOptions, PackageJson, Program, Session, select_manifest_entry_paths,
};

use crate::harness::print::color;
use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, fixtures_dir, format_diagnostics,
    load_expected_failures, save_expected_failures,
};

use super::manifest::{CompilerOptionsConfig, EcosystemManifest, EcosystemPhase};

const DEFAULT_INCLUDE_PATTERNS: &[&str] = &["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"];
const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &["**/node_modules/**", "**/dist/**", "**/build/**"];

const PATCH_STAMP_FILE: &str = ".destack_patch_stamp";
const README_SECTION_SUMMARY_RESULTS: &str = "summary-results";

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
}

/// A suite implementation that runs ecosystem phase tests.
#[derive(Debug)]
pub struct EcosystemSuite {
    phases: Vec<EcosystemPhase>,
    include: Vec<String>,
    exclude: Vec<String>,
    max_files: Option<usize>,
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

        let fetch_failures = auto_fetch_missing_checkouts(
            &manifests,
            &phases,
            test_options.filter.as_deref(),
            &checkouts_dir,
            &patches_dir,
            test_options.list,
        );

        Self {
            phases,
            include: options.include.clone(),
            exclude: options.exclude.clone(),
            max_files: options.max_files,
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
        if let Err(error) = ensure_patches_applied(&package_patches_dir, &package_dir) {
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

        let phase_result = run_phase_tier(&package_dir, manifest, phase, &files);
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
            update_ecosystem_readme(&self.manifests, &self.phases, &package_rows, is_partial);
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
            Self::Ignored => "---",
            Self::Unknown => "---",
        }
    }

    fn parse(value: &str) -> Self {
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "✓" | "✓✓" | "[✓]" | "v" | "pass" | "ok" | "green" => Self::Pass,
            "x" | "xx" | "[x]" | "fail" | "failed" | "red" => Self::Fail,
            "---" | "[--]" | "-" | "ignored" | "skip" | "skipped" | "watch" => Self::Ignored,
            _ => Self::Unknown,
        }
    }
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

    let summary_section = format_readme_summary_rows(&merged_rows);
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

fn format_readme_summary_rows(
    rows: &BTreeMap<String, BTreeMap<EcosystemPhase, ReadmeCellStatus>>,
) -> String {
    let phases = EcosystemPhase::all();

    let mut header = String::from("| Package ");
    for phase in phases {
        header.push_str(&format!("| {} ", phase.name()));
    }
    header.push_str("| Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |");

    let mut separator = String::from("|:--------");
    for _ in phases {
        separator.push_str("|:--------:");
    }
    separator.push_str("|-------:|-------:|--------:|------:|--------:|-----------:|");

    let mut lines = Vec::new();
    lines.push(header);
    lines.push(separator);

    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut total_ignored = 0usize;

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
        let total = passed + failed + ignored;
        let rate = format_rate(passed, failed);
        let inclusive_rate = format_inclusive_rate(passed, total);

        total_passed += passed;
        total_failed += failed;
        total_ignored += ignored;

        let mut row_line = format!("| {package_name:<7} ");
        for status in &statuses {
            row_line.push_str(&format!("| {:^8} ", status.as_str()));
        }
        row_line.push_str(&format!(
            "| {passed:>5}  | {failed:>5}  | {ignored:>7}  | {total:>5} | {rate:>7} | {inclusive_rate:>9} |"
        ));
        lines.push(row_line);
    }

    let total_cases = total_passed + total_failed + total_ignored;
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
    footer_separator.push_str("|--------|--------|---------|-------|---------|------------|");
    lines.push(footer_separator);

    let mut total_line = format!("| {:<7} ", "total");
    for phase_cell in total_phase_cells {
        total_line.push_str(&format!("| {:^8} ", phase_cell));
    }
    total_line.push_str(&format!(
        "| {:>5}  | {:>5}  | {:>7}  | {:>5} | {:>7} | {:>9} |",
        total_passed, total_failed, total_ignored, total_cases, total_rate, total_inclusive_rate
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

        if cells.len() < phases.len() + 9 {
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
            row.insert(*phase, ReadmeCellStatus::parse(cells[2 + index]));
        }
        rows.insert(package_name.to_string(), row);
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
    println!(
        "{}",
        color::bold(
            "════════════════════════════════════════════════════════════════════════════════════════════════════"
        )
    );
    println!(
        "{}",
        color::bold("                                  ECOSYSTEM PHASE SUMMARY")
    );
    println!(
        "{}",
        color::bold(
            "════════════════════════════════════════════════════════════════════════════════════════════════════"
        )
    );
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

/// Run one phase tier for one package workload.
fn run_phase_tier(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    phase: EcosystemPhase,
    files: &[PathBuf],
) -> TestResult {
    match phase {
        EcosystemPhase::Parse => run_parse_phase(package_dir, manifest, files),
        EcosystemPhase::Resolve | EcosystemPhase::Analyze | EcosystemPhase::Lower => {
            run_compiler_phase(package_dir, phase, files)
        }
    }
}

/// Run parser level checks across files.
fn run_parse_phase(
    package_dir: &Path,
    manifest: &EcosystemManifest,
    files: &[PathBuf],
) -> TestResult {
    let mut failures = Vec::new();
    let mut failure_messages = Vec::new();

    for path in files {
        match parse_file(path, manifest) {
            Ok(()) => {}
            Err(output) => {
                let relative = path.strip_prefix(package_dir).unwrap_or(path);
                failure_messages.push(format!("{}:\n{}", relative.display(), output.trim_end()));
                failures.push(relative.to_path_buf());
            }
        }
    }

    if failures.is_empty() {
        return TestResult::Passed;
    }

    TestResult::Failed {
        message: format!(
            "{} files failed to parse with full diagnostics:\n\n{}",
            failures.len(),
            failure_messages.join("\n\n")
        ),
    }
}

/// Run compiler phase checks across selected entrypoints.
fn run_compiler_phase(package_dir: &Path, phase: EcosystemPhase, files: &[PathBuf]) -> TestResult {
    let entrypoints = select_phase_entrypoints(package_dir, files);
    if entrypoints.is_empty() {
        return TestResult::Failed {
            message: format!("no entrypoints selected for phase '{}'", phase.name(),),
        };
    }

    let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let session = Arc::new(Session::new(package_dir.to_path_buf()).with_fs(file_system));
    let program = session.add_root(package_dir.to_path_buf());

    let compiler = Compiler::new(
        session,
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );

    let mut module_ids = BTreeSet::new();
    for path in &entrypoints {
        let module_id = match compiler.resolve_path_to_module(path) {
            Ok(id) => id,
            Err(error) => {
                return TestResult::Failed {
                    message: format!("failed to resolve module {}: {error:?}", path.display()),
                };
            }
        };

        module_ids.insert(module_id);
    }

    if module_ids.is_empty() {
        return TestResult::Failed {
            message: "no modules resolved from selected entrypoints".to_string(),
        };
    }

    for module_id in module_ids.iter().copied() {
        enqueue_phase_task(&compiler, &program, module_id, phase);
    }

    compiler.compile();
    drop(compiler);

    let diagnostics = program.diagnostics.collect();
    let errors = diagnostics
        .iter()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();

    if errors.is_empty() {
        return TestResult::Passed;
    }

    let mut error_diagnostics = DiagnosticCollection::new();
    for diagnostic in errors.iter().cloned() {
        error_diagnostics.insert(diagnostic);
    }

    let diagnostic_output = format_diagnostics(
        &program.files,
        &error_diagnostics,
        PrintOptions::new()
            .with_colorizer(source_colorizer())
            .with_module_count(entrypoints.len()),
    );

    TestResult::Failed {
        message: format!(
            "phase '{}' failed with {} errors across {} entrypoints and {} discovered files:\n\n{}",
            phase.name(),
            errors.len(),
            entrypoints.len(),
            files.len(),
            diagnostic_output.trim_end()
        ),
    }
}

/// Enqueue one compiler task that represents a phase boundary.
fn enqueue_phase_task(
    compiler: &Compiler,
    program: &Arc<Program>,
    module_id: ModuleId,
    phase: EcosystemPhase,
) {
    let module = compiler.module_stamp(module_id);

    match phase {
        EcosystemPhase::Parse => {}
        EcosystemPhase::Resolve => {
            let profile_id = program.default_profile_id_for_module(module_id);
            let profile = compiler.profile_stamp(profile_id);
            compiler.enqueue(ResolveTask::ResolveModule {
                module,
                profile,
                graph: compiler.module_graph_stamp(profile_id),
            });
        }
        EcosystemPhase::Analyze => {
            let profile_id = program.default_profile_id_for_module(module_id);
            let profile = compiler.profile_stamp(profile_id);
            compiler.enqueue(AnalyzeTask::AnalyzeModule { module, profile });
        }
        EcosystemPhase::Lower => {
            let target = program.ensure_target_for_module(module_id);
            let profile_id = program.profile_id_for_target_or_default(module_id, &target);
            let profile = compiler.profile_stamp(profile_id);
            compiler.enqueue(OptimizeTask::OptimizeModule {
                module,
                profile,
                target,
            });
        }
    }
}

/// Select phase entrypoints from package metadata, with deterministic fallback heuristics.
fn select_phase_entrypoints(package_dir: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
    let mut candidates = files.to_vec();

    // filter declaration files for semantic and lowering phases
    candidates.retain(|path| !is_declaration_file(path.as_path()));

    candidates.sort_by_key(|path| entrypoint_sort_key(package_dir, path.as_path()));

    // prefer package.json entry fields when possible
    let manifest_entrypoints = select_manifest_entrypoints(package_dir, &candidates);
    let selected = if manifest_entrypoints.is_empty() {
        candidates
    } else {
        manifest_entrypoints
    };

    selected
}

/// Build deterministic sort keys for phase entrypoints.
fn entrypoint_sort_key(package_dir: &Path, path: &Path) -> (u8, usize, String) {
    let relative = path.strip_prefix(package_dir).unwrap_or(path);
    let file_name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    let root_priority = match file_name {
        "index.ts" | "index.tsx" | "mod.ts" | "mod.tsx" => 0,
        "main.ts" | "main.tsx" | "app.ts" | "app.tsx" => 1,
        _ => 2,
    };
    let depth = relative.components().count();

    (root_priority, depth, relative.to_string_lossy().to_string())
}

/// Return whether this path points to a declaration file.
fn is_declaration_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    file_name.ends_with(".d.ts") || file_name.ends_with(".d.mts") || file_name.ends_with(".d.cts")
}

/// Select candidate files that correspond to package.json entry fields.
fn select_manifest_entrypoints(package_dir: &Path, candidates: &[PathBuf]) -> Vec<PathBuf> {
    let entry_targets = load_package_entry_targets(package_dir);
    if entry_targets.is_empty() {
        return Vec::new();
    }
    select_manifest_entry_paths(package_dir, candidates, &entry_targets)
}

/// Load package entry target strings from package.json.
fn load_package_entry_targets(package_dir: &Path) -> Vec<String> {
    let package_json_path = package_dir.join("package.json");
    let Ok(content) = fs::read_to_string(&package_json_path) else {
        return Vec::new();
    };
    let Ok(package_json) = serde_json::from_str::<PackageJson>(&content) else {
        return Vec::new();
    };

    package_json.entry_targets()
}

/// Parse one source file and fail when parser diagnostics contain errors.
fn parse_file(path: &Path, manifest: &EcosystemManifest) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("read error: {error}"))?;

    let file_type = FileType::from_path(path).unwrap_or(FileType::TypeScript);
    let language_type = language_type_for_parse(file_type, &manifest.compiler_options);

    let uri = Uri::from_path(path);
    let files = Arc::new(FileRegistry::new());
    let file_system: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();

    let program = Arc::new(Program::from_options(
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        file_system,
        files,
    ));

    let file_id = program.files.next_id();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input")
        .to_string();
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        content,
    );
    program.files.insert(file);
    let file = program.files.get(file_id);

    let mut parser = Parser::lex_file(file, language_type);
    let _ = parser.parse();
    program.diagnostics.merge_from(&parser.diagnostics);

    let errors = program
        .diagnostics
        .iter()
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();

    if errors.is_empty() {
        return Ok(());
    }

    let mut error_diagnostics = DiagnosticCollection::new();
    for diagnostic in errors.iter().cloned() {
        error_diagnostics.insert(diagnostic);
    }

    let diagnostic_output = format_diagnostics(
        &program.files,
        &error_diagnostics,
        PrintOptions::new()
            .with_colorizer(source_colorizer())
            .with_module_count(1),
    );

    Err(diagnostic_output.trim_end().to_string())
}

/// Resolve parse language mode for a discovered source file.
fn language_type_for_parse(
    file_type: FileType,
    compiler_options: &CompilerOptionsConfig,
) -> LanguageType {
    // allow explicit js to jsx parse mode override
    if file_type == FileType::JavaScript && compiler_options.js_as_jsx() {
        return LanguageType::JavaScriptXml;
    }

    LanguageType::from(file_type)
}

/// Ensure patch overlays are applied exactly once per patch snapshot.
fn ensure_patches_applied(patches_dir: &Path, package_dir: &Path) -> Result<(), String> {
    if !patches_dir.exists() {
        return Ok(());
    }

    let patch_stamp = compute_patch_stamp(patches_dir)?;
    let stamp_path = package_dir.join(PATCH_STAMP_FILE);

    if let Ok(existing_stamp) = fs::read_to_string(&stamp_path)
        && existing_stamp.trim() == patch_stamp
    {
        return Ok(());
    }

    copy_dir_recursive(patches_dir, package_dir)?;
    fs::write(&stamp_path, format!("{patch_stamp}\n"))
        .map_err(|error| format!("failed writing {}: {error}", stamp_path.display()))?;
    Ok(())
}

/// Compute a stable content stamp for a patch directory.
fn compute_patch_stamp(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files_recursive(root, &mut files)?;
    files.sort();

    let mut hasher = DefaultHasher::new();
    for file in files {
        let relative = file.strip_prefix(root).unwrap_or(file.as_path());
        relative.to_string_lossy().hash(&mut hasher);

        let content = fs::read(file.as_path())
            .map_err(|error| format!("failed reading {}: {error}", file.display()))?;
        content.hash(&mut hasher);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

/// Collect file paths recursively from a directory.
fn collect_files_recursive(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }

    for entry in
        fs::read_dir(root).map_err(|error| format!("read_dir {}: {error}", root.display()))?
    {
        let entry = entry.map_err(|error| format!("read_dir entry {}: {error}", root.display()))?;
        let path = entry.path();

        if path.is_dir() {
            collect_files_recursive(path.as_path(), output)?;
            continue;
        }

        output.push(path);
    }

    Ok(())
}

/// Copy directory contents recursively.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }

    for entry in
        fs::read_dir(src).map_err(|error| format!("read_dir {}: {error}", src.display()))?
    {
        let entry = entry.map_err(|error| format!("read_dir entry {}: {error}", src.display()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)
                .map_err(|error| format!("create_dir_all {}: {error}", dst_path.display()))?;
            copy_dir_recursive(src_path.as_path(), dst_path.as_path())?;
            continue;
        }

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create_dir_all {}: {error}", parent.display()))?;
        }

        if dst_path.exists() {
            let src_bytes = fs::read(src_path.as_path())
                .map_err(|error| format!("read {}: {error}", src_path.display()))?;
            let dst_bytes = fs::read(dst_path.as_path())
                .map_err(|error| format!("read {}: {error}", dst_path.display()))?;
            if src_bytes == dst_bytes {
                continue;
            }
        }

        fs::copy(src_path.as_path(), dst_path.as_path()).map_err(|error| {
            format!(
                "copy {} -> {}: {error}",
                src_path.display(),
                dst_path.display()
            )
        })?;
    }

    Ok(())
}

/// Run a git command in a directory and return a descriptive error on failure.
fn run_git(args: &[&str], current_dir: &Path, context: &str) -> Result<(), String> {
    let status = Command::new("git")
        .args(args)
        .current_dir(current_dir)
        .status()
        .map_err(|error| {
            format!(
                "git {context} failed to start in {}: {error}",
                current_dir.display()
            )
        })?;

    if status.success() {
        return Ok(());
    }

    Err(format!(
        "git {context} failed in {} with args: {:?}",
        current_dir.display(),
        args,
    ))
}

/// Auto fetch missing package checkouts before running tests.
fn auto_fetch_missing_checkouts(
    manifests: &[EcosystemManifest],
    phases: &[EcosystemPhase],
    filter: Option<&str>,
    checkouts_dir: &Path,
    patches_dir: &Path,
    is_list_mode: bool,
) -> HashMap<String, String> {
    if is_list_mode {
        return HashMap::new();
    }

    let mut missing = Vec::new();
    for manifest in manifests {
        if !manifest_matches_filter(manifest, phases, filter) {
            continue;
        }

        let package_dir = checkouts_dir.join(&manifest.package.name);
        if !package_dir.exists() {
            missing.push(manifest);
        }
    }

    if missing.is_empty() {
        return HashMap::new();
    }

    println!();
    println!(
        "auto-fetching {} missing ecosystem checkouts",
        missing.len()
    );

    let mut failures = HashMap::new();
    for manifest in missing {
        print!(
            "  {}@{} ... ",
            manifest.package.name, manifest.package.git_ref
        );
        let fetch_result = fetch_package(manifest, checkouts_dir, FetchOptions { refresh: false });
        match fetch_result {
            Ok(package_dir) => {
                let package_patches_dir = patches_dir.join(&manifest.package.name);
                match ensure_patches_applied(&package_patches_dir, &package_dir) {
                    Ok(()) => {
                        println!("ok");
                    }
                    Err(error) => {
                        println!("FAILED: patch apply failed: {error}");
                        failures.insert(manifest.package.name.clone(), error);
                    }
                }
            }
            Err(error) => {
                println!("FAILED: {error}");
                failures.insert(manifest.package.name.clone(), error);
            }
        }
    }

    println!();
    failures
}

/// Return whether one manifest can produce a selected case for the current filter.
fn manifest_matches_filter(
    manifest: &EcosystemManifest,
    phases: &[EcosystemPhase],
    filter: Option<&str>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    if manifest.package.name.contains(filter) {
        return true;
    }

    phases
        .iter()
        .copied()
        .any(|phase| case_id_for(manifest.package.name.as_str(), phase).contains(filter))
}

/// Fetch one package checkout into checkouts directory.
pub fn fetch_package(
    manifest: &EcosystemManifest,
    checkouts_dir: &Path,
    options: FetchOptions,
) -> Result<PathBuf, String> {
    let package_dir = checkouts_dir.join(&manifest.package.name);

    fs::create_dir_all(checkouts_dir).map_err(|error| {
        format!(
            "failed to create checkouts dir {}: {error}",
            checkouts_dir.display()
        )
    })?;

    if package_dir.exists() && !options.refresh {
        // update existing clone to requested ref and clean local mutations
        run_git(
            &["fetch", "--depth=1", "origin", &manifest.package.git_ref],
            &package_dir,
            "fetch",
        )?;
        run_git(
            &["checkout", "--force", &manifest.package.git_ref],
            &package_dir,
            "checkout",
        )?;
        run_git(&["reset", "--hard"], &package_dir, "reset")?;
        run_git(&["clean", "-fd"], &package_dir, "clean")?;
        return Ok(package_dir);
    }

    if package_dir.exists() {
        fs::remove_dir_all(&package_dir).map_err(|error| {
            format!(
                "failed to remove existing package dir {}: {error}",
                package_dir.display()
            )
        })?;
    }

    run_git(
        &[
            "clone",
            "--no-checkout",
            "--filter=blob:none",
            &manifest.package.repo,
            &manifest.package.name,
        ],
        checkouts_dir,
        "clone",
    )?;

    run_git(
        &["checkout", "--force", &manifest.package.git_ref],
        &package_dir,
        "checkout",
    )?;

    Ok(package_dir)
}

/// Fetch all package checkouts declared by ecosystem manifests.
pub fn fetch_all_packages(options: FetchOptions) -> ExitCode {
    let ecosystem_dir = fixtures_dir().join("ecosystem");
    let packages_dir = ecosystem_dir.join("packages");
    let checkouts_dir = ecosystem_dir.join("checkouts");
    let patches_dir = ecosystem_dir.join("patches");

    let manifest_paths = EcosystemManifest::discover_all(&packages_dir);
    println!("fetching {} packages...", manifest_paths.len());

    let mut any_failed = false;
    for path in manifest_paths {
        match EcosystemManifest::load(&path) {
            Ok(manifest) => {
                print!(
                    "  {}@{} ... ",
                    manifest.package.name, manifest.package.git_ref
                );
                match fetch_package(&manifest, &checkouts_dir, options) {
                    Ok(package_dir) => {
                        let package_patches_dir = patches_dir.join(&manifest.package.name);
                        match ensure_patches_applied(&package_patches_dir, &package_dir) {
                            Ok(()) => println!("ok"),
                            Err(error) => {
                                println!("FAILED: patch apply failed: {error}");
                                any_failed = true;
                            }
                        }
                    }
                    Err(error) => {
                        println!("FAILED: {error}");
                        any_failed = true;
                    }
                }
            }
            Err(error) => {
                eprintln!("  error loading {}: {error}", path.display());
                any_failed = true;
            }
        }
    }

    if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ReadmeCellStatus, format_readme_summary_rows, language_type_for_parse,
        parse_readme_summary_rows, path_is_supported_source, replace_readme_section,
    };
    use crate::ecosystem::manifest::{CompilerOptionsConfig, EcosystemPhase};
    use destack_source::{FileType, LanguageType};
    use std::collections::BTreeMap;
    use std::path::Path;

    #[test]
    fn test_parse_readme_summary_rows_keeps_ignored_cells() {
        // include ignored cells in package rows and ensure we still parse them
        let content = r#"
<!-- begin:summary-results -->
| Package | parse | resolve | analyze | lower | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:--------|:--------:|:--------:|:--------:|:--------:|-------:|-------:|--------:|------:|--------:|-----------:|
| astro |   ---    |    x     |    ✓     |   ---    |     1  |     1  |       2  |     4 |  50.00% |    25.00% |
<!-- end:summary-results -->
"#;

        // parse the row and assert each phase cell maps to the expected status
        let rows = parse_readme_summary_rows(content).unwrap();
        let row = rows.get("astro").unwrap();
        assert_eq!(
            row.get(&EcosystemPhase::Parse).copied(),
            Some(ReadmeCellStatus::Ignored)
        );
        assert_eq!(
            row.get(&EcosystemPhase::Resolve).copied(),
            Some(ReadmeCellStatus::Fail)
        );
        assert_eq!(
            row.get(&EcosystemPhase::Analyze).copied(),
            Some(ReadmeCellStatus::Pass)
        );
        assert_eq!(
            row.get(&EcosystemPhase::Lower).copied(),
            Some(ReadmeCellStatus::Ignored)
        );
    }

    #[test]
    fn test_format_readme_summary_rows_total_phase_cells_show_run_counts() {
        let mut rows = BTreeMap::new();

        let mut astro = BTreeMap::new();
        astro.insert(EcosystemPhase::Parse, ReadmeCellStatus::Fail);
        astro.insert(EcosystemPhase::Resolve, ReadmeCellStatus::Ignored);
        astro.insert(EcosystemPhase::Analyze, ReadmeCellStatus::Ignored);
        astro.insert(EcosystemPhase::Lower, ReadmeCellStatus::Ignored);
        rows.insert("astro".to_string(), astro);

        let mut semver = BTreeMap::new();
        semver.insert(EcosystemPhase::Parse, ReadmeCellStatus::Pass);
        semver.insert(EcosystemPhase::Resolve, ReadmeCellStatus::Pass);
        semver.insert(EcosystemPhase::Analyze, ReadmeCellStatus::Ignored);
        semver.insert(EcosystemPhase::Lower, ReadmeCellStatus::Ignored);
        rows.insert("semver".to_string(), semver);

        let table = format_readme_summary_rows(&rows);
        let total_line = table
            .lines()
            .find(|line| line.trim_start().starts_with("| total"))
            .unwrap();

        assert!(total_line.contains("|   1/2    "));
        assert!(total_line.contains("|   1/1    "));
        assert!(total_line.contains("|   ---    "));
    }

    #[test]
    fn test_replace_readme_section_uses_end_marker_after_begin() {
        // ensure replacement uses the matching end marker after the section begin marker
        let content = r#"
<!-- end:summary-results -->
prefix
<!-- begin:summary-results -->
old content
<!-- end:summary-results -->
suffix
"#;

        let new_content =
            replace_readme_section(content, "summary-results", "new content").unwrap();

        assert!(
            new_content.contains(
                "<!-- begin:summary-results -->\nnew content\n<!-- end:summary-results -->"
            )
        );
        assert!(new_content.contains("prefix"));
        assert!(new_content.contains("suffix"));
    }

    #[test]
    fn test_path_is_supported_source_accepts_typescript_declaration() {
        assert!(path_is_supported_source(Path::new("index.d.ts")));
        assert!(path_is_supported_source(Path::new("index.d.mts")));
        assert!(path_is_supported_source(Path::new("index.d.cts")));
    }

    #[test]
    fn test_language_type_for_parse_promotes_js_to_jsx_for_jsx_packages() {
        let compiler_options = CompilerOptionsConfig {
            js_as_jsx: Some(true),
        };

        let language = language_type_for_parse(FileType::JavaScript, &compiler_options);
        assert_eq!(language, LanguageType::JavaScriptXml);
    }

    #[test]
    fn test_language_type_for_parse_keeps_js_without_jsx_tag() {
        let compiler_options = CompilerOptionsConfig { js_as_jsx: None };

        let language = language_type_for_parse(FileType::JavaScript, &compiler_options);
        assert_eq!(language, LanguageType::JavaScript);
    }

    #[test]
    fn test_language_type_for_parse_keeps_typescript_declaration() {
        let compiler_options = CompilerOptionsConfig {
            js_as_jsx: Some(true),
        };

        let language = language_type_for_parse(FileType::TypeScriptDeclaration, &compiler_options);
        assert_eq!(language, LanguageType::TypeScriptDeclaration);
    }
}
