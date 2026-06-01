use destack_query as query;
use destack_query::{CodeAction, CodeActionKind};
use destack_source::{Edit, FileEdit, FileId, Span};

use crate::core::{CaseResult, module_artifact_diagnostics};
use crate::query::runner::position::resolve_query_position;
use crate::query::runner::snapshot::{
    compare_snapshot_lines, looks_like_snapshot, parse_snapshot_top_directive,
};
use crate::query::runner::span::{file_for, format_span_line_col, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a code_actions test.
///
/// Verifies that code actions are available at the expected positions.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no code_actions expectation defined".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> CaseResult {
    let parsed_expectation = match parse_code_action_expectation(exp.content.trim()) {
        Ok(parsed_expectation) => parsed_expectation,
        Err(message) => return CaseResult::Failed { message },
    };
    let content = parsed_expectation.expected.trim();

    // empty expectation is an error
    if content.is_empty() {
        return CaseResult::Failed {
            message: "code_actions expectation is empty".to_string(),
        };
    }

    // resolve the target into a span
    let range = match resolve_query_span(session, &exp.target) {
        Ok(range) => range,
        Err(message) => return CaseResult::Failed { message },
    };

    // run the query for the resolved span
    let context = parsed_expectation.context;
    let diagnostics = diagnostics_for_file(session, range.file);
    let ctx = session.module_context(range.file);
    let workspace = session.workspace_context();
    let actions = ctx.code_actions(&workspace, range, &diagnostics, &context);

    // validate invariants before comparing against expectations
    if let Err(message) = validate_code_action_invariants(session, &actions) {
        return CaseResult::Failed { message };
    }

    // prefer protocol shaped snapshots when the expectation is structured
    if looks_like_snapshot(content, &["[", "title=", "kind=", "edits="]) {
        // apply an optional top directive for large result sets
        let (top_limit, expected_snapshot) = parse_snapshot_top_directive(content);

        // format lines and apply the top limit when present
        let mut lines = code_action_snapshot(session, &actions);
        if let Some(limit) = top_limit {
            lines.truncate(limit);
        }

        return compare_snapshot_lines(
            &format!("code_actions snapshot at '{}'", exp.target),
            &lines,
            &expected_snapshot,
        );
    }

    // treat <none> as an explicit empty result expectation
    if content == "<none>" {
        if actions.is_empty() {
            return CaseResult::Passed;
        }

        let titles: Vec<_> = actions.iter().map(|action| action.title.clone()).collect();
        return CaseResult::Failed {
            message: format!(
                "code_actions at '{}' expected none, got {} actions: {titles:?}",
                exp.target,
                actions.len(),
            ),
        };
    }

    // require exact title lists in non snapshot mode
    let expected_titles: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    let actual_titles: Vec<&str> = actions.iter().map(|action| action.title.as_str()).collect();

    if actual_titles != expected_titles {
        return CaseResult::Failed {
            message: format!(
                "code_actions title list mismatch at '{}'\n\nexpected: {expected_titles:?}\nactual:   {actual_titles:?}",
                exp.target
            ),
        };
    }

    CaseResult::Passed
}

/// Parsed expectation for code action tests.
#[derive(Debug)]
struct ParsedCodeActionExpectation {
    /// The query context passed to the code action query.
    context: query::CodeActionContext,
    /// The expectation content after directives are removed.
    expected: String,
}

/// Parse query directives from a code action expectation.
fn parse_code_action_expectation(content: &str) -> Result<ParsedCodeActionExpectation, String> {
    // start with default query context
    let mut context = query::CodeActionContext::default();
    let mut expected_lines = Vec::new();

    // parse each line as either a directive or snapshot content
    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(raw_only) = trimmed.strip_prefix("only:") {
            context.only = parse_only_kinds(raw_only)?;
            continue;
        }

        if let Some(raw_include_disabled) = trimmed.strip_prefix("include_disabled:") {
            context.include_disabled = parse_bool(raw_include_disabled, "include_disabled")?;
            continue;
        }

        expected_lines.push(line);
    }

    Ok(ParsedCodeActionExpectation {
        context,
        expected: expected_lines.join("\n"),
    })
}

/// Parse a boolean directive value.
fn parse_bool(raw: &str, directive: &str) -> Result<bool, String> {
    // normalize and parse the directive value
    let value = raw.trim().to_ascii_lowercase();
    match value.as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!(
            "invalid code_actions directive '{directive}': expected boolean, got '{raw}'"
        )),
    }
}

