use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use destack_artifact::{ArtifactKey, MemoryCacheStore};
use destack_compiler::Compiler;
use destack_parser::source_colorizer;
use destack_source::{
    DiagnosticSeverity, File, FileType, MemoryFileSystem, ModuleId, PrintOptions, TargetId, Uri,
};
use destack_workspace::{HostEnvironment, Repository, Revision, parse_jsonc_file};
use serde_json::json;

use crate::core::print::color;
use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Suite, current_workspace_revision, fixtures_dir,
    format_diagnostics, module_artifact_diagnostics, module_target_artifact_diagnostics,
    profile_id_for_target_or_default, provide_workspace_artifacts, save_expected_failures,
    write_workspace_text_file,
};
use crate::mdtest::{
    MdTestCase, discover_md_files, load_mdtest_expected_failures, parse_mdtest_file,
    run_with_timeout, select_profile_for_mdtest, slug,
};

/// Test suite for type checking specification tests.
#[derive(Debug, Default)]
pub struct SpecificationSuite {
    /// Map from test full name to the parsed test case.
    tests: HashMap<String, MdTestCase>,
    /// List of discovered test cases.
    cases: Vec<Case>,
    /// Known failing tests for baseline tracking.
    expected_failures: HashSet<String>,
    /// Location of the known failures file.
    expected_failures_path: PathBuf,
}

impl SpecificationSuite {
    /// Load all specification tests from the fixtures/specification directory.
    pub fn load() -> Self {
        // setup fixtures and suite container
        let fixtures = fixtures_dir();
        let mut suite = Self::default();

        // discover spec markdown files
        let spec_dir = fixtures.join("specification");
        for md_path in discover_md_files(&spec_dir).unwrap_or_default() {
            suite.add_file(&spec_dir, &md_path);
        }

        suite.expected_failures = load_mdtest_expected_failures(&spec_dir);
        suite.expected_failures_path = spec_dir.join("known-failures.txt");
        suite
    }

    fn add_file(&mut self, base_dir: &Path, md_path: &Path) {
        // parse tests from the markdown file
        let cases = match parse_mdtest_file(md_path) {
            Ok(cases) => cases,
            Err(error) => {
                panic!("failed to parse {}: {error}", md_path.display());
            }
        };

        // compute file path labels
        let relative_path = md_path.strip_prefix(base_dir).unwrap_or(md_path);
        let relative_name = relative_path.to_string_lossy();

        // register each test case
        for case in cases {
            let name = format!(
                "{relative_name}/{}/{}",
                slug(&case.section),
                slug(&case.name)
            );
            let test_case = Case::file(name, md_path.to_path_buf(), "destack_test::specification")
                .with_skipped(case.skip);

            self.tests.insert(test_case.full_name(), case);
            self.cases.push(test_case);
        }
    }
}

impl Suite for SpecificationSuite {
    fn name(&self) -> &'static str {
        "specification"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn expected_failures(&self, _options: &RunOptions) -> Option<&HashSet<String>> {
        if self.expected_failures.is_empty() {
            None
        } else {
            Some(&self.expected_failures)
        }
    }

    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult {
        // resolve the parsed md test
        let Some(md_test) = self.tests.get(&case.full_name()) else {
            return CaseResult::Failed {
                message: "test not found".to_string(),
            };
        };

        // select timeout and run the test
        let timeout = context
            .timeout
            .unwrap_or_else(|| context.options.case_timeout());
        run_with_timeout(md_test.clone(), timeout, run_specification_test)
    }

    fn timeout(&self) -> Option<Duration> {
        // timeout enforced by run_with_timeout
        None
    }

    fn report(&self, results: &[(Case, CaseResult)], context: &RunContext<'_>) {
        if !context.options.update_known_failures {
            return;
        }

        // collect current failures for baseline updates
        let mut failures = HashSet::new();
        for (case, result) in results {
            if result.is_failed() {
                failures.insert(case.full_name());
            }
        }

        if let Err(error) = save_expected_failures(&self.expected_failures_path, &failures) {
            eprintln!(
                "failed to update {}: {error}",
                self.expected_failures_path.display()
            );
            return;
        }

        println!(
            "  {} updated with {} failures",
            self.expected_failures_path.display(),
            failures.len()
        );
    }
}

