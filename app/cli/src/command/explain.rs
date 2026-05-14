use clap::{Args, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

use destack_compiler::{
    CheckError, CheckWarning, DeclareError, DeclareWarning, DiagnosticDefinition, ElaborateError,
    ElaborateWarning, ExpandError, ExpandWarning, ExportError, ExportWarning, GenerateError,
    GenerateWarning, ImportError, ImportWarning, LinkError, LinkWarning, LowerError, LowerWarning,
    MaterializeError, MaterializeWarning, OptimizeError, OptimizeWarning, VerifyError,
    VerifyWarning,
};

use crate::common::{
    CommandReport, ListEntry, ListGroup, ListPrinter, ListSpacing, ReportArgs,
    grouped_list_payload, print_grouped_list_with, print_report, report_error,
};
use crate::console;

/// Arguments for the explain command.
#[derive(Args, Debug, Clone)]
pub struct ExplainArgs {
    /// Diagnostic code or rule id to explain.
    #[arg(value_name = "CODE", required_unless_present = "list")]
    pub code: Option<String>,

    /// List compiler diagnostics instead of explaining a single code.
    #[arg(long)]
    pub list: bool,

    /// Diagnostic types to list.
    #[arg(long, value_enum, default_value = "all")]
    pub kind: DiagnosticKindFilter,

    /// Severity filter for compiler diagnostic listings.
    #[arg(long, value_enum, default_value = "all")]
    pub severity: DiagnosticSeverityFilter,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// Severity filter for compiler diagnostic listings.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverityFilter {
    /// List only errors.
    Error,
    /// List only warnings.
    Warning,
    /// List errors and warnings.
    All,
}

/// Diagnostic kind filter for listings.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKindFilter {
    /// List compiler diagnostics.
    Compiler,
    /// List lint rules.
    Lint,
    /// List compiler diagnostics and lint rules.
    All,
}

impl DiagnosticKindFilter {
    /// Check whether the filter includes compiler diagnostics.
    fn includes_compiler(self) -> bool {
        matches!(self, Self::Compiler | Self::All)
    }

    /// Check whether the filter includes lint rules.
    fn includes_lint(self) -> bool {
        matches!(self, Self::Lint | Self::All)
    }
}

impl DiagnosticSeverityFilter {
    /// Check whether the filter includes the given severity.
    fn includes(self, severity: CompilerSeverity) -> bool {
        // match the severity against the filter
        match self {
            Self::Error => matches!(severity, CompilerSeverity::Error),
            Self::Warning => matches!(severity, CompilerSeverity::Warning),
            Self::All => true,
        }
    }
}

/// Severity for compiler diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompilerSeverity {
    /// An error diagnostic.
    Error,
    /// A warning diagnostic.
    Warning,
}

/// Grouping metadata for compiler diagnostics.
#[derive(Debug, Clone, Copy)]
struct CompilerDiagnosticGroup {
    /// Compiler phase for the diagnostics.
    phase: CompilerPhase,
    /// Severity for the diagnostics.
    severity: CompilerSeverity,
    /// Diagnostics defined in the group.
    definitions: &'static [DiagnosticDefinition],
}

/// Compiler diagnostic phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompilerPhase {
    /// Declare module symbols.
    Declare,
    /// Import, parse, and bind source into DIR.
    Import,
    /// Expand compile-time structural declarations.
    Expand,
    /// Export module surface declarations.
    Export,
    /// Check declared and exported DIR.
    Check,
    /// Elaborate analyzed DIR.
    Elaborate,
    /// Materialize checked DIR.
    Materialize,
    /// Lower DIR into MIR.
    Lower,
    /// Verify MIR invariants.
    Verify,
    /// Optimize MIR.
    Optimize,
    /// Generate build products.
    Generate,
    /// Link build products.
    Link,
}

/// Lint rule metadata returned by the explain command.
#[derive(Serialize)]
struct LintExplainEntry {
    /// Diagnostic code for the rule.
    code: &'static str,
    /// Rule identifier without category prefix.
    id: &'static str,
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
    /// Documentation URL when available.
    docs_url: Option<&'static str>,
}

