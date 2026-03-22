use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use destack_compiler::{BuildKey, Compiler, CompilerOptions};
use destack_parser::source_colorizer;
use destack_source::{File, FileType, MemoryFileSystem, ModuleId, PrintOptions, Uri};
use destack_workspace::{
    ArtifactKey, Destack, DestackOptions, OutputFormat, Session, TargetId, TargetOptions,
};

use crate::core::print::color;
use crate::core::{
    Case, CaseResult, MarkdownSuiteIndex, RunContext, RunOptions, SharedMemoryWorkspace, Suite,
    discover_markdown_suite, expected_failures_view, fixtures_dir, format_diagnostics,
    update_failure_baseline,
};
use crate::mdtest::{
    MdTestCase, run_with_timeout, select_profile_for_mdtest, setup_test_environment_with_session,
    slug,
};

/// The collected specification diagnostics for one case.
#[derive(Debug, Clone, Default)]
pub(super) struct SpecificationDiagnostics {
    /// The emitted error messages.
    pub errors: Vec<String>,
    /// The emitted warning messages.
    pub warnings: Vec<String>,
}

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
    pub fn load() -> Result<Self, String> {
        let fixtures = fixtures_dir();
        let spec_dir = fixtures.join("specification");
        let MarkdownSuiteIndex {
            cases,
            entries,
            expected_failures,
            expected_failures_path,
        } = discover_markdown_suite(&spec_dir, "destack_test::specification", Some)?;

        Ok(Self {
            tests: entries,
            cases,
            expected_failures,
            expected_failures_path,
        })
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
        expected_failures_view(&self.expected_failures)
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

        let failure_count = match update_failure_baseline(&self.expected_failures_path, results) {
            Ok(failure_count) => failure_count,
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };

        println!(
            "  {} updated with {} failures",
            self.expected_failures_path.display(),
            failure_count
        );
    }
}

#[derive(Debug)]
struct SharedSpecEnvironment {
    /// The shared in memory workspace.
    workspace: SharedMemoryWorkspace,
}

impl SharedSpecEnvironment {
    /// Create a new shared environment for spec tests.
    fn new() -> Self {
        Self {
            workspace: SharedMemoryWorkspace::new("/test/spec"),
        }
    }

    /// Allocate a unique root directory for a test case.
    fn root_for(&self, test: &MdTestCase) -> PathBuf {
        let section = slug(&test.section);
        let name = slug(&test.name);
        self.workspace.allocate_root(&format!("{section}-{name}"))
    }

    /// Return the shared session.
    fn session(&self) -> Arc<Session> {
        self.workspace.session()
    }

    /// Return the shared file system.
    fn fs(&self) -> Arc<MemoryFileSystem> {
        self.workspace.fs()
    }
}

thread_local! {
    static SHARED_SPEC_ENV: SharedSpecEnvironment = SharedSpecEnvironment::new();
}

/// Run a single spec test: compile the code and compare errors against expectations.
fn run_specification_test(test: &MdTestCase) -> CaseResult {
    let compiled = match compile_specification_test(test) {
        Ok(compiled) => compiled,
        Err(message) => return CaseResult::Failed { message },
    };

    let (expected_errors, expected_warnings) = split_expected_diagnostics(&test.bullet_items);
    let error_result = compare_expected("error", &expected_errors, &compiled.diagnostics.errors);
    let warning_result = if expected_warnings.is_empty() {
        CaseResult::Passed
    } else {
        compare_expected(
            "warning",
            &expected_warnings,
            &compiled.diagnostics.warnings,
        )
    };
    let result = merge_results(error_result, warning_result);

    match result {
        CaseResult::Failed { mut message } => {
            if !compiled.rendered_diagnostics.is_empty() {
                if !message.is_empty() {
                    message.push('\n');
                    message.push('\n');
                }
                message.push_str(&compiled.rendered_diagnostics);
            }
            CaseResult::Failed { message }
        }
        other => other,
    }
}

#[derive(Debug, Clone)]
pub(super) struct CompiledSpecificationTest {
    /// The collected diagnostics.
    pub diagnostics: SpecificationDiagnostics,
    /// The rendered diagnostics for failure output.
    pub rendered_diagnostics: String,
}

