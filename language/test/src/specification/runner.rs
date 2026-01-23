use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, OptimizeTask};
use destack_parser::source_colorizer;
use destack_source::{
    File, FileType, MemoryFileSystem, ModuleStamp, PrintOptions, ProfileStamp, Uri,
};
use destack_workspace::{
    DsConfig, DsConfigOptions, DsConfigTargetOptions, MemoryCacheStore, OutputFormat, TargetId,
};

use crate::harness::print::color;
use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, fixtures_dir, format_diagnostics,
};
use crate::mdtest::{
    MdTestCase, TEST_TIMEOUT_SECONDS, discover_md_files, parse_mdtest_file, run_with_timeout,
    select_profile_for_mdtest, setup_test_environment_with_session, slug,
};

/// Test suite for type checking specification tests.
#[derive(Debug, Default)]
pub struct SpecificationSuite {
    /// Map from test full name to the parsed test case.
    tests: HashMap<String, MdTestCase>,
    /// List of discovered test cases.
    cases: Vec<TestCase>,
}

impl SpecificationSuite {
    /// Load all specification tests from the fixtures/specification directory.
    pub fn load() -> Self {
        let fixtures = fixtures_dir();
        let mut suite = Self::default();

        let spec_dir = fixtures.join("specification");
        for md_path in discover_md_files(&spec_dir).unwrap_or_default() {
            suite.add_file(&spec_dir, &md_path);
        }

        suite
    }

    fn add_file(&mut self, base_dir: &Path, md_path: &Path) {
        let cases = match parse_mdtest_file(md_path) {
            Ok(cases) => cases,
            Err(error) => {
                eprintln!("failed to parse {}: {error}", md_path.display());
                return;
            }
        };

        let relative_path = md_path.strip_prefix(base_dir).unwrap_or(md_path);
        let relative_name = relative_path.to_string_lossy();

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

    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult {
        let Some(md_test) = self.tests.get(&case.full_name()) else {
            return TestResult::Failed {
                message: "test not found".to_string(),
            };
        };

        let timeout = context
            .timeout
            .unwrap_or_else(|| Duration::from_secs(TEST_TIMEOUT_SECONDS));
        run_with_timeout(md_test.clone(), timeout, run_specification_test)
    }

    fn timeout(&self) -> Option<Duration> {
        // timeout is enforced by run_with_timeout
        None
    }
}

#[derive(Debug)]
struct SharedSpecEnvironment {
    /// The shared test session.
    session: Arc<destack_workspace::Session>,
    /// The shared in-memory file system.
    fs: Arc<MemoryFileSystem>,
    /// The next unique test id.
    next_id: AtomicUsize,
}

impl SharedSpecEnvironment {
    /// Create a new shared environment for spec tests.
    fn new() -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let cwd = PathBuf::from("/test/spec");
        let session = Arc::new(
            destack_workspace::Session::new(cwd.clone())
                .with_fs(fs.clone())
                .with_cache_store(Arc::new(MemoryCacheStore::new())),
        );
        Self {
            session,
            fs,
            next_id: AtomicUsize::new(0),
        }
    }

    /// Allocate a unique root directory for a test case.
    fn root_for(&self, test: &MdTestCase) -> PathBuf {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let section = slug(&test.section);
        let name = slug(&test.name);
        PathBuf::from("/test/spec").join(format!("{section}-{name}-{id}"))
    }
}

thread_local! {
    static SHARED_SPEC_ENV: SharedSpecEnvironment = SharedSpecEnvironment::new();
}

/// Run a single spec test: compile the code and compare errors against expectations.
fn run_specification_test(test: &MdTestCase) -> TestResult {
    let (session, program, main_path) = SHARED_SPEC_ENV.with(|env| {
        let root = env.root_for(test);
        setup_test_environment_with_session(test, env.session.clone(), env.fs.clone(), root)
    });

    // compile with single worker for deterministic results
    let mut compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            load_libs: false,
            workers: 1,
            ..Default::default()
        },
    );

    // find the main file to compile
    let module_id = match compiler.resolve_path_to_module(&main_path) {
        Ok(id) => id,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to resolve module: {e:?}"),
            };
        }
    };

    // apply dsconfig.json for compiler options and targets
    let prefer_native = test_option_bool(test, "native").unwrap_or(false);
    if let Err(error) = apply_dsconfig_for_spec(&program, module_id, &main_path, prefer_native) {
        return TestResult::Failed { message: error };
    }

    // run analysis
    let (profile, load_libs) = select_profile_for_mdtest(&program, module_id, test, prefer_native);
    compiler.options.load_libs = load_libs;
    let module_version = program.modules.get(module_id).read().version;
    let profile_version = program
        .profiles
        .get(profile)
        .unwrap_or_else(|| panic!("missing profile data for {profile:?}"))
        .version;
    compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate {
        module: ModuleStamp::new(module_id, module_version),
        profile: ProfileStamp::new(profile, profile_version),
    });

    // run optimize passes only for native spec tests
    let run_optimize = prefer_native;
    if run_optimize {
        let diagnostic_target = program.ensure_target_for_module(module_id);
        let diagnostic_profile =
            program.profile_id_for_target_or_default(module_id, &diagnostic_target);
        let diagnostic_profile_version = program
            .profiles
            .get(diagnostic_profile)
            .unwrap_or_else(|| panic!("missing profile data for {diagnostic_profile:?}"))
            .version;
        compiler.enqueue(OptimizeTask::OptimizeModule {
            module: ModuleStamp::new(module_id, module_version),
            profile: ProfileStamp::new(diagnostic_profile, diagnostic_profile_version),
            target: diagnostic_target,
        });
    }
    compiler.compile();
    drop(compiler);

    // collect actual errors
    let diagnostics = program.diagnostics.collect();
    let diagnostics_vec = diagnostics.iter();
    let actual_errors: Vec<String> = diagnostics_vec
        .iter()
        .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
        .map(|d| d.message.clone())
        .collect();

    // compare against expected errors (bullet items in markdown)
    let result = compare_errors(&test.bullet_items, &actual_errors);

    // on failure, append rendered diagnostics for context
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

