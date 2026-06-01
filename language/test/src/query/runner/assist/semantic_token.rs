use destack_query as query;
use destack_query::{SemanticTokenModifiers, SemanticTokenType};
use destack_source::Span;

use crate::core::CaseResult;
use crate::query::runner::position::resolve_query_span;
use crate::query::runner::snapshot::{
    compare_snapshot, looks_like_snapshot, normalize_expected_snapshot,
};
use crate::query::runner::span::{format_span_line_col, source_for_file};
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a semantic_tokens test.
///
/// Tests that semantic tokens are correctly generated for a file.
/// The expectation format is a list of tokens, one per line:
/// ```text
/// <token_text>: <type> [<modifier1>, <modifier2>]
/// ```
///
/// For example:
/// ```text
/// MyClass: class [declaration]
/// foo: function [declaration, async]
/// x: parameter [declaration, readonly]
/// ```
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no semantic_tokens expectation defined".to_string(),
        };
    };

    let ctx = session.primary_module_context();
    let tokens = ctx.semantic_tokens();
    run_with_expectation(session, exp, &tokens)
}

/// Run a semantic_tokens_range test.
pub fn run_range(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> CaseResult {
    let Some(exp) = expectation else {
        return CaseResult::Skipped {
            reason: "no semantic_tokens_range expectation defined".to_string(),
        };
    };

    let range = match resolve_query_span(session, &exp.target) {
        Ok(range) => range,
        Err(message) => return CaseResult::Failed { message },
    };

    let ctx = session.module_context(range.file);
    let tokens = ctx.semantic_tokens_range(range);
    run_with_expectation(session, exp, &tokens)
}

/// Run with markdown expectation.
fn run_with_expectation(
    session: &QueryTestSession,
    exp: &QueryExpectation,
    tokens: &[query::SemanticToken],
) -> CaseResult {
    let content = exp.content.trim();
    let snapshot = format_tokens_snapshot(session, tokens);

    // empty expectation is an error
    if content.is_empty() {
        return CaseResult::Failed {
            message: format!("semantic_tokens expectation is empty, got:\n{snapshot}"),
        };
    }

    // "<none>" means we expect no tokens
    if content == "<none>" {
        return if tokens.is_empty() {
            CaseResult::Passed
        } else {
            CaseResult::Failed {
                message: format!("semantic_tokens expected no tokens, got:\n{snapshot}"),
            }
        };
    }

    // validate basic invariants before comparing expectations
    if let Err(message) = validate_token_invariants(session, tokens) {
        return CaseResult::Failed { message };
    }

    // support snapshot expectations for gold standard assertions
    if looks_like_snapshot(content, &["range=", "kind="]) {
        return compare_snapshot("semantic_tokens", &snapshot, content);
    }

    // parse expected tokens
    let expected: Vec<ExpectedToken> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_expected_token)
        .collect();

    if expected.is_empty() {
        return CaseResult::Failed {
            message: format!("failed to parse any expected tokens from:\n{content}"),
        };
    }

    // convert actual tokens to comparable format
    let actual: Vec<ExpectedToken> = tokens
        .iter()
        .filter_map(|t| token_to_expected(session, t))
        .collect();

    // compare
    if actual.len() != expected.len() {
        return CaseResult::Failed {
            message: format!(
                "semantic_tokens count mismatch: expected {}, got {}\nActual:\n{snapshot}",
                expected.len(),
                actual.len(),
            ),
        };
    }

    for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
        if exp.text != act.text {
            return CaseResult::Failed {
                message: format!(
                    "token {} text mismatch: expected '{}', got '{}'",
                    i, exp.text, act.text
                ),
            };
        }
        if exp.token_type != act.token_type {
            return CaseResult::Failed {
                message: format!(
                    "token '{}' type mismatch: expected {:?}, got {:?}",
                    exp.text, exp.token_type, act.token_type
                ),
            };
        }
        // for modifiers, check that expected modifiers are present
        for modifier in &exp.modifiers {
            if !act.modifiers.contains(modifier) {
                return CaseResult::Failed {
                    message: format!("token '{}' missing modifier: {:?}", exp.text, modifier),
                };
            }
        }
    }

    CaseResult::Passed
}

/// A parsed expected token.
#[derive(Debug)]
struct ExpectedToken {
    text: String,
    token_type: SemanticTokenType,
    modifiers: Vec<SemanticTokenModifiers>,
}

/// Parse an expected token line like "foo: function [declaration, async]"
fn parse_expected_token(line: &str) -> Option<ExpectedToken> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // split on first colon
    let (text, rest) = line.split_once(':')?;
    let text = text.trim().to_string();
    let rest = rest.trim();

    // check for modifiers in brackets
    let (type_str, modifiers_str) = if let Some(bracket_start) = rest.find('[') {
        let type_str = rest[..bracket_start].trim();
        let bracket_end = rest.find(']')?;
        let modifiers_str = &rest[bracket_start + 1..bracket_end];
        (type_str, Some(modifiers_str))
    } else {
        (rest, None)
    };

    let token_type = parse_token_type(type_str)?;
    let modifiers = modifiers_str
        .map(|s| {
            s.split(',')
                .filter_map(|m| parse_modifier(m.trim()))
                .collect()
        })
        .unwrap_or_default();

    Some(ExpectedToken {
        text,
        token_type,
        modifiers,
    })
}

