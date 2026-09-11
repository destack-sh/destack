use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow draining a collection only to collect the same elements again.
    pub DRAIN_COLLECT {
        id: "drain-collect",
        summary: "Disallow draining a collection only to collect the same elements again",
        explanation: r#"
Draining an entire collection and collecting it into the same representation allocates replacement storage.
Instead, you SHOULD call `take` to move out the existing collection and leave its default value.

`take` transfers the existing allocation to the returned collection, so the emptied source no longer retains its capacity.
"#,
        example: {
            reported: r#"
function removeAll(values: &exclusive int32[]): int32[] {
    return values.drain().toArray();
}
"#,
            accepted: r#"
import { take } from "destack:memory";

function removeAll(values: &exclusive int32[]): int32[] {
    return take(values);
}
"#,
        },
        provenance: [Clippy("drain_collect")],
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report complete drains collected back into the same representation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical iterator collection calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(collect) = module.member_call(expression) else {
            continue;
        };
        if collect.is_optional() || !collect.arguments.is_empty() {
            continue;
        }
        let member = module.language_member(expression)?;
        let is_collection = member == Some(dir::LanguageItem::Iterator.member("toArray"))
            || member == Some(dir::LanguageItem::Iterator.member("collect"));
        if !is_collection {
            continue;
        }
        let Some(result) = module.representation_item(expression.into_any())? else {
            continue;
        };

        // require one complete canonical drain of the same collection kind
        let Some(drain) = module.member_call(collect.receiver) else {
            continue;
        };
        if drain.is_optional()
            || !drain.arguments.is_empty()
            || !drain.generic_arguments.is_empty()
            || module.language_member(collect.receiver)? != Some(result.member("drain"))
        {
            continue;
        }
        let receiver_type = module.node_type_id(drain.receiver.into_any())?;
        if module.representation_item(drain.receiver.into_any())? != Some(result) {
            continue;
        }
        if module.place_access(drain.receiver)? != Some(dir::Access::Exclusive) {
            continue;
        }

        // preserve grouping beneath a newly introduced exclusive borrow
        let span = module.source_extent(expression.into_any())?;
        let is_borrowed =
            module.dir.default_ownership(receiver_type)? == Some(dir::Ownership::Borrowed);
        let precedence = if is_borrowed {
            dir::OperatorPrecedence::Lowest
        } else {
            dir::OperatorPrecedence::Prefix
        };
        let receiver = module.expression_source(drain.receiver, precedence)?;
        let borrow = if is_borrowed { "" } else { "&exclusive " };

        // recommend moving the existing allocation from the complete expression
        let diagnostic = lint
            .diagnostic(
                "complete drain is collected into the same collection type",
                span,
            )
            .help(format!("use `take({borrow}{receiver})`"));
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an Array drain collected into another Array.
    #[test]
    fn test_reports_array_drain_collect() {
        let session = TestSession::dir(
            &DRAIN_COLLECT,
            r#"
function removeAll(values: &exclusive int32[]): int32[] {
    return values.drain().toArray();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[drain-collect]: complete drain is collected into the same collection type
 ──▶ main.ds:2:12
  │
1 │ function removeAll(values: &exclusive int32[]): int32[] {
2 │     return values.drain().toArray();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = help: use `take(values)`
"#,
        );
    }

    /// Report an explicitly typed collection of a complete drain.
    #[test]
    fn test_reports_generic_array_drain_collect() {
        let session = TestSession::dir(
            &DRAIN_COLLECT,
            r#"
function removeAll(values: &exclusive int32[]): int32[] {
    return values.drain().collect<^int32[]>();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[drain-collect]: complete drain is collected into the same collection type
 ──▶ main.ds:2:12
  │
1 │ function removeAll(values: &exclusive int32[]): int32[] {
2 │     return values.drain().collect<^int32[]>();
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = help: use `take(values)`
"#,
        );
    }

    /// Accept a drain that does not hold exclusive access to its collection.
    #[test]
    fn test_accepts_shared_drain_collect() {
        let session = TestSession::dir(
            &DRAIN_COLLECT,
            r#"
import { ConcurrentSet } from "destack:collections";

function removeAll(values: &ConcurrentSet<int32>): ^ConcurrentSet<int32> {
    return values.drain().collect();
}

function removeAliased(values: &int32[]): int32[] {
    return values.drain().toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a drained iterator transformed before collection.
    #[test]
    fn test_accepts_transformed_drain() {
        let session = TestSession::dir(
            &DRAIN_COLLECT,
            r#"
function removeAll(values: &exclusive int32[]): int64[] {
    return values.drain().map((value) => value as int64).toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept collection into a different representation.
    #[test]
    fn test_accepts_different_collection() {
        let session = TestSession::dir(
            &DRAIN_COLLECT,
            r#"
import { Set } from "destack:collections";

function removeAll(values: &exclusive int32[]): Set<int32> {
    return values.drain().collect();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined drain and collection methods.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &DRAIN_COLLECT,
            r#"
class Drain {
    toArray(): int32[] {
        return [];
    }
}

class Values {
    drain(): Drain {
        return new Drain();
    }
}

function removeAll(values: Values): int32[] {
    return values.drain().toArray();
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