fn apply_dsconfig_for_spec(
    program: &destack_workspace::Program,
    module_id: destack_source::ModuleId,
    main_path: &Path,
    prefer_native: bool,
) -> Result<(), String> {
    // locate dsconfig.json in the test root
    let root = main_path.parent().unwrap_or_else(|| Path::new("/"));
    let dsconfig_path = root.join("dsconfig.json");

    // skip when no dsconfig.json exists
    let has_dsconfig = program
        .fs
        .exists(&dsconfig_path)
        .map_err(|e| format!("failed to stat dsconfig.json: {e}"))?;
    if !has_dsconfig {
        // default spec tests to js unless explicitly marked native
        if prefer_native {
            return Ok(());
        }

        let file_id = program.files.next_id();
        let mut options = DsConfigOptions::default();
        let target = DsConfigTargetOptions {
            output: OutputFormat::Js,
            ..Default::default()
        };
        options.targets.insert("default".to_string(), target);
        options.default_target = Some("default".to_string());

        let dsconfig = DsConfig {
            file_id,
            path: dsconfig_path.clone(),
            directory: root.to_path_buf(),
            options,
            content: Default::default(),
        };

        let package_id = {
            let module = program.modules.get(module_id);
            let module = module.read();
            module.package_id
        };
        let package = program.packages.get(package_id);
        let mut package = package.write();
        package.dsconfig = Some(dsconfig.clone());
        package.targets.clear();
        for (name, options) in dsconfig.options.targets.iter() {
            let target = options.to_target(name);
            let target_id = TargetId::new(package_id, name);
            package.targets.insert(target_id, target);
        }

        return Ok(());
    }

    // read and parse dsconfig.json
    let content = program
        .fs
        .read_to_string(&dsconfig_path)
        .map_err(|e| format!("failed to read dsconfig.json: {e}"))?;
    let name = dsconfig_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let uri = Uri::from_path(&dsconfig_path);
    let file_id = program.files.next_id();
    let file = File::from_text_as_jsonc(
        file_id,
        name,
        uri,
        Some(dsconfig_path),
        FileType::Json,
        content,
    )
    .map_err(|e| format!("failed to parse dsconfig.json: {e}"))?;
    let file = Arc::new(file);
    let mut dsconfig = DsConfig::parse(&file).map_err(|e| e.to_string())?;

    // prefer js defaults for spec tests unless a native target is required
    if !prefer_native && dsconfig.options.targets.is_empty() {
        let target = DsConfigTargetOptions {
            output: OutputFormat::Js,
            ..Default::default()
        };
        dsconfig
            .options
            .targets
            .insert("default".to_string(), target);
        dsconfig.options.default_target = Some("default".to_string());
    }

    // attach dsconfig and targets to the module package
    let package_id = {
        let module = program.modules.get(module_id);
        let module = module.read();
        module.package_id
    };
    let package = program.packages.get(package_id);
    let mut package = package.write();
    package.dsconfig = Some(dsconfig.clone());
    package.targets.clear();
    for (name, options) in dsconfig.options.targets.iter() {
        let target = options.to_target(name);
        let target_id = TargetId::new(package_id, name);
        package.targets.insert(target_id, target);
    }

    Ok(())
}

/// Compare expected errors against actual errors.
fn compare_errors(expected: &[String], actual: &[String]) -> TestResult {
    let expected_patterns: Vec<ExpectedError> =
        expected.iter().map(|s| ExpectedError::parse(s)).collect();
    let actual_normalized: Vec<String> = actual.iter().map(|s| normalize_error(s)).collect();

    // find expected errors that didn't occur
    let mut missing: Vec<&str> = Vec::new();
    for expected in &expected_patterns {
        if !actual_normalized.iter().any(|a| expected.matches(a)) {
            missing.push(expected.as_str());
        }
    }

    // find actual errors that weren't expected
    let mut unexpected: Vec<&str> = Vec::new();
    for act in &actual_normalized {
        if !expected_patterns.iter().any(|e| e.matches(act)) {
            unexpected.push(act.as_str());
        }
    }

    // if no errors are missing or unexpected, return passed
    if missing.is_empty() && unexpected.is_empty() {
        return TestResult::Passed;
    }

    // build failure message
    let mut message = String::new();
    if !missing.is_empty() {
        message.push_str(&format!("{}\n", color::red("missing expected errors:")));
        for err in &missing {
            message.push_str(&format!("  {}\n", color::red(&format!("- {err}"))));
        }
    }
    if !unexpected.is_empty() {
        if !message.is_empty() {
            message.push('\n');
        }
        message.push_str(&format!(
            "{}\n",
            color::green("additional unexpected errors:")
        ));
        for err in &unexpected {
            message.push_str(&format!("  {}\n", color::green(&format!("+ {err}"))));
        }
    }

    TestResult::Failed { message }
}

/// Normalize an error message for fuzzy comparison.
fn normalize_error(s: &str) -> String {
    let s = s.trim().to_lowercase();
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
            ExpectedError::Contains(expected) => actual.contains(expected),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            ExpectedError::Exact(s) => s,
            ExpectedError::Contains(s) => s,
        }
    }
}
