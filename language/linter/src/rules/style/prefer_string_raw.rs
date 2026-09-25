use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer String.raw when backslashes are escaped only to preserve themselves.
    pub PREFER_STRING_RAW {
        id: "prefer-string-raw",
        summary: "Prefer String.raw when backslashes are escaped only to preserve themselves",
        explanation: r#"
Repeatedly escaping literal backslashes obscures paths, regular-expression source, and similar text.
Instead, you SHOULD use a `String.raw` template when it preserves the same value without doubled backslashes.

Strings containing value-producing escapes remain unchanged.
"#,
        example: {
            reported: r#"
function path(): string {
    return "C:\\windows\\system32";
}
"#,
            accepted: r#"
function path(): string {
    return String.raw`C:\windows\system32`;
}
"#,
        },
        provenance: [Unicorn("prefer-string-raw")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report ordinary strings whose backslashes are clearer in raw templates.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored ordinary strings and untagged templates
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let suggestion = match node {
            dir::Expression::Literal(dir::Literal::String(_)) => string_fix(module, expression)?,
            dir::Expression::TemplateExpression { .. } => template_fix(module, expression)?,
            _ => None,
        };
        let Some(file) = suggestion else {
            continue;
        };

        // replace the complete literal form
        let span = module.source_extent(expression.into_any())?;
        let fix = lint.fix("use a raw string template", file)?;
        let diagnostic = lint
            .diagnostic("literal doubles backslashes to preserve them", span)
            .suggestion(fix);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build a raw-template replacement for one quoted string.
fn string_fix(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<FilePatch>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let source = module.source(span)?;
    if !source.starts_with('"') || !source.ends_with('"') {
        return Ok(None);
    }
    let Some(body) = source.get(1..source.len().saturating_sub(1)) else {
        return Ok(None);
    };
    let Some(body) = raw_body(body, '"') else {
        return Ok(None);
    };
    if body.ends_with('\\') || body.contains('`') || body.contains("${") {
        return Ok(None);
    }

    let replacement = format!("String.raw`{body}`");
    let mut file = FilePatch::new(span.file);
    file.replace(span, replacement);

    Ok(Some(file))
}

/// Build a raw-tag insertion and escape removals for one template.
fn template_fix(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<FilePatch>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let parsed = module.stages.parsed.file(span.file).ok_or_else(|| {
        ProviderError::internal(format!(
            "template source file {:?} is absent from lint module {:?}",
            span.file, module.id
        ))
    })?;
    let mut file = FilePatch::new(span.file);
    let mut has_escaped_backslash = false;

    // transform only lexer-owned template chunks
    for token in parsed.iter_token_spans() {
        if !span.contains_span(token.span) {
            continue;
        }
        let (prefix, suffix) = match token.token.ty() {
            dir::TokenType::TemplateString => (1, 1),
            dir::TokenType::TemplateStringStart => (1, 2),
            dir::TokenType::TemplateStringMiddle => (1, 2),
            dir::TokenType::TemplateStringEnd => (1, 1),
            _ => continue,
        };
        let source = module.source(token.span)?;
        let Some(body) = source.get(prefix..source.len().saturating_sub(suffix)) else {
            return Ok(None);
        };
        let Some(offsets) = escaped_backslashes(body) else {
            return Ok(None);
        };
        if body.ends_with("\\\\") {
            return Ok(None);
        }

        // remove one slash from each escaped backslash
        for offset in offsets {
            let start = token.span.start + (prefix + offset) as u32;
            file.delete(Span::at(token.span.file, start, 1));
            has_escaped_backslash = true;
        }
    }
    if !has_escaped_backslash {
        return Ok(None);
    }

    file.insert(span.start, "String.raw");
    file.sort();

    Ok(Some(file))
}

/// Return a raw body when every escape only preserves a slash or delimiter.
fn raw_body(body: &str, delimiter: char) -> Option<String> {
    let mut raw = String::with_capacity(body.len());
    let mut characters = body.chars();
    let mut has_escaped_backslash = false;

    // remove escape markers that raw templates make redundant
    while let Some(character) = characters.next() {
        if character != '\\' {
            raw.push(character);
            continue;
        }
        let escaped = characters.next()?;
        if escaped == '\\' {
            raw.push('\\');
            has_escaped_backslash = true;
        } else if escaped == delimiter {
            raw.push(delimiter);
        } else {
            return None;
        }
    }

    has_escaped_backslash.then_some(raw)
}

/// Return redundant slash offsets when a template has no other escapes.
fn escaped_backslashes(body: &str) -> Option<Vec<usize>> {
    let mut offsets = Vec::new();
    let mut characters = body.char_indices();

    // require every escape to preserve a literal backslash
    while let Some((offset, character)) = characters.next() {
        if character != '\\' {
            continue;
        }
        let (_, escaped) = characters.next()?;
        if escaped != '\\' {
            return None;
        }
        offsets.push(offset);
    }

    Some(offsets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a quoted path with a raw template.
    #[test]
    fn test_replaces_quoted_path() {
        let session = TestSession::dir(
            &PREFER_STRING_RAW,
            r#"
function path(): string {
    return "C:\\windows\\system32";
}
"#,
        );

        session.assert_fixes(
            r#"
function path(): string {
    return String.raw`C:\windows\system32`;
}
"#,
        );
    }

    /// Add a raw tag to an interpolated template without touching its expression.
    #[test]
    fn test_replaces_interpolated_template() {
        let session = TestSession::dir(
            &PREFER_STRING_RAW,
            r#"
function path(name: string): string {
    return `C:\\users\\profile-${name}\\file`;
}
"#,
        );

        session.assert_fixes(
            r#"
function path(name: string): string {
    return String.raw`C:\users\profile-${name}\file`;
}
"#,
        );
    }

    /// Accept value-producing escapes mixed with escaped backslashes.
    #[test]
    fn test_accepts_value_escape() {
        let session = TestSession::dir(
            &PREFER_STRING_RAW,
            r#"
function path(): string {
    return "C:\\windows\nnext";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept strings that cannot form the same raw template.
    #[test]
    fn test_accepts_template_syntax_text() {
        let session = TestSession::dir(
            &PREFER_STRING_RAW,
            r#"
function source(): string {
    return "\\${value}`";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an existing raw template tag.
    #[test]
    fn test_accepts_tagged_template() {
        let session = TestSession::dir(
            &PREFER_STRING_RAW,
            r#"
function value(): string {
    return String.raw`C:\windows`;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
