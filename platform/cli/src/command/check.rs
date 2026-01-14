use clap::{Args, ValueEnum};
use destack_compiler::StatsSnapshot;
use destack_source::DiagnosticOptions;

use crate::common::fix::{FixOptions, run_with_fixes};
use crate::common::format::{DiagnosticFormat, FormatOptions, format_diagnostics_with_writer};
use crate::common::{
    CommandReport, CommandStats, CompilerMode, DiagnosticArgs, InputArgs, LineWriter, ProgramArgs,
    ProgressMode, ProgressReporter, ReportArgs, collect_diagnostics_json, is_tty, print_report,
    report_error,
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

/// Write a line using the optional writer.
fn write_line(line_writer: Option<&LineWriter>, line: &str) {
    // use the line writer when provided
    if let Some(writer) = line_writer {
        writer(line);
    } else {
        eprintln!("{line}");
    }
}

/// Summary info for stats output.
#[derive(Debug)]
pub struct StatsSummary<'a> {
    /// Action verb to display (e.g., "Checked", "Built", "Linted").
    pub verb: &'a str,
    /// Number of modules processed.
    pub modules: usize,
    /// Number of profiles used.
    pub profiles: usize,
    /// Number of targets built.
    pub targets: usize,
    /// Number of errors found.
    pub errors: usize,
    /// Number of warnings found.
    pub warnings: usize,
}

