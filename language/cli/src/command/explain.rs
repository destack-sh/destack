use std::cmp::Reverse;
use std::collections::BTreeMap;

use clap::{Args, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use tspp_linter as linter;
use tspp_session::diagnostic;
use tspp_source::{DiagnosticDefinition, DiagnosticSeverity};

use crate::common::{
    CommandReport, ListEntry, ListGroup, ListPrinter, ListSpacing, ReportArgs,
    grouped_list_payload, print_grouped_list_with, print_report, report_error,
};
use crate::console;

/// Arguments for the explain command.
#[derive(Args, Debug, Clone)]
pub struct ExplainArgs {
    /// Diagnostic or lint id to explain.
    #[arg(value_name = "ID", required_unless_present = "list")]
    pub id: Option<String>,

    /// List diagnostics instead of explaining one id.
    #[arg(long)]
    pub list: bool,

    /// Diagnostic types to list.
    #[arg(long, value_enum, default_value = "all")]
    pub kind: DiagnosticKindFilter,

    /// Severity filter for diagnostic listings.
    #[arg(long, value_enum, default_value = "all")]
    pub severity: DiagnosticSeverityFilter,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Severity filter for diagnostic listings.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverityFilter {
    /// List only errors.
    Error,
    /// List only warnings.
    Warning,
    /// List errors and warnings.
    All,
}

impl DiagnosticSeverityFilter {
    /// Return whether this filter includes one severity.
    fn includes(self, severity: DiagnosticSeverity) -> bool {
        match self {
            Self::Error => severity == DiagnosticSeverity::Error,
            Self::Warning => severity == DiagnosticSeverity::Warning,
            Self::All => true,
        }
    }
}

/// Diagnostic kind filter for listings.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKindFilter {
    /// List diagnostics.
    Diagnostic,
    /// List lint rules.
    Lint,
    /// List diagnostics and lint rules.
    All,
}

impl DiagnosticKindFilter {
    /// Return whether this filter includes diagnostics.
    fn includes_diagnostics(self) -> bool {
        matches!(self, Self::Diagnostic | Self::All)
    }

    /// Return whether this filter includes lint rules.
    fn includes_lints(self) -> bool {
        matches!(self, Self::Lint | Self::All)
    }
}

/// Static diagnostic metadata returned by the explain command.
#[derive(Serialize)]
struct DiagnosticEntry {
    /// The canonical diagnostic id.
    id: &'static str,
    /// The diagnostic description.
    description: &'static str,
    /// The diagnostic severity.
    severity: &'static str,
    /// Whether source controls may select the diagnostic.
    controllable: bool,
}

impl From<&'static DiagnosticDefinition> for DiagnosticEntry {
    /// Build one serializable diagnostic entry.
    fn from(definition: &'static DiagnosticDefinition) -> Self {
        Self {
            id: definition.id,
            description: definition.description,
            severity: definition.severity.family_name(),
            controllable: definition.is_controllable,
        }
    }
}

/// Lint rule metadata returned by the explain command.
#[derive(Serialize)]
struct LintEntry {
    /// The canonical lint id.
    id: &'static str,
    /// The rule category.
    category: &'static str,
    /// The concise rule summary.
    summary: &'static str,
    /// Whether the rule provides a fix.
    fixable: bool,
    /// The standard level before configuration overrides.
    level: &'static str,
    /// The compiler representation inspected by the rule.
    tier: &'static str,
    /// The compilation scope inspected by the rule.
    scope: &'static str,
}

/// One categorized listing group.
#[derive(Serialize)]
struct CategoryListing<T> {
    /// The category name.
    category: String,
    /// The entries in the category.
    entries: Vec<T>,
}

/// JSON payload for diagnostic listings.
#[derive(Serialize)]
struct ExplainListPayload {
    /// Diagnostics grouped by severity.
    diagnostics: Value,
    /// Lint rules grouped by category.
    lints: Value,
}

/// JSON payload for one explained diagnostic or lint rule.
#[derive(Serialize)]
struct ExplainDiagnosticPayload {
    /// The explained diagnostic or lint rule.
    diagnostic: ExplainPayload,
}

/// One explained diagnostic or lint rule.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExplainPayload {
    /// One built-in diagnostic.
    Diagnostic {
        /// The diagnostic metadata.
        #[serde(flatten)]
        entry: DiagnosticEntry,
    },
    /// One lint rule.
    Lint {
        /// The lint rule metadata.
        #[serde(flatten)]
        entry: LintEntry,
    },
}