/// Parse kind filters from an `only:` directive.
fn parse_only_kinds(raw: &str) -> Result<Vec<CodeActionKind>, String> {
    // split the directive into normalized kind names
    let mut kinds = Vec::new();
    for part in raw.split(',') {
        let kind_name = part.trim();
        if kind_name.is_empty() {
            continue;
        }

        let kind = parse_code_action_kind(kind_name)?;
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }

    Ok(kinds)
}

/// Parse a single code action kind alias.
fn parse_code_action_kind(kind_name: &str) -> Result<CodeActionKind, String> {
    // normalize casing and separators for kind matching
    let normalized = kind_name
        .trim()
        .to_ascii_lowercase()
        .replace(['.', '-'], "_");

    match normalized.as_str() {
        "quick_fix" | "quickfix" => Ok(CodeActionKind::QuickFix),
        "refactor" => Ok(CodeActionKind::Refactor),
        "refactor_extract" => Ok(CodeActionKind::RefactorExtract),
        "refactor_inline" => Ok(CodeActionKind::RefactorInline),
        "refactor_rewrite" => Ok(CodeActionKind::RefactorRewrite),
        "source" => Ok(CodeActionKind::Source),
        "source_fix_all" | "source_fixall" => Ok(CodeActionKind::SourceFixAll),
        _ => Err(format!(
            "unknown code action kind in 'only:' directive: '{kind_name}'"
        )),
    }
}

/// Resolve a query target into a span.
fn resolve_query_span(session: &QueryTestSession, target: &str) -> Result<Span, String> {
    // resolve cursor targets like $0 into an empty span
    if target.starts_with('$') {
        let (file_id, offset) = resolve_query_position(session, target)?;
        return Ok(Span::new(file_id, offset, offset));
    }

    // resolve marker targets into their full span
    let Some(marker) = session.markers.range(target) else {
        return Err(format!("marker '{target}' not found"));
    };

    Ok(marker.span)
}

/// Collect current artifact diagnostics for one query file.
fn diagnostics_for_file(
    session: &QueryTestSession,
    file_id: FileId,
) -> Vec<destack_source::Diagnostic> {
    let Some(module_id) = session
        .repository
        .module_id_for_file(session.revision, file_id)
        .ok()
        .flatten()
    else {
        return Vec::new();
    };
    let profile_id = session.module_profile_id(module_id);

    module_artifact_diagnostics(&session.repository, session.revision, module_id, profile_id)
        .iter()
        .filter(|diagnostic| diagnostic.primary_label().span.file == file_id)
        .cloned()
        .collect()
}

/// Validate basic code action invariants.
fn validate_code_action_invariants(
    session: &QueryTestSession,
    actions: &[CodeAction],
) -> Result<(), String> {
    let mut errors = Vec::new();

    // enforce deterministic ordering and deduplication
    let mut previous: Option<(u8, u8, &str, &str, String)> = None;
    for action in actions {
        // require non empty titles
        if action.title.trim().is_empty() {
            errors.push("code action title is empty".to_string());
        }

        // ensure all edit spans stay within their file bounds
        for file_edit in &action.edits.files {
            let source = source_for_file(session, file_edit.file);
            let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

            for edit in &file_edit.edits {
                if edit.span.start > edit.span.end {
                    errors.push(format!(
                        "edit span start {} is after end {}",
                        edit.span.start, edit.span.end
                    ));
                }

                if edit.span.end > source_len {
                    errors.push(format!(
                        "edit span end {} exceeds source length {}",
                        edit.span.end, source_len
                    ));
                }
            }
        }

        // compare against the previous action key
        let key = code_action_key(action);
        if let Some(prev) = previous {
            if key < prev {
                errors.push(format!(
                    "code actions are not sorted: {prev:?} before {key:?}"
                ));
            }

            if key == prev {
                errors.push(format!("duplicate code action {key:?}"));
            }
        }

        previous = Some(key);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "code_actions invariant violations:\n{}",
            errors.join("\n")
        ))
    }
}

/// Decide whether an expectation is a structured snapshot.
/// Format code actions as a protocol shaped snapshot.
fn code_action_snapshot(session: &QueryTestSession, actions: &[CodeAction]) -> Vec<String> {
    let mut lines = Vec::new();

    for (index, action) in actions.iter().enumerate() {
        let line = format_code_action_line(session, index, action);
        lines.push(line);
    }

    lines
}