/// Run a single spec test: compile the code and compare errors against expectations.
fn run_specification_test(test: &MdTestCase) -> CaseResult {
    // build one isolated in-memory environment per test
    let (repository, _root, main_path) = {
        let root = specification_root_for(test);
        let cwd = PathBuf::from("/test/spec");
        let fs = Arc::new(MemoryFileSystem::new());
        let repository = Arc::new(Repository::new(
            cwd,
            Arc::new(MemoryCacheStore::new()),
            fs.clone(),
            HostEnvironment::capture_process(),
        ));
        crate::mdtest::setup_test_environment_with_repository(test, repository, fs, root)
    };
    let prefer_native = test_option_bool(test, "native").unwrap_or(false);

    // compile with single worker for deterministic results
    let compiler = Compiler::new(repository.clone());

    // run the spec body, then always remove the isolated repository root
    (|| {
        let revision;

        // apply config options and targets
        let module_id = match apply_destack_config_for_spec(&repository, &main_path, prefer_native)
        {
            Ok((next_revision, next_module_id)) => {
                revision = next_revision;
                next_module_id
            }
            Err(error) => return CaseResult::Failed { message: error },
        };

        // select profile and lib loading
        let (profile, _load_libraries) =
            select_profile_for_mdtest(&repository, revision, module_id, test, prefer_native);
        let profile = profile.id();

        // enqueue check task
        let mut artifact_keys = vec![ArtifactKey::DirChecked {
            module: module_id,
            profile,
        }];

        // run optimize passes only for native spec tests
        let run_optimize = prefer_native;
        let mut diagnostic_target = None;
        let mut diagnostic_profile = None;
        if run_optimize {
            // select target and profile for diagnostics
            let module = repository
                .module(revision, module_id)
                .unwrap_or_else(|error| panic!("failed to read module: {error}"))
                .unwrap_or_else(|| panic!("missing module {module_id:?}"));
            let next_target = repository
                .package_default_target(revision, module.package_id)
                .unwrap_or_else(|error| panic!("failed to resolve diagnostic target: {error}"))
                .map(|(target_id, _)| target_id)
                .unwrap_or_else(|| TargetId::new(module.package_id, "default"));
            let next_profile =
                profile_id_for_target_or_default(&repository, revision, module_id, &next_target);
            diagnostic_target = Some(next_target);
            diagnostic_profile = Some(next_profile);
            artifact_keys.push(ArtifactKey::MirOptimized {
                module: module_id,
                profile: next_profile,
                target: next_target,
            });
        }

        let compiler = Arc::new(compiler);
        let revision =
            provide_workspace_artifacts(repository.clone(), compiler.clone(), &artifact_keys);
        drop(compiler);

        // collect actual diagnostics
        let diagnostics = if run_optimize {
            let diagnostic_target =
                diagnostic_target.expect("missing diagnostic target for optimized spec run");
            let diagnostic_profile =
                diagnostic_profile.expect("missing diagnostic profile for optimized spec run");
            module_target_artifact_diagnostics(
                &repository,
                revision,
                module_id,
                diagnostic_profile,
                diagnostic_target,
            )
        } else {
            module_artifact_diagnostics(&repository, revision, module_id, profile)
        };
        let actual_errors: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .map(|d| d.message.clone())
            .collect();
        let actual_warnings: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .map(|d| d.message.clone())
            .collect();

        // split expected errors and warnings from bullet items
        let (expected_errors, expected_warnings) = split_expected_diagnostics(&test.bullet_items);

        // compare against expected diagnostics
        let error_result = compare_expected("error", &expected_errors, &actual_errors);
        let warning_result = if expected_warnings.is_empty() {
            CaseResult::Passed
        } else {
            compare_expected("warning", &expected_warnings, &actual_warnings)
        };
        let result = merge_results(error_result, warning_result);

        // append rendered diagnostics for failures
        match result {
            CaseResult::Failed { mut message } => {
                let file_for_id = |file_id| {
                    repository.file(revision, file_id).unwrap_or_else(|error| {
                        panic!("failed to load diagnostic file {file_id:?}: {error}")
                    })
                };
                let options = PrintOptions::new().with_colorizer(source_colorizer());
                let rendered = format_diagnostics(&file_for_id, &diagnostics, options);
                if !rendered.is_empty() {
                    if !message.is_empty() {
                        message.push('\n');
                        message.push('\n');
                    }
                    message.push_str(&rendered);
                }
                CaseResult::Failed { message }
            }
            other => other,
        }
    })()
}

