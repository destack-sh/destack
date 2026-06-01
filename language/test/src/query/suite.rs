use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::core::{
    Case, CaseResult, MarkdownSuiteEntry, MarkdownSuiteIndex, RunContext, RunOptions, Suite,
    discover_markdown_suite, expected_failures_view, fixtures_dir, update_failure_baseline,
};
use crate::mdtest::{
    MdTestCase, MdTestFile, MdTestLibs, parse_mdtest_libs, run_with_timeout, slug,
};
use crate::query::{QueryTestSession, runner};
use destack_source::{BatchEdit, Edit};

/// A query expectation block parsed from markdown.
#[derive(Debug, Clone)]
pub struct QueryExpectation {
    /// The kind of query: "completion", "hover", "rename", etc.
    pub kind: String,
    /// Target cursor ($0) or marker name.
    pub target: String,
    /// Additional arguments (e.g., new name for rename).
    pub args: Vec<String>,
    /// The expectation content (multiline).
    pub content: String,
}

/// A query test case with interpreted data from markdown.
#[derive(Debug, Clone)]
struct QueryTestCase {
    /// The underlying raw test case.
    base: MdTestCase,
    /// Query expectations parsed from `query` blocks.
    query_expectations: Vec<QueryExpectation>,
    /// Expected output files for refactor tests.
    #[allow(dead_code)]
    expected_files: Vec<MdTestFile>,
}

impl QueryTestCase {
    /// Interpret a raw MdTestCase as a query test.
    fn from_mdtest(base: MdTestCase) -> Option<Self> {
        let mut query_expectations = Vec::new();
        let mut expected_files = Vec::new();

        for block in &base.extra_blocks {
            if let Some(query) = parse_query_block(&block.language, &block.content) {
                query_expectations.push(query);
            } else if let Some(file) = parse_expected_block(&block.language, &block.content) {
                expected_files.push(file);
            }
        }

        // must have query expectations or expected files to be a query test
        if query_expectations.is_empty() && expected_files.is_empty() {
            return None;
        }

        Some(Self {
            base,
            query_expectations,
            expected_files,
        })
    }
}

