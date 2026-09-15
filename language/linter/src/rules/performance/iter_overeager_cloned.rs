use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const DISCARDING_ADAPTER_METHODS: &[&str] = &["take", "takeWhile", "drop", "dropWhile", "filter"];
const PREDICATE_SELECTOR_METHODS: &[&str] = &["find"];
const ARGUMENTLESS_SELECTOR_METHODS: &[&str] = &["first", "last"];

declare_lint! {
    /// Delay iterator cloning until after operations that discard elements.
    pub ITER_OVEREAGER_CLONED {
        id: "iter-overeager-cloned",
        summary: "Delay iterator cloning until after operations that discard elements",
        explanation: r#"
Cloning iterator values before filtering or selection may clone values that the operation discards.
Instead, you SHOULD apply `cloned` after filtering or limiting adapters and clone only values returned by terminal selectors.
"#,
        example: {
            reported: r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function prefix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
    return values.cloned().take(count);
}
"#,
            accepted: r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function prefix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
    return values.take(count).cloned();
}
"#,
        },
        provenance: [Clippy("iter_overeager_cloned")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One iterator operation after which cloning can be delayed.
#[derive(Debug, Clone, Copy)]
enum DiscardingOperation {
    /// One lazy adapter with a required argument.
    Adapter {
        /// The canonical method name.
        method: &'static str,
        /// The adapter argument.
        argument: dir::LocalNodeId<dir::Expression>,
    },
    /// One terminal selector with an optional predicate.
    Selector {
        /// The canonical method name.
        method: &'static str,
        /// The optional selector predicate.
        predicate: Option<dir::LocalNodeId<dir::Expression>>,
    },
}

impl DiscardingOperation {
    /// Select one supported canonical iterator operation.
    fn select(
        module: &DirModule<'_>,
        member: dir::LanguageMember,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<Self> {
        // select lazy adapters with one positional argument
        if let Some(method) = Self::select_method(member, DISCARDING_ADAPTER_METHODS) {
            let argument = Self::positional_argument(module, arguments)?;

            Some(Self::Adapter { method, argument })
        }
        // select terminal operations with one predicate
        else if let Some(method) = Self::select_method(member, PREDICATE_SELECTOR_METHODS) {
            let predicate = Self::positional_argument(module, arguments)?;

            Some(Self::Selector {
                method,
                predicate: Some(predicate),
            })
        }
        // select terminal operations without arguments
        else if let Some(method) = Self::select_method(member, ARGUMENTLESS_SELECTOR_METHODS)
            && arguments.is_empty()
        {
            Some(Self::Selector {
                method,
                predicate: None,
            })
        }
        // reject every other iterator operation
        else {
            None
        }
    }

    /// Return this operation's retained argument, if any.
    fn argument(self) -> Option<dir::LocalNodeId<dir::Expression>> {
        match self {
            Self::Adapter { argument, .. } => Some(argument),
            Self::Selector { predicate, .. } => predicate,
        }
    }

    /// Return the predicate whose element access changes, if any.
    fn predicate(self) -> Option<dir::LocalNodeId<dir::Expression>> {
        match self {
            Self::Adapter {
                method: "filter" | "takeWhile" | "dropWhile",
                argument,
            } => Some(argument),
            Self::Selector { predicate, .. } => predicate,
            _ => None,
        }
    }

    /// Return the selected method name from one candidate set.
    fn select_method(
        member: dir::LanguageMember,
        methods: &'static [&'static str],
    ) -> Option<&'static str> {
        methods
            .iter()
            .copied()
            .find(|method| member == dir::LanguageItem::Iterator.member(method))
    }

    /// Return the only positional call argument.
    fn positional_argument(
        module: &DirModule<'_>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let [argument] = arguments else {
            return None;
        };
        let dir::Argument::Positional { value } = module.view().get(*argument) else {
            return None;
        };

        Some(*value)
    }
}

/// Report iterator cloning before operations that can discard elements.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical iterator operations that can discard values
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(operation) = module.member_call(expression) else {
            continue;
        };
        let Some(member) = module.language_member(expression)? else {
            continue;
        };
        if operation.is_optional() || !operation.generic_arguments.is_empty() {
            continue;
        }
        let Some(discarding) = DiscardingOperation::select(module, member, operation.arguments)
        else {
            continue;
        };
        if let Some(predicate) = discarding.predicate()
            && !predicate_accepts_readonly_borrow(module, predicate)?
        {
            continue;
        }

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

        // leave Copy elements to cloned-instead-of-copied
        let Some(element) = module.cloned_element(operation.receiver)? else {
            continue;
        };
        if module.satisfies_copy(element)? {
            continue;
        }

        // move cloning after the discarding adapter
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic(
            "iterator cloning precedes an operation that can discard values",
            span,
        );
        if let Some(suggestion) = suggestion(module, lint, expression, cloned.receiver, discarding)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one inferred predicate remains valid over a readonly borrow.
fn predicate_accepts_readonly_borrow(
    module: &DirModule<'_>,
    predicate: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let Some(lambda) = module.lambda(predicate) else {
        return Ok(false);
    };
    let Some(parameter) = lambda.signature.parameters.first() else {
        return Ok(true);
    };
    if !matches!(
        module.view().get(*parameter),
        dir::Parameter::Named {
            declared_type: None,
            default: None,
            is_optional: false,
            ..
        }
    ) {
        return Ok(false);
    }
    let Some(body) = lambda.body else {
        return Ok(false);
    };

    // require every element use to accept another readonly borrow
    let symbol = module.declaration_symbol(*parameter)?;

    module.binding_accepts_readonly_borrow(symbol, body.into_any(), None)
}