/// Format a single code action line.
fn format_code_action_line(
    session: &QueryTestSession,
    index: usize,
    action: &CodeAction,
) -> String {
    // render the action kind and diagnostic code
    let kind = code_action_kind_name(action.kind);
    let diagnostic = action.diagnostic_code.as_deref().unwrap_or("<none>");

    // render the edit summary for snapshot comparisons
    let edits = format_action_edits(session, &action.edits.files);

    format!(
        "[{index}] title={} kind={kind} preferred={} diag={diagnostic} edits={edits}",
        action.title, action.is_preferred
    )
}

/// Format the edits for a code action.
fn format_action_edits(session: &QueryTestSession, file_edits: &[FileEdit]) -> String {
    if file_edits.is_empty() {
        return "<none>".to_string();
    }

    // sort file edits by file id for deterministic snapshots
    let mut file_edits = file_edits.to_vec();
    file_edits.sort_by_key(|file_edit| file_edit.file.0);

    let mut parts = Vec::new();
    for file_edit in file_edits {
        let file_part = format_file_edit(session, file_edit.file, &file_edit.edits);
        parts.push(file_part);
    }

    parts.join("; ")
}

/// Format a file edit and its edits.
fn format_file_edit(session: &QueryTestSession, file_id: FileId, edits: &[Edit]) -> String {
    let Some(file) = file_for(session, file_id) else {
        return format!("file=<unknown:{}> edits=<unknown>", file_id.0);
    };

    let mut edits = edits.to_vec();
    edits.sort_by_key(edit_key);

    let mut parts = Vec::new();
    for edit in edits {
        let range = format_span_line_col(&file.source, edit.span);
        let text = edit.new_text.replace('\n', "\\n");
        parts.push(format!("{}:{range}=>\"{text}\"", file.name));
    }

    parts.join(", ")
}

/// Build a stable key for a code action in tests.
fn code_action_key(action: &CodeAction) -> (u8, u8, &str, &str, String) {
    // extract ranking components for stable comparisons
    let kind_rank = code_action_kind_rank(action.kind);
    let preferred_rank = if action.is_preferred { 0 } else { 1 };
    let title = action.title.as_str();
    let diagnostic = action.diagnostic_code.as_deref().unwrap_or("");
    let edits = action_edit_key(&action.edits.files);

    (kind_rank, preferred_rank, title, diagnostic, edits)
}

/// Rank code action kinds for stable ordering in tests.
fn code_action_kind_rank(kind: CodeActionKind) -> u8 {
    match kind {
        CodeActionKind::QuickFix => 0,
        CodeActionKind::Refactor => 1,
        CodeActionKind::RefactorExtract => 2,
        CodeActionKind::RefactorInline => 3,
        CodeActionKind::RefactorRewrite => 4,
        CodeActionKind::Source => 5,
        CodeActionKind::SourceFixAll => 6,
    }
}

/// Format a code action kind as a lowercase name.
fn code_action_kind_name(kind: CodeActionKind) -> &'static str {
    match kind {
        CodeActionKind::QuickFix => "quick_fix",
        CodeActionKind::Refactor => "refactor",
        CodeActionKind::RefactorExtract => "refactor_extract",
        CodeActionKind::RefactorInline => "refactor_inline",
        CodeActionKind::RefactorRewrite => "refactor_rewrite",
        CodeActionKind::Source => "source",
        CodeActionKind::SourceFixAll => "source_fix_all",
    }
}

/// Build a stable edit key for a list of file edits.
fn action_edit_key(file_edits: &[FileEdit]) -> String {
    let mut keys = Vec::new();

    for file_edit in file_edits {
        let key = file_edit_key(file_edit.file, &file_edit.edits);
        keys.push(key);
    }

    keys.sort();
    keys.join("|")
}

/// Build a stable key for a file edit.
fn file_edit_key(file_id: FileId, edits: &[Edit]) -> String {
    let mut edits = edits.to_vec();
    edits.sort_by_key(edit_key);

    let mut parts = Vec::new();
    parts.push(format!("file={}", file_id.0));

    for edit in edits {
        let (start, end, file, text) = edit_key(&edit);
        parts.push(format!("{file}:{start}-{end}=>{text}"));
    }

    parts.join(",")
}

/// Build a stable key for a single edit.
fn edit_key(edit: &Edit) -> (u32, u32, u128, String) {
    // extract span coordinates and replacement text
    let file = edit.span.file.0;
    let start = edit.span.start;
    let end = edit.span.end;
    let text = edit.new_text.clone();

    (start, end, file, text)
}