/// Explain one diagnostic or lint rule, or list them.
pub fn run(args: &ExplainArgs) -> i32 {
    if args.list {
        return list_diagnostics(args);
    }

    // require one exact canonical id
    let Some(id) = args.id.as_deref() else {
        return report_error("explain", &args.report, "diagnostic id is required");
    };
    if id.is_empty() {
        return report_error("explain", &args.report, "diagnostic id is required");
    }

    // resolve one exact built-in id
    if let Some(definition) = diagnostic::definitions().find(|definition| definition.id == id) {
        return output_diagnostic(args, definition.into());
    }
    if let Some(rule) = linter::Lint::all().find(|rule| rule.id.as_ref() == id) {
        return output_lint(args, lint_entry(rule));
    }

    report_error(
        "explain",
        &args.report,
        &format!("unknown diagnostic id: {id}"),
    )
}

/// List all selected diagnostics and lint rules.
fn list_diagnostics(args: &ExplainArgs) -> i32 {
    let diagnostics = if args.kind.includes_diagnostics() {
        collect_diagnostics(args, diagnostic::definitions())
    } else {
        Vec::new()
    };
    let lints = if args.kind.includes_lints() {
        collect_lints()
    } else {
        Vec::new()
    };

    if args.report.is_json() {
        return print_json_listing(args, diagnostics, lints);
    }

    if args.kind.includes_diagnostics() {
        print_diagnostics(diagnostics);
    }
    if args.kind.includes_lints() {
        print_lints(lints);
    }

    0
}

/// Collect selected diagnostic definitions.
fn collect_diagnostics(
    args: &ExplainArgs,
    definitions: impl Iterator<Item = &'static DiagnosticDefinition>,
) -> Vec<DiagnosticEntry> {
    let mut definitions = definitions
        .filter(|definition| args.severity.includes(definition.severity))
        .collect::<Vec<_>>();
    definitions.sort_unstable_by_key(|definition| (Reverse(definition.severity), definition.id));
    definitions.into_iter().map(DiagnosticEntry::from).collect()
}

/// Collect every lint rule.
fn collect_lints() -> Vec<LintEntry> {
    let mut entries = linter::Lint::all().map(lint_entry).collect::<Vec<_>>();
    entries.sort_unstable_by_key(|entry| (entry.category, entry.id));

    entries
}

/// Build one lint rule entry.
fn lint_entry(rule: &'static linter::Lint) -> LintEntry {
    LintEntry {
        id: rule.id.as_ref(),
        category: rule.category.name(),
        summary: rule.summary.as_ref(),
        fixable: rule.is_fixable(),
        level: rule.default_level.name(),
        tier: rule.tier().name(),
        scope: rule.scope().name(),
    }
}

/// Print one JSON diagnostic listing.
fn print_json_listing(
    args: &ExplainArgs,
    diagnostics: Vec<DiagnosticEntry>,
    lints: Vec<LintEntry>,
) -> i32 {
    let diagnostic_total = diagnostics.len();
    let lint_total = lints.len();
    let payload = ExplainListPayload {
        diagnostics: grouped_list_payload(category_diagnostics(diagnostics), diagnostic_total),
        lints: grouped_list_payload(category_lints(lints), lint_total),
    };
    let data = match serde_json::to_value(payload) {
        Ok(data) => data,
        Err(error) => {
            return report_error(
                "explain",
                &args.report,
                &format!("failed to serialize payload: {error}"),
            );
        }
    };

    let mut report = CommandReport::success("explain", 0);
    report.data = Some(data);
    print_report(&report, args.report.format());

    0
}

/// Group diagnostics by severity.
fn category_diagnostics(entries: Vec<DiagnosticEntry>) -> Vec<CategoryListing<DiagnosticEntry>> {
    let mut categories = Vec::<CategoryListing<DiagnosticEntry>>::new();
    for entry in entries {
        let category = entry.severity.to_string();
        match categories.last_mut() {
            Some(listing) if listing.category == category => listing.entries.push(entry),
            _ => categories.push(CategoryListing {
                category,
                entries: vec![entry],
            }),
        }
    }

    categories
}

/// Group lint rules by category.
fn category_lints(entries: Vec<LintEntry>) -> Vec<CategoryListing<LintEntry>> {
    let mut categories = BTreeMap::<String, Vec<LintEntry>>::new();
    for entry in entries {
        categories
            .entry(entry.category.to_string())
            .or_default()
            .push(entry);
    }

    categories
        .into_iter()
        .map(|(category, entries)| CategoryListing { category, entries })
        .collect()
}

