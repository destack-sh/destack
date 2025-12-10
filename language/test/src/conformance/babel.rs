//! Babel parser conformance tests.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

use super::runner::{ConformanceSuite, SuiteResult, run_conformance_suite};
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
        Self { root, conformance_dir }
    }

    fn discover_recursive(&self, dir: &Path, prefix: &str) -> Vec<String> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // check if this directory is a test case (has input.*)
                    let has_input = std::fs::read_dir(&path)
                        .map(|entries| {
                            entries.flatten().any(|e| {
                                e.file_name()
                                    .to_string_lossy()
                                    .starts_with("input.")
                            })
                        })
                        .unwrap_or(false);

                    if has_input {
                        let name = path.file_name().unwrap().to_string_lossy();
                        let test_name = if prefix.is_empty() {
                            name.to_string()
                        } else {
                            format!("{prefix}/{name}")
                        };
                        tests.push(test_name);
                    } else {
                        // recurse into subdirectory
                        let name = path.file_name().unwrap().to_string_lossy();
                        let new_prefix = if prefix.is_empty() {
                            name.to_string()
                        } else {
                            format!("{prefix}/{name}")
                        };
                        tests.extend(self.discover_recursive(&path, &new_prefix));
                    }
                }
            }
        }

        tests.sort();
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
            if let Ok(content) = std::fs::read_to_string(&parent_options) {
                if content.contains("\"throws\"") {
                    return true;
                }
            }
        }

        false
    }

    fn get_input_file(&self, test_dir: &Path) -> Option<(PathBuf, FileType)> {
        for ext in &["ts", "tsx", "js", "jsx", "mjs"] {
            let input = test_dir.join(format!("input.{ext}"));
            if input.exists() {
                let file_type = match *ext {
                    "ts" => FileType::TypeScript,
                    "tsx" => FileType::TypeScriptXml,
                    "jsx" => FileType::JavaScriptXml,
                    _ => FileType::JavaScript,
                };
                return Some((input, file_type));
            }
        }
        None
    }

    fn parse_and_check(&self, path: &Path, content: &str, file_type: FileType) -> bool {
        let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let files = Arc::new(FileRegistry::new());
        let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
        let program = Arc::new(Program::new(LanguageOptions::default(), cwd, fs, files));

        let uri = Uri::from_path(path);
        let file_id = program.files.next_id();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let file = File::from_text(
            file_id,
            name,
            uri,
            Some(path.to_path_buf()),
            file_type,
            content.to_string(),
        );
        program.files.insert(file);
        let file = program.files.get(file_id);

        let mut parser = Parser::lex_file(file, program.language);
        let _ = parser.parse();

        !parser.diagnostics.is_empty()
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

    fn discover_tests(&self) -> Vec<String> {
        // discover tests from typescript and jsx directories
        let mut tests = Vec::new();

        for category in &["typescript", "jsx", "flow"] {
            let category_dir = self.root.join(category);
            if category_dir.exists() {
                tests.extend(
                    self.discover_recursive(&category_dir, category)
                );
            }
        }

        tests
    }

    fn run_test(&self, name: &str) -> bool {
        let test_dir = self.root.join(name);

        let Some((input_path, file_type)) = self.get_input_file(&test_dir) else {
            return false;
        };

        let content = match std::fs::read_to_string(&input_path) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let should_fail = self.should_throw(&test_dir);
        let has_errors = self.parse_and_check(&input_path, &content, file_type);

        if should_fail {
            has_errors
        } else {
            !has_errors
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
