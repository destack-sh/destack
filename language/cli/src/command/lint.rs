use clap::Args;
use tspp_linter as linter;

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
pub async fn run(args: &LintArgs) -> i32 {
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
        format: args.format,
        quiet: args.quiet,
        no_diagnostics: args.no_diagnostics,
        max_warnings: args.max_warnings,
        statistics: args.statistics,
        progress: args.progress,
    };

    check::run_with_command(&check_args, "lint").await
}

/// One lint rule exposed through the CLI.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LintRuleEntry<'a> {
    /// The canonical lint id.
    id: &'a str,
    /// Rule category name.
    category: &'static str,
    /// Standard level before configuration overrides.
    level: &'static str,
    /// Concise rule summary.
    summary: &'a str,
    /// Rule rationale and reporting criteria.
    explanation: &'a str,
    /// The correction kind.
    fixability: &'static str,
    /// Compiler representation inspected by the rule.
    tier: &'static str,
    /// Compilation scope inspected by the rule.
    scope: &'static str,
    /// The canonical reported and accepted source pair.
    example: LintExampleEntry<'a>,
    /// Earlier rules that informed this rule.
    provenance: Vec<LintProvenanceEntry<'a>>,
    /// The lint declaration location.
    source: LintSourceEntry,
}

impl<'a> From<&'a linter::Lint> for LintRuleEntry<'a> {
    /// Project one registered lint into its CLI representation.
    fn from(lint: &'a linter::Lint) -> Self {
        let provenance = lint
            .provenance
            .iter()
            .map(LintProvenanceEntry::from)
            .collect();

        Self {
            id: lint.id.as_ref(),
            category: lint.category.name(),
            level: lint.default_level.name(),
            summary: lint.summary.as_ref(),
            explanation: lint.explanation.as_ref(),
            fixability: lint.fixability.name(),
            tier: lint.tier().name(),
            scope: lint.scope().name(),
            example: LintExampleEntry::from(&lint.example),
            provenance,
            source: LintSourceEntry {
                path: lint.source_path,
                line: lint.source_line,
            },
        }
    }
}

/// One canonical lint source pair.
#[derive(serde::Serialize)]
struct LintExampleEntry<'a> {
    /// The source that produces the lint.
    reported: LintExampleSourceEntry<'a>,
    /// The source that expresses the same intent without the lint.
    accepted: LintExampleSourceEntry<'a>,
}

impl<'a> From<&'a linter::LintExample> for LintExampleEntry<'a> {
    /// Project one canonical lint example.
    fn from(example: &'a linter::LintExample) -> Self {
        Self {
            reported: LintExampleSourceEntry::from(&example.reported),
            accepted: LintExampleSourceEntry::from(&example.accepted),
        }
    }
}

/// One source file in a canonical lint example.
#[derive(serde::Serialize)]
struct LintExampleSourceEntry<'a> {
    /// The source path.
    path: &'a str,
    /// The source text.
    source: &'a str,
}

impl<'a> From<&'a linter::LintExampleSource> for LintExampleSourceEntry<'a> {
    /// Project one canonical lint source.
    fn from(source: &'a linter::LintExampleSource) -> Self {
        Self {
            path: source.path(),
            source: source.source(),
        }
    }
}

/// One earlier rule that informed a lint.
#[derive(serde::Serialize)]
struct LintProvenanceEntry<'a> {
    /// The project defining the earlier rule.
    source: &'static str,
    /// The earlier rule name.
    rule: &'a str,
    /// The earlier rule documentation.
    url: String,
}

impl<'a> From<&'a linter::LintProvenance> for LintProvenanceEntry<'a> {
    /// Project one lint provenance entry.
    fn from(provenance: &'a linter::LintProvenance) -> Self {
        Self {
            source: provenance.source.name(),
            rule: provenance.rule,
            url: provenance.source.rule_url(provenance.rule),
        }
    }
}

/// One lint declaration location.
#[derive(serde::Serialize)]
struct LintSourceEntry {
    /// The repository-relative source path.
    path: &'static str,
    /// The one-based source line.
    line: u32,
}

/// List lint rules in text or JSON output.
fn list_rules(args: &LintArgs) -> i32 {
    let mut entries = linter::Lint::all()
        .map(LintRuleEntry::from)
        .collect::<Vec<_>>();

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
            let details = format!(
                "{} · {} · {} {} · {}",
                entry.category, entry.level, entry.tier, entry.scope, entry.fixability
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
