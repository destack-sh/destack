use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_compiler::{Compiler, CompilerOptions};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Runner, SharedMemoryWorkspace, Suite,
    check_repository_diagnostic_collection, current_workspace_revision,
    default_profile_id_for_module, discover_file_cases, fixtures_dir, provide_workspace_artifacts,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct CompilerSmokeSuite;

impl Suite for CompilerSmokeSuite {
    fn name(&self) -> &'static str {
        "smoke-compiler"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let smoke_directory = fixtures_dir().join("smoke").join("compiler");
        discover_file_cases(&smoke_directory, &["ds"], "destack_test::smoke::compiler")
            .expect("failed to discover tests")
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        run_compiler_case(case)
    }
}

/// Run all compiler smoke tests.
pub fn run_compiler_smoke_tests(options: &RunOptions) -> std::process::ExitCode {
    Runner::run_suite(CompilerSmokeSuite, options)
}

/// Run a single compiler smoke test.
fn run_compiler_case(test: &Case) -> CaseResult {
    // read file content from disk
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return CaseResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };

    // set up repository and program with memory filesystem containing the test file
    let cwd = test.path.parent().unwrap().to_path_buf();
    let workspace = SharedMemoryWorkspace::new(cwd.clone());
    let memory_fs = workspace.fs();
    memory_fs
        .add_file(&test.path, content.as_bytes())
        .expect("failed to add test file to memory fs");
    let repository = workspace.repository();
    // compile the file
    let compiler = Arc::new(Compiler::new(
        repository.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    ));
    let revision = current_workspace_revision(&repository);
    let module_id = match compiler.resolve_path_to_module(revision, &test.path) {
        Ok(id) => id,
        Err(e) => {
            return CaseResult::Failed {
                message: format!("failed to resolve module: {e:?}"),
            };
        }
    };
    let profile = default_profile_id_for_module(&repository, revision, module_id);
    let artifact_keys = vec![ArtifactKey::DirAnalyzed {
        module: module_id,
        profile,
    }];
    let _revision = provide_workspace_artifacts(repository.clone(), compiler, &artifact_keys);

    // check for unexpected diagnostics
    let diagnostics = repository.module_artifact_diagnostics(revision, module_id, profile);

    check_repository_diagnostic_collection(test, &repository, revision, &diagnostics)
}
