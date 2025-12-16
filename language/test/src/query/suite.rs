use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::harness::{RunContext, Suite, TestCase, TestOptions, TestResult, fixtures_dir};
use crate::mdtest::{
    MdTestCase, MdTestFile, TEST_TIMEOUT_SECONDS, discover_md_files, parse_mdtest_file,
    run_with_timeout, slug,
};

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
struct QueryExpectation {
    /// The kind of query: "completion", "hover", "rename", etc.
    kind: String,
    /// Target cursor ($0) or marker name.
    #[allow(dead_code)]
    target: String,
    /// Additional arguments (e.g., new name for rename).
    #[allow(dead_code)]
    args: Vec<String>,
    /// The expectation content (multiline).
    #[allow(dead_code)]
    content: String,
}

/// A query test case with interpreted data from markdown.
#[derive(Debug, Clone)]
struct QueryTestCase {
    /// The underlying raw test case.
    base: MdTestCase,
    /// The type of query test.
    test_type: QueryTestType,
    /// Query expectations parsed from `query` blocks.
    query_expectations: Vec<QueryExpectation>,
    /// Expected output files for refactor tests.
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
        args: parts[3..].iter().map(|s| s.trim_matches('"').to_string()).collect(),
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
    })
}

/// Infer the test type from query expectations and expected files.
fn infer_test_type(queries: &[QueryExpectation], expected: &[MdTestFile]) -> QueryTestType {
    if !expected.is_empty() {
        return QueryTestType::Refactor;
    }

    if let Some(query) = queries.first() {
        return match query.kind.as_str() {
            "completion" | "hover" | "signature" | "inlay_hints" | "folding" | "semantic_tokens"
            | "code_lens" => QueryTestType::Assist,
            "rename" | "prepare_rename" => QueryTestType::Refactor,
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

        for base_case in cases {
            // only include tests that have query expectations or expected files
            let Some(query_case) = QueryTestCase::from_mdtest(base_case) else {
                continue;
            };

            let name = format!(
                "{relative_name}/{}/{}",
                slug(&query_case.base.section),
                slug(&query_case.base.name)
            );
            let test_case = TestCase::file(name, md_path.to_path_buf(), "destack_test::query");

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

    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult {
        let Some(query_test) = self.tests.get(&case.full_name()) else {
            return TestResult::Failed {
                message: "test not found".to_string(),
            };
        };

        let timeout = context.timeout.unwrap_or(Duration::from_secs(TEST_TIMEOUT_SECONDS));
        let test = query_test.clone();
        run_with_timeout(test.base.clone(), timeout, move |_| run_query_test(&test))
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(TEST_TIMEOUT_SECONDS))
    }
}

/// Dispatch to the appropriate test runner based on test type.
fn run_query_test(test: &QueryTestCase) -> TestResult {
    match test.test_type {
        QueryTestType::Navigation => run_navigation_test(test),
        QueryTestType::Assist => run_assist_test(test),
        QueryTestType::Refactor => run_refactor_test(test),
    }
}

fn run_navigation_test(test: &QueryTestCase) -> TestResult {
    if test.query_expectations.is_empty() {
        return TestResult::Skipped {
            reason: "no query expectations".to_string(),
        };
    }

    TestResult::Skipped {
        reason: "#Incomplete: navigation tests not yet implemented".to_string(),
    }
}

fn run_assist_test(test: &QueryTestCase) -> TestResult {
    if test.query_expectations.is_empty() {
        return TestResult::Skipped {
            reason: "no query expectations".to_string(),
        };
    }

    TestResult::Skipped {
        reason: "#Incomplete: assist tests not yet implemented".to_string(),
    }
}

fn run_refactor_test(test: &QueryTestCase) -> TestResult {
    if test.expected_files.is_empty() {
        return TestResult::Skipped {
            reason: "no expected files".to_string(),
        };
    }

    TestResult::Skipped {
        reason: "#Incomplete: refactor tests not yet implemented".to_string(),
    }
}
