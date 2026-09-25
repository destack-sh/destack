use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow unused Array.map and Iterator.map results.
    pub NO_UNUSED_MAP_RESULT {
        id: "no-unused-map-result",
        summary: "Disallow unused Array.map and Iterator.map results",
        explanation: r#"
Discarding an Array map wastes its allocated result, while discarding an Iterator map leaves its lazy callback unconsumed.
Instead, you SHOULD call `forEach` when the callback work is intended or remove the unused map operation.
"#,
        example: {
            reported: r#"
function append(values: int32[], output: int32[]): void {
    values.map((value) => output.push(value));
}
"#,
            accepted: r#"
function append(values: int32[], output: int32[]): void {
    values.forEach((value) => output.push(value));
}
"#,
        },
        provenance: [SonarJs("no-ignored-return")],
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report discarded Array and Iterator map calls.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect mapped values discarded by their containing block
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional() || !module.is_discarded_expression(expression) {
            continue;
        }

        // distinguish eager allocation from lazy construction
        let is_eager = match module.language_member(expression)? {
            Some(member) if member == dir::LanguageItem::Array.member("map") => true,
            Some(member) if member == dir::LanguageItem::Iterator.member("map") => false,
            _ => continue,
        };

        // report every discarded map and rewrite only eager evaluation
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("mapped result is discarded", span);
        if is_eager {
            if let Some(suggestion) = suggestion(module, lint, call.callee)? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
        } else {
            diagnostic = diagnostic.help("consume the Iterator or remove the unused map");
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the corresponding forEach call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    callee: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let name = module.main_span(callee.into_any())?;
    if module.has_unretained_comment(name, &[])? {
        return Ok(None);
    }

    // replace only the selected member name
    let patch = Patch::replace(name, "forEach");
    let suggestion = lint.suggestion("consume the callback with `forEach`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a discarded Array map call.
    #[test]
    fn test_replaces_discarded_array_map() {
        TestSession::assert_example(&NO_UNUSED_MAP_RESULT);
    }

    /// Report a discarded lazy Iterator map without changing its evaluation.
    #[test]
    fn test_reports_discarded_iterator_map() {
        let session = TestSession::dir(
            &NO_UNUSED_MAP_RESULT,
            r#"
import { Iterator } from "tspp:iter";

function append(values: Iterator<int32>, output: int32[]): void {
    values.map((value) => output.push(value));
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[no-unused-map-result]: mapped result is discarded
 ──▶ main.tspp:4:5
  │
2 │
3 │ function append(values: Iterator<int32>, output: int32[]): void {
4 │     values.map((value) => output.push(value));
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │ }
  │

 = help: consume the Iterator or remove the unused map
"#,
        );
    }

    /// Accept a returned mapped Array.
    #[test]
    fn test_accepts_returned_array_map() {
        let session = TestSession::dir(
            &NO_UNUSED_MAP_RESULT,
            r#"
function double(values: int32[]): int32[] {
    return values.map((value) => value * 2);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a returned lazy Iterator map.
    #[test]
    fn test_accepts_returned_iterator_map() {
        let session = TestSession::dir(
            &NO_UNUSED_MAP_RESULT,
            r#"
import { Iterator } from "tspp:iter";

function double(values: Iterator<int32>): Iterator<int32> {
    return values.map((value) => value * 2);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
