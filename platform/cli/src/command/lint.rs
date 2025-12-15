use clap::Args;

use crate::common::{CompileContext, DiagnosticArgs, InputArgs, ProgramArgs};

#[derive(Args, Debug, Clone)]
pub struct LintArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// The diagnostic options.
    #[command(flatten)]
    pub diagnostics: DiagnosticArgs,
}

/// Lint source files for style and correctness issues.
pub fn run(args: &LintArgs) -> i32 {
    let context = CompileContext::for_lint(&args.program, &args.diagnostics);

    let sources = match context.load_sources(&args.input) {
        Ok(s) => s,
        Err(code) => return code,
    };

    if let Err(code) = context.enqueue(&sources) {
        return code;
    }

    context.compile().finish()
}
