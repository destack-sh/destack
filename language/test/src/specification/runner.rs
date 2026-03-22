use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use destack_compiler::{Compiler, CompilerOptions};
use destack_parser::source_colorizer;
use destack_source::{File, FileType, MemoryFileSystem, ModuleId, PrintOptions, Uri};
use destack_workspace::{
    ArtifactKey, Destack, EmitFormat, MemoryCacheStore, Session, TargetId, TargetOptions,
};
use serde_json::json;

use crate::harness::print::color;
use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, fixtures_dir, format_diagnostics,
    save_expected_failures,
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
    cases: Vec<TestCase>,
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
            let test_case =
                TestCase::file(name, md_path.to_path_buf(), "destack_test::specification")
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

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn expected_failures(&self, _options: &TestOptions) -> Option<&HashSet<String>> {
        if self.expected_failures.is_empty() {
            None
        } else {
            Some(&self.expected_failures)
        }
    }

    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult {
        // resolve the parsed md test
        let Some(md_test) = self.tests.get(&case.full_name()) else {
            return TestResult::Failed {
                message: "test not found".to_string(),
            };
        };

        // select timeout and run the test
        let timeout = context
            .timeout
            .unwrap_or_else(|| context.options.mdtest_timeout());
        run_with_timeout(md_test.clone(), timeout, run_specification_test)
    }

    fn timeout(&self) -> Option<Duration> {
        // timeout enforced by run_with_timeout
        None
    }

    fn report(&self, results: &[(TestCase, TestResult)], context: &RunContext<'_>) {
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
fn run_specification_test(test: &MdTestCase) -> TestResult {
    // build one isolated in-memory environment per test
    let (session, program, root, main_path) = {
        let root = specification_root_for(test);
        let cwd = PathBuf::from("/test/spec");
        let fs = Arc::new(MemoryFileSystem::new());
        let session = Arc::new(
            Session::new(cwd)
                .with_fs(fs.clone())
                .with_cache_store(Arc::new(MemoryCacheStore::new())),
        );
        crate::mdtest::setup_test_environment_with_session(test, session, fs, root)
    };
    let prefer_native = test_option_bool(test, "native").unwrap_or(false);
    let verify_mir = !prefer_native;

    // compile with single worker for deterministic results
    let mut compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            load_libraries: false,
            workers: 1,
            verify_mir,
            ..Default::default()
        },
    );

    // run the spec body, then always remove the isolated session root
    let result = (|| {
        // resolve the main module to compile
        let module_id = match compiler.resolve_path_to_module(&main_path) {
            Ok(id) => id,
            Err(e) => {
                return TestResult::Failed {
                    message: format!("failed to resolve module: {e:?}"),
                };
            }
        };

        // apply config options and targets
        if let Err(error) =
            apply_destack_config_for_spec(&program, module_id, &main_path, prefer_native)
        {
            return TestResult::Failed { message: error };
        }

        // select profile and lib loading
        let (profile, mut load_libraries) =
            select_profile_for_mdtest(&program, module_id, test, prefer_native);

        // native spec cases need builtin libraries for lowering
        if prefer_native {
            load_libraries = true;
        }

        // load libs only when explicitly requested
        if !load_libraries && has_explicit_libs(&program, module_id) {
            load_libraries = true;
        }

        // enqueue analysis task
        compiler.options.load_libraries = load_libraries;
        compiler.enqueue(ArtifactKey::DirAnalyzed {
            module: module_id,
            profile,
        });

        // run optimize passes only for native spec tests
        let run_optimize = prefer_native;
        if run_optimize {
            // select target and profile for diagnostics
            let diagnostic_target = program.ensure_target_for_module(module_id);
            let diagnostic_profile =
                program.profile_id_for_target_or_default(module_id, &diagnostic_target);
            compiler.enqueue(ArtifactKey::MirOptimized {
                module: module_id,
                profile: diagnostic_profile,
                target: diagnostic_target,
            });
        }

        compiler.compile();
        drop(compiler);

        // collect actual diagnostics
        let diagnostics = program.diagnostics.collect();
        let diagnostics_vec = diagnostics.iter();
        let actual_errors: Vec<String> = diagnostics_vec
            .iter()
            .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
            .map(|d| d.message.clone())
            .collect();
        let actual_warnings: Vec<String> = diagnostics_vec
            .iter()
            .filter(|d| d.severity == destack_source::DiagnosticSeverity::Warning)
            .map(|d| d.message.clone())
            .collect();

        // split expected errors and warnings from bullet items
        let (expected_errors, expected_warnings) = split_expected_diagnostics(&test.bullet_items);

        // compare against expected diagnostics
        let error_result = compare_expected("error", &expected_errors, &actual_errors);
        let warning_result = if expected_warnings.is_empty() {
            TestResult::Passed
        } else {
            compare_expected("warning", &expected_warnings, &actual_warnings)
        };
        let result = merge_results(error_result, warning_result);

        // append rendered diagnostics for failures
        match result {
            TestResult::Failed { mut message } => {
                let options = PrintOptions::new().with_colorizer(source_colorizer());
                let rendered = format_diagnostics(&program.files, &diagnostics, options);
                if !rendered.is_empty() {
                    if !message.is_empty() {
                        message.push('\n');
                        message.push('\n');
                    }
                    message.push_str(&rendered);
                }
                TestResult::Failed { message }
            }
            other => other,
        }
    })();

    let _ = session.remove_root(&root);

    result
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