/// Print a stats summary line after diagnostics.
pub fn print_stats_summary(
    summary: &StatsSummary<'_>,
    stats: &StatsSnapshot,
    line_writer: Option<&LineWriter>,
) {
    // compute elapsed seconds for throughput calculations
    let elapsed_secs = stats.elapsed.as_secs_f64();

    // counts: "N modules[, M profiles][, K targets]"
    let mut parts = Vec::new();
    parts.push(pluralize(summary.modules, "module"));
    if summary.profiles > 1 {
        parts.push(pluralize(summary.profiles, "profile"));
    }
    if summary.targets > 0 {
        parts.push(pluralize(summary.targets, "target"));
    }
    let counts = parts.join(", ");

    // build main message and colorize based on status
    let elapsed_str = console::format_duration(stats.elapsed);
    let (icon, main_part) = if summary.errors > 0 {
        let status = format!(" with {}", pluralize(summary.errors, "error"));
        let main = format!("{} {counts}{status} in {elapsed_str}", summary.verb);
        // bold + bright red
        (
            console::failure_icon(),
            console::style_for_stream(&main, &["1", "91"], console::Stream::Stderr),
        )
    } else if summary.warnings > 0 {
        let status = format!(" with {}", pluralize(summary.warnings, "warning"));
        let main = format!("{} {counts}{status} in {elapsed_str}", summary.verb);
        // bold + bright yellow
        (
            console::success_icon(),
            console::style_for_stream(&main, &["1", "93"], console::Stream::Stderr),
        )
    } else {
        let main = format!("{} {counts} in {elapsed_str}", summary.verb);
        // bold + green
        (
            console::success_icon(),
            console::style_for_stream(&main, &["1", "32"], console::Stream::Stderr),
        )
    };

    // line count with throughput (bold only, after the ·)
    let lines_suffix = if stats.lines_processed > 0 && elapsed_secs > 0.001 {
        let throughput = stats.lines_processed as f64 / elapsed_secs;
        console::bold(&format!(
            " · {} lines · {} lines/s",
            format_number(stats.lines_processed),
            format_compact(throughput as usize)
        ))
    } else if stats.lines_processed > 0 {
        console::bold(&format!(
            " · {} lines",
            format_number(stats.lines_processed)
        ))
    } else {
        String::new()
    };

    write_line(line_writer, &format!("{icon} {main_part}{lines_suffix}"));

    // show MIR optimization metrics if any optimizations were performed
    if stats.mir_functions_optimized > 0 && stats.mir_instructions_before > 0 {
        let instr_before = stats.mir_instructions_before;
        let instr_after = stats.mir_instructions_after;
        let blocks_before = stats.mir_blocks_before;
        let blocks_after = stats.mir_blocks_after;

        // calculate percentages (negative = reduction)
        let instr_delta = if instr_before > 0 {
            ((instr_after as f64 - instr_before as f64) / instr_before as f64 * 100.0) as i32
        } else {
            0
        };
        let blocks_delta = if blocks_before > 0 {
            ((blocks_after as f64 - blocks_before as f64) / blocks_before as f64 * 100.0) as i32
        } else {
            0
        };

        // format delta with sign and color
        let format_delta = |delta: i32| -> String {
            if delta < 0 {
                console::green(&format!("{delta}%"))
            } else if delta > 0 {
                console::yellow(&format!("+{delta}%"))
            } else {
                console::dim("0%")
            }
        };

        let arrow = console::SYMBOL_ARROW;
        let mir_line = format!(
            "    {} optimized {} functions: {} {arrow} {} instructions ({}), {} {arrow} {} blocks ({})",
            console::dim(console::SYMBOL_ARROW),
            stats.mir_functions_optimized,
            format_number(instr_before),
            format_number(instr_after),
            format_delta(instr_delta),
            format_number(blocks_before),
            format_number(blocks_after),
            format_delta(blocks_delta),
        );
        write_line(line_writer, &mir_line);
    }

    // show per-package breakdown if multiple packages (excluding internal ones)
    // aggregate by package name to avoid duplicates
    let mut package_map: std::collections::HashMap<String, (usize, usize, std::time::Duration)> =
        std::collections::HashMap::new();
    for pkg in &stats.packages {
        let name = pkg.name.as_deref().unwrap_or("");
        // skip internal packages
        if name.starts_with('<') || pkg.lines == 0 {
            continue;
        }
        let entry =
            package_map
                .entry(name.to_string())
                .or_insert((0, 0, std::time::Duration::ZERO));
        entry.0 += pkg.modules;
        entry.1 += pkg.lines;
        entry.2 += pkg.duration;
    }

    if !package_map.is_empty() {
        // sort: builtin packages last, then by lines descending
        let mut packages: Vec<_> = package_map.into_iter().collect();
        packages.sort_by(|(a_name, _), (b_name, _)| {
            let a_is_builtin = a_name.contains("builtin");
            let b_is_builtin = b_name.contains("builtin");

            match (a_is_builtin, b_is_builtin) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => a_name.cmp(b_name),
            }
        });

        for (name, (modules, lines, duration)) in packages {
            let duration_str = if duration.as_nanos() > 0 {
                console::cyan(&console::format_duration(duration))
            } else {
                String::new()
            };
            let modules_str = pluralize(modules, "module");
            let lines_str = format!("{} lines", format_number(lines));
            let throughput_str = if duration.as_nanos() > 0 {
                let duration_secs = duration.as_secs_f64();
                if lines > 0 && duration_secs > 0.001 {
                    let throughput = lines as f64 / duration_secs;
                    format!(" · {} lines/s", format_compact(throughput as usize))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            write_line(
                line_writer,
                &format!(
                    "    {}  {} · {} · {}{}",
                    console::cyan(&name),
                    duration_str,
                    modules_str,
                    lines_str,
                    throughput_str
                ),
            );
        }
    }

    // show per-phase timing (labels dimmed, times in cyan)
    // skip phases with 0 duration
    let visible_phases: Vec<_> = stats
        .phases
        .iter()
        .filter(|p| p.duration.as_nanos() > 0)
        .collect();

    if !visible_phases.is_empty() {
        let phase_parts: Vec<String> = visible_phases
            .iter()
            .map(|p| {
                let name = console::dim(p.phase.name());
                let duration = console::cyan(&console::format_duration(p.duration));
                format!("{name} {duration}")
            })
            .collect();

        let sep = console::dim(" · ");
        write_line(line_writer, &format!("    {}", phase_parts.join(&sep)));
    }
}

/// Format a number with thousands separators.
fn format_number(n: usize) -> String {
    // build the formatted digits in reverse order
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    // reverse the string into the final output
    result.chars().rev().collect()
}

/// Format a number in compact form (e.g., 1.2k, 3.5M).
fn format_compact(n: usize) -> String {
    // format millions in compact form
    if n >= 1_000_000 {
        let m = n as f64 / 1_000_000.0;
        if m >= 10.0 {
            format!("{m:.0}M")
        } else {
            format!("{m:.1}M")
        }
    } else if n >= 1_000 {
        // format thousands in compact form
        let k = n as f64 / 1_000.0;
        if k >= 10.0 {
            format!("{k:.0}k")
        } else {
            format!("{k:.1}k")
        }
    } else {
        // return the raw number for small values
        n.to_string()
    }
}

/// Format a count with a singular or plural word.
fn pluralize(n: usize, word: &str) -> String {
    // choose the singular form for one
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}
