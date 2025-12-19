use clap::{Args, ValueEnum};
use destack_source::{DiagnosticOptions, ModuleId};

use crate::common::fix::{FixOptions, run_with_fixes};
use crate::common::format::{DiagnosticFormat, FormatOptions, format_diagnostics};
use crate::common::{CompilerContext, CompilerMode, DiagnosticArgs, InputArgs, ProgramArgs};
use crate::console;

/// Output format for diagnostics.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Format {
    /// Human-readable text output (default).
    #[default]
    Text,
    /// JSON output for tooling integration.
    Json,
    /// GitHub Actions annotations format.
    Github,
}

impl From<Format> for DiagnosticFormat {
    fn from(format: Format) -> Self {
        match format {
            Format::Text => DiagnosticFormat::Text,
            Format::Json => DiagnosticFormat::Json,
            Format::Github => DiagnosticFormat::Github,
        }
    }
}

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

    // build format options
    let format_options = FormatOptions {
        format: args.format.into(),
        quiet: args.quiet,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
    };

    // if using fix mode, handle separately
    if args.fix || args.diff {
        let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
        let fix_options = FixOptions {
            apply: args.fix,
            include_unsafe: args.unsafe_fixes,
            diff: args.diff,
        };
        let fix_result = run_with_fixes(
            context.program.clone(),
            &modules,
            &diagnostic_options,
            &fix_options,
            &format_options,
        );

        // also get type errors from the compiler
        let compile_result = context.into_result();
        let diagnostics = compile_result
            .program
            .diagnostics
            .collect()
            .map(&compile_result.diagnostic_options);

        let module_count = modules.len();
        let output_result = format_diagnostics(
            &compile_result.program.files,
            &diagnostics,
            &format_options,
            module_count,
        );

        // return non-zero if any issues remain
        if fix_result.unfixable_count > 0 || output_result.error_count > 0 {
            return 1;
        }
        if output_result.max_warnings_exceeded {
            console::warn(&format!(
                "warning count ({}) exceeds --max-warnings ({})",
                output_result.warning_count,
                args.max_warnings.unwrap_or(0)
            ));
            return 1;
        }
        return output_result.exit_code();
    }

    // normal path: format and print diagnostics
    let compile_result = context.into_result();
    let diagnostics = compile_result
        .program
        .diagnostics
        .collect()
        .map(&compile_result.diagnostic_options);

    let module_count = modules.len();
    let result = format_diagnostics(
        &compile_result.program.files,
        &diagnostics,
        &format_options,
        module_count,
    );

    if result.max_warnings_exceeded {
        console::warn(&format!(
            "warning count ({}) exceeds --max-warnings ({})",
            result.warning_count,
            args.max_warnings.unwrap_or(0)
        ));
        return 1;
    }

    result.exit_code()
}