/// Print diagnostics grouped by severity.
fn print_diagnostics(entries: Vec<DiagnosticEntry>) {
    let is_color_enabled = console::color_enabled(console::Stream::Stdout);
    let groups = category_diagnostics(entries)
        .into_iter()
        .map(|group| {
            let heading = format_category(&group.category, is_color_enabled);
            let entries = group
                .entries
                .into_iter()
                .map(|entry| {
                    let id = format_id(entry.id, entry.severity, is_color_enabled);
                    ListEntry::new(format!("  {id} - {}", entry.description))
                })
                .collect();

            ListGroup::new(heading, entries)
        })
        .collect::<Vec<_>>();
    let printer = ListPrinter::plain();
    print_grouped_list_with(&groups, ListSpacing::Spaced, &printer);
}

/// Print lint rules grouped by category.
fn print_lints(entries: Vec<LintEntry>) {
    let is_color_enabled = console::color_enabled(console::Stream::Stdout);
    let groups = category_lints(entries)
        .into_iter()
        .map(|group| {
            let heading = format_category(&group.category, is_color_enabled);
            let entries = group
                .entries
                .into_iter()
                .map(|entry| {
                    let id = format_id(entry.id, "warning", is_color_enabled);
                    let fixability = if entry.fixable { "fixable" } else { "no-fix" };
                    let details = format!(
                        "{} · {} {} · {fixability}",
                        entry.level, entry.tier, entry.scope
                    );
                    let details = format_details(&details, is_color_enabled);

                    ListEntry::new(format!("  {id} - {}", entry.summary))
                        .line(format!("  {details}"))
                })
                .collect();

            ListGroup::new(heading, entries)
        })
        .collect::<Vec<_>>();
    let printer = ListPrinter::plain();
    print_grouped_list_with(&groups, ListSpacing::Spaced, &printer);
}

/// Output one diagnostic definition.
fn output_diagnostic(args: &ExplainArgs, entry: DiagnosticEntry) -> i32 {
    if args.report.is_json() {
        return output_json(args, ExplainPayload::Diagnostic { entry });
    }

    console::info(entry.id);
    console::info(&format!("severity: {}", entry.severity));
    console::info(&format!("controllable: {}", entry.controllable));
    console::info(&format!("description: {}", entry.description));

    0
}

/// Output one lint rule.
fn output_lint(args: &ExplainArgs, entry: LintEntry) -> i32 {
    if args.report.is_json() {
        return output_json(args, ExplainPayload::Lint { entry });
    }

    console::info(entry.id);
    console::info(&format!("category: {}", entry.category));
    console::info(&format!("level: {}", entry.level));
    console::info(&format!("tier: {}", entry.tier));
    console::info(&format!("scope: {}", entry.scope));
    console::info(&format!("fixable: {}", entry.fixable));
    console::info(&format!("summary: {}", entry.summary));

    0
}

/// Output one explained diagnostic or lint rule as JSON.
fn output_json(args: &ExplainArgs, diagnostic: ExplainPayload) -> i32 {
    let payload = ExplainDiagnosticPayload { diagnostic };
    let data = match serde_json::to_value(payload) {
        Ok(data) => data,
        Err(error) => {
            return report_error(
                "explain",
                &args.report,
                &format!("failed to serialize payload: {error}"),
            );
        }
    };

    let mut report = CommandReport::success("explain", 0);
    report.data = Some(data);
    print_report(&report, args.report.format());

    0
}

/// Format one diagnostic category heading.
fn format_category(category: &str, is_color_enabled: bool) -> String {
    if !is_color_enabled {
        return category.to_string();
    }

    let color = match category {
        "error" => "91",
        "warning" => "93",
        _ => "32",
    };

    console::style(category, &["1", color])
}

/// Format one canonical diagnostic id.
fn format_id(id: &str, severity: &str, is_color_enabled: bool) -> String {
    if !is_color_enabled {
        return id.to_string();
    }

    let color = match severity {
        "error" => "1;91",
        "warning" => "1;93",
        _ => "1;36",
    };

    console::color(id, color)
}

/// Format one lint detail line.
fn format_details(details: &str, is_color_enabled: bool) -> String {
    if is_color_enabled {
        return console::dim(details);
    }

    details.to_string()
}