/// Compiler diagnostic metadata returned by the explain command.
#[derive(Serialize)]
struct CompilerExplainEntry {
    /// Diagnostic code for the compiler issue.
    code: &'static str,
    /// Variant name for the diagnostic.
    name: &'static str,
    /// Human-readable description.
    description: &'static str,
    /// Compiler phase that owns the diagnostic.
    phase: &'static str,
    /// Severity label for the diagnostic.
    severity: &'static str,
}

/// Compiler diagnostic listing entry.
#[derive(Serialize)]
struct CompilerListEntry {
    /// Diagnostic code for the compiler issue.
    code: &'static str,
    /// Variant name for the diagnostic.
    name: &'static str,
    /// Human-readable description.
    description: &'static str,
    /// Compiler phase that owns the diagnostic.
    phase: &'static str,
    /// Severity label for the diagnostic.
    severity: &'static str,
}

/// Lint rule listing entry.
#[derive(Serialize)]
struct LintListEntry {
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

/// Categorized diagnostics for JSON output.
#[derive(Serialize)]
struct CategoryListing<T> {
    /// Category label.
    category: String,
    /// Entries within the category.
    entries: Vec<T>,
}

/// JSON payload for explain list output.
#[derive(Serialize)]
struct ExplainListPayload {
    /// Compiler diagnostics grouped by category.
    compiler: Value,
    /// Lint diagnostics grouped by category.
    lint: Value,
}

/// JSON payload for a single diagnostic entry.
#[derive(Serialize)]
struct ExplainDiagnosticPayload {
    /// The diagnostic payload content.
    diagnostic: ExplainPayload,
}

/// Explain payload for JSON output.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExplainPayload {
    /// Lint rule details.
    Lint {
        /// Lint rule entry data.
        #[serde(flatten)]
        entry: LintExplainEntry,
    },
    /// Compiler diagnostic details.
    Compiler {
        /// Compiler diagnostic entry data.
        #[serde(flatten)]
        entry: CompilerExplainEntry,
    },
}

/// Compiler diagnostics grouped by phase and severity.
const COMPILER_DIAGNOSTIC_GROUPS: &[CompilerDiagnosticGroup] = &[
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Declare,
        severity: CompilerSeverity::Error,
        definitions: DeclareError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Import,
        severity: CompilerSeverity::Error,
        definitions: ImportError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Expand,
        severity: CompilerSeverity::Error,
        definitions: ExpandError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Export,
        severity: CompilerSeverity::Error,
        definitions: ExportError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Check,
        severity: CompilerSeverity::Error,
        definitions: CheckError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Elaborate,
        severity: CompilerSeverity::Error,
        definitions: ElaborateError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Materialize,
        severity: CompilerSeverity::Error,
        definitions: MaterializeError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Lower,
        severity: CompilerSeverity::Error,
        definitions: LowerError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Verify,
        severity: CompilerSeverity::Error,
        definitions: VerifyError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Optimize,
        severity: CompilerSeverity::Error,
        definitions: OptimizeError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Generate,
        severity: CompilerSeverity::Error,
        definitions: GenerateError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Link,
        severity: CompilerSeverity::Error,
        definitions: LinkError::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Declare,
        severity: CompilerSeverity::Warning,
        definitions: DeclareWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Import,
        severity: CompilerSeverity::Warning,
        definitions: ImportWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Expand,
        severity: CompilerSeverity::Warning,
        definitions: ExpandWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Export,
        severity: CompilerSeverity::Warning,
        definitions: ExportWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Check,
        severity: CompilerSeverity::Warning,
        definitions: CheckWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Elaborate,
        severity: CompilerSeverity::Warning,
        definitions: ElaborateWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Materialize,
        severity: CompilerSeverity::Warning,
        definitions: MaterializeWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Lower,
        severity: CompilerSeverity::Warning,
        definitions: LowerWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Verify,
        severity: CompilerSeverity::Warning,
        definitions: VerifyWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Optimize,
        severity: CompilerSeverity::Warning,
        definitions: OptimizeWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Generate,
        severity: CompilerSeverity::Warning,
        definitions: GenerateWarning::ALL,
    },
    CompilerDiagnosticGroup {
        phase: CompilerPhase::Link,
        severity: CompilerSeverity::Warning,
        definitions: LinkWarning::ALL,
    },
];

