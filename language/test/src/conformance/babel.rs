//! Babel parser conformance tests.

use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::parse::{ParseOptions, ParseOutcome, parse_file};
use super::runner::{ConformanceSuite, SuiteResult, Test, TestOutcome, run_conformance_suite};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of babel parser tests
const BABEL_VERSION: &str = "7.26";
const BABEL_COMMIT: &str = "b8ef443"; // short sha

/// Babel parser conformance suite.
#[derive(Debug, Clone)]
pub struct BabelSuite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl BabelSuite {
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("conformance");
        let root = conformance_dir.join("babel");
        Self {
            root,
            conformance_dir,
        }
    }

    fn discover_recursive(&self, dir: &Path, prefix: &str) -> Vec<Test> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // check if this directory is a test case (has input.*)
                    let input_info = self.get_input_file(&path);

                    if let Some((_, file_type)) = input_info {
                        let dir_name = path.file_name().unwrap().to_string_lossy();
                        let name = if prefix.is_empty() {
                            dir_name.to_string()
                        } else {
                            format!("{prefix}/{dir_name}")
                        };
                        let expect_error = self.should_throw(&path);
                        tests.push(Test {
                            name,
                            file_type,
                            expect_error,
                        });
                    } else {
                        // recurse into subdirectory
                        let dir_name = path.file_name().unwrap().to_string_lossy();
                        let new_prefix = if prefix.is_empty() {
                            dir_name.to_string()
                        } else {
                            format!("{prefix}/{dir_name}")
                        };
                        tests.extend(self.discover_recursive(&path, &new_prefix));
                    }
                }
            }
        }

        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }

    fn should_throw(&self, test_dir: &Path) -> bool {
        // check options.json for "throws" key
        let options_path = test_dir.join("options.json");
        if let Ok(content) = std::fs::read_to_string(&options_path) {
            return content.contains("\"throws\"");
        }

        // also check parent directories for options.json
        if let Some(parent) = test_dir.parent() {
            let parent_options = parent.join("options.json");
            if let Ok(content) = std::fs::read_to_string(&parent_options)
                && content.contains("\"throws\"")
            {
                return true;
            }
        }

        false
    }

    fn get_input_file(&self, test_dir: &Path) -> Option<(PathBuf, FileType)> {
        // check if test is in a tsx or jsx directory (babel enables JSX based on directory)
        let path_str = test_dir.to_string_lossy();
        let in_tsx_dir = path_str.contains("/tsx/") || path_str.contains("/tsx-");
        let in_jsx_dir = path_str.contains("/jsx/") || path_str.contains("/jsx-");

        for ext in &["ts", "tsx", "js", "jsx", "mjs"] {
            let input = test_dir.join(format!("input.{ext}"));
            if input.exists() {
                let file_type = match *ext {
                    "tsx" => FileType::TypeScriptXml,
                    "ts" if in_tsx_dir => FileType::TypeScriptXml,
                    "ts" => FileType::TypeScript,
                    "jsx" => FileType::JavaScriptXml,
                    "js" if in_jsx_dir => FileType::JavaScriptXml,
                    _ => FileType::JavaScript,
                };
                return Some((input, file_type));
            }
        }
        None
    }
}

impl Default for BabelSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceSuite for BabelSuite {
    fn name(&self) -> &str {
        "babel"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("babel-known-failures.txt")
    }

    fn discover_tests(&self) -> Vec<Test> {
        // discover tests from typescript and jsx directories
        // (flow is intentionally excluded, we don't support Flow, only TypeScript)
        let mut tests = Vec::new();

        for category in &["typescript", "jsx"] {
            let category_dir = self.root.join(category);
            if category_dir.exists() {
                tests.extend(self.discover_recursive(&category_dir, category));
            }
        }

        tests
    }

    fn run_test(&self, test: &Test) -> TestOutcome {
        let test_dir = self.root.join(&test.name);

        let Some((input_path, _)) = self.get_input_file(&test_dir) else {
            return TestOutcome::Failed;
        };

        let content = match std::fs::read_to_string(&input_path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        let parse_outcome = parse_file(&input_path, &content, test.file_type, ParseOptions::default());

        match (test.expect_error, parse_outcome) {
            (true, ParseOutcome::Error) => TestOutcome::Passed,
            (true, ParseOutcome::Ok) => TestOutcome::Failed,
            (false, ParseOutcome::Ok) => TestOutcome::Passed,
            (false, ParseOutcome::Error) => TestOutcome::Failed,
        }
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download Babel parser tests (version {BABEL_VERSION}, commit {BABEL_COMMIT}):\n\
             \n\
               just language/install-fixtures\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/conformance/babel-fetch.sh\n"
        )
    }
}

/// Run Babel conformance tests.
pub fn run_babel(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = BabelSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
