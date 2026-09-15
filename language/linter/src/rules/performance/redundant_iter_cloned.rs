use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const BORROWING_CALLBACK_METHODS: &[&str] = &[
    "map",
    "filterMap",
    "flatMap",
    "mapWhile",
    "findMap",
    "forEach",
    "some",
    "every",
    "findIndex",
];

declare_lint! {
    /// Disallow cloning iterator elements that are only borrowed afterward.
    pub REDUNDANT_ITER_CLONED {
        id: "redundant-iter-cloned",
        summary: "Disallow cloning iterator elements that are only borrowed afterward",
        explanation: r#"
Cloning iterator elements is unnecessary when the following operation only observes them through readonly borrows.
Instead, you SHOULD operate on the borrowed elements directly.
"#,
        example: {
            reported: r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function lengths(values: Iterator<&immutable Label>): Iterator<isize> {
    return values.cloned().map((value) => value.values.length);
}
"#,
            accepted: r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function lengths(values: Iterator<&immutable Label>): Iterator<isize> {
    return values.map((value) => value.values.length);
}
"#,
        },
        provenance: [Clippy("redundant_iter_cloned")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report cloned adapters followed only by borrowing or nonobserving operations.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical iterator operations whose receiver is cloned
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(operation) = module.member_call(expression) else {
            continue;
        };
        if operation.is_optional() || !operation.generic_arguments.is_empty() {
            continue;
        }
        let Some(member) = module.language_member(expression)? else {
            continue;
        };

        // require one canonical zero-argument cloned receiver
        let Some(cloned) = module.member_call(operation.receiver) else {
            continue;
        };
        if cloned.is_optional()
            || !cloned.generic_arguments.is_empty()
            || !cloned.arguments.is_empty()
            || module.language_member(operation.receiver)?
                != Some(dir::LanguageItem::Iterator.member("cloned"))
        {
            continue;
        }

        // prove the operation cannot consume or return the cloned values
        let has_borrowing_callback = member.owner == dir::LanguageItem::Iterator
            && BORROWING_CALLBACK_METHODS
                .iter()
                .any(|name| member == dir::LanguageItem::Iterator.member(name));
        let is_redundant = match member {
            member if member == dir::LanguageItem::Iterator.member("count") => {
                operation.arguments.is_empty()
            }
            _ if has_borrowing_callback => callback_borrows_element(module, operation.arguments)?,
            _ => false,
        };
        if !is_redundant {
            continue;
        }

        // remove cloning while preserving the complete iterator operation
        let span = module.source_extent(operation.receiver.into_any())?;
        let mut diagnostic = lint.diagnostic("iterator elements are cloned before borrowing", span);
        if let Some(suggestion) = suggestion(module, lint, operation.receiver, cloned.receiver)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one callback only observes its element through readonly borrows.
fn callback_borrows_element(
    module: &DirModule<'_>,
    arguments: &[dir::LocalNodeId<dir::Argument>],
) -> Result<bool, ProviderError> {
    // select one direct lambda argument with one inferred parameter
    let [argument] = arguments else {
        return Ok(false);
    };
    let dir::Argument::Positional { value: callback } = module.view().get(*argument) else {
        return Ok(false);
    };
    let Some(lambda) = module.lambda(*callback) else {
        return Ok(false);
    };
    let Some(parameter) = lambda.signature.parameters.first() else {
        return Ok(true);
    };
    let dir::Parameter::Named {
        declared_type: None,
        default: None,
        is_optional: false,
        ..
    } = module.view().get(*parameter)
    else {
        return Ok(false);
    };
    let Some(body) = lambda.body else {
        return Ok(false);
    };

    // require every element use to accept a readonly borrow
    let symbol = module.declaration_symbol(*parameter)?;

    module.binding_accepts_readonly_borrow(symbol, body.into_any(), None)
}

/// Remove one cloned adapter suffix.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    cloned: dir::LocalNodeId<dir::Expression>,
    iterator: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(cloned.into_any())?;
    let retained = module.source_extent(iterator.into_any())?;
    if !extent.contains_span(retained) {
        return Err(ProviderError::internal(
            "cloned iterator extent does not contain its receiver",
        ));
    }
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // delete only the cloned call suffix
    let suffix = Span::new(extent.file, retained.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    file.delete(suffix);
    let suggestion = lint.fix("use the borrowed iterator elements directly", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove cloning before counting values without observing them.
    #[test]
    fn test_removes_cloned_before_count() {
        let session = TestSession::dir(
            &REDUNDANT_ITER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function count(values: Iterator<&immutable Label>): isize {
    return values.cloned().count();
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function count(values: Iterator<&immutable Label>): isize {
    return values.count();
}
"#,
        );
    }

    /// Remove cloning when inferred callback uses only observe the value.
    #[test]
    fn test_removes_cloned_before_readonly_uses() {
        let session = TestSession::dir(
            &REDUNDANT_ITER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function visit(values: Iterator<&immutable Label>): void {
    values.cloned().forEach((value) => {
        value.values.length;
    });
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function visit(values: Iterator<&immutable Label>): void {
    values.forEach((value) => {
        value.values.length;
    });
}
"#,
        );
    }

    /// Remove cloning before findMap when its callback only observes each value.
    #[test]
    fn test_removes_cloned_before_find_map() {
        let session = TestSession::dir(
            &REDUNDANT_ITER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function firstLength(values: Iterator<&immutable Label>): isize | undefined {
    return values.cloned().findMap((value) => value.values.length);
}
"#,
        );

        session.assert_fixes(
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function firstLength(values: Iterator<&immutable Label>): isize | undefined {
    return values.findMap((value) => value.values.length);
}
"#,
        );
    }

    /// Preserve cloning when collected values must remain owned.
    #[test]
    fn test_accepts_owned_collection() {
        let session = TestSession::dir(
            &REDUNDANT_ITER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function collect(values: Iterator<&immutable Label>): Label[] {
    return values.cloned().toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve cloning before a callback that accepts owned values.
    #[test]
    fn test_accepts_owned_callback() {
        let session = TestSession::dir(
            &REDUNDANT_ITER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function consume(values: Iterator<&immutable Label>): void {
    values.cloned().forEach((value: Label) => {
        value.values.length;
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