/// Explain a diagnostic or lint rule.
pub fn run(args: &ExplainArgs) -> i32 {
    // route to list mode first
    if args.list {
        return list_diagnostics(args);
    }

    // resolve the requested diagnostic code
    let Some(code) = args.code.as_ref() else {
        return report_error("explain", &args.report, "diagnostic code is required");
    };
    let needle = code.trim();
    if needle.is_empty() {
        return report_error("explain", &args.report, "diagnostic code is required");
    }

    // prefer lint rule resolution first
    if let Some(entry) = find_lint_entry(needle) {
        return output_lint_entry(args, entry);
    }

    // fall back to compiler diagnostics
    if let Some(entry) = find_compiler_entry(needle) {
        return output_compiler_entry(args, entry);
    }

    // report missing diagnostics
    report_error(
        "explain",
        &args.report,
        &format!("unknown diagnostic code or rule id: {needle}"),
    )
}

/// List compiler diagnostics and lint rules in text or JSON form.
fn list_diagnostics(args: &ExplainArgs) -> i32 {
    // gather compiler diagnostics when requested
    let compiler_entries = if args.kind.includes_compiler() {
        collect_compiler_list_entries(args.severity)
    } else {
        Vec::new()
    };
    let compiler_total = compiler_entries.len();

    // gather lint rules when requested
    let lint_entries = if args.kind.includes_lint() {
        collect_lint_list_entries()
    } else {
        Vec::new()
    };
    let lint_total = lint_entries.len();

    // emit json output for tooling
    if args.report.is_json() {
        let payload = ExplainListPayload {
            compiler: grouped_list_payload(
                category_compiler_entries(compiler_entries),
                compiler_total,
            ),
            lint: grouped_list_payload(category_lint_entries(lint_entries), lint_total),
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
        return 0;
    }

    // print compiler diagnostics grouped by category
    if args.kind.includes_compiler() {
        print_compiler_listing(compiler_entries);
    }

    // print lint diagnostics grouped by category
    if args.kind.includes_lint() {
        print_lint_listing(lint_entries);
    }

    0
}

/// Collect compiler diagnostic list entries filtered by severity.
fn collect_compiler_list_entries(filter: DiagnosticSeverityFilter) -> Vec<CompilerListEntry> {
    // gather diagnostics from the registry groups
    let mut entries = Vec::new();
    for group in COMPILER_DIAGNOSTIC_GROUPS {
        // skip groups that do not match the filter
        if !filter.includes(group.severity) {
            continue;
        }

        // capture the group labels once
        let phase = phase_label(group.phase);
        let severity = severity_label(group.severity);

        // register each definition in the group
        for definition in group.definitions {
            entries.push(CompilerListEntry {
                code: definition.code,
                name: definition.name,
                description: definition.description,
                phase,
                severity,
            });
        }
    }

    entries
}

/// Collect lint rule list entries.
fn collect_lint_list_entries() -> Vec<LintListEntry> {
    // gather lint rule metadata
    destack_linter::all_rules()
        .into_iter()
        .map(|rule| {
            let meta = rule.meta();
            LintListEntry {
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
        .collect()
}

/// Build categorized compiler diagnostics for JSON output.
fn category_compiler_entries(
    entries: Vec<CompilerListEntry>,
) -> Vec<CategoryListing<CompilerListEntry>> {
    // bucket diagnostics by category label
    let mut categories: BTreeMap<String, Vec<CompilerListEntry>> = BTreeMap::new();
    for entry in entries {
        let category = compiler_category(entry.phase, entry.severity);
        categories.entry(category).or_default().push(entry);
    }

    // sort entries within each category
    categories
        .into_iter()
        .map(|(category, mut entries)| {
            entries.sort_by(|a, b| a.code.cmp(b.code));
            CategoryListing { category, entries }
        })
        .collect()
}

/// Build categorized lint diagnostics for JSON output.
fn category_lint_entries(entries: Vec<LintListEntry>) -> Vec<CategoryListing<LintListEntry>> {
    // bucket lint rules by category label
    let mut categories: BTreeMap<String, Vec<LintListEntry>> = BTreeMap::new();
    for entry in entries {
        categories
            .entry(entry.category.to_string())
            .or_default()
            .push(entry);
    }

    // sort entries within each category
    categories
        .into_iter()
        .map(|(category, mut entries)| {
            entries.sort_by(|a, b| a.id.cmp(&b.id));
            CategoryListing { category, entries }
        })
        .collect()
}

/// Print compiler diagnostics grouped by category.
fn print_compiler_listing(entries: Vec<CompilerListEntry>) {
    // resolve color support once
    let color_enabled = console::color_enabled(console::Stream::Stdout);

    // bucket diagnostics by category label
    let mut categories: BTreeMap<String, Vec<CompilerListEntry>> = BTreeMap::new();
    for entry in entries {
        let category = compiler_category(entry.phase, entry.severity);
        categories.entry(category).or_default().push(entry);
    }

    // render category groupings
    let mut groups = Vec::new();
    for (category, mut entries) in categories {
        entries.sort_by(|a, b| a.code.cmp(b.code));
        let heading = format_category_heading(&category, color_enabled);
        let list_entries = entries
            .iter()
            .map(|entry| {
                let code = format_compiler_code(entry.code, entry.severity, color_enabled);
                let summary = format!("  {code} {} - {}", entry.name, entry.description);
                ListEntry::new(summary)
            })
            .collect();
        groups.push(ListGroup::new(heading, list_entries));
    }
    let printer = ListPrinter::plain();
    print_grouped_list_with(&groups, ListSpacing::Spaced, &printer);
}

/// Print lint rules grouped by category.
fn print_lint_listing(entries: Vec<LintListEntry>) {
    // resolve color support once
    let color_enabled = console::color_enabled(console::Stream::Stdout);

    // bucket lint rules by category label
    let mut categories: BTreeMap<String, Vec<LintListEntry>> = BTreeMap::new();
    for entry in entries {
        categories
            .entry(entry.category.to_string())
            .or_default()
            .push(entry);
    }

    // render category groupings
    let mut groups = Vec::new();
    for (category, mut entries) in categories {
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        let heading = format_category_heading(&category, color_enabled);
        let list_entries = entries
            .iter()
            .map(|entry| {
                let id = format_rule_id(&entry.id, color_enabled);
                let code = format_rule_code(entry.code, color_enabled);
                let summary = format!("  {id} ({code}) - {}", entry.description);
                let details = format!(
                    "{} · {} · {}",
                    if entry.fixable { "fixable" } else { "no-fix" },
                    if entry.recommended {
                        "recommended"
                    } else {
                        "optional"
                    },
                    entry.stability,
                );
                let details = format!("  {}", format_details_line(&details, color_enabled));
                ListEntry::new(summary).line(details)
            })
            .collect();
        groups.push(ListGroup::new(heading, list_entries));
    }
    let printer = ListPrinter::plain();
    print_grouped_list_with(&groups, ListSpacing::Spaced, &printer);
}

/// Find a lint rule entry matching the given identifier.
fn find_lint_entry(needle: &str) -> Option<LintExplainEntry> {
    // search the lint rule registry
    destack_linter::all_rules()
        .iter()
        .map(|rule| rule.meta())
        .find(|meta| {
            let full_id = meta.full_id();
            meta.code.eq_ignore_ascii_case(needle)
                || meta.id.eq_ignore_ascii_case(needle)
                || full_id.eq_ignore_ascii_case(needle)
        })
        .map(|meta| LintExplainEntry {
            code: meta.code,
            id: meta.id,
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
            docs_url: meta.docs_url,
        })
}

/// Find a compiler diagnostic entry matching the given code or name.
fn find_compiler_entry(needle: &str) -> Option<CompilerExplainEntry> {
    // scan compiler diagnostics for a matching code
    for group in COMPILER_DIAGNOSTIC_GROUPS {
        // resolve group labels once
        let phase = phase_label(group.phase);
        let severity = severity_label(group.severity);

        // check each definition for a match
        for definition in group.definitions {
            let matches = definition.code.eq_ignore_ascii_case(needle)
                || definition.name.eq_ignore_ascii_case(needle);
            if !matches {
                continue;
            }

            return Some(CompilerExplainEntry {
                code: definition.code,
                name: definition.name,
                description: definition.description,
                phase,
                severity,
            });
        }
    }

    None
}

/// Output a lint rule entry in the requested format.
fn output_lint_entry(args: &ExplainArgs, entry: LintExplainEntry) -> i32 {
    // emit json output for tooling
    if args.report.is_json() {
        let payload = ExplainDiagnosticPayload {
            diagnostic: ExplainPayload::Lint { entry },
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
        return 0;
    }

    // print the lint rule details
    console::info(&format!("{} ({})", entry.id, entry.code));
    console::info(&format!("name: {}", entry.name));
    console::info(&format!("category: {}", entry.category));
    console::info(&format!("fixable: {}", entry.fixable));
    console::info(&format!("recommended: {}", entry.recommended));
    console::info(&format!("stability: {}", entry.stability));
    console::info(&format!("description: {}", entry.description));
    if let Some(url) = entry.docs_url {
        console::info(&format!("docs: {url}"));
    }

    0
}

/// Output a compiler diagnostic entry in the requested format.
fn output_compiler_entry(args: &ExplainArgs, entry: CompilerExplainEntry) -> i32 {
    // emit json output for tooling
    if args.report.is_json() {
        let payload = ExplainDiagnosticPayload {
            diagnostic: ExplainPayload::Compiler { entry },
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
        return 0;
    }

    // print the compiler diagnostic details
    console::info(&format!("{} ({})", entry.name, entry.code));
    console::info(&format!("phase: {}", entry.phase));
    console::info(&format!("severity: {}", entry.severity));
    console::info(&format!("description: {}", entry.description));

    0
}

/// Render a phase label for compiler diagnostics.
fn phase_label(phase: CompilerPhase) -> &'static str {
    // map phase enum to its label
    match phase {
        CompilerPhase::Declare => "declare",
        CompilerPhase::Import => "import",
        CompilerPhase::Expand => "expand",
        CompilerPhase::Export => "export",
        CompilerPhase::Check => "check",
        CompilerPhase::Elaborate => "elaborate",
        CompilerPhase::Materialize => "materialize",
        CompilerPhase::Lower => "lower",
        CompilerPhase::Verify => "verify",
        CompilerPhase::Optimize => "optimize",
        CompilerPhase::Generate => "generate",
        CompilerPhase::Link => "link",
    }
}

/// Render a severity label for compiler diagnostics.
fn severity_label(severity: CompilerSeverity) -> &'static str {
    // map severity enum to its label
    match severity {
        CompilerSeverity::Error => "error",
        CompilerSeverity::Warning => "warning",
    }
}

/// Build a category label for compiler diagnostics.
fn compiler_category(phase: &str, severity: &str) -> String {
    format!("{phase} {severity}")
}

/// Format a category heading line.
fn format_category_heading(text: &str, color_enabled: bool) -> String {
    if color_enabled {
        let style = if text.ends_with(" error") {
            ["1", "91"]
        } else if text.ends_with(" warning") {
            ["1", "93"]
        } else {
            ["1", "32"]
        };
        return console::style(text, &style);
    }

    text.to_string()
}

/// Format a compiler diagnostic code.
fn format_compiler_code(code: &str, severity: &str, color_enabled: bool) -> String {
    if !color_enabled {
        return code.to_string();
    }

    let style = match severity {
        "error" => "1;91",
        "warning" => "1;93",
        _ => "1;36",
    };

    console::color(code, style)
}

/// Format a lint rule identifier.
fn format_rule_id(id: &str, color_enabled: bool) -> String {
    if color_enabled {
        return console::style(id, &["1", "36"]);
    }

    id.to_string()
}

/// Format a lint rule code.
fn format_rule_code(code: &str, color_enabled: bool) -> String {
    if color_enabled {
        return console::dim(code);
    }

    code.to_string()
}

/// Format the lint details line.
fn format_details_line(details: &str, color_enabled: bool) -> String {
    if color_enabled {
        return console::dim(details);
    }

    details.to_string()
}
