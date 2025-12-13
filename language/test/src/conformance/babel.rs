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

    fn discover_recursive(&self, directory: &Path, prefix: &str) -> Vec<Test> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // check if this directory is a test case (has input.*)
                    let input_info = self.get_input_file(&path);

                    // if this directory is a test case, register it
                    if let Some((_, file_type)) = input_info {
                        // skip script-mode tests (we only support strict module mode)
                        if self.is_script_mode(&path) {
                            continue;
                        }
                        let directory_name = path.file_name().unwrap().to_string_lossy();
                        let name = if prefix.is_empty() {
                            directory_name.to_string()
                        } else {
                            format!("{prefix}/{directory_name}")
                        };
                        let expect_error = self.should_throw(&path);
                        tests.push(Test {
                            name,
                            file_type,
                            expect_error,
                        });
                    } else {
                        // otherwise recurse into the subdirectory
                        let directory_name = path.file_name().unwrap().to_string_lossy();
                        let new_prefix = if prefix.is_empty() {
                            directory_name.to_string()
                        } else {
                            format!("{prefix}/{directory_name}")
                        };
                        tests.extend(self.discover_recursive(&path, &new_prefix));
                    }
                }
            }
        }

        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }

    /// Check if a test uses script mode (sourceType: "script").
    /// We only support strict module mode, so we skip these tests.
    fn is_script_mode(&self, test_dir: &Path) -> bool {
        fn has_script_source_type(path: &Path) -> bool {
            if let Ok(content) = std::fs::read_to_string(path) {
                return content.contains("\"sourceType\": \"script\"")
                    || content.contains("\"sourceType\":\"script\"");
            }
            false
        }

        // check options.json in the test directory
        let options_path = test_dir.join("options.json");
        if has_script_source_type(&options_path) {
            return true;
        }

        // also check parent directories for options.json (babel allows inheritance)
        if let Some(parent) = test_dir.parent() {
            if has_script_source_type(&parent.join("options.json")) {
                return true;
            }
        }

        false
    }

    fn should_throw(&self, test_dir: &Path) -> bool {
        // check options.json for "throws" key
        let options_path = test_dir.join("options.json");
        if let Ok(content) = std::fs::read_to_string(&options_path)
            && content.contains("\"throws\"")
        {
            return true;
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

        // check output.json for "errors" array with content
        // babel stores expected errors in output.json as: "errors": ["SyntaxError: ..."]
        let output_path = test_dir.join("output.json");
        if let Ok(content) = std::fs::read_to_string(&output_path) {
            // look for non-empty errors array: "errors": [ followed by content before ]
            if let Some(errors_start) = content.find("\"errors\":") {
                let after_errors = &content[errors_start..];
                // check if there's actual content in the errors array (not just "errors": [])
                if let Some(bracket_start) = after_errors.find('[') {
                    let after_bracket = &after_errors[bracket_start + 1..];
                    // trim whitespace and check if next char is not ]
                    let trimmed = after_bracket.trim_start();
                    if !trimmed.starts_with(']') {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if options.json enables JSX plugin.
    fn has_jsx_plugin(test_dir: &Path) -> bool {
        fn check(path: &Path) -> bool {
            std::fs::read_to_string(path)
                .map(|c| c.contains("\"jsx\""))
                .unwrap_or(false)
        }
        check(&test_dir.join("options.json"))
            || test_dir
                .parent()
                .map(|p| check(&p.join("options.json")))
                .unwrap_or(false)
    }

    fn get_input_file(&self, test_dir: &Path) -> Option<(PathBuf, FileType)> {
        let path_str = test_dir.to_string_lossy();
        let in_tsx_dir = path_str.contains("/tsx/") || path_str.contains("/tsx-");
        let in_jsx_dir = path_str.contains("/jsx/") || path_str.contains("/jsx-");
        let has_jsx = Self::has_jsx_plugin(test_dir);
        for ext in &["ts", "tsx", "js", "jsx", "mjs"] {
            let input = test_dir.join(format!("input.{ext}"));
            if input.exists() {
                let file_type = match *ext {
                    "tsx" => FileType::TypeScriptXml,
                    "ts" if in_tsx_dir || has_jsx => FileType::TypeScriptXml,
                    "ts" => FileType::TypeScript,
                    "jsx" => FileType::JavaScriptXml,
                    "js" if in_jsx_dir || has_jsx => FileType::JavaScriptXml,
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

    fn discover(&self) -> Vec<Test> {
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

    fn run(&self, test: &Test) -> TestOutcome {
        let test_dir = self.root.join(&test.name);

        let Some((input_path, _)) = self.get_input_file(&test_dir) else {
            return TestOutcome::Failed;
        };

        let content = match std::fs::read_to_string(&input_path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        let parse_outcome = parse_file(
            &input_path,
            &content,
            test.file_type,
            ParseOptions::default(),
        );

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

    fn category_for_test(&self, test_name: &str) -> String {
        // babel tests are typescript/subcategory/... or jsx/subcategory/...
        // use the second segment as the category
        let parts: Vec<&str> = test_name.split('/').collect();
        if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            parts.first().unwrap_or(&"unknown").to_string()
        }
    }
}

/// Run Babel conformance tests.
pub fn run_babel(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = BabelSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
