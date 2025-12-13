use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler};
use destack_source::{
    DiagnosticSeverity, FileRegistry, FileSystem, FileType, LanguageOptions, LanguageType,
    MemoryFileSystem,
};
use destack_workspace::Program;

/// Outcome of checking a file for conformance testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseOutcome {
    /// File parsed without relevant errors.
    Ok,
    /// File had relevant errors.
    Error,
}

/// What category of errors a test cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum TestArea {
    /// Pure grammar-level conformance.
    Parse,
    /// "Early" error semi-semantic conformance.
    #[default]
    Early,
}

impl TestArea {
    /// Check if an error code is relevant for this test area.
    pub(super) fn is_relevant_error(&self, code: &str) -> bool {
        // nocheckin TODO #Cleanup: use more rigorous diagnostic definition?
        // (reorganize warnings/errors and other diagnostics incl. future lints to use some macro,
        //  such that we can easily check against them and list them statically like for docs and tests)
        match self {
            TestArea::Parse => code.starts_with("EP") || code.starts_with("EI"),
            TestArea::Early => {
                code.starts_with("EP")
                    || code.starts_with("EI")
                    || code.starts_with("EB")
                    || code == "ER010" // MissingTarget
                    || code == "ER011" // InvalidTarget
                    || code == "EA020" // InvalidLineage
                    || code == "EA021" // InvalidBreak
                    || code == "EA022" // InvalidContinue
                    || code == "EA023" // InvalidAwait
                    || code == "EA024" // InvalidYield
                    || code == "EA025" // InvalidReturn
                    || code == "EA026" // InvalidConstructor
                    || code == "EA027" // InvalidInterface
                    || code == "EA028" // InvalidFunction
                    || code == "EA029" // InvalidMethod
                    || code == "EA030" // InvalidMemberModifier
                    || code == "EA031" // InvalidParameterProperty
                    || code == "EA032" // InvalidStaticBlockModifier
            }
        }
    }
}

/// Options for parsing in conformance tests.
#[derive(Debug, Clone, Default)]
pub(super) struct ParseOptions {
    /// What category of errors to check for.
    pub area: TestArea,
}

/// Parse and bind a file, return the outcome.
pub(super) fn parse_file(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
) -> ParseOutcome {
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let memory_fs = Arc::new(MemoryFileSystem::new());
    memory_fs
        .add_file(path, content.as_bytes())
        .expect("failed to add file to memory fs");
    let fs: Arc<dyn FileSystem> = memory_fs;

    // set language type based on file type for proper compatibility mode
    let language_type = LanguageType::from(file_type);
    let language = LanguageOptions::default().with_type(language_type);
    let program = Arc::new(Program::new(language, cwd, fs, files));

    // create compiler and resolve module
    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
            workers: 1,
            ..Default::default()
        },
    );

    let module_id = match compiler.resolve_path_to_module(&path.to_path_buf()) {
        Ok(id) => id,
        Err(_) => return ParseOutcome::Error,
    };

    // run up to analyze
    compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module: module_id });
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        compiler.compile();
    }));

    // panics during compilation are treated as errors
    if result.is_err() {
        return ParseOutcome::Error;
    }

    // check for errors relevant to the test area
    let has_relevant_error = program.diagnostics.iter().into_iter().any(|d| {
        d.severity == DiagnosticSeverity::Error && options.area.is_relevant_error(&d.code)
    });

    if has_relevant_error {
        ParseOutcome::Error
    } else {
        ParseOutcome::Ok
    }
}
