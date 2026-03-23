use destack_artifact::ArtifactKey;
use destack_compiler::{Compiler, CompilerOptions};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Runner, SharedMemoryWorkspace, Suite,
    check_diagnostics, discover_file_cases, fixtures_dir,
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

    // set up session and program with memory filesystem containing the test file
    let cwd = test.path.parent().unwrap().to_path_buf();
    let workspace = SharedMemoryWorkspace::new(cwd.clone());
    let memory_fs = workspace.fs();
    memory_fs
        .add_file(&test.path, content.as_bytes())
        .expect("failed to add test file to memory fs");
    let session = workspace.session();
    let program = session.add_root(cwd);

    // compile the file
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );
    let module_id = match compiler.resolve_path_to_module(&test.path) {
        Ok(id) => id,
        Err(e) => {
            return CaseResult::Failed {
                message: format!("failed to resolve module: {e:?}"),
            };
        }
    };
    let profile = program.default_profile_id_for_module(module_id);
    compiler.enqueue(ArtifactKey::DirAnalyzed {
        module: module_id,
        profile,
    });
    compiler.compile();
    drop(compiler);

    // check for unexpected diagnostics
    check_diagnostics(test, &program.files, &program.diagnostics)
}