/// Move one cloned call after an operation that can discard elements.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    iterator: dir::LocalNodeId<dir::Expression>,
    operation: DiscardingOperation,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let iterator_extent = module.source_extent(iterator.into_any())?;
    let argument_extent = operation
        .argument()
        .map(|argument| module.source_extent(argument.into_any()))
        .transpose()?;
    if !extent.contains_span(iterator_extent)
        || argument_extent.is_some_and(|argument| !extent.contains_span(argument))
    {
        return Err(ProviderError::internal(
            "iterator adapter extent does not contain its retained expressions",
        ));
    }
    let mut retained = vec![iterator_extent];
    retained.extend(argument_extent);
    if module.has_unretained_comment(extent, &retained)? {
        return Ok(None);
    }

    // rebuild the same chain with cloning after the discarding operation
    let iterator = module.expression_source(iterator, dir::OperatorPrecedence::Postfix)?;
    let replacement = match operation {
        DiscardingOperation::Adapter { method, argument } => {
            let argument = module.expression_source(argument, dir::OperatorPrecedence::Lowest)?;

            format!("{iterator}.{method}({argument}).cloned()")
        }
        DiscardingOperation::Selector { method, predicate } => {
            let argument = predicate
                .map(|predicate| {
                    module.expression_source(predicate, dir::OperatorPrecedence::Lowest)
                })
                .transpose()?
                .unwrap_or_default();

            format!("{iterator}.{method}({argument})?.clone()")
        }
    };
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("clone only retained iterator values", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Move cloning after a take adapter.
    #[test]
    fn test_moves_cloned_after_take() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function prefix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
    return values.cloned().take(count);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[iter-overeager-cloned]: iterator cloning precedes an operation that can discard values
 ──▶ main.ds:8:12
  │
6 │
7 │ function prefix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
8 │     return values.cloned().take(count);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^
9 │ }
  │

 = suggestion: clone only retained iterator values (requires review)
--- a/main.ds
+++ b/main.ds

    7│ function prefix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
-   8│     return values.cloned().take(count);
+   8│     return values.take(count).cloned();
    9│ }
"#,
        );
        session.assert_suggestions(
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function prefix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
    return values.take(count).cloned();
}
"#,
        );
    }

    /// Move cloning after a drop adapter.
    #[test]
    fn test_moves_cloned_after_drop() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function suffix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
    return values.cloned().drop(count);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function suffix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
    return values.drop(count).cloned();
}
"#,
        );
    }

    /// Accept cloning after the adapter.
    #[test]
    fn test_accepts_lazy_cloning() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function prefix(values: Iterator<&immutable Label>, count: isize): Iterator<Label> {
    return values.take(count).cloned();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Move cloning after predicate adapters.
    #[test]
    fn test_moves_cloned_after_predicate_adapters() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function nonempty(values: Iterator<&immutable Label>): Iterator<Label> {
    return values.cloned().filter((value) => value.values.length > 0);
}

function prefix(values: Iterator<&immutable Label>): Iterator<Label> {
    return values.cloned().takeWhile((value) => value.values.length > 0);
}

function suffix(values: Iterator<&immutable Label>): Iterator<Label> {
    return values.cloned().dropWhile((value) => value.values.length === 0);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function nonempty(values: Iterator<&immutable Label>): Iterator<Label> {
    return values.filter((value) => value.values.length > 0).cloned();
}

function prefix(values: Iterator<&immutable Label>): Iterator<Label> {
    return values.takeWhile((value) => value.values.length > 0).cloned();
}

function suffix(values: Iterator<&immutable Label>): Iterator<Label> {
    return values.dropWhile((value) => value.values.length === 0).cloned();
}
"#,
        );
    }

    /// Clone only values returned by terminal selection operations.
    #[test]
    fn test_moves_cloned_after_terminal_operations() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function first(values: Iterator<&immutable Label>): Label | undefined {
    return values.cloned().first();
}

function last(values: Iterator<&immutable Label>): Label | undefined {
    return values.cloned().last();
}

function find(values: Iterator<&immutable Label>): Label | undefined {
    return values.cloned().find((value) => value.values.length > 0);
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function first(values: Iterator<&immutable Label>): Label | undefined {
    return values.first()?.clone();
}

function last(values: Iterator<&immutable Label>): Label | undefined {
    return values.last()?.clone();
}

function find(values: Iterator<&immutable Label>): Label | undefined {
    return values.find((value) => value.values.length > 0)?.clone();
}
"#,
        );
    }

    /// Leave Copy elements to cloned-instead-of-copied.
    #[test]
    fn test_accepts_copy_elements() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
import { Iterator } from "destack:iter";

function prefix(values: Iterator<&immutable int32>): Iterator<int32> {
    return values.cloned().take(2);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined methods with the same names.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
class Values {
    cloned(): this {
        return this;
    }

    take(count: isize): this {
        return this;
    }
}

function prefix(values: Values): Values {
    return values.cloned().take(2);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain comments without offering a destructive suggestion.
    #[test]
    fn test_retains_comment() {
        let session = TestSession::dir(
            &ITER_OVEREAGER_CLONED,
            r#"
import { Iterator } from "destack:iter";

struct Label {
    values: ^int32[];
}

function prefix(values: Iterator<&immutable Label>): Iterator<Label> {
    return values.cloned(/* retain */).take(2);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[iter-overeager-cloned]: iterator cloning precedes an operation that can discard values
 ──▶ main.ds:8:12
  │
6 │
7 │ function prefix(values: Iterator<&immutable Label>): Iterator<Label> {
8 │     return values.cloned(/* retain */).take(2);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
9 │ }
  │
"#,
        );
    }
}
