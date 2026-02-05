use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, fixtures_dir, save_expected_failures,
};
use crate::mdtest::{
    MdTestCase, MdTestFile, MdTestLibs, TEST_TIMEOUT_SECONDS, discover_md_files,
    load_mdtest_expected_failures, parse_mdtest_file, parse_mdtest_libs, run_with_timeout, slug,
};
use crate::query::{QueryTestSession, runner};
use destack_source::{BatchEdit, Edit, MemoryFileSystem};
use destack_workspace::{MemoryCacheStore, query};

/// Type of query test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryTestType {
    /// Navigation queries: goto definition, find references, etc.
    Navigation,
    /// Assist queries: completion, hover, signature help, etc.
    Assist,
    /// Refactor operations: rename, extract, etc.
    Refactor,
}

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
    /// The type of query test (used for categorization, not dispatch).
    #[allow(dead_code)]
    test_type: QueryTestType,
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

        let test_type = infer_test_type(&query_expectations, &expected_files);

        Some(Self {
            base,
            test_type,
            query_expectations,
            expected_files,
        })
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

/// Infer the test type from query expectations and expected files.
fn infer_test_type(queries: &[QueryExpectation], expected: &[MdTestFile]) -> QueryTestType {
    if !expected.is_empty() {
        return QueryTestType::Refactor;
    }

    if let Some(query) = queries.first() {
        return match query.kind.as_str() {
            "completion"
            | "hover"
            | "signature"
            | "inlay_hints"
            | "folding"
            | "semantic_tokens"
            | "semantic_tokens_range"
            | "code_lens"
            | "resolve_code_lens" => QueryTestType::Assist,
            "rename" | "prepare_rename" | "file_rename" | "rename_files" | "extract_function"
            | "inline" | "change_signature" => QueryTestType::Refactor,
            _ => QueryTestType::Navigation,
        };
    }

    QueryTestType::Navigation
}

/// Test suite for LSP query tests (navigation, assist, refactor).
#[derive(Debug, Default)]
pub struct QuerySuite {
    /// Map from test full name to the parsed query test case.
    tests: HashMap<String, QueryTestCase>,
    /// List of discovered test cases.
    cases: Vec<TestCase>,
    /// Known failing tests for baseline tracking.
    expected_failures: HashSet<String>,
    /// Location of the known failures file.
    expected_failures_path: PathBuf,
}

impl QuerySuite {
    /// Load all query tests from the fixtures/query directory.
    pub fn load() -> Self {
        let fixtures = fixtures_dir();
        let mut suite = Self::default();

        let query_dir = fixtures.join("query");
        for md_path in discover_md_files(&query_dir).unwrap_or_default() {
            suite.add_file(&query_dir, &md_path);
        }

        suite.expected_failures = load_mdtest_expected_failures(&query_dir);
        suite.expected_failures_path = query_dir.join("known-failures.txt");
        suite
    }

    fn add_file(&mut self, base_dir: &Path, md_path: &Path) {
        let cases = match parse_mdtest_file(md_path) {
            Ok(cases) => cases,
            Err(error) => {
                panic!("failed to parse {}: {error}", md_path.display());
            }
        };

        let relative_path = md_path.strip_prefix(base_dir).unwrap_or(md_path);
        let relative_name = relative_path.to_string_lossy();

        for base_case in cases {
            // only include tests that have query expectations or expected files
            let skip = base_case.skip;
            let Some(query_case) = QueryTestCase::from_mdtest(base_case) else {
                continue;
            };

            let name = format!(
                "{relative_name}/{}/{}",
                slug(&query_case.base.section),
                slug(&query_case.base.name)
            );
            let test_case = TestCase::file(name, md_path.to_path_buf(), "destack_test::query")
                .with_skipped(skip);

            self.tests.insert(test_case.full_name(), query_case);
            self.cases.push(test_case);
        }
    }
}

impl Suite for QuerySuite {
    fn name(&self) -> &'static str {
        "query"
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

