use clap::{Args, ValueEnum};
use destack_source::DiagnosticOptions;

use crate::common::fix::{FixOptions, run_with_fixes};
use crate::common::format::{DiagnosticFormat, FormatOptions, format_diagnostics_with_writer};
use crate::common::{
    CommandReport, CommandStats, CompilerMode, DiagnosticArgs, InputArgs, ProgramArgs,
    ProgressMode, ProgressReporter, ReportArgs, StatsSummary, collect_diagnostics_json, is_tty,
    print_report, print_stats_summary, report_error,
};
use crate::console;
use crate::pipeline::compile::{CompileRequest, prepare_compile};

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
    /// Convert from CLI format to diagnostic format.
    fn from(format: Format) -> Self {
        match format {
            Format::Text => DiagnosticFormat::Text,
            Format::Json => DiagnosticFormat::Json,
            Format::Github => DiagnosticFormat::Github,
        }
    }
}

/// Progress display mode.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Progress {
    /// Auto-detect: show progress on TTY, hide otherwise.
    #[default]
    Auto,
    /// Always show progress spinner.
    On,
    /// Never show progress.
    Off,
    /// Show detailed progress with task counts.
    Detailed,
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

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

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

    /// Suppress diagnostics output, still print the summary line in text mode.
    #[arg(long = "no-diagnostics")]
    pub no_diagnostics: bool,

    /// Exit with error if warning count exceeds this threshold.
    #[arg(long = "max-warnings", value_name = "N")]
    pub max_warnings: Option<usize>,

    /// Show statistics grouped by rule.
    #[arg(long)]
    pub statistics: bool,

    /// Show progress indicator (auto, on, off, detailed).
    #[arg(long, value_enum, default_value = "auto")]
    pub progress: Progress,
}

/// Check source files for type errors and lint issues.
pub fn run(args: &CheckArgs) -> i32 {
    run_with_command(args, "check")
}

