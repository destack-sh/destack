//! SWC parser conformance tests.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

use super::runner::{ConformanceSuite, SuiteResult, run_conformance_suite};
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

    fn run_test(&self, name: &str) -> bool {
        let path = self.root.join(name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return false,
        };

        // determine file type from extension
        let file_type = if name.ends_with(".tsx") {
            FileType::TypeScriptXml
        } else if name.ends_with(".ts") {
            FileType::TypeScript
        } else if name.ends_with(".jsx") {
            FileType::JavaScriptXml
        } else {
            FileType::JavaScript
        };

        // SWC tests: if in "errors" directory, should fail; otherwise should pass
        let should_fail = name.contains("/errors/") || name.contains("typescript-errors");
        let has_errors = self.parse_and_check(&path, &content, file_type);

        if should_fail { has_errors } else { !has_errors }
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
