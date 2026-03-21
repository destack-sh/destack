use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::expected::{load_expected_output, load_prettier_expected_case};
use super::fixtures::{
    expect_error_from_path, is_formattable_file_type, should_skip_directory,
    should_skip_fixture_file,
};
use super::format::{default_conformance_formatter_options, run_formatter_case};
use super::runner::{
    ConformanceSuite, ExpectedOutput, SuiteResult, Test, TestOutcome, run_conformance_suite,
};
use crate::conformance::formatter;
use crate::harness::TestOptions;

// pinned version of Prettier formatter fixtures
const PRETTIER_VERSION: &str = "3.x";
const PRETTIER_REF: &str = "main";

/// Prettier formatter conformance suite.
#[derive(Debug, Clone)]
pub struct PrettierSuite {
    root: PathBuf,
    suite_dir: PathBuf,
}

impl PrettierSuite {
    pub fn new() -> Self {
        let suite_dir = formatter::suite_dir("prettier");
        let root = formatter::suite_tests_dir("prettier");
        Self { root, suite_dir }
    }

    fn discover_in_dir(&self, dir: &Path, tests: &mut Vec<Test>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                let directory_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                if should_skip_directory(directory_name) {
                    continue;
                }
                self.discover_in_dir(&path, tests);
                continue;
            }
            if !path.is_file() || should_skip_fixture_file(&path) {
                continue;
            }

            let relative = path.strip_prefix(&self.root).unwrap_or(&path);
            let test_name = relative.to_string_lossy().replace('\\', "/");

            let Some(file_type) = infer_prettier_file_type(&path, &test_name) else {
                continue;
            };
            if !is_formattable_file_type(file_type) {
                continue;
            }

            let test = if expect_error_from_path(&test_name) {
                Test::fail(test_name, file_type)
            } else {
                Test::pass(test_name, file_type)
            };

            let snapshot_path = path
                .parent()
                .map(|directory| directory.join("__snapshots__").join("format.test.js.snap"));
            let test = if let Some(snapshot_path) = snapshot_path
                && snapshot_path.is_file()
                && let Some(file_name) = path.file_name().and_then(|value| value.to_str())
                && let Ok(relative_snapshot) = snapshot_path.strip_prefix(&self.root)
            {
                test.with_expected_output(ExpectedOutput::PrettierSnapshot {
                    path: relative_snapshot.to_path_buf(),
                    key: format!("{file_name} format 1"),
                })
            } else {
                test
            };

            tests.push(test);
        }
    }
}

/// Infer the parser file type for a prettier fixture path.
fn infer_prettier_file_type(path: &Path, _test_name: &str) -> Option<FileType> {
    let file_type = FileType::from_path(path)?;

    // parse `.js` conformance fixtures in jsx-capable mode:
    // prettier keeps jsx-heavy fixtures under `js/` paths, so plain js mode
    // can fail on valid fixture inputs like `<Hello />`
    if file_type == FileType::JavaScript {
        return Some(FileType::JavaScriptXml);
    }

    Some(file_type)
}

impl Default for PrettierSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceSuite for PrettierSuite {
    fn name(&self) -> &str {
        "prettier"
    }

    fn suite_dir(&self) -> &Path {
        &self.suite_dir
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn discover(&self) -> Vec<Test> {
        let mut tests = Vec::new();
        self.discover_in_dir(&self.root, &mut tests);
        tests
    }

    fn run(&self, test: &Test, show_diff: bool) -> TestOutcome {
        let path = self.root.join(&test.name);
        let mut formatter_options = default_conformance_formatter_options();
        let expected_output = match &test.expected_output {
            ExpectedOutput::PrettierSnapshot { path, key } => {
                if let Some(expected_case) =
                    load_prettier_expected_case(self.root(), path, key, formatter_options)
                {
                    formatter_options = expected_case.formatter_options;
                    Some(expected_case.output)
                } else {
                    None
                }
            }
            _ => load_expected_output(self.root(), &test.expected_output),
        };

        run_formatter_case(
            &path,
            test.file_type,
            expected_output.as_deref(),
            formatter_options,
            test.expect_error,
            show_diff,
        )
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download Prettier formatter fixtures (version {PRETTIER_VERSION}, ref {PRETTIER_REF}):\n\
             \n\
               just language/install-formatter-conformance\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/conformance/formatter/prettier/fetch.sh\n"
        )
    }
}

/// Run Prettier formatter conformance tests.
pub fn run_prettier(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = PrettierSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use destack_source::FileType;

    use super::infer_prettier_file_type;

    #[test]
    fn test_infer_prettier_file_type_promotes_jsx_directory_js() {
        let file_type = infer_prettier_file_type(Path::new("sample.js"), "jsx/jsx/sample.js");
        assert_eq!(file_type, Some(FileType::JavaScriptXml));
    }

    #[test]
    fn test_infer_prettier_file_type_promotes_js_outside_jsx_directory() {
        let file_type = infer_prettier_file_type(Path::new("sample.js"), "js/module/sample.js");
        assert_eq!(file_type, Some(FileType::JavaScriptXml));
    }
}
