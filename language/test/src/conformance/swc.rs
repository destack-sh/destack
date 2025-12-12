use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::parse::{ParseOptions, ParseOutcome, parse_file};
use super::runner::{ConformanceSuite, SuiteResult, Test, TestOutcome, run_conformance_suite};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of SWC parser tests
const SWC_VERSION: &str = "1.x";
const SWC_COMMIT: &str = "5b9d77c"; // short sha

/// SWC parser conformance suite.
#[derive(Debug, Clone)]
pub struct SwcSuite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl SwcSuite {
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("conformance");
        let root = conformance_dir.join("swc");
        Self {
            root,
            conformance_dir,
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

    fn discover_recursive(&self, dir: &Path, prefix: &str, category: &str) -> Vec<Test> {
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

                        tests.push(Test {
                            name,
                            file_type,
                            expect_error,
                        });
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
        if category.contains("tsx") || name.ends_with(".tsx") {
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

impl ConformanceSuite for SwcSuite {
    fn name(&self) -> &str {
        "swc"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("swc-known-failures.txt")
    }

    fn discover(&self) -> Vec<Test> {
        let mut tests = Vec::new();

        // discover tests from typescript, jsx, and js directories
        for category in &["typescript", "jsx", "js"] {
            let category_dir = self.root.join(category);
            if category_dir.exists() {
                tests.extend(self.discover_recursive(&category_dir, category, category));
            }
        }

        tests
    }

    fn run(&self, test: &Test) -> TestOutcome {
        let path = self.root.join(&test.name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        let parse_outcome = parse_file(&path, &content, test.file_type, ParseOptions::default());

        match (test.expect_error, parse_outcome) {
            (true, ParseOutcome::Error) => TestOutcome::Passed,
            (true, ParseOutcome::Ok) => TestOutcome::Failed,
            (false, ParseOutcome::Ok) => TestOutcome::Passed,
            (false, ParseOutcome::Error) => TestOutcome::Failed,
        }
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download SWC parser tests (version {SWC_VERSION}, commit {SWC_COMMIT}):\n\
             \n\
               just language/install-fixtures\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/conformance/swc-fetch.sh\n"
        )
    }

    fn category_for_test(&self, test_name: &str) -> String {
        // swc tests are typescript/subcategory/... or jsx/subcategory/... or js/subcategory/...
        // use the second segment as the category
        let parts: Vec<&str> = test_name.split('/').collect();
        let category = if parts.len() >= 2 {
            parts[1]
        } else {
            parts.first().unwrap_or(&"unknown")
        };

        // combine all issue-* into one "issue" category
        if category.starts_with("issue-") || category.starts_with("jssue-") {
            "issue".to_string()
        } else {
            category.to_string()
        }
    }
}

/// Run SWC conformance tests.
pub fn run_swc(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = SwcSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
