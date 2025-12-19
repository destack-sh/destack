use clap::Args;

use super::check;
use crate::common::{DiagnosticArgs, InputArgs, ProgramArgs};

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

    /// Automatically fix problems.
    #[arg(long)]
    pub fix: bool,

    /// Apply unsafe fixes in addition to safe fixes (requires --fix).
    #[arg(long = "unsafe-fixes")]
    pub unsafe_fixes: bool,

    /// Show what --fix would change without applying.
    #[arg(long)]
    pub diff: bool,
}

/// Lint source files for style and correctness issues.
/// (This is an alias for `check` which includes linting by default).
pub fn run(args: &LintArgs) -> i32 {
    // convert to CheckArgs and delegate
    let check_args = check::CheckArgs {
        input: args.input.clone(),
        program: args.program.clone(),
        diagnostics: args.diagnostics.clone(),
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        diff: args.diff,
        no_lint: false, // lint always includes linting
    };

    check::run(&check_args)
}
