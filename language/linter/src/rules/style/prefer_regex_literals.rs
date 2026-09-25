use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer regex literals for constant patterns and flags.
    pub PREFER_REGEX_LITERALS {
        id: "prefer-regex-literals",
        summary: "Prefer regex literals for constant patterns and flags",
        explanation: r#"
Constructing a regular expression from constant strings defers a fixed pattern to runtime and doubles string escaping.
Instead, you SHOULD use a regular expression literal when both the pattern and flags are constant.
"#,
        example: {
            reported: r#"
const word = new RegExp("\\w+", "u");
"#,
            accepted: r#"
const word = /\w+/u;
"#,
        },
        provenance: [Eslint("prefer-regex-literals")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report constant canonical RegExp construction.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical RegExp construction with one or two positional arguments
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::New { arguments, .. } = node else {
            continue;
        };
        if module.language_item(expression)? != Some(dir::LanguageItem::RegExp) {
            continue;
        }
        let Some((pattern, flags)) = constant_arguments(module, arguments)? else {
            continue;
        };
        let Some(literal) = regex_literal(pattern, flags) else {
            continue;
        };

        // replace the complete construction when comments are retained
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("constant strings construct a regular expression", span);
        if !module.has_unretained_comment(span, &[])? {
            let suggestion = suggestion(lint, span, literal)?;
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the constant pattern and flags from one RegExp argument list.
fn constant_arguments<'a>(
    module: &'a DirModule<'_>,
    arguments: &[dir::LocalNodeId<dir::Argument>],
) -> Result<Option<(&'a str, &'a str)>, ProviderError> {
    let (pattern, flags) = match arguments {
        [] => return Ok(Some(("", ""))),
        [pattern] => (pattern, None),
        [pattern, flags] => (pattern, Some(flags)),
        _ => return Ok(None),
    };
    let Some(pattern) = module.view().get(*pattern).value() else {
        return Ok(None);
    };
    let Some(dir::Literal::String(pattern)) = module.scalar_constant(pattern)? else {
        return Ok(None);
    };
    let flags = if let Some(flags) = flags {
        let Some(flags) = module.view().get(*flags).value() else {
            return Ok(None);
        };
        let Some(dir::Literal::String(flags)) = module.scalar_constant(flags)? else {
            return Ok(None);
        };

        module.dir.strings.get(flags)
    } else {
        ""
    };

    Ok(Some((module.dir.strings.get(pattern), flags)))
}

/// Build one regex literal from constant pattern and flag strings.
fn regex_literal(pattern: &str, flags: &str) -> Option<String> {
    if !valid_flags(flags) {
        return None;
    }

    // escape literal delimiters and source line terminators
    let mut source = String::with_capacity(pattern.len() + flags.len() + 2);
    let mut backslashes = 0;
    source.push('/');
    if pattern.is_empty() {
        source.push_str("(?:)");
    } else {
        for character in pattern.chars() {
            match character {
                '\\' => {
                    source.push(character);
                    backslashes += 1;
                }
                '/' => {
                    if backslashes % 2 == 0 {
                        source.push('\\');
                    }
                    source.push('/');
                    backslashes = 0;
                }
                '\n' => {
                    source.push_str("\\n");
                    backslashes = 0;
                }
                '\r' => {
                    source.push_str("\\r");
                    backslashes = 0;
                }
                '\u{2028}' => {
                    source.push_str("\\u2028");
                    backslashes = 0;
                }
                '\u{2029}' => {
                    source.push_str("\\u2029");
                    backslashes = 0;
                }
                _ => {
                    source.push(character);
                    backslashes = 0;
                }
            }
        }
        if backslashes % 2 != 0 {
            return None;
        }
    }
    source.push('/');
    source.push_str(flags);

    Some(source)
}

/// Return whether one flag string is accepted by an ECMAScript regex literal.
fn valid_flags(flags: &str) -> bool {
    let mut seen = [false; 8];
    for flag in flags.chars() {
        let index = match flag {
            'd' => 0,
            'g' => 1,
            'i' => 2,
            'm' => 3,
            's' => 4,
            'u' => 5,
            'v' => 6,
            'y' => 7,
            _ => return false,
        };
        if seen[index] {
            return false;
        }
        seen[index] = true;
    }

    !seen[5] || !seen[6]
}

/// Build one regex literal replacement.
fn suggestion(
    lint: &Lint,
    span: tspp_source::Span,
    literal: String,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let patch = Patch::replace(span, literal);

    lint.suggestion("use a regular expression literal", patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace constant pattern and flag strings.
    #[test]
    fn test_replaces_constant_constructor() {
        let session = TestSession::dir(
            &PREFER_REGEX_LITERALS,
            r#"
const word = new RegExp("\\w+", "u");
"#,
        );

        session.assert_suggestions(
            r#"
const word = /\w+/u;
"#,
        );
    }

    /// Escape literal delimiters and line terminators.
    #[test]
    fn test_escapes_literal_source() {
        let session = TestSession::dir(
            &PREFER_REGEX_LITERALS,
            r#"
const path = new RegExp("a/b\\n");
"#,
        );

        session.assert_suggestions(
            r#"
const path = /a\/b\n/;
"#,
        );
    }

    /// Spell an empty pattern without producing a line comment.
    #[test]
    fn test_replaces_empty_pattern() {
        let session = TestSession::dir(
            &PREFER_REGEX_LITERALS,
            r#"
const empty = new RegExp("");
"#,
        );

        session.assert_suggestions(
            r#"
const empty = /(?:)/;
"#,
        );
    }

    /// Replace construction without arguments with an empty literal.
    #[test]
    fn test_replaces_missing_pattern() {
        let session = TestSession::dir(
            &PREFER_REGEX_LITERALS,
            r#"
const empty = new RegExp();
"#,
        );

        session.assert_suggestions(
            r#"
const empty = /(?:)/;
"#,
        );
    }

    /// Preserve the escape parity of literal delimiters.
    #[test]
    fn test_escapes_delimiters_after_backslashes() {
        let session = TestSession::dir(
            &PREFER_REGEX_LITERALS,
            r#"
const slash = new RegExp("\\/");
const backslashSlash = new RegExp("\\\\/");
"#,
        );

        session.assert_suggestions(
            r#"
const slash = /\//;
const backslashSlash = /\\\//;
"#,
        );
    }

    /// Accept dynamic patterns whose compilation remains at runtime.
    #[test]
    fn test_accepts_dynamic_pattern() {
        let session = TestSession::dir(
            &PREFER_REGEX_LITERALS,
            r#"
function compile(pattern: string): RegExp {
    return new RegExp(pattern);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept invalid constant flags instead of producing invalid source.
    #[test]
    fn test_accepts_invalid_flags() {
        let session = TestSession::dir(
            &PREFER_REGEX_LITERALS,
            r#"
const word = new RegExp("word", "gg");
"#,
        );

        session.assert_no_diagnostics();
    }
}
