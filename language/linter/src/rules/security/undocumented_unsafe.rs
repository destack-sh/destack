use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require a safety rationale for every unsafe declaration and expression.
    pub UNDOCUMENTED_UNSAFE {
        id: "undocumented-unsafe",
        summary: "Require a safety rationale for every unsafe declaration and expression",
        explanation: r#"
Unsafe declarations impose obligations on their callers, and local unsafe regions rely on invariants the checker cannot verify.
Instead, you SHOULD document caller obligations under `# Safety` and justify local regions with an immediately preceding `SAFETY:` comment or a nonempty `@unsafe` reason.
"#,
        example: {
            reported: r#"
function execute(): void {
    @unsafe
    {}
}
"#,
            accepted: r#"
function execute(): void {
    // SAFETY: no unsafe operation escapes this region
    @unsafe
    {}
}
"#,
        },
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report canonical unsafe applications without the required authored rationale.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect checked applications of the canonical unsafe decorator
    for (_, application) in module.decorators.iter_applications() {
        if !matches!(
            application.resolution.target,
            dir::DecoratorTarget::LanguageItem {
                item: dir::LanguageItem::Unsafe,
                ..
            }
        ) {
            continue;
        }
        let owner = application.owner.local_id;
        let decorator = application.source.local_id;
        let span = module.source_extent(decorator.into_any())?;

        // require unsafe declarations to document their caller obligations
        if module
            .bindings
            .declaration_symbol(application.owner)
            .is_some()
        {
            if !has_safety_section(module, owner) {
                let diagnostic = lint
                    .diagnostic("unsafe declaration has no `# Safety` section", span)
                    .help("document every obligation callers must uphold under `# Safety`");
                output.report(diagnostic);
            }
            continue;
        }

        // require local unsafe regions to carry an associated rationale
        if has_reason(module, application)? || has_safety_comment(module, span)? {
            continue;
        }
        let diagnostic = lint
            .diagnostic("unsafe region has no safety rationale", span)
            .help("add an immediately preceding `SAFETY:` comment or an `@unsafe` reason");
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one node's normalized documentation contains a safety section.
fn has_safety_section(module: &DirModule<'_>, owner: dir::LocalNodeIdAny) -> bool {
    let view = module.view();
    let Some(documentation) = view.get_documentation_any(owner) else {
        return false;
    };
    let documentation = module.dir.strings.get(documentation.markdown);

    documentation.lines().any(|line| line.trim() == "# Safety")
}

/// Return whether one checked unsafe decorator carries a nonempty reason.
fn has_reason(
    module: &DirModule<'_>,
    application: &dir::DecoratorApplication,
) -> Result<bool, ProviderError> {
    let value = module.dir.get_static(application.value)?;
    let (_, value) = value.as_newtype().ok_or_else(|| {
        ProviderError::internal("canonical unsafe decorator value is not a newtype")
    })?;
    let elements = value.as_tuple().ok_or_else(|| {
        ProviderError::internal("canonical unsafe decorator backing value is not a tuple")
    })?;
    let has_reason = match elements {
        [] => false,
        [reason] => {
            let reason = reason.as_string().ok_or_else(|| {
                ProviderError::internal("canonical unsafe decorator reason is not a string")
            })?;

            !module.dir.strings.get(reason).trim().is_empty()
        }
        _ => {
            return Err(ProviderError::internal(
                "canonical unsafe decorator value has multiple arguments",
            ));
        }
    };

    Ok(has_reason)
}

/// Return whether a safety comment is attached immediately before one decorator.
fn has_safety_comment(module: &DirModule<'_>, decorator: Span) -> Result<bool, ProviderError> {
    let file = module.file(decorator.file)?;
    let parsed = module.parsed.file(decorator.file).ok_or_else(|| {
        ProviderError::internal(format!(
            "source file {:?} is absent from parsed lint module {:?}",
            decorator.file, module.id
        ))
    })?;

    // require the parser's exact leading-comment attachment
    for comment in &parsed.comments {
        if comment.following_token_start() != Some(decorator.start) {
            continue;
        }
        let (comment_line, _) = file.get_position(comment.span.end).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment span {:?} has no source position",
                comment.span
            ))
        })?;
        let (decorator_line, _) = file.get_position(decorator.start).ok_or_else(|| {
            ProviderError::internal(format!(
                "decorator span {decorator:?} has no source position"
            ))
        })?;
        if decorator_line != comment_line + 1 {
            continue;
        }
        let content = module.source(comment.content_span())?;
        if content.trim_start().starts_with("SAFETY:") {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an unsafe declaration without caller documentation.
    #[test]
    fn test_reports_unsafe_declaration_without_safety_section() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
/// Read one raw pointer.
@unsafe
export declare function read<T>(pointer: *T): ^T;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[undocumented-unsafe]: unsafe declaration has no `# Safety` section
 ──▶ main.ds:2:1
  │
1 │ /// Read one raw pointer.
2 │ @unsafe
  │ ^^^^^^^
3 │ export declare function read<T>(pointer: *T): ^T;
  │

 = help: document every obligation callers must uphold under `# Safety`
"#,
        );
    }

    /// Accept an unsafe declaration with caller documentation.
    #[test]
    fn test_accepts_unsafe_declaration_with_safety_section() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
/// Read one raw pointer.
///
/// # Safety
///
/// `pointer` must address one live initialized `T`.
@unsafe
export declare function read<T>(pointer: *T): ^T;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a private unsafe declaration without caller documentation.
    #[test]
    fn test_reports_private_unsafe_declaration() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
/// Read one raw pointer.
@unsafe
declare function read<T>(pointer: *T): ^T;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[undocumented-unsafe]: unsafe declaration has no `# Safety` section
 ──▶ main.ds:2:1
  │
1 │ /// Read one raw pointer.
2 │ @unsafe
  │ ^^^^^^^
3 │ declare function read<T>(pointer: *T): ^T;
  │

 = help: document every obligation callers must uphold under `# Safety`
"#,
        );
    }

    /// Report a local unsafe block without an associated rationale.
    #[test]
    fn test_reports_unsafe_block_without_rationale() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
function run(): void {
    @unsafe
    {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[undocumented-unsafe]: unsafe region has no safety rationale
 ──▶ main.ds:2:5
  │
1 │ function run(): void {
2 │     @unsafe
  │     ^^^^^^^
3 │     {}
4 │ }
  │

 = help: add an immediately preceding `SAFETY:` comment or an `@unsafe` reason
"#,
        );
    }

    /// Accept a local unsafe block with an attached safety comment.
    #[test]
    fn test_accepts_unsafe_block_with_safety_comment() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
function run(): void {
    // SAFETY: no unsafe operation escapes this empty tracer
    @unsafe
    {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Reject a detached safety comment separated by a blank line.
    #[test]
    fn test_reports_unsafe_block_with_detached_safety_comment() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
function run(): void {
    // SAFETY: no unsafe operation escapes this empty tracer

    @unsafe
    {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[undocumented-unsafe]: unsafe region has no safety rationale
 ──▶ main.ds:4:5
  │
2 │     // SAFETY: no unsafe operation escapes this empty tracer
3 │
4 │     @unsafe
  │     ^^^^^^^
5 │     {}
6 │ }
  │

 = help: add an immediately preceding `SAFETY:` comment or an `@unsafe` reason
"#,
        );
    }

    /// Accept a local unsafe block with a checked decorator reason.
    #[test]
    fn test_accepts_unsafe_block_with_decorator_reason() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
function run(): void {
    @unsafe("no unsafe operation escapes this empty tracer")
    {}
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