/// Check source files with a custom command label.
pub fn run_with_command(args: &CheckArgs, command_name: &str) -> i32 {
    // validate flag combinations
    if !args.report.is_json() {
        if args.unsafe_fixes && !args.fix && !args.diff {
            console::warn("--unsafe-fixes has no effect without --fix or --diff");
        }

        if args.no_lint && (args.fix || args.diff) {
            console::warn("--fix and --diff have no effect with --no-lint");
        }
    }

    // reject json output with fix modes
    if args.report.is_json() && (args.fix || args.diff) {
        return report_error(
            command_name,
            &args.report,
            "--output-format json is not supported with --fix or --diff",
        );
    }

    // determine progress mode
    let progress_mode = match args.progress {
        Progress::Auto => {
            // show progress on tty unless quiet mode, json output, or stdin input
            if is_tty()
                && !args.quiet
                && !args.input.stdin
                && !matches!(args.format, Format::Json | Format::Github)
                && !args.report.is_json()
            {
                ProgressMode::Spinner
            } else {
                ProgressMode::None
            }
        }
        Progress::On => ProgressMode::Spinner,
        Progress::Off => ProgressMode::None,
        Progress::Detailed => ProgressMode::Detailed,
    };

    // create progress reporter
    let progress_reporter = ProgressReporter::with_label(progress_mode, "Checking");
    let finish_progress = || {
        if let Some(reporter) = &progress_reporter {
            reporter.finish();
        }
    };
    let event_handler = progress_reporter.as_ref().map(|p| p.handler());

    // pick the compiler mode based on fix and lint flags
    let mode = if args.no_lint || args.fix || args.diff {
        CompilerMode::Check
    } else {
        CompilerMode::Lint
    };

    let setup = match prepare_compile(CompileRequest {
        command: command_name,
        input: &args.input,
        program: &args.program,
        diagnostics: &args.diagnostics,
        report: &args.report,
        mode,
        target_name: None,
        allow_dsconfig_fallback: true,
        event_handler,
    }) {
        Ok(setup) => setup,
        Err(code) => {
            finish_progress();
            return code;
        }
    };

    if let Some(progress_reporter) = &progress_reporter {
        progress_reporter.set_stats_source(
            setup.context.compiler.stats.clone(),
            Some(setup.context.program.clone()),
        );
    }

    // compile type checks and optional linting
    setup.context.run_compile();

    // build format options
    let format_options = FormatOptions {
        format: args.format.into(),
        quiet: args.quiet,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
        suppress_diagnostics: args.no_diagnostics,
    };
    let line_writer = progress_reporter
        .as_ref()
        .map(|reporter| reporter.line_writer());

    // handle fix mode separately
    if args.fix || args.diff {
        // configure diagnostics and fixes
        let diagnostic_options: DiagnosticOptions = args.diagnostics.clone().into();
        let fix_options = FixOptions {
            apply: args.fix,
            include_unsafe: args.unsafe_fixes,
            diff: args.diff,
        };
        let fix_result = run_with_fixes(
            setup.context.program.clone(),
            &setup.modules,
            &diagnostic_options,
            &fix_options,
            &format_options,
            line_writer.as_ref(),
        );

        // collect type errors from the compiler
        let compile_result = setup.context.into_result();
        let diagnostics = compile_result
            .program
            .diagnostics
            .collect()
            .map(&compile_result.diagnostic_options);

        // emit diagnostics for the compile pass
        let module_count = setup.modules.len();
        let output_result = format_diagnostics_with_writer(
            &compile_result.program.files,
            &diagnostics,
            &format_options,
            module_count,
            line_writer.as_ref(),
        );

        // return failure when issues remain
        if fix_result.unfixable_count > 0 || output_result.error_count > 0 {
            finish_progress();
            return 1;
        }
        // return failure when warning threshold is exceeded
        if output_result.max_warnings_exceeded {
            console::warn(&format!(
                "warning count ({}) exceeds --max-warnings ({})",
                output_result.warning_count,
                args.max_warnings.unwrap_or(0)
            ));
            finish_progress();
            return 1;
        }
        finish_progress();
        return output_result.exit_code();
    }

    // gather stats and diagnostics
    let stats = setup.context.stats();
    let compile_result = setup.context.into_result();
    let diagnostics = compile_result
        .program
        .diagnostics
        .collect()
        .map(&compile_result.diagnostic_options);

    // emit json report when requested
    if args.report.is_json() {
        let json_options = FormatOptions {
            format: DiagnosticFormat::Json,
            quiet: args.quiet,
            max_warnings: args.max_warnings,
            statistics: args.statistics,
            suppress_diagnostics: false,
        };
        let (output, format_result) =
            collect_diagnostics_json(&compile_result.program.files, &diagnostics, &json_options);
        let mut report = if format_result.exit_code() == 0 {
            CommandReport::success(command_name, 0)
        } else {
            CommandReport::failure(command_name, format_result.exit_code())
        };
        report.diagnostics = Some(output);
        report.stats = Some(CommandStats::from_snapshot(&stats));
        print_report(&report, args.report.format());
        finish_progress();
        return format_result.exit_code();
    }

    // emit formatted diagnostics
    let module_count = setup.modules.len();
    let result = format_diagnostics_with_writer(
        &compile_result.program.files,
        &diagnostics,
        &format_options,
        module_count,
        line_writer.as_ref(),
    );

    // print stats summary in text mode
    if matches!(args.format, Format::Text) {
        let profile_count = compile_result.program.profiles.len();
        let summary = StatsSummary {
            verb: "Checked",
            modules: module_count,
            profiles: profile_count,
            targets: 0, // check doesn't build targets
            errors: result.error_count,
            warnings: result.warning_count,
        };
        print_stats_summary(&summary, &compile_result.stats, line_writer.as_ref());
    }

    // enforce max warning threshold when configured
    if result.max_warnings_exceeded {
        console::warn(&format!(
            "warning count ({}) exceeds --max-warnings ({})",
            result.warning_count,
            args.max_warnings.unwrap_or(0)
        ));
        finish_progress();
        return 1;
    }

    finish_progress();
    result.exit_code()
}
