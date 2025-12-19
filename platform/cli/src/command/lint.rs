use clap::Args;

use super::check::{self, Format};
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

    /// Output format (text, json, github).
    #[arg(long, short = 'f', value_enum, default_value = "text")]
    pub format: Format,

    /// Only show errors, suppress warnings.
    #[arg(long, short = 'q')]
    pub quiet: bool,

    /// Exit with error if warning count exceeds this threshold.
    #[arg(long = "max-warnings", value_name = "N")]
    pub max_warnings: Option<usize>,

    /// Show statistics grouped by rule.
    #[arg(long)]
    pub statistics: bool,
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
        format: args.format,
        quiet: args.quiet,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
    };

    check::run(&check_args)
}
