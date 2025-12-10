//! SWC parser conformance tests.

use std::path::{Path, PathBuf};

use super::parse::{ParseOptions, ParseOutcome, file_type_from_path, parse_file};
use super::runner::{ConformanceSuite, SuiteResult, TestOutcome, run_conformance_suite};
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

    fn discover_recursive(&self, dir: &Path, prefix: &str) -> Vec<String> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy();

                if path.is_dir() {
                    let new_prefix = if prefix.is_empty() {
                        name.to_string()
                    } else {
                        format!("{prefix}/{name}")
                    };
                    tests.extend(self.discover_recursive(&path, &new_prefix));
                } else if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy();
                    if matches!(ext.as_ref(), "ts" | "tsx" | "js" | "jsx") {
                        // skip .d.ts files
                        if name.ends_with(".d.ts") {
                            continue;
                        }
                        let stem = path.file_stem().unwrap().to_string_lossy();
                        let test_name = if prefix.is_empty() {
                            format!("{stem}.{ext}")
                        } else {
                            format!("{prefix}/{stem}.{ext}")
                        };
                        tests.push(test_name);
                    }
                }
            }
        }

        tests.sort();
        tests
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

    fn discover_tests(&self) -> Vec<String> {
        let mut tests = Vec::new();

        // discover tests from typescript, jsx, and js directories
        for category in &["typescript", "jsx", "js"] {
            let category_dir = self.root.join(category);
            if category_dir.exists() {
                tests.extend(self.discover_recursive(&category_dir, category));
            }
        }

        tests
    }

    fn run_test(&self, name: &str) -> TestOutcome {
        let path = self.root.join(name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        let file_type = file_type_from_path(&path);
        let parse_outcome = parse_file(&path, &content, file_type, ParseOptions::default());

        // SWC tests: if in "errors" directory, should fail; otherwise should pass
        let should_fail = name.contains("/errors/") || name.contains("typescript-errors");

        match (should_fail, parse_outcome) {
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
}

/// Run SWC conformance tests.
pub fn run_swc(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = SwcSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