fn has_explicit_libs(program: &destack_workspace::Program, module_id: ModuleId) -> bool {
    // resolve the module package
    let package_id = {
        let module = program.modules.get(module_id);
        let module = module.as_ref();
        module.package_id
    };
    let package = program.packages.get(package_id);
    let package = package.read();
    let Some(config) = package.config.as_ref() else {
        return false;
    };

    // check compiler lib entries
    if !config.options.compiler.lib.is_empty() {
        return true;
    }

    // check target lib entries
    config
        .options
        .targets
        .values()
        .any(|target| target.lib.is_some())
}

fn apply_destack_config_for_spec(
    program: &destack_workspace::Program,
    module_id: destack_source::ModuleId,
    main_path: &Path,
    prefer_native: bool,
) -> Result<(), String> {
    // locate destack.json in the test root
    let root = main_path.parent().unwrap_or_else(|| Path::new("/"));
    let destack_config_path = root.join("destack.json");

    // skip when no destack.json exists
    let has_destack_config = program
        .fs
        .exists(&destack_config_path)
        .map_err(|e| format!("failed to stat destack.json: {e}"))?;
    if !has_destack_config {
        // default spec tests to js unless explicitly marked native
        if prefer_native {
            return Ok(());
        }

        // materialize a real default config so later path based lookups stay honest
        materialize_default_spec_destack_config(program, &destack_config_path)
            .map_err(|error| format!("failed to write default destack.json: {error}"))?;
    }

    // read and parse destack.json through the real JSONC-aware loader
    let content = program
        .fs
        .read_to_string(&destack_config_path)
        .map_err(|error| format!("failed to read destack.json: {error}"))?;
    let name = destack_config_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let uri = Uri::from_path(&destack_config_path);
    let file_id = program.files.next_id();
    let file = File::from_text_as_jsonc(
        file_id,
        name,
        uri,
        Some(destack_config_path),
        FileType::Json,
        content,
    )
    .map_err(|error| format!("failed to parse destack.json: {error}"))?;
    let file = Arc::new(file);
    let mut config = Destack::parse(&file).map_err(|error| error.to_string())?;

    // prefer js defaults for spec tests unless a native target is required
    if !prefer_native && config.options.targets.is_empty() {
        let target = TargetOptions {
            emit: EmitFormat::Js,
            ..Default::default()
        };
        config.options.targets.insert("default".to_string(), target);
        config.options.default_target = Some("default".to_string());
    }

    // attach the config and targets to the module package
    let package_id = {
        let module = program.modules.get(module_id);
        let module = module.as_ref();
        module.package_id
    };
    let package = program.packages.get(package_id);
    let mut package = package.write();
    package.config = Some(config.clone());
    package.targets.clear();
    for (name, options) in config.options.targets.iter() {
        let target = options.to_target(name);
        let target_id = TargetId::new(package_id, name);
        package.targets.insert(target_id, target);
    }
    Ok(())
}

/// Materialize the default spec `destack.json` for TypeScript checking.
fn materialize_default_spec_destack_config(
    program: &destack_workspace::Program,
    destack_config_path: &Path,
) -> std::io::Result<()> {
    // build the default spec config payload
    let content = json!({
        "compiler": {
            "checkTs": true,
            "checkJs": true
        },
        "targets": {
            "default": {
                "emit": "js"
            }
        },
        "defaultTarget": "default"
    });
    let content = serde_json::to_string_pretty(&content)
        .expect("default spec config serialization should succeed");

    // write the config file through the shared filesystem
    program.fs.write_string(destack_config_path, &content)
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
fn compare_expected(kind: &str, expected: &[String], actual: &[String]) -> TestResult {
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
        return TestResult::Passed;
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

    TestResult::Failed { message }
}

/// Merge two diagnostic comparison results.
fn merge_results(first: TestResult, second: TestResult) -> TestResult {
    // merge two diagnostic comparison results
    match (first, second) {
        (TestResult::Passed, TestResult::Passed) => TestResult::Passed,
        (TestResult::Failed { message }, TestResult::Passed)
        | (TestResult::Passed, TestResult::Failed { message }) => TestResult::Failed { message },
        (TestResult::Failed { message: left }, TestResult::Failed { message: right }) => {
            let message = format!("{left}\n\n{right}");
            TestResult::Failed { message }
        }
        (TestResult::Skipped { reason }, _) | (_, TestResult::Skipped { reason }) => {
            TestResult::Skipped { reason }
        }
        (TestResult::Suite { .. }, other) | (other, TestResult::Suite { .. }) => other,
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
