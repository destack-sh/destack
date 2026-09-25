use tspp_dir as dir;
use tspp_source::Patch;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer clear over discarding a complete drain.
    pub PREFER_CLEAR {
        id: "prefer-clear",
        summary: "Prefer clear over discarding a complete drain",
        explanation: r#"
Discarding a complete drain constructs an Iterator whose removed values are never observed.
Instead, you SHOULD call `clear` to remove the values directly.
"#,
        example: {
            reported: r#"
function reset(values: int32[]): void {
    values.drain();
}
"#,
            accepted: r#"
function reset(values: int32[]): void {
    values.clear();
}
"#,
        },
        provenance: [Clippy("clear_with_drain")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report canonical drain calls whose results are discarded.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect direct expression statements that call drain without arguments
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional() || !call.generic_arguments.is_empty() || !call.arguments.is_empty() {
            continue;
        }
        let Some(member) = module.language_member(expression)? else {
            continue;
        };

        // require a discarded complete drain of a canonical collection
        let is_complete_drain = [
            dir::LanguageItem::Array,
            dir::LanguageItem::BinaryHeap,
            dir::LanguageItem::ConcurrentMap,
            dir::LanguageItem::ConcurrentSet,
            dir::LanguageItem::Deque,
            dir::LanguageItem::LinkedList,
            dir::LanguageItem::Map,
            dir::LanguageItem::Set,
            dir::LanguageItem::Slab,
            dir::LanguageItem::SmallArray,
            dir::LanguageItem::SortedMap,
            dir::LanguageItem::SortedSet,
        ]
        .into_iter()
        .any(|item| member == item.member("drain"));
        if !is_complete_drain {
            continue;
        }
        if !module.is_discarded_expression(expression) {
            continue;
        }

        // replace only the canonical method name
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("drained values are discarded", span);
        let member = module.main_span(call.callee.into_any())?;
        let patch = Patch::replace(member, "clear");
        let fix = lint.fix("clear the collection directly", patch)?;
        diagnostic = diagnostic.suggestion(fix);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a discarded complete Array drain.
    #[test]
    fn test_replaces_discarded_drain() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
function reset(values: int32[]): void {
    values.drain();
}
"#,
        );

        session.assert_fixes(
            r#"
function reset(values: int32[]): void {
    values.clear();
}
"#,
        );
    }

    /// Replace a discarded complete Map drain.
    #[test]
    fn test_replaces_map_drain() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
function reset(values: Map<string, int32>): void {
    values.drain();
}
"#,
        );

        session.assert_fixes(
            r#"
function reset(values: Map<string, int32>): void {
    values.clear();
}
"#,
        );
    }

    /// Replace a discarded complete Set drain.
    #[test]
    fn test_replaces_set_drain() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
function reset(values: Set<int32>): void {
    values.drain();
}
"#,
        );

        session.assert_fixes(
            r#"
function reset(values: Set<int32>): void {
    values.clear();
}
"#,
        );
    }

    /// Accept a drain whose removed values are consumed.
    #[test]
    fn test_accepts_consumed_drain() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
function remove(values: int32[]): Iterator<int32> {
    return values.drain();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a drain used as an expression-block value.
    #[test]
    fn test_accepts_block_value() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
function remove(values: int32[]): Iterator<int32> {
    return do {
        values.drain()
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined drain method.
    #[test]
    fn test_accepts_user_drain() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
class Values {
    drain(): void {
        // intentionally empty
    }
}

function reset(values: Values): void {
    values.drain();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a discarded complete BinaryHeap drain.
    #[test]
    fn test_replaces_binary_heap_drain() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
import { BinaryHeap } from "tspp:collections";

function reset(values: BinaryHeap<int32>): void {
    values.drain();
}
"#,
        );

        session.assert_fixes(
            r#"
import { BinaryHeap } from "tspp:collections";

function reset(values: BinaryHeap<int32>): void {
    values.clear();
}
"#,
        );
    }

    /// Replace a discarded complete ConcurrentMap drain.
    #[test]
    fn test_replaces_concurrent_map_drain() {
        let session = TestSession::dir(
            &PREFER_CLEAR,
            r#"
import { ConcurrentMap } from "tspp:collections";

function reset(values: ConcurrentMap<string, int32>): void {
    values.drain();
}
"#,
        );

        session.assert_fixes(
            r#"
import { ConcurrentMap } from "tspp:collections";

function reset(values: ConcurrentMap<string, int32>): void {
    values.clear();
}
"#,
        );
    }
}
