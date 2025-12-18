use destack_workspace::query::{self, SemanticTokenModifiers, SemanticTokenType};

use crate::harness::TestResult;
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
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no semantic_tokens expectation defined".to_string(),
        };
    };

    let tokens = query::semantic_tokens(&session.session, session.file_id);
    run_with_expectation(session, exp, &tokens)
}

/// Run with markdown expectation.
fn run_with_expectation(
    session: &QueryTestSession,
    exp: &QueryExpectation,
    tokens: &[query::SemanticToken],
) -> TestResult {
    let content = exp.content.trim();

    // empty expectation is an error
    if content.is_empty() {
        let formatted = format_tokens(session, tokens);
        return TestResult::Failed {
            message: format!("semantic_tokens expectation is empty, got:\n{formatted}",),
        };
    }

    // "<none>" means we expect no tokens
    if content == "<none>" {
        return if tokens.is_empty() {
            TestResult::Passed
        } else {
            let formatted = format_tokens(session, tokens);
            TestResult::Failed {
                message: format!("semantic_tokens expected no tokens, got:\n{formatted}"),
            }
        };
    }

    // parse expected tokens
    let expected: Vec<ExpectedToken> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_expected_token)
        .collect();

    if expected.is_empty() {
        return TestResult::Failed {
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
        let formatted = format_tokens(session, tokens);
        return TestResult::Failed {
            message: format!(
                "semantic_tokens count mismatch: expected {}, got {}\nActual:\n{formatted}",
                expected.len(),
                actual.len(),
            ),
        };
    }

    for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
        if exp.text != act.text {
            return TestResult::Failed {
                message: format!(
                    "token {} text mismatch: expected '{}', got '{}'",
                    i, exp.text, act.text
                ),
            };
        }
        if exp.token_type != act.token_type {
            return TestResult::Failed {
                message: format!(
                    "token '{}' type mismatch: expected {:?}, got {:?}",
                    exp.text, exp.token_type, act.token_type
                ),
            };
        }
        // for modifiers, check that expected modifiers are present
        for modifier in &exp.modifiers {
            if !act.modifiers.contains(modifier) {
                return TestResult::Failed {
                    message: format!("token '{}' missing modifier: {:?}", exp.text, modifier),
                };
            }
        }
    }

    TestResult::Passed
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

/// Collect all modifiers from a bitset.
fn collect_modifiers(mods: SemanticTokenModifiers) -> Vec<SemanticTokenModifiers> {
    let mut result = Vec::new();
    if mods.contains(SemanticTokenModifiers::DECLARATION) {
        result.push(SemanticTokenModifiers::DECLARATION);
    }
    if mods.contains(SemanticTokenModifiers::DEFINITION) {
        result.push(SemanticTokenModifiers::DEFINITION);
    }
    if mods.contains(SemanticTokenModifiers::READONLY) {
        result.push(SemanticTokenModifiers::READONLY);
    }
    if mods.contains(SemanticTokenModifiers::STATIC) {
        result.push(SemanticTokenModifiers::STATIC);
    }
    if mods.contains(SemanticTokenModifiers::DEPRECATED) {
        result.push(SemanticTokenModifiers::DEPRECATED);
    }
    if mods.contains(SemanticTokenModifiers::ABSTRACT) {
        result.push(SemanticTokenModifiers::ABSTRACT);
    }
    if mods.contains(SemanticTokenModifiers::ASYNC) {
        result.push(SemanticTokenModifiers::ASYNC);
    }
    if mods.contains(SemanticTokenModifiers::MODIFICATION) {
        result.push(SemanticTokenModifiers::MODIFICATION);
    }
    if mods.contains(SemanticTokenModifiers::DOCUMENTATION) {
        result.push(SemanticTokenModifiers::DOCUMENTATION);
    }
    if mods.contains(SemanticTokenModifiers::DEFAULT_LIBRARY) {
        result.push(SemanticTokenModifiers::DEFAULT_LIBRARY);
    }
    if mods.contains(SemanticTokenModifiers::MUTABLE) {
        result.push(SemanticTokenModifiers::MUTABLE);
    }
    result
}

/// Format tokens for error messages.
fn format_tokens(session: &QueryTestSession, tokens: &[query::SemanticToken]) -> String {
    tokens
        .iter()
        .filter_map(|t| {
            let start = t.span.start as usize;
            let end = t.span.end as usize;
            let text = session.source.get(start..end)?;
            let modifiers = format_modifiers(t.modifiers);
            if modifiers.is_empty() {
                Some(format!("{}: {:?}", text, t.token_type))
            } else {
                Some(format!("{}: {:?} [{}]", text, t.token_type, modifiers))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format modifiers for display.
fn format_modifiers(mods: SemanticTokenModifiers) -> String {
    let mut parts = Vec::new();
    if mods.contains(SemanticTokenModifiers::DECLARATION) {
        parts.push("declaration");
    }
    if mods.contains(SemanticTokenModifiers::DEFINITION) {
        parts.push("definition");
    }
    if mods.contains(SemanticTokenModifiers::READONLY) {
        parts.push("readonly");
    }
    if mods.contains(SemanticTokenModifiers::STATIC) {
        parts.push("static");
    }
    if mods.contains(SemanticTokenModifiers::DEPRECATED) {
        parts.push("deprecated");
    }
    if mods.contains(SemanticTokenModifiers::ABSTRACT) {
        parts.push("abstract");
    }
    if mods.contains(SemanticTokenModifiers::ASYNC) {
        parts.push("async");
    }
    if mods.contains(SemanticTokenModifiers::MODIFICATION) {
        parts.push("modification");
    }
    if mods.contains(SemanticTokenModifiers::DOCUMENTATION) {
        parts.push("documentation");
    }
    if mods.contains(SemanticTokenModifiers::DEFAULT_LIBRARY) {
        parts.push("default_library");
    }
    if mods.contains(SemanticTokenModifiers::MUTABLE) {
        parts.push("mutable");
    }
    parts.join(", ")
}