impl MarkdownSuiteEntry for QueryTestCase {
    fn section(&self) -> &str {
        &self.base.section
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_skipped(&self) -> bool {
        self.base.skip
    }
}

/// Parse a query block: "query <kind> <target> [args...]"
fn parse_query_block(lang: &str, content: &str) -> Option<QueryExpectation> {
    let parts: Vec<&str> = lang.split_whitespace().collect();
    if parts.len() < 3 || parts[0] != "query" {
        return None;
    }

    Some(QueryExpectation {
        kind: parts[1].to_string(),
        target: parts[2].to_string(),
        args: parts[3..]
            .iter()
            .map(|s| s.trim_matches('"').to_string())
            .collect(),
        content: content.to_string(),
    })
}

/// Parse an expected block: "expected:filename"
fn parse_expected_block(lang: &str, content: &str) -> Option<MdTestFile> {
    let filename = lang.strip_prefix("expected:")?.trim();
    if filename.is_empty() {
        return None;
    }

    // auto-derive .ds extension
    let path = if filename.contains('.') {
        filename.to_string()
    } else {
        format!("{filename}.ds")
    };

    Some(MdTestFile {
        path,
        content: content.to_string(),
        options: HashMap::new(),
    })
}

/// Test suite for LSP query tests (navigation, assist, refactor).
#[derive(Debug, Default)]
pub struct QuerySuite {
    /// Map from test full name to the parsed query test case.
    tests: HashMap<String, QueryTestCase>,
    /// List of discovered test cases.
    cases: Vec<Case>,
    /// Known failing tests for baseline tracking.
    expected_failures: HashSet<String>,
    /// Location of the known failures file.
    expected_failures_path: PathBuf,
}

impl QuerySuite {
    /// Load all query tests from the fixtures/query directory.
    pub fn load() -> Result<Self, String> {
        let fixtures = fixtures_dir();
        let query_dir = fixtures.join("query");
        let MarkdownSuiteIndex {
            cases,
            entries,
            expected_failures,
            expected_failures_path,
        } = discover_markdown_suite(
            &query_dir,
            "destack_test::query",
            QueryTestCase::from_mdtest,
        )?;

        Ok(Self {
            tests: entries,
            cases,
            expected_failures,
            expected_failures_path,
        })
    }
}

impl Suite for QuerySuite {
    fn name(&self) -> &'static str {
        "query"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn expected_failures(&self, _options: &RunOptions) -> Option<&HashSet<String>> {
        expected_failures_view(&self.expected_failures)
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

    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult {
        let Some(query_test) = self.tests.get(&case.full_name()) else {
            return CaseResult::Failed {
                message: "test not found".to_string(),
            };
        };

        let timeout = context.timeout.unwrap_or_else(|| {
            let has_libs = parse_mdtest_libs(&query_test.base)
                .map(|libs| !matches!(libs, MdTestLibs::None))
                .unwrap_or(false);
            let timeout_ms = if has_libs {
                context.options.case_timeout_ms.saturating_mul(5) // #Performance
            } else {
                context.options.case_timeout_ms
            };
            Duration::from_millis(timeout_ms.max(1))
        });
        let test = query_test.clone();
        run_with_timeout(test.base.clone(), timeout, move |_| run_query_test(&test))
    }

    fn timeout(&self) -> Option<Duration> {
        // timeout is enforced by run_with_timeout
        None
    }
}

/// Dispatch to the appropriate test runner based on test type.
fn run_query_test(test: &QueryTestCase) -> CaseResult {
    // reject empty tests
    if test.base.files.is_empty() {
        return CaseResult::Skipped {
            reason: "no source files".to_string(),
        };
    }

    // build the per case query session
    let setup_start = Instant::now();
    let session = match QueryTestSession::from_mdtest(&test.base) {
        Ok(session) => session,
        Err(message) => {
            return CaseResult::Failed { message };
        }
    };
    let setup_duration = setup_start.elapsed();

    // run the case body
    let run_start = Instant::now();
    let result = if !test.expected_files.is_empty() {
        run_expected_files(test, &session)
    } else if !test.query_expectations.is_empty() {
        run_query_expectations(&session, &test.query_expectations)
    } else {
        CaseResult::Failed {
            message: "query test is missing an explicit query block".to_string(),
        }
    };
    let run_duration = run_start.elapsed();

    // log timings when requested
    log_query_case_timing(&test.base, setup_duration, run_duration);

    result
}

/// Print one query case timing line when timing is enabled.
fn log_query_case_timing(test: &MdTestCase, setup: Duration, run: Duration) {
    if std::env::var_os("DESTACK_QUERY_TIMING").is_none() {
        return;
    }

    let section = slug(&test.section);
    let name = slug(&test.name);

    eprintln!(
        "query timing {section}/{name}: setup={setup:?} run={run:?} total={:?}",
        setup + run,
    );
}

/// Run a query expectation and validate expected file outputs.
fn run_expected_files(test: &QueryTestCase, session: &QueryTestSession) -> CaseResult {
    // ensure we have a query expectation
    let expectation = match test.query_expectations.as_slice() {
        [expectation] => expectation,
        [] => {
            return CaseResult::Failed {
                message: "expected query block for expected files".to_string(),
            };
        }
        _ => {
            return CaseResult::Failed {
                message: "expected a single query block for expected files".to_string(),
            };
        }
    };

    // dispatch expected file validation by query kind
    match expectation.kind.as_str() {
        "rename" => run_rename_expected_files(session, expectation, &test.expected_files),
        "file_rename" | "rename_files" => {
            run_file_rename_expected_files(session, expectation, &test.expected_files)
        }
        "extract_function" => {
            run_extract_function_expected_files(session, expectation, &test.expected_files)
        }
        "extract_variable" => {
            run_extract_variable_expected_files(session, expectation, &test.expected_files)
        }
        "inline" => run_inline_expected_files(session, expectation, &test.expected_files),
        "change_signature" => {
            run_change_signature_expected_files(session, expectation, &test.expected_files)
        }
        other => CaseResult::Failed {
            message: format!("expected file validation not implemented for {other}"),
        },
    }
}

/// Run a rename expectation and validate edited file contents.
fn run_rename_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> CaseResult {
    // resolve the marker and new name
    let Some(marker) = session.markers.range(&expectation.target) else {
        return CaseResult::Failed {
            message: format!("marker '{}' not found", expectation.target),
        };
    };
    let new_name = expectation
        .args
        .first()
        .map(|name| name.as_str())
        .unwrap_or("newName");

    // resolve target file and offset
    let file_id = marker.span.file;
    let offset = marker.span.start;

    // run rename
    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    let result = ctx.rename(&workspace, offset, new_name);
    let Some(result) = result else {
        return CaseResult::Failed {
            message: format!("rename at '{}' returned None", expectation.target),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result) {
        Ok(applied) => applied,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    compare_expected_files(session, &applied, expected_files, "rename")
}

/// Run a file rename expectation and validate edited file contents.
fn run_file_rename_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> CaseResult {
    // parse rename entries from the expectation
    let renames = match runner::refactor::file_rename::parse_rename_entries(session, expectation) {
        Ok(renames) => renames,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    // run file rename edits
    let workspace = session.workspace_context();
    let result = workspace.rename_files(&renames);
    let Some(result) = result else {
        return CaseResult::Failed {
            message: "file_rename returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result) {
        Ok(applied) => applied,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    compare_expected_files(session, &applied, expected_files, "file_rename")
}

/// Run an extract function expectation and validate edited file contents.
fn run_extract_function_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> CaseResult {
    // resolve the selection span
    let selection = match runner::position::resolve_query_span(session, &expectation.target) {
        Ok(span) => span,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    let new_name = expectation
        .args
        .first()
        .map(|name| name.as_str())
        .unwrap_or("extracted");

    // run extract function edits
    let ctx = session.module_context(selection.file);
    let result = ctx.extract_function(selection, new_name);
    let Some(result) = result else {
        return CaseResult::Failed {
            message: "extract_function returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result) {
        Ok(applied) => applied,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    compare_expected_files(session, &applied, expected_files, "extract_function")
}

/// Run an extract variable expectation and validate edited file contents.
fn run_extract_variable_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> CaseResult {
    // resolve the selection span
    let selection = match runner::position::resolve_query_span(session, &expectation.target) {
        Ok(span) => span,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    let new_name = expectation
        .args
        .first()
        .map(|name| name.as_str())
        .unwrap_or("extracted");

    // run extract variable edits
    let ctx = session.module_context(selection.file);
    let result = ctx.extract_variable(selection, new_name);
    let Some(result) = result else {
        return CaseResult::Failed {
            message: "extract_variable returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result) {
        Ok(applied) => applied,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    compare_expected_files(session, &applied, expected_files, "extract_variable")
}

/// Run an inline expectation and validate edited file contents.
fn run_inline_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> CaseResult {
    // resolve the target position
    let (file_id, offset) =
        match runner::position::resolve_query_position(session, &expectation.target) {
            Ok(pos) => pos,
            Err(error) => {
                return CaseResult::Failed { message: error };
            }
        };

    // run inline edits
    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    let result = ctx.inline_symbol(&workspace, offset);
    let Some(result) = result else {
        return CaseResult::Failed {
            message: "inline returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result) {
        Ok(applied) => applied,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    compare_expected_files(session, &applied, expected_files, "inline")
}

/// Run a change signature expectation and validate edited file contents.
fn run_change_signature_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> CaseResult {
    // resolve the target position
    let (file_id, offset) =
        match runner::position::resolve_query_position(session, &expectation.target) {
            Ok(pos) => pos,
            Err(error) => {
                return CaseResult::Failed { message: error };
            }
        };

    let new_parameters = expectation
        .args
        .first()
        .map(|value| value.as_str())
        .unwrap_or("");
    let new_arguments = expectation
        .args
        .get(1)
        .map(|value| value.as_str())
        .unwrap_or("");

    // run change signature edits
    let ctx = session.module_context(file_id);
    let workspace = session.workspace_context();
    let result = ctx.change_signature(&workspace, offset, new_parameters, new_arguments);
    let Some(result) = result else {
        return CaseResult::Failed {
            message: "change_signature returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result) {
        Ok(applied) => applied,
        Err(error) => {
            return CaseResult::Failed { message: error };
        }
    };

    compare_expected_files(session, &applied, expected_files, "change_signature")
}

/// Compare expected refactor file outputs against the applied edits.
fn compare_expected_files(
    session: &QueryTestSession,
    applied: &HashMap<String, String>,
    expected_files: &[MdTestFile],
    operation: &str,
) -> CaseResult {
    // build expected file lookup
    let expected_paths: HashSet<&str> = expected_files
        .iter()
        .map(|file| file.path.as_str())
        .collect();

    // ensure all edited files have expectations
    for path in applied.keys() {
        if !expected_paths.contains(path.as_str()) {
            return CaseResult::Failed {
                message: format!("missing expected output for '{path}'"),
            };
        }
    }

    // compare each expected file with actual output
    for expected in expected_files {
        let actual = match applied.get(&expected.path) {
            Some(content) => content.clone(),
            None => {
                let Some(file) = session.file(&expected.path) else {
                    return CaseResult::Failed {
                        message: format!("missing source for '{}'", expected.path),
                    };
                };
                file.source.clone()
            }
        };

        let actual = actual.trim_end();
        let expected_content = expected.content.trim_end();
        if actual != expected_content {
            // debug output
            if std::env::var("DESTACK_QUERY_DEBUG_DIFF").is_ok() {
                eprintln!("{operation} debug: path={}", expected.path);
                eprintln!("--- expected ---\n{expected_content}");
                eprintln!("--- actual ---\n{actual}");
            }

            return CaseResult::Failed {
                message: format!("{operation} output mismatch for '{}'", expected.path),
            };
        }
    }

    CaseResult::Passed
}

/// Apply a batch of edits and return updated contents keyed by file path.
fn apply_batch_edit(
    session: &QueryTestSession,
    edits: &BatchEdit,
) -> Result<HashMap<String, String>, String> {
    // map file ids to sources
    let mut sources = HashMap::new();
    for (name, file) in &session.files {
        sources.insert(file.file_id, (name.clone(), file.source.clone()));
    }

    // apply edits per file
    let mut outputs = HashMap::new();
    for file_edit in &edits.files {
        let Some((name, source)) = sources.get(&file_edit.file) else {
            return Err(format!(
                "edit target file {:?} not found in session",
                file_edit.file
            ));
        };

        let updated = apply_file_edits(source, &file_edit.edits)?;
        outputs.insert(name.clone(), updated);
    }

    Ok(outputs)
}

/// Apply a set of edits to a source string.
fn apply_file_edits(source: &str, edits: &[Edit]) -> Result<String, String> {
    // return original source when no edits
    if edits.is_empty() {
        return Ok(source.to_string());
    }

    // sort edits by start descending
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by(|left, right| right.span.start.cmp(&left.span.start));

    // apply edits from the end
    let mut updated = source.to_string();
    for edit in sorted_edits {
        let start = edit.span.start as usize;
        let end = edit.span.end as usize;

        // validate span boundaries
        if start > end || end > updated.len() {
            eprintln!(
                "edit span out of bounds: start={start} end={end} len={}",
                updated.len()
            );
            eprintln!("edit span: {:?}", edit.span);
            eprintln!("edit new_text: {}", edit.new_text);
            return Err("edit span is out of bounds".to_string());
        }
        if !updated.is_char_boundary(start) || !updated.is_char_boundary(end) {
            return Err("edit span is not on a char boundary".to_string());
        }

        updated.replace_range(start..end, &edit.new_text);
    }

    Ok(updated)
}

fn run_query_expectations(
    session: &QueryTestSession,
    expectations: &[QueryExpectation],
) -> CaseResult {
    // reject empty expectation lists
    if expectations.is_empty() {
        return CaseResult::Failed {
            message: "no query expectations provided".to_string(),
        };
    }

    // track results for aggregated output
    let mut passed = 0usize;
    let mut skipped = Vec::new();

    for expectation in expectations {
        // execute the query for this expectation
        let result = dispatch_query(&expectation.kind, session, Some(expectation));

        // handle the query result
        match result {
            CaseResult::Passed => {
                passed += 1;
            }
            CaseResult::Skipped { reason } => {
                skipped.push(format!(
                    "{} {}: {reason}",
                    expectation.kind, expectation.target
                ));
            }
            CaseResult::Failed { message } => {
                return CaseResult::Failed {
                    message: format!(
                        "query {} {} failed: {message}",
                        expectation.kind, expectation.target
                    ),
                };
            }
            CaseResult::Suite { .. } => {
                return CaseResult::Failed {
                    message: format!(
                        "query {} {} returned suite result",
                        expectation.kind, expectation.target
                    ),
                };
            }
        }
    }

    // return when any expectation passed
    if passed > 0 {
        return CaseResult::Passed;
    }

    // return aggregated skip reasons when nothing ran
    if !skipped.is_empty() {
        return CaseResult::Skipped {
            reason: format!("all query expectations skipped: {}", skipped.join(", ")),
        };
    }

    CaseResult::Failed {
        message: "no query expectations executed".to_string(),
    }
}

/// Dispatch to the appropriate runner based on query kind.
fn dispatch_query(
    query_kind: &str,
    session: &QueryTestSession,
    expectation: Option<&QueryExpectation>,
) -> CaseResult {
    // dispatch to appropriate runner
    match query_kind {
        // navigation
        "goto_definition" => runner::navigation::definition::run(session, expectation),
        "goto_declaration" => runner::navigation::definition::run_declaration(session, expectation),
        "goto_type_definition" => {
            runner::navigation::definition::run_type_definition(session, expectation)
        }
        "find_references" => runner::navigation::find_references::run(session, expectation),
        "document_highlight" => runner::navigation::document_highlight::run(session, expectation),
        "goto_implementation" => runner::navigation::implementation::run(session, expectation),
        "document_symbols" => runner::navigation::document_symbol::run(session, expectation),
        "parity_tsserver_document_symbols" => {
            runner::parity::tsserver_document_symbol::run(session, expectation)
        }
        "workspace_symbols" => runner::navigation::workspace_symbol::run(session, expectation),
        "selection_range" => runner::navigation::selection_range::run(session, expectation),
        "call_hierarchy" => runner::navigation::call_hierarchy::run(session, expectation),
        "type_hierarchy" => runner::navigation::type_hierarchy::run(session, expectation),
        "document_link" => runner::navigation::document_link::run(session, expectation),

        // refactor
        "rename" => runner::refactor::rename::run(session, expectation),
        "prepare_rename" => runner::refactor::prepare_rename::run(session, expectation),
        "file_rename" | "rename_files" => runner::refactor::file_rename::run(session, expectation),
        "extract_function" => runner::refactor::extract_function::run(session, expectation),
        "extract_variable" => runner::refactor::extract_variable::run(session, expectation),
        "inline" => runner::refactor::inline::run(session, expectation),
        "change_signature" => runner::refactor::change_signature::run(session, expectation),

        // assist
        "completion" => runner::assist::completion::run(session, expectation),
        "hover" => runner::assist::hover::run(session, expectation),
        "signature_help" => runner::assist::signature_help::run(session, expectation),
        "inlay_hints" => runner::assist::inlay_hint::run(session, expectation),
        "folding_ranges" => runner::assist::folding::run(session, expectation),
        "semantic_tokens" => runner::assist::semantic_token::run(session, expectation),
        "semantic_tokens_range" => runner::assist::semantic_token::run_range(session, expectation),
        "code_lens" => runner::assist::code_lens::run(session, expectation),
        "resolve_code_lens" => runner::assist::code_lens::run_resolve(session, expectation),

        // diagnostic
        "code_actions" => runner::diagnostic::code_action::run(session, expectation),

        // fallback to failed?
        _ => CaseResult::Failed {
            message: format!("unknown query kind: {query_kind}"),
        },
    }
}