    fn report(&self, results: &[(TestCase, TestResult)], context: &RunContext<'_>) {
        if !context.options.update_known_failures {
            return;
        }

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

    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult {
        let Some(query_test) = self.tests.get(&case.full_name()) else {
            return TestResult::Failed {
                message: "test not found".to_string(),
            };
        };

        let timeout = context.timeout.unwrap_or_else(|| {
            let has_libs = parse_mdtest_libs(&query_test.base)
                .map(|libs| !matches!(libs, MdTestLibs::None))
                .unwrap_or(false);
            let seconds = if has_libs {
                TEST_TIMEOUT_SECONDS.saturating_mul(5)
            } else {
                TEST_TIMEOUT_SECONDS
            };
            Duration::from_secs(seconds)
        });
        let test = query_test.clone();
        run_with_timeout(test.base.clone(), timeout, move |_| run_query_test(&test))
    }

    fn timeout(&self) -> Option<Duration> {
        // timeout is enforced by run_with_timeout
        None
    }
}

#[derive(Debug)]
struct SharedQueryEnvironment {
    /// The shared test session.
    session: Arc<destack_workspace::Session>,
    /// The shared in-memory file system.
    fs: Arc<MemoryFileSystem>,
    /// The next unique test id.
    next_id: AtomicUsize,
}

impl SharedQueryEnvironment {
    /// Create a new shared environment for query tests.
    fn new() -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let cwd = PathBuf::from("/test/query");
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
        PathBuf::from("/test/query").join(format!("{section}-{name}-{id}"))
    }
}

thread_local! {
    static SHARED_QUERY_ENV: SharedQueryEnvironment = SharedQueryEnvironment::new();
}

/// Dispatch to the appropriate test runner based on test type.
fn run_query_test(test: &QueryTestCase) -> TestResult {
    // reject empty tests
    if test.base.files.is_empty() {
        return TestResult::Skipped {
            reason: "no source files".to_string(),
        };
    }

    // create test session from markdown files
    let session = SHARED_QUERY_ENV.with(|env| {
        let root = env.root_for(&test.base);
        QueryTestSession::from_mdtest_with_session(
            &test.base,
            env.session.clone(),
            env.fs.clone(),
            root,
        )
    });

    // run expected file validation when provided
    if !test.expected_files.is_empty() {
        return run_expected_files(test, &session);
    }

    // run explicit query expectations when provided
    if !test.query_expectations.is_empty() {
        return run_query_expectations(&session, &test.query_expectations);
    }

    // determine query kind from expectations or markers
    let query_kind = test
        .query_expectations
        .first()
        .map(|q| q.kind.as_str())
        .or(session.markers.test_type.as_deref())
        .unwrap_or("unknown");

    // get the first query expectation if any
    let expectation = test.query_expectations.first();

    // dispatch the query
    dispatch_query(query_kind, &session, expectation)
}

