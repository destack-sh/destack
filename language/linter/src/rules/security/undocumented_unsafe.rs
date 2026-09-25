use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require a safety rationale for every unsafe declaration and expression.
    pub UNDOCUMENTED_UNSAFE {
        id: "undocumented-unsafe",
        summary: "Require a safety rationale for every unsafe declaration and expression",
        explanation: r#"
Unsafe callables impose obligations on their callers, while unsafe implementations and local regions rely on invariants the checker cannot verify.
Instead, you SHOULD document caller obligations under `# Safety` and justify implementations and local regions with an immediately preceding `SAFETY:` comment or a nonempty `@unsafe` reason.
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
        provenance: [
            Clippy("missing_safety_doc"),
            Clippy("undocumented_unsafe_blocks"),
        ],
        category: Security,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report canonical unsafe applications without the required authored rationale.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect applications of the canonical unsafe decorator
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

        // require unsafe callables to document their caller obligations
        if module.callable_parameters(owner).is_some() {
            if !has_safety_section(module, owner) {
                let diagnostic = lint
                    .diagnostic("unsafe declaration has no `# Safety` section", span)
                    .help("document every obligation callers must uphold under `# Safety`");
                output.report(diagnostic);
            }
            continue;
        }

        // require unsafe implementations and local regions to carry a rationale
        if has_reason(module, application)? || has_safety_comment(module, owner)? {
            continue;
        }
        let noun = match owner.ty {
            dir::NodeType::Declaration
            | dir::NodeType::Member
            | dir::NodeType::Property
            | dir::NodeType::TypeMember => "unsafe implementation",
            _ => "unsafe region",
        };
        let diagnostic = lint
            .diagnostic(format!("{noun} has no safety rationale"), span)
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

/// Return whether one unsafe decorator carries a nonempty reason.
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

/// Return whether a safety comment is attached immediately before one decorator list.
fn has_safety_comment(
    module: &DirModule<'_>,
    owner: dir::LocalNodeIdAny,
) -> Result<bool, ProviderError> {
    let first = module
        .view()
        .get_decorators_any(owner)
        .into_iter()
        .next()
        .ok_or_else(|| {
            ProviderError::internal(format!(
                "unsafe decorator owner {owner:?} has no authored decorators"
            ))
        })?;
    let first = module.source_extent(first.into_any())?;
    let file = module.file(first.file)?;
    let parsed = module.stages.parsed.file(first.file).ok_or_else(|| {
        ProviderError::internal(format!(
            "source file {:?} is absent from parsed lint module {:?}",
            first.file, module.id
        ))
    })?;

    let (decorator_line, _) = file.get_position(first.start).ok_or_else(|| {
        ProviderError::internal(format!("decorator span {first:?} has no source position"))
    })?;
    let mut following_line = decorator_line;

    // inspect the contiguous leading comment group from bottom to top
    for comment in parsed.comments.iter().rev() {
        if comment.following_token_start() != Some(first.start) {
            continue;
        }
        let (start_line, _) = file.get_position(comment.span.start).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment span {:?} has no source position",
                comment.span
            ))
        })?;
        let (end_line, _) = file.get_position(comment.span.end).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment span {:?} has no source position",
                comment.span
            ))
        })?;
        if end_line + 1 != following_line {
            break;
        }
        if comment
            .text(file.text())
            .trim_start()
            .starts_with("SAFETY:")
        {
            return Ok(true);
        }
        following_line = start_line;
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
 ──▶ main.tspp:2:1
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
 ──▶ main.tspp:2:1
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
 ──▶ main.tspp:2:5
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

    /// Accept a local unsafe block with a multiline safety comment.
    #[test]
    fn test_accepts_unsafe_block_with_multiline_safety_comment() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
function run(): void {
    // SAFETY:
    // no unsafe operation escapes this empty tracer
    @unsafe
    {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a safety comment above the complete decorator list.
    #[test]
    fn test_accepts_safety_comment_above_decorators() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
function run(): void {
    // SAFETY: no unsafe operation escapes this empty tracer
    @cold
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
 ──▶ main.tspp:4:5
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

    /// Accept a local unsafe block with a decorator reason.
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

    /// Report an unsafe implementation without a rationale.
    #[test]
    fn test_reports_unsafe_implementation_without_rationale() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
import { Unpin } from "tspp:memory";

newtype Handle = uint32;

@unsafe
export extension of Handle implements Unpin {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[undocumented-unsafe]: unsafe implementation has no safety rationale
 ──▶ main.tspp:5:1
  │
3 │ newtype Handle = uint32;
4 │
5 │ @unsafe
  │ ^^^^^^^
6 │ export extension of Handle implements Unpin {}
  │

 = help: add an immediately preceding `SAFETY:` comment or an `@unsafe` reason
"#,
        );
    }

    /// Accept an unsafe implementation with an attached rationale.
    #[test]
    fn test_accepts_unsafe_implementation_with_rationale() {
        let session = TestSession::dir(
            &UNDOCUMENTED_UNSAFE,
            r#"
import { Unpin } from "tspp:memory";

newtype Handle = uint32;

// SAFETY: handle stores no self-references
@unsafe
export extension of Handle implements Unpin {}
"#,
        );

        session.assert_no_diagnostics();
    }
}