/// Build the isolated root directory for one specification case.
fn specification_root_for(test: &MdTestCase) -> PathBuf {
    let section = slug(&test.section);
    let name = slug(&test.name);
    PathBuf::from("/test/spec").join(format!("{section}-{name}"))
}

fn test_option_bool(test: &MdTestCase, key: &str) -> Option<bool> {
    // parse boolean test options
    let value = test.options.get(key)?;
    match value.trim().to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn apply_destack_config_for_spec(
    repository: &Repository,
    main_path: &Path,
    prefer_native: bool,
) -> Result<(Revision, ModuleId), String> {
    // locate destack.json in the test root
    let root = main_path.parent().unwrap_or_else(|| Path::new("/"));
    let destack_config_path = root.join("destack.json");

    // detect whether destack.json already exists on disk
    let has_destack_config = repository
        .file_system()
        .exists(&destack_config_path)
        .map_err(|e| format!("failed to stat destack.json: {e}"))?;
    if !has_destack_config && prefer_native {
        let revision = current_workspace_revision(repository);
        let module_id = repository
            .module_id_for_path(revision, main_path)
            .map_err(|error| format!("failed to resolve main module: {error}"))?
            .ok_or_else(|| "missing main module".to_string())?;

        return Ok((revision, module_id));
    }

    // load the existing config when present
    let mut value = if has_destack_config {
        let content = repository
            .file_system()
            .read_to_string(&destack_config_path)
            .map_err(|error| format!("failed to read destack.json: {error}"))?;
        let name = destack_config_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let uri = Uri::from_path(&destack_config_path);
        let file_id = repository.file_id(&destack_config_path);
        let file = File::from_text(
            file_id,
            name,
            uri,
            Some(destack_config_path.clone()),
            FileType::Json,
            content,
        );
        parse_jsonc_file(&file).map_err(|error| format!("failed to parse destack.json: {error}"))?
    } else {
        json!({})
    };

    // prefer js defaults for spec tests unless a native target is required
    let mut needs_write = !has_destack_config;
    let revision = current_workspace_revision(repository);
    let has_targets = if has_destack_config {
        let destack_config_file_id = repository.file_id(&destack_config_path);

        !repository
            .destack_declaration_for_file(revision, destack_config_file_id)
            .map_err(|error| format!("failed to load destack.json: {error}"))?
            .map(|declaration| declaration.as_ref().clone())
            .ok_or_else(|| "failed to load destack.json".to_string())?
            .package_options()
            .targets
            .is_empty()
    } else {
        false
    };

    if !prefer_native && !has_targets {
        value = json!({
            "compiler": {
                "checkTs": true,
                "checkJs": true,
            },
            "targets": {
                "default": {
                    "emit": "js",
                }
            },
            "defaultTarget": "default",
        });
        needs_write = true;
    }

    // materialize the config as real source when needed
    if needs_write {
        let content = serde_json::to_string_pretty(&value)
            .map_err(|error| format!("failed to serialize destack.json: {error}"))?;
        repository
            .file_system()
            .write_string(&destack_config_path, &content)
            .map_err(|error| format!("failed to write destack.json: {error}"))?;
        write_workspace_text_file(repository, &destack_config_path, &content);
    }

    let revision = current_workspace_revision(repository);
    let module_id = repository
        .module_id_for_path(revision, main_path)
        .map_err(|error| format!("failed to resolve main module after config update: {error}"))?
        .ok_or_else(|| "missing main module after config update".to_string())?;

    Ok((revision, module_id))
}
/// Split expected diagnostics into error and warning buckets.
fn split_expected_diagnostics(items: &[String]) -> (Vec<String>, Vec<String>) {
    // allocate result buckets
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // split bullet items into expected errors and warnings
    for item in items {
        let trimmed = item.trim();
        let lower = trimmed.to_lowercase();

        // prefer explicit warning prefix
        if lower.starts_with("warning:") {
            let rest = trimmed[("warning:".len())..].trim();
            warnings.push(rest.to_string());
            continue;
        }

        // accept short warning prefix
        if lower.starts_with("warn:") {
            let rest = trimmed[("warn:".len())..].trim();
            warnings.push(rest.to_string());
            continue;
        }

        // default to errors
        errors.push(trimmed.to_string());
    }

    // return the split expectations
    (errors, warnings)
}

/// Compare expected diagnostics against actual diagnostics.
fn compare_expected(kind: &str, expected: &[String], actual: &[String]) -> CaseResult {
    // normalize expected and actual diagnostics
    let expected_patterns: Vec<ExpectedError> =
        expected.iter().map(|s| ExpectedError::parse(s)).collect();
    let actual_normalized: Vec<String> = actual.iter().map(|s| normalize_error(s)).collect();

    // find expected diagnostics that did not occur
    let mut missing: Vec<&str> = Vec::new();
    for expected in &expected_patterns {
        if !actual_normalized.iter().any(|a| expected.matches(a)) {
            missing.push(expected.as_str());
        }
    }

    // find unexpected actual diagnostics
    let mut unexpected: Vec<&str> = Vec::new();
    for act in &actual_normalized {
        if !expected_patterns.iter().any(|e| e.matches(act)) {
            unexpected.push(act.as_str());
        }
    }

    // return success when nothing is missing or unexpected
    if missing.is_empty() && unexpected.is_empty() {
        return CaseResult::Passed;
    }

    // build failure message
    let mut message = String::new();
    if !missing.is_empty() {
        message.push_str(&format!(
            "{}\n",
            color::red(&format!("missing expected {kind}s:"))
        ));
        for diag in &missing {
            message.push_str(&format!("  {}\n", color::red(&format!("- {diag}"))));
        }
    }
    if !unexpected.is_empty() {
        // add unexpected diagnostics block
        if !message.is_empty() {
            message.push('\n');
        }
        message.push_str(&format!(
            "{}\n",
            color::green(&format!("additional unexpected {kind}s:"))
        ));
        for diag in &unexpected {
            message.push_str(&format!("  {}\n", color::green(&format!("+ {diag}"))));
        }
    }

    CaseResult::Failed { message }
}

/// Merge two diagnostic comparison results.
fn merge_results(first: CaseResult, second: CaseResult) -> CaseResult {
    // merge two diagnostic comparison results
    match (first, second) {
        (CaseResult::Passed, CaseResult::Passed) => CaseResult::Passed,
        (CaseResult::Failed { message }, CaseResult::Passed)
        | (CaseResult::Passed, CaseResult::Failed { message }) => CaseResult::Failed { message },
        (CaseResult::Failed { message: left }, CaseResult::Failed { message: right }) => {
            let message = format!("{left}\n\n{right}");
            CaseResult::Failed { message }
        }
        (CaseResult::Skipped { reason }, _) | (_, CaseResult::Skipped { reason }) => {
            CaseResult::Skipped { reason }
        }
        (CaseResult::Suite { .. }, other) | (other, CaseResult::Suite { .. }) => other,
    }
}

/// Normalize an error message for fuzzy comparison.
fn normalize_error(s: &str) -> String {
    // normalize whitespace and punctuation
    let s = s.trim().to_lowercase();
    let s = s.replace('`', "");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// An expected error pattern: either exact match or substring match.
#[derive(Debug, Clone)]
enum ExpectedError {
    /// Exact match against normalized error message.
    Exact(String),
    /// Substring match (use "contains: foo" in test).
    Contains(String),
}

impl ExpectedError {
    /// Parse an expected error string.
    /// Use "contains: foo" prefix for substring matching.
    fn parse(raw: &str) -> Self {
        let raw = raw.trim();
        let lower = raw.to_lowercase();
        if let Some(rest) = lower.strip_prefix("contains:") {
            return Self::Contains(normalize_error(rest));
        }
        Self::Exact(normalize_error(raw))
    }

    /// Check if an actual error matches this expectation.
    fn matches(&self, actual: &str) -> bool {
        match self {
            ExpectedError::Exact(expected) => actual == expected,
            ExpectedError::Contains(expected) => contains_with_type_placeholder(actual, expected),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            ExpectedError::Exact(s) => s,
            ExpectedError::Contains(s) => s,
        }
    }
}

/// Check if an error message contains an expected substring with type wildcards.
fn contains_with_type_placeholder(actual: &str, expected: &str) -> bool {
    // fast path for standard contains checks
    if !expected.contains("<<type>>") {
        return actual.contains(expected);
    }

    // split on the placeholder and match parts in order
    let mut offset = 0;
    for part in expected.split("<<type>>") {
        if part.is_empty() {
            continue;
        }

        let Some(position) = actual[offset..].find(part) else {
            return false;
        };

        offset += position + part.len();
    }

    true
}
