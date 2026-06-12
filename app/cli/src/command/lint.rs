use clap::Args;

use super::check::{self, Format, Progress};
use crate::common::{
    CommandReport, DiagnosticArgs, InputArgs, ListEntry, ListPrinter, ListSpacing, ProgramArgs,
    ReportArgs, list_payload, print_list_with, print_report, report_error,
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

    // convert to CheckArgs and delegate
    let check_args = check::CheckArgs {
        input: args.input.clone(),
        program: args.program.clone(),
        diagnostics: args.diagnostics.clone(),
        report: args.report.clone(),
        fix: args.fix,
        unsafe_fixes: args.unsafe_fixes,
        diff: args.diff,
        no_lint: false, // lint always includes linting
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
struct RuleEntry {
    /// Fully qualified rule identifier.
    id: String,
    /// Diagnostic code for the rule.
    code: &'static str,
    /// Display name for the rule.
    name: &'static str,
    /// Rule category name.
    category: &'static str,
    /// Human-readable description.
    description: &'static str,
    /// Whether the rule provides a fix.
    fixable: bool,
    /// Whether the rule is enabled by default.
    recommended: bool,
    /// Stability label for the rule.
    stability: &'static str,
}

/// List lint rules in text or JSON output.
fn list_rules(args: &LintArgs) -> i32 {
    let mut entries: Vec<RuleEntry> = destack_linter::all_rules()
        .into_iter()
        .map(|rule| {
            let meta = rule.meta();
            RuleEntry {
                id: meta.full_id(),
                code: meta.code,
                name: meta.name,
                category: meta.category.name(),
                description: meta.description,
                fixable: meta.is_fixable(),
                recommended: meta.is_recommended(),
                stability: if meta.is_stable() {
                    "stable"
                } else {
                    "experimental"
                },
            }
        })
        .collect();

    entries.sort_by(|a, b| a.id.cmp(&b.id));

    if args.report.is_json() {
        let mut report = CommandReport::success("lint", 0);
        report.data = Some(list_payload(entries));
        print_report(&report, args.report.format());
        return 0;
    }

    let list_entries = entries
        .into_iter()
        .map(|entry| {
            let summary = format!("{} ({})", entry.id, entry.code);
            let details = format!(
                "{} · {} · {}",
                entry.category,
                if entry.fixable { "fixable" } else { "no-fix" },
                if entry.recommended {
                    "recommended"
                } else {
                    "optional"
                },
            );
            ListEntry::new(summary)
                .line(details)
                .line(entry.description.to_string())
        })
        .collect::<Vec<_>>();
    let printer = ListPrinter::plain();
    print_list_with(&list_entries, ListSpacing::Spaced, &printer);

    0
}
