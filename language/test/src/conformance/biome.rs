//! Biome parser conformance tests.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

use super::runner::{ConformanceSuite, SuiteResult, run_conformance_suite};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of Biome parser tests
const BIOME_VERSION: &str = "1.x";
const BIOME_COMMIT: &str = "9f1b3b0"; // short sha

/// Biome parser conformance suite.
#[derive(Debug, Clone)]
pub struct BiomeSuite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl BiomeSuite {
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("conformance");
        let root = conformance_dir.join("biome");
        Self { root, conformance_dir }
    }

    fn discover_in_dir(&self, dir: &Path, prefix: &str) -> Vec<String> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy();
                    // only include source files, not snapshots
                    if matches!(ext.as_ref(), "ts" | "tsx" | "js" | "jsx") {
                        let name = path.file_name().unwrap().to_string_lossy();
                        // skip .d.ts files
                        if name.ends_with(".d.ts") {
                            continue;
                        }
                        tests.push(format!("{prefix}/{name}"));
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

impl Default for BiomeSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceSuite for BiomeSuite {
    fn name(&self) -> &str {
        "biome"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("biome-known-failures.txt")
    }

    fn discover_tests(&self) -> Vec<String> {
        let mut tests = Vec::new();

        // Biome has ok/ (should pass) and error/ (should fail) directories
        let ok_dir = self.root.join("ok");
        if ok_dir.exists() {
            tests.extend(self.discover_in_dir(&ok_dir, "ok"));
        }

        let error_dir = self.root.join("error");
        if error_dir.exists() {
            tests.extend(self.discover_in_dir(&error_dir, "error"));
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

        // ok/ should pass, error/ should fail
        let should_fail = name.starts_with("error/");
        let has_errors = self.parse_and_check(&path, &content, file_type);

        if should_fail {
            has_errors
        } else {
            !has_errors
        }
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download Biome parser tests (version {BIOME_VERSION}, commit {BIOME_COMMIT}):\n\
             \n\
               just language/install-fixtures\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/conformance/biome-fetch.sh\n"
        )
    }
}

/// Run Biome conformance tests.
pub fn run_biome(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = BiomeSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
