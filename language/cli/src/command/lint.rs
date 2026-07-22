use clap::Args;
use destack_linter as linter;

use super::check::{self, Format, Progress};
use crate::common::{
    CommandReport, InputArgs, ListEntry, ListPrinter, ListSpacing, ProgramArgs, ReportArgs,
    list_payload, print_list_with, print_report, report_error,
};

/// Arguments for the lint command.
#[derive(Args, Debug, Clone)]
pub struct LintArgs {
    /// Input arguments.
    #[command(flatten)]
    pub input: InputArgs,

    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

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

    /// Show statistics grouped by diagnostic id.
    #[arg(long)]
    pub statistics: bool,

    /// Show progress indicator (auto, on, off, detailed).
    #[arg(long, value_enum, default_value = "auto")]
    pub progress: Progress,

    /// List available lint rules and exit.
    #[arg(long = "list-rules")]
    pub list_rules: bool,
}

/// Lint source files for style and correctness issues.
/// This is an alias for `check` with linting enabled.
pub fn run(args: &LintArgs) -> i32 {
    if args.list_rules {
        if args.program.watch {
            return report_error(
                "lint",
                &args.report,
                "--watch is not supported with --list-rules",
            );
        }
        return list_rules(args);
    }

    // run the check command with linting enabled
    let check_args = check::CheckArgs {
        input: args.input.clone(),
        program: args.program.clone(),
        report: args.report.clone(),
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        diff: args.diff,
        no_lint: false,
        timings: false,
        format: args.format,
        quiet: args.quiet,
        no_diagnostics: args.no_diagnostics,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
        progress: args.progress,
    };

    check::run_with_command(&check_args, "lint")
}

/// Lint rule metadata for list output.
#[derive(serde::Serialize)]
struct LintListEntry {
    /// The canonical lint id.
    id: &'static str,
    /// Rule category name.
    category: &'static str,
    /// Standard level before configuration overrides.
    level: &'static str,
    /// Concise rule summary.
    summary: &'static str,
    /// Whether the rule provides a fix.
    fixable: bool,
    /// Compiler representation inspected by the rule.
    tier: &'static str,
    /// Compilation scope inspected by the rule.
    scope: &'static str,
}

/// List lint rules in text or JSON output.
fn list_rules(args: &LintArgs) -> i32 {
    let mut entries: Vec<LintListEntry> = linter::LINTS
        .iter()
        .map(|rule| LintListEntry {
            id: rule.id.as_ref(),
            category: rule.category.name(),
            summary: rule.summary.as_ref(),
            fixable: rule.is_fixable(),
            level: rule.default_level.name(),
            tier: rule.tier().name(),
            scope: rule.scope().name(),
        })
        .collect();

    entries.sort_by(|a, b| a.id.cmp(b.id));

    if args.report.is_json() {
        let mut report = CommandReport::success("lint", 0);
        report.data = Some(list_payload(entries));
        print_report(&report, args.report.format());
        return 0;
    }

    let list_entries = entries
        .into_iter()
        .map(|entry| {
            let fixability = if entry.fixable { "fixable" } else { "no-fix" };
            let details = format!(
                "{} · {} · {} {} · {fixability}",
                entry.category, entry.level, entry.tier, entry.scope
            );
            ListEntry::new(entry.id)
                .line(details)
                .line(entry.summary.to_string())
        })
        .collect::<Vec<_>>();
    let printer = ListPrinter::plain();
    print_list_with(&list_entries, ListSpacing::Spaced, &printer);

    0
}
