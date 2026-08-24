use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer copying iterator elements whose type is Copy.
    pub CLONED_INSTEAD_OF_COPIED {
        id: "cloned-instead-of-copied",
        summary: "Prefer copying iterator elements whose type is Copy",
        explanation: r#"
Calling `cloned` on Copy iterator elements permits custom cloning work when intrinsic duplication is sufficient.
Instead, you SHOULD call `copied` to state the Copy requirement directly.
"#,
        example: {
            reported: r#"
import { Iterator } from "destack:iter";

function copy(values: Iterator<&readonly int32>): int32[] {
    return values.cloned().toArray();
}
"#,
            accepted: r#"
import { Iterator } from "destack:iter";

function copy(values: Iterator<&readonly int32>): int32[] {
    return values.copied().toArray();
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical cloned adapters whose borrowed element type satisfies Copy.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical zero-argument Iterator.cloned calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if !call.generic_arguments.is_empty()
            || !call.arguments.is_empty()
            || module.language_member(expression)?
                != Some(dir::LanguageItem::Iterator.member("cloned"))
        {
            continue;
        }

        // require the inferred element parameter to satisfy Copy
        let Some(bindings) = module.call_generic_bindings(expression)? else {
            continue;
        };
        let Some(element) = bindings.first() else {
            return Err(ProviderError::internal(
                "checked Iterator.cloned call has no element type argument",
            ));
        };
        if !module.satisfies_copy(expression.into_any(), element.argument)? {
            continue;
        }

        // replace only the selected adapter name
        let span = module.source_extent(expression.into_any())?;
        let name = module.main_span(call.callee.into_any())?;
        let patch = Patch::replace(name, "copied");
        let suggestion = lint.fix("copy the iterator elements", patch)?;
        let diagnostic = lint
            .diagnostic("Copy iterator element is cloned", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace cloned for a directly declared borrowed iterator.
    #[test]
    fn test_replaces_direct_iterator_clone() {
        let session = TestSession::dir(
            &CLONED_INSTEAD_OF_COPIED,
            r#"
import { Iterator } from "destack:iter";

declare function values(): Iterator<&readonly int32>;

function copy(): int32[] {
    return values().cloned().toArray();
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "destack:iter";

declare function values(): Iterator<&readonly int32>;

function copy(): int32[] {
    return values().copied().toArray();
}
"#,
        );
    }

    /// Replace cloned when the iterator element has a generic Copy bound.
    #[test]
    fn test_replaces_generic_copy_elements() {
        let session = TestSession::dir(
            &CLONED_INSTEAD_OF_COPIED,
            r#"
import { Iterator } from "destack:iter";

function copy<T: Copy>(values: Iterator<&readonly T>): T[] {
    return values.cloned().toArray();
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "destack:iter";

function copy<T: Copy>(values: Iterator<&readonly T>): T[] {
    return values.copied().toArray();
}
"#,
        );
    }

    /// Accept cloned for iterator elements that require cloning work.
    #[test]
    fn test_accepts_noncopy_elements() {
        let session = TestSession::dir(
            &CLONED_INSTEAD_OF_COPIED,
            r#"
struct Label {
    values: ^int32[];
}

function copy(values: &readonly Label[]): Label[] {
    return values.iterator().cloned().toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve an optional Iterator receiver while selecting copied.
    #[test]
    fn test_replaces_optional_iterator_clone() {
        let session = TestSession::dir(
            &CLONED_INSTEAD_OF_COPIED,
            r#"
import { Iterator } from "destack:iter";

function copy(values: Iterator<&readonly int32> | undefined): int32[] | undefined {
    return values?.cloned().toArray();
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "destack:iter";

function copy(values: Iterator<&readonly int32> | undefined): int32[] | undefined {
    return values?.copied().toArray();
}
"#,
        );
    }
}