/// Compile one specification test and collect diagnostics.
pub(super) fn compile_specification_test(
    test: &MdTestCase,
) -> Result<CompiledSpecificationTest, String> {
    let (session, program, root, main_path) = SHARED_SPEC_ENV.with(|env| {
        let root = env.root_for(test);
        setup_test_environment_with_session(test, env.session(), env.fs(), root)
    });
    let prefer_native = test_option_bool(test, "native").unwrap_or(false);
    let verify_mir = !prefer_native;

    let mut compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            load_libs: false,
            workers: 1,
            verify_mir,
            ..Default::default()
        },
    );

    let result = (|| {
        let module_id = compiler
            .resolve_path_to_module(&main_path)
            .map_err(|error| format!("failed to resolve module: {error:?}"))?;

        apply_destack_config_for_spec(&program, module_id, &main_path, prefer_native)?;

        let (profile, mut load_libs) =
            select_profile_for_mdtest(&program, module_id, test, prefer_native);

        if !load_libs && has_explicit_libs(&program, module_id) {
            load_libs = true;
        }

        compiler.options.load_libs = load_libs;
        compiler.enqueue(BuildKey::Artifact(ArtifactKey::DirAnalyzed {
            module: module_id,
            profile,
        }));

        if prefer_native {
            let diagnostic_target = program.ensure_target_for_module(module_id);
            let diagnostic_profile =
                program.profile_id_for_target_or_default(module_id, &diagnostic_target);
            compiler.enqueue(BuildKey::Artifact(ArtifactKey::MirOptimized {
                module: module_id,
                profile: diagnostic_profile,
                target: diagnostic_target,
            }));
        }

        compiler.compile();
        drop(compiler);

        let diagnostics = program.diagnostics.collect();
        let diagnostics_vec = diagnostics.iter();
        let errors = diagnostics_vec
            .iter()
            .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
            .map(|d| d.message.clone())
            .collect();
        let warnings = diagnostics_vec
            .iter()
            .filter(|d| d.severity == destack_source::DiagnosticSeverity::Warning)
            .map(|d| d.message.clone())
            .collect();

        let options = PrintOptions::new().with_colorizer(source_colorizer());
        let rendered_diagnostics = format_diagnostics(&program.files, &diagnostics, options);

        Ok(CompiledSpecificationTest {
            diagnostics: SpecificationDiagnostics { errors, warnings },
            rendered_diagnostics,
        })
    })();

    let _ = session.remove_root(&root);

    result
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

        // build a default config for js output
        let file_id = program.files.next_id();
        let mut options = DestackOptions::default();
        options.compiler.check_ts = true;
        options.compiler.check_js = true;
        let target = TargetOptions {
            output: OutputFormat::Js,
            ..Default::default()
        };
        options.targets.insert("default".to_string(), target);
        options.default_target = Some("default".to_string());

        let config = Destack {
            file_id,
            path: destack_config_path.clone(),
            directory: root.to_path_buf(),
            options,
            content: Default::default(),
        };

        // attach the default config to the package
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

        return Ok(());
    }

    // read and parse destack.json
    let content = program
        .fs
        .read_to_string(&destack_config_path)
        .map_err(|e| format!("failed to read destack.json: {e}"))?;
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
    .map_err(|e| format!("failed to parse destack.json: {e}"))?;
    let file = Arc::new(file);
    let mut config = Destack::parse(&file).map_err(|e| e.to_string())?;

    // prefer js defaults for spec tests unless a native target is required
    if !prefer_native && config.options.targets.is_empty() {
        let target = TargetOptions {
            output: OutputFormat::Js,
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

/// Split expected diagnostics into error and warning buckets.
pub(super) fn split_expected_diagnostics(items: &[String]) -> (Vec<String>, Vec<String>) {
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
pub(super) fn compare_expected(kind: &str, expected: &[String], actual: &[String]) -> CaseResult {
    // normalize expected and actual diagnostics
    let expected_normalized: Vec<String> = expected.iter().map(|s| normalize_error(s)).collect();
    let actual_normalized: Vec<String> = actual.iter().map(|s| normalize_error(s)).collect();
    let mut unmatched_actuals = vec![true; actual_normalized.len()];

    // match expected diagnostics in order
    let mut missing: Vec<&str> = Vec::new();
    for expected in &expected_normalized {
        let Some(index) = actual_normalized
            .iter()
            .enumerate()
            .find_map(|(index, actual)| {
                unmatched_actuals[index]
                    .then(|| actual == expected)
                    .filter(|matched| *matched)
                    .map(|_| index)
            })
        else {
            missing.push(expected.as_str());
            continue;
        };

        unmatched_actuals[index] = false;
    }

    // any unmatched actuals are unexpected
    let mut unexpected: Vec<&str> = Vec::new();
    for (index, actual) in actual_normalized.iter().enumerate() {
        if unmatched_actuals[index] {
            unexpected.push(actual.as_str());
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

#[cfg(test)]
mod tests {
    use super::{CaseResult, compare_expected};

    #[test]
    fn test_compare_expected_counts_duplicate_expected_diagnostics() {
        let result = compare_expected(
            "error",
            &["duplicate".into(), "duplicate".into()],
            &["duplicate".into()],
        );

        assert!(matches!(result, CaseResult::Failed { .. }));
    }

    #[test]
    fn test_compare_expected_counts_duplicate_actual_diagnostics() {
        let result = compare_expected(
            "error",
            &["duplicate".into()],
            &["duplicate".into(), "duplicate".into()],
        );

        assert!(matches!(result, CaseResult::Failed { .. }));
    }
}