/// Run a query expectation and validate expected file outputs.
fn run_expected_files(test: &QueryTestCase, session: &QueryTestSession) -> TestResult {
    // ensure we have a query expectation
    let expectation = match test.query_expectations.as_slice() {
        [expectation] => expectation,
        [] => {
            return TestResult::Failed {
                message: "expected query block for expected files".to_string(),
            };
        }
        _ => {
            return TestResult::Failed {
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
        "inline" => run_inline_expected_files(session, expectation, &test.expected_files),
        "change_signature" => {
            run_change_signature_expected_files(session, expectation, &test.expected_files)
        }
        other => TestResult::Failed {
            message: format!("expected file validation not implemented for {other}"),
        },
    }
}

/// Run a rename expectation and validate edited file contents.
fn run_rename_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> TestResult {
    // resolve the marker and new name
    let Some(marker) = session.markers.range(&expectation.target) else {
        return TestResult::Failed {
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
    let result = query::rename(&session.session, file_id, offset, new_name);
    let Some(result) = result else {
        return TestResult::Failed {
            message: format!("rename at '{}' returned None", expectation.target),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result.edits) {
        Ok(applied) => applied,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    // build expected file lookup
    let mut expected_paths = HashSet::new();
    for file in expected_files {
        expected_paths.insert(file.path.as_str());
    }

    // ensure all edited files have expectations
    for path in applied.keys() {
        if !expected_paths.contains(path.as_str()) {
            return TestResult::Failed {
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
                    return TestResult::Failed {
                        message: format!("missing source for '{}'", expected.path),
                    };
                };
                file.source.clone()
            }
        };

        let actual = actual.trim_end();
        let expected_content = expected.content.trim_end();
        if actual != expected_content {
            // print debug output when explicitly requested
            if std::env::var("DESTACK_QUERY_DEBUG_DIFF").is_ok() {
                eprintln!("rename debug: path={}", expected.path);
                eprintln!("--- expected ---\n{expected_content}");
                eprintln!("--- actual ---\n{actual}");
            }

            return TestResult::Failed {
                message: format!("rename output mismatch for '{}'", expected.path),
            };
        }
    }

    TestResult::Passed
}

/// Run a file rename expectation and validate edited file contents.
fn run_file_rename_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> TestResult {
    // parse rename entries from the expectation
    let renames = match runner::refactor::file_rename::parse_rename_entries(session, expectation) {
        Ok(renames) => renames,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    // run file rename edits
    let result = query::rename_files(&session.session, &renames);
    let Some(result) = result else {
        return TestResult::Failed {
            message: "file_rename returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result.edits) {
        Ok(applied) => applied,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    // build expected file lookup
    let mut expected_paths = HashSet::new();
    for file in expected_files {
        expected_paths.insert(file.path.as_str());
    }

    // ensure all edited files have expectations
    for path in applied.keys() {
        if !expected_paths.contains(path.as_str()) {
            return TestResult::Failed {
                message: format!("missing expected output for '{path}'"),
            };
        }
    }

    // compare each expected file with actual output
    for expected in expected_files {
        // resolve actual content for the expected path
        let actual = match applied.get(&expected.path) {
            Some(content) => content.clone(),
            None => {
                let Some(file) = session.file(&expected.path) else {
                    return TestResult::Failed {
                        message: format!("missing source for '{}'", expected.path),
                    };
                };
                file.source.clone()
            }
        };

        // compare actual content with expected output
        let actual = actual.trim_end();
        let expected_content = expected.content.trim_end();
        if actual != expected_content {
            // print debug output when explicitly requested
            if std::env::var("DESTACK_QUERY_DEBUG_DIFF").is_ok() {
                eprintln!("file_rename debug: path={}", expected.path);
                eprintln!("--- expected ---\n{expected_content}");
                eprintln!("--- actual ---\n{actual}");
            }

            return TestResult::Failed {
                message: format!("file_rename output mismatch for '{}'", expected.path),
            };
        }
    }

    TestResult::Passed
}

/// Run an extract function expectation and validate edited file contents.
fn run_extract_function_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> TestResult {
    // resolve the selection span
    let selection = match runner::position::resolve_query_span(session, &expectation.target) {
        Ok(span) => span,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    let new_name = expectation
        .args
        .first()
        .map(|name| name.as_str())
        .unwrap_or("extracted");

    // run extract function edits
    let result = query::extract_function(&session.session, session.file_id, selection, new_name);
    let Some(result) = result else {
        return TestResult::Failed {
            message: "extract_function returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result.edits) {
        Ok(applied) => applied,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    // build expected file lookup
    let mut expected_paths = HashSet::new();
    for file in expected_files {
        expected_paths.insert(file.path.as_str());
    }

    // ensure all edited files have expectations
    for path in applied.keys() {
        if !expected_paths.contains(path.as_str()) {
            return TestResult::Failed {
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
                    return TestResult::Failed {
                        message: format!("missing source for '{}'", expected.path),
                    };
                };
                file.source.clone()
            }
        };

        let actual = actual.trim_end();
        let expected_content = expected.content.trim_end();
        if actual != expected_content {
            if std::env::var("DESTACK_QUERY_DEBUG_DIFF").is_ok() {
                eprintln!("extract_function debug: path={}", expected.path);
                eprintln!("--- expected ---\n{expected_content}");
                eprintln!("--- actual ---\n{actual}");
            }

            return TestResult::Failed {
                message: format!("extract_function output mismatch for '{}'", expected.path),
            };
        }
    }

    TestResult::Passed
}

/// Run an inline expectation and validate edited file contents.
fn run_inline_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> TestResult {
    // resolve the target position
    let (file_id, offset) =
        match runner::position::resolve_query_position(session, &expectation.target) {
            Ok(pos) => pos,
            Err(error) => {
                return TestResult::Failed { message: error };
            }
        };

    // run inline edits
    let result = query::inline_symbol(&session.session, file_id, offset);
    let Some(result) = result else {
        return TestResult::Failed {
            message: "inline returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result.edits) {
        Ok(applied) => applied,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    // build expected file lookup
    let mut expected_paths = HashSet::new();
    for file in expected_files {
        expected_paths.insert(file.path.as_str());
    }

    // ensure all edited files have expectations
    for path in applied.keys() {
        if !expected_paths.contains(path.as_str()) {
            return TestResult::Failed {
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
                    return TestResult::Failed {
                        message: format!("missing source for '{}'", expected.path),
                    };
                };
                file.source.clone()
            }
        };

        let actual = actual.trim_end();
        let expected_content = expected.content.trim_end();
        if actual != expected_content {
            if std::env::var("DESTACK_QUERY_DEBUG_DIFF").is_ok() {
                eprintln!("inline debug: path={}", expected.path);
                eprintln!("--- expected ---\n{expected_content}");
                eprintln!("--- actual ---\n{actual}");
            }

            return TestResult::Failed {
                message: format!("inline output mismatch for '{}'", expected.path),
            };
        }
    }

    TestResult::Passed
}

/// Run a change signature expectation and validate edited file contents.
fn run_change_signature_expected_files(
    session: &QueryTestSession,
    expectation: &QueryExpectation,
    expected_files: &[MdTestFile],
) -> TestResult {
    // resolve the target position
    let (file_id, offset) =
        match runner::position::resolve_query_position(session, &expectation.target) {
            Ok(pos) => pos,
            Err(error) => {
                return TestResult::Failed { message: error };
            }
        };

    let new_parameters = expectation
        .args
        .get(0)
        .map(|value| value.as_str())
        .unwrap_or("");
    let new_arguments = expectation
        .args
        .get(1)
        .map(|value| value.as_str())
        .unwrap_or("");

    // run change signature edits
    let result = query::change_signature(
        &session.session,
        file_id,
        offset,
        new_parameters,
        new_arguments,
    );
    let Some(result) = result else {
        return TestResult::Failed {
            message: "change_signature returned no edits".to_string(),
        };
    };

    // apply edits to sources
    let applied = match apply_batch_edit(session, &result.edits) {
        Ok(applied) => applied,
        Err(error) => {
            return TestResult::Failed { message: error };
        }
    };

    // build expected file lookup
    let mut expected_paths = HashSet::new();
    for file in expected_files {
        expected_paths.insert(file.path.as_str());
    }

    // ensure all edited files have expectations
    for path in applied.keys() {
        if !expected_paths.contains(path.as_str()) {
            return TestResult::Failed {
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
                    return TestResult::Failed {
                        message: format!("missing source for '{}'", expected.path),
                    };
                };
                file.source.clone()
            }
        };

        let actual = actual.trim_end();
        let expected_content = expected.content.trim_end();
        if actual != expected_content {
            if std::env::var("DESTACK_QUERY_DEBUG_DIFF").is_ok() {
                eprintln!("change_signature debug: path={}", expected.path);
                eprintln!("--- expected ---\n{expected_content}");
                eprintln!("--- actual ---\n{actual}");
            }

            return TestResult::Failed {
                message: format!("change_signature output mismatch for '{}'", expected.path),
            };
        }
    }

    TestResult::Passed
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
) -> TestResult {
    // reject empty expectation lists
    if expectations.is_empty() {
        return TestResult::Failed {
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
            TestResult::Passed => {
                passed += 1;
            }
            TestResult::Skipped { reason } => {
                skipped.push(format!(
                    "{} {}: {reason}",
                    expectation.kind, expectation.target
                ));
            }
            TestResult::Failed { message } => {
                return TestResult::Failed {
                    message: format!(
                        "query {} {} failed: {message}",
                        expectation.kind, expectation.target
                    ),
                };
            }
            TestResult::Suite { .. } => {
                return TestResult::Failed {
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
        return TestResult::Passed;
    }

    // return aggregated skip reasons when nothing ran
    if !skipped.is_empty() {
        return TestResult::Skipped {
            reason: format!("all query expectations skipped: {}", skipped.join(", ")),
        };
    }

    TestResult::Failed {
        message: "no query expectations executed".to_string(),
    }
}

/// Dispatch to the appropriate runner based on query kind.
fn dispatch_query(
    query_kind: &str,
    session: &QueryTestSession,
    expectation: Option<&QueryExpectation>,
) -> TestResult {
    // dispatch to appropriate runner
    match query_kind {
        // navigation
        "goto_definition" | "definition" => {
            runner::navigation::definition::run(session, expectation)
        }
        "goto_declaration" | "declaration" => {
            runner::navigation::definition::run_declaration(session, expectation)
        }
        "goto_type_definition" | "type_definition" => {
            runner::navigation::definition::run_type_definition(session, expectation)
        }
        "find_references" | "references" => {
            runner::navigation::find_references::run(session, expectation)
        }
        "document_highlight" | "highlight" => {
            runner::navigation::document_highlight::run(session, expectation)
        }
        "goto_implementation" | "implementation" => {
            runner::navigation::implementation::run(session, expectation)
        }
        "document_symbols" | "symbols" => {
            runner::navigation::document_symbol::run(session, expectation)
        }
        "parity_tsserver_document_symbols" => {
            runner::parity::tsserver_document_symbol::run(session, expectation)
        }
        "workspace_symbols" | "workspace" => {
            runner::navigation::workspace_symbol::run(session, expectation)
        }
        "selection_range" | "selection_ranges" => {
            runner::navigation::selection_range::run(session, expectation)
        }
        "call_hierarchy" => runner::navigation::call_hierarchy::run(session, expectation),
        "type_hierarchy" => runner::navigation::type_hierarchy::run(session, expectation),
        "document_link" | "document_links" | "link" => {
            runner::navigation::document_link::run(session, expectation)
        }
        "resolve_document_link" | "document_link/resolve" => {
            runner::navigation::document_link::run_resolve(session, expectation)
        }

        // refactor
        "rename" => runner::refactor::rename::run(session, expectation),
        "prepare_rename" => runner::refactor::prepare_rename::run(session, expectation),
        "file_rename" | "rename_files" => runner::refactor::file_rename::run(session, expectation),
        "extract_function" => runner::refactor::extract_function::run(session, expectation),
        "inline" => runner::refactor::inline::run(session, expectation),
        "change_signature" => runner::refactor::change_signature::run(session, expectation),

        // assist
        "completion" => runner::assist::completion::run(session, expectation),
        "hover" => runner::assist::hover::run(session, expectation),
        "signature_help" | "signature" => runner::assist::signature_help::run(session, expectation),
        "inlay_hints" | "inlay" => runner::assist::inlay_hint::run(session, expectation),
        "folding_ranges" | "folding" => runner::assist::folding::run(session, expectation),
        "semantic_tokens" | "semantic" => runner::assist::semantic_token::run(session, expectation),
        "semantic_tokens_range" => runner::assist::semantic_token::run_range(session, expectation),
        "code_lens" => runner::assist::code_lens::run(session, expectation),
        "resolve_code_lens" | "code_lens/resolve" => {
            runner::assist::code_lens::run_resolve(session, expectation)
        }

        // diagnostic
        "code_actions" | "code_action" => {
            runner::diagnostic::code_action::run(session, expectation)
        }

        // fallback to failed?
        _ => TestResult::Failed {
            message: format!("unknown query kind: {query_kind}"),
        },
    }
}
