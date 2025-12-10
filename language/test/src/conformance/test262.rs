use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

use super::runner::{ConformanceSuite, SuiteResult, run_conformance_suite};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of test262-parser-tests
// update this when upgrading the test suite
const TEST262_VERSION: &str = "0.1.0";
const TEST262_COMMIT: &str = "521b4ab"; // short sha

/// Test262 parser conformance suite.
#[derive(Debug, Clone)]
pub struct Test262Suite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl Test262Suite {
    /// Create suite with default paths.
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("conformance");
        let root = conformance_dir.join("test262");
        Self {
            root,
            conformance_dir,
        }
    }

    /// Create suite with custom root.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let conformance_dir = root.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self {
            root,
            conformance_dir,
        }
    }

    fn discover_in_dir(&self, dir: &Path, prefix: &str) -> Vec<String> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "js")
                    && let Some(name) = path.file_stem()
                {
                    tests.push(format!("{prefix}/{}", name.to_string_lossy()));
                }
            }
        }

        tests.sort();
        tests
    }

    fn parse_and_check(&self, path: &Path, content: &str) -> bool {
        // set up minimal program context
        let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let files = Arc::new(FileRegistry::new());
        let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
        let program = Arc::new(Program::new(LanguageOptions::default(), cwd, fs, files));

        // create file
        let uri = Uri::from_path(path);
        let file_id = program.files.next_id();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let file = File::from_text(
            file_id,
            name,
            uri,
            Some(path.to_path_buf()),
            FileType::JavaScript,
            content.to_string(),
        );
        program.files.insert(file);
        let file = program.files.get(file_id);

        // parse
        let mut parser = Parser::lex_file(file, program.language);
        let _ = parser.parse();

        // return whether there were errors
        !parser.diagnostics.is_empty()
    }
}

impl Default for Test262Suite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceSuite for Test262Suite {
    fn name(&self) -> &str {
        "test262"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("test262-known-failures.txt")
    }

    fn discover_tests(&self) -> Vec<String> {
        let mut tests = Vec::new();

        // pass/ directory: files that should parse successfully
        let pass_dir = self.root.join("pass");
        if pass_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_dir, "pass"));
        }

        // fail/ directory: files that should fail to parse
        let fail_dir = self.root.join("fail");
        if fail_dir.exists() {
            tests.extend(self.discover_in_dir(&fail_dir, "fail"));
        }

        // pass-explicit/ directory: files that should parse in module mode
        let pass_explicit_dir = self.root.join("pass-explicit");
        if pass_explicit_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_explicit_dir, "pass-explicit"));
        }

        // early/ directory: files with early errors (parse succeeds, has semantic errors)
        let early_dir = self.root.join("early");
        if early_dir.exists() {
            tests.extend(self.discover_in_dir(&early_dir, "early"));
        }

        tests
    }

    fn run_test(&self, name: &str) -> bool {
        let parts: Vec<&str> = name.splitn(2, '/').collect();
        if parts.len() != 2 {
            return false;
        }

        let (category, test_name) = (parts[0], parts[1]);
        let path = self.root.join(category).join(format!("{test_name}.js"));

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let should_pass = matches!(category, "pass" | "pass-explicit");
        let has_errors = self.parse_and_check(&path, &content);

        // pass tests should parse without errors
        // fail tests should have errors
        if should_pass { !has_errors } else { has_errors }
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download test262 parser tests (version {TEST262_VERSION}, commit {TEST262_COMMIT}):\n\
             \n\
               just language/install-fixtures\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/conformance/test262-fetch.sh\n"
        )
    }
}

/// Run test262 conformance tests.
pub fn run_test262(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = Test262Suite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
