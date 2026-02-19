use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::fixtures::{
    expect_error_from_path, is_formattable_file_type, should_skip_directory,
    should_skip_fixture_file,
};
use super::format::{default_conformance_formatter_options, run_formatter_case};
use super::runner::{ConformanceSuite, SuiteResult, Test, TestOutcome, run_conformance_suite};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of Prettier formatter fixtures
const PRETTIER_VERSION: &str = "3.x";
const PRETTIER_REF: &str = "main";

/// Prettier formatter conformance suite.
#[derive(Debug, Clone)]
pub struct PrettierSuite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl PrettierSuite {
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("formatter").join("conformance");
        let root = conformance_dir
            .join("staging")
            .join("prettier")
            .join("tests")
            .join("format");
        Self {
            root,
            conformance_dir,
        }
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

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("prettier-known-failures.txt")
    }

    fn discover(&self) -> Vec<Test> {
        let mut tests = Vec::new();
        self.discover_in_dir(&self.root, &mut tests);
        tests
    }

    fn run(&self, test: &Test, show_diff: bool) -> TestOutcome {
        let path = self.root.join(&test.name);

        run_formatter_case(
            &path,
            test.file_type,
            None,
            default_conformance_formatter_options(),
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
               ./language/test/fixtures/formatter/conformance/prettier-fetch.sh\n"
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
