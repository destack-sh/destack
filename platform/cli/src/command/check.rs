use clap::Args;
use destack_source::{DiagnosticOptions, ModuleId};

use crate::common::fix::{FixOptions, run_with_fixes};
use crate::common::{CompilerContext, CompilerMode, DiagnosticArgs, InputArgs, ProgramArgs};
use crate::console;

#[derive(Args, Debug, Clone)]
pub struct CheckArgs {
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

    /// Only type-check, skip linting.
    #[arg(long = "no-lint")]
    pub no_lint: bool,
}

/// Check source files for type errors and lint issues.
pub fn run(args: &CheckArgs) -> i32 {
    // validate flag combinations
    if args.unsafe_fixes && !args.fix && !args.diff {
        console::warn("--unsafe-fixes has no effect without --fix or --diff");
    }

    if args.no_lint && (args.fix || args.diff) {
        console::warn("--fix and --diff have no effect with --no-lint");
    }

    // when --fix or --diff, run linting manually via run_with_fixes (to get actual fixes)
    // otherwise, run linting through the compiler
    let mode = if args.no_lint || args.fix || args.diff {
        CompilerMode::Check
    } else {
        CompilerMode::Lint
    };
    let context = CompilerContext::new(&args.program, &args.diagnostics, mode);
    let sources = match context.load_sources(&args.input) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let modules: Vec<ModuleId> = match context.enqueue(&sources) {
        Ok(m) => m,
        Err(code) => return code,
    };

    // compile (type check, and lint if not --no-lint)
    context.run_compile();

    // if no-lint or no fix options, just finish normally
    if args.no_lint || (!args.fix && !args.diff) {
        return context.into_result().finish();
    }

    // run fix logic
    let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
    let fix_options = FixOptions {
        apply: args.fix,
        include_unsafe: args.unsafe_fixes,
        diff: args.diff,
    };
    let result = run_with_fixes(
        context.program.clone(),
        &modules,
        &diagnostic_options,
        &fix_options,
    );

    // also print type errors from the compiler
    let compile_result = context.into_result();
    let type_errors = compile_result.finish();

    // return non-zero if any issues remain
    if result.unfixable_count > 0 || type_errors != 0 {
        1
    } else {
        0
    }
}
