use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::parse::{ParseOptions, ParseOutcome, TestArea, parse_file};
use crate::conformance::{
    Case, CaseOutcome, ConformanceDriver, ConformanceSuiteResult, run_conformance_driver,
    suite_fixtures_dir, suite_tests_dir,
};
use crate::core::RunOptions;

// pinned version of SWC parser tests
const SWC_VERSION: &str = "1.x";
const SWC_COMMIT: &str = "5b9d77c"; // short sha

/// SWC parser conformance suite.
#[derive(Debug, Clone)]
pub struct SwcSuite {
    tests_dir: PathBuf,
    suite_dir: PathBuf,
}

impl SwcSuite {
    /// Create one SWC conformance suite.
    pub fn new() -> Self {
        let suite_dir = suite_fixtures_dir("ecma", "swc");
        let tests_dir = suite_tests_dir("ecma", "swc");
        Self {
            tests_dir,
            suite_dir,
        }
    }

    fn should_skip_test_by_name(name: &str) -> bool {
        // NOTE: we intentionally exclude non-standard/proposal syntax suites for now
        // because conformance is focused on JS/TS/JSX that we intend to support.
        // These categories are typically ahead of the official ECMAScript baseline.
        name.starts_with("js/explicit-resource-management/")
            || name.starts_with("js/import-assertions-with-keyword/")
            || name.starts_with("js/import-assertions/")
            || name.starts_with("js/import-attributes-deprecatedAssertKeyword/")
            || name.starts_with("js/import-attributes/")
            || name.starts_with("js/source-phase-imports/")
            || name.starts_with("js/deferred-import-evaluation/")
    }

    fn discover_recursive(&self, dir: &Path, prefix: &str, category: &str) -> Vec<Case> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                if path.is_dir() {
                    let new_prefix = if prefix.is_empty() {
                        file_name.to_string()
                    } else {
                        format!("{prefix}/{file_name}")
                    };
                    tests.extend(self.discover_recursive(&path, &new_prefix, category));
                } else if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy();
                    if matches!(ext.as_ref(), "ts" | "tsx" | "js" | "jsx") {
                        // skip .d.ts files
                        if file_name.ends_with(".d.ts") {
                            continue;
                        }
                        let stem = path.file_stem().unwrap().to_string_lossy();
                        let name = if prefix.is_empty() {
                            format!("{stem}.{ext}")
                        } else {
                            format!("{prefix}/{stem}.{ext}")
                        };

                        if Self::should_skip_test_by_name(&name) {
                            continue;
                        }

                        // determine file type based on category and extension
                        let file_type = Self::file_type_for_category(category, &name);
                        let expect_error =
                            name.contains("/errors/") || name.contains("typescript-errors");

                        let test = if expect_error {
                            Case::fail(name, file_type)
                        } else {
                            Case::pass(name, file_type)
                        };
                        tests.push(test);
                    }
                }
            }
        }

        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }

    /// Determine file type based on SWC category directory.
    fn file_type_for_category(category: &str, name: &str) -> FileType {
        // tsx directory = TypeScript with JSX
        let in_tsx_dir = name.contains("/tsx/") || name.contains("/tsx-");
        if category.contains("tsx") || in_tsx_dir || name.ends_with(".tsx") {
            FileType::TypeScriptXml
        } else if category == "typescript" || name.ends_with(".ts") {
            FileType::TypeScript
        } else if category == "jsx" || name.ends_with(".jsx") {
            FileType::JavaScriptXml
        } else {
            FileType::JavaScript
        }
    }
}

impl Default for SwcSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceDriver for SwcSuite {
    fn name(&self) -> &str {
        "swc"
    }

    fn suite_dir(&self) -> &Path {
        &self.suite_dir
    }

    fn tests_dir(&self) -> &Path {
        &self.tests_dir
    }

    fn discover_cases(&self) -> Vec<Case> {
        let mut tests = Vec::new();

        // discover tests from typescript, jsx, and js directories
        for category in &["typescript", "jsx", "js"] {
            let category_dir = self.tests_dir.join(category);
            if category_dir.exists() {
                tests.extend(self.discover_recursive(&category_dir, category, category));
            }
        }

        tests
    }

    fn run(&self, test: &Case, _show_diff: bool) -> CaseOutcome {
        let path = self.tests_dir.join(&test.name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return CaseOutcome::FailedRead,
        };

        let area = if test.expect_error {
            TestArea::EarlySyntax
        } else {
            TestArea::Parse
        };
        let parse_outcome = parse_file(
            &path,
            &content,
            test.file_type,
            ParseOptions {
                area,
                disallow_ambiguous_tree_literal: false,
            },
        );

        match (test.expect_error, parse_outcome) {
            (true, ParseOutcome::Error) => CaseOutcome::Passed,
            (true, ParseOutcome::Ok) => CaseOutcome::FailedParse,
            (false, ParseOutcome::Ok) => CaseOutcome::Passed,
            (false, ParseOutcome::Error) => CaseOutcome::FailedParse,
        }
    }

    fn fetch_instructions(&self) -> String {
        format!(
            "To download SWC parser tests (version {SWC_VERSION}, commit {SWC_COMMIT}):\n\
             \n\
               just language/install-conformance-ecma\n\
             \n\
             Or manually:\n\
               python3 ./language/test/fixtures/conformance/fetch-suite.py ./language/test/fixtures/conformance/ecma/swc\n"
        )
    }

    fn category_for_case(&self, test_name: &str) -> String {
        // swc tests are typescript/subcategory/... or jsx/subcategory/... or js/subcategory/...
        // use the second segment as the category
        let parts: Vec<&str> = test_name.split('/').collect();
        let category = if parts.len() >= 2 {
            parts[1]
        } else {
            parts.first().unwrap_or(&"unknown")
        };

        // combine issue-* and deno-* into single categories
        if category.starts_with("issue-") || category.starts_with("jssue-") {
            "issue".to_string()
        } else if category.starts_with("deno-") {
            "deno".to_string()
        } else {
            category.to_string()
        }
    }
}

/// Run SWC conformance tests.
pub fn run_swc(
    options: &RunOptions,
    update_known_failures: bool,
) -> Option<ConformanceSuiteResult> {
    let suite = SwcSuite::new();
    run_conformance_driver(&suite, options, update_known_failures)
}