/// Parse a token type name.
fn parse_token_type(s: &str) -> Option<SemanticTokenType> {
    match s.to_lowercase().as_str() {
        "namespace" => Some(SemanticTokenType::Namespace),
        "type" => Some(SemanticTokenType::Type),
        "class" => Some(SemanticTokenType::Class),
        "enum" => Some(SemanticTokenType::Enum),
        "interface" => Some(SemanticTokenType::Interface),
        "struct" => Some(SemanticTokenType::Struct),
        "typeparameter" | "type_parameter" => Some(SemanticTokenType::TypeParameter),
        "parameter" => Some(SemanticTokenType::Parameter),
        "variable" => Some(SemanticTokenType::Variable),
        "property" => Some(SemanticTokenType::Property),
        "enummember" | "enum_member" => Some(SemanticTokenType::EnumMember),
        "function" => Some(SemanticTokenType::Function),
        "method" => Some(SemanticTokenType::Method),
        "macro" => Some(SemanticTokenType::Macro),
        "keyword" => Some(SemanticTokenType::Keyword),
        "modifier" => Some(SemanticTokenType::Modifier),
        "comment" => Some(SemanticTokenType::Comment),
        "string" => Some(SemanticTokenType::String),
        "number" => Some(SemanticTokenType::Number),
        "regexp" => Some(SemanticTokenType::Regexp),
        "operator" => Some(SemanticTokenType::Operator),
        "decorator" => Some(SemanticTokenType::Decorator),
        "label" => Some(SemanticTokenType::Label),
        _ => None,
    }
}

/// Parse a modifier name.
fn parse_modifier(s: &str) -> Option<SemanticTokenModifiers> {
    match s.to_lowercase().as_str() {
        "declaration" => Some(SemanticTokenModifiers::DECLARATION),
        "definition" => Some(SemanticTokenModifiers::DEFINITION),
        "readonly" => Some(SemanticTokenModifiers::READONLY),
        "static" => Some(SemanticTokenModifiers::STATIC),
        "deprecated" => Some(SemanticTokenModifiers::DEPRECATED),
        "abstract" => Some(SemanticTokenModifiers::ABSTRACT),
        "async" => Some(SemanticTokenModifiers::ASYNC),
        "modification" => Some(SemanticTokenModifiers::MODIFICATION),
        "documentation" => Some(SemanticTokenModifiers::DOCUMENTATION),
        "defaultlibrary" | "default_library" => Some(SemanticTokenModifiers::DEFAULT_LIBRARY),
        "mutable" => Some(SemanticTokenModifiers::MUTABLE),
        _ => None,
    }
}

/// Convert an actual token to ExpectedToken for comparison.
fn token_to_expected(
    session: &QueryTestSession,
    token: &query::SemanticToken,
) -> Option<ExpectedToken> {
    let start = token.span.start as usize;
    let end = token.span.end as usize;
    let text = session.source.get(start..end)?.to_string();

    let modifiers = collect_modifiers(token.modifiers);

    Some(ExpectedToken {
        text,
        token_type: token.token_type,
        modifiers,
    })
}

/// Validate token invariants like bounds, order, and overlap.
fn validate_token_invariants(
    session: &QueryTestSession,
    tokens: &[query::SemanticToken],
) -> Result<(), String> {
    // resolve the source once for this file
    let source = source_for_file(session, session.file_id);
    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);

    // track the previous span to check ordering and overlap
    let mut previous_span: Option<Span> = None;

    for token in tokens {
        // validate span bounds against the source length
        validate_span_bounds(token.span, source_len)?;

        // ensure tokens are sorted and non overlapping
        if let Some(prev) = previous_span {
            // reject tokens that move backwards in the file
            if token.span.start < prev.start {
                let prev_range = format_span_line_col(source, prev);
                let this_range = format_span_line_col(source, token.span);
                return Err(format!(
                    "semantic_tokens out of order: previous {prev_range}, current {this_range}",
                ));
            }

            // reject tokens that still overlap after deduplication
            if token.span.start < prev.end {
                let prev_range = format_span_line_col(source, prev);
                let this_range = format_span_line_col(source, token.span);
                return Err(format!(
                    "semantic_tokens overlap: previous {prev_range}, current {this_range}",
                ));
            }
        }

        previous_span = Some(token.span);
    }

    Ok(())
}

/// Validate that a span is well formed and within the source bounds.
fn validate_span_bounds(span: Span, source_len: u32) -> Result<(), String> {
    // reject inverted or empty spans early
    if span.start >= span.end {
        return Err(format!(
            "semantic_tokens invalid span: start {} >= end {}",
            span.start, span.end,
        ));
    }

    // reject spans that exceed the source bounds
    if span.end > source_len {
        return Err(format!(
            "semantic_tokens span out of bounds: end {} > source_len {}",
            span.end, source_len,
        ));
    }

    Ok(())
}

/// Collect all modifiers from a bitset.
fn collect_modifiers(mods: SemanticTokenModifiers) -> Vec<SemanticTokenModifiers> {
    // collect modifiers in a deterministic order
    let mut result = Vec::new();
    let ordered = [
        SemanticTokenModifiers::DECLARATION,
        SemanticTokenModifiers::DEFINITION,
        SemanticTokenModifiers::READONLY,
        SemanticTokenModifiers::STATIC,
        SemanticTokenModifiers::DEPRECATED,
        SemanticTokenModifiers::ABSTRACT,
        SemanticTokenModifiers::ASYNC,
        SemanticTokenModifiers::MODIFICATION,
        SemanticTokenModifiers::DOCUMENTATION,
        SemanticTokenModifiers::DEFAULT_LIBRARY,
        SemanticTokenModifiers::MUTABLE,
    ];

    for modifier in ordered {
        if mods.contains(modifier) {
            result.push(modifier);
        }
    }

    result
}

/// Detect whether an expectation uses the snapshot format.
/// Format tokens into the snapshot format.
fn format_tokens_snapshot(session: &QueryTestSession, tokens: &[query::SemanticToken]) -> String {
    // resolve the correct source for this file
    let source = source_for_file(session, session.file_id);

    let mut lines = Vec::new();
    for token in tokens {
        // compute the token range and text
        let range = format_span_line_col(source, token.span);
        let text = token_text(source, token.span).unwrap_or("<missing>".to_string());

        // compute stable names for the token kind and modifiers
        let kind = semantic_token_kind_name(token.token_type);
        let modifiers = semantic_token_modifiers_name(token.modifiers);

        lines.push(format!(
            "range={range} text={text} kind={kind} modifiers={modifiers}",
        ));
    }

    normalize_expected_snapshot(&lines.join("\n"))
}

/// Extract the token text for a span.
fn token_text(source: &str, span: Span) -> Option<String> {
    let start = usize::try_from(span.start).ok()?;
    let end = usize::try_from(span.end).ok()?;
    source.get(start..end).map(|text| text.to_string())
}

/// Format a token kind as a stable string.
fn semantic_token_kind_name(token_type: SemanticTokenType) -> &'static str {
    match token_type {
        SemanticTokenType::Namespace => "namespace",
        SemanticTokenType::Type => "type",
        SemanticTokenType::Class => "class",
        SemanticTokenType::Enum => "enum",
        SemanticTokenType::Interface => "interface",
        SemanticTokenType::Struct => "struct",
        SemanticTokenType::TypeParameter => "type_parameter",
        SemanticTokenType::Parameter => "parameter",
        SemanticTokenType::Variable => "variable",
        SemanticTokenType::Property => "property",
        SemanticTokenType::EnumMember => "enum_member",
        SemanticTokenType::Function => "function",
        SemanticTokenType::Method => "method",
        SemanticTokenType::Macro => "macro",
        SemanticTokenType::Keyword => "keyword",
        SemanticTokenType::Modifier => "modifier",
        SemanticTokenType::Comment => "comment",
        SemanticTokenType::String => "string",
        SemanticTokenType::Number => "number",
        SemanticTokenType::Regexp => "regexp",
        SemanticTokenType::Operator => "operator",
        SemanticTokenType::Decorator => "decorator",
        SemanticTokenType::Label => "label",
    }
}

/// Format token modifiers as a stable string.
fn semantic_token_modifiers_name(modifiers: SemanticTokenModifiers) -> String {
    let modifiers = collect_modifiers(modifiers);
    if modifiers.is_empty() {
        return "<none>".to_string();
    }

    let mut names = Vec::with_capacity(modifiers.len());
    for modifier in modifiers {
        names.push(semantic_token_modifier_name(modifier));
    }

    names.join(",")
}

/// Format a token modifier as a stable string.
fn semantic_token_modifier_name(modifier: SemanticTokenModifiers) -> &'static str {
    match modifier {
        SemanticTokenModifiers::DECLARATION => "declaration",
        SemanticTokenModifiers::DEFINITION => "definition",
        SemanticTokenModifiers::READONLY => "readonly",
        SemanticTokenModifiers::STATIC => "static",
        SemanticTokenModifiers::DEPRECATED => "deprecated",
        SemanticTokenModifiers::ABSTRACT => "abstract",
        SemanticTokenModifiers::ASYNC => "async",
        SemanticTokenModifiers::MODIFICATION => "modification",
        SemanticTokenModifiers::DOCUMENTATION => "documentation",
        SemanticTokenModifiers::DEFAULT_LIBRARY => "default_library",
        SemanticTokenModifiers::MUTABLE => "mutable",
        _ => "<unknown>",
    }
}
