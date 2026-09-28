use crate::rules::declare_lint;
use crate::{Lint, LintOutput, LintResult, MirModule};
use tspp_dir as dir;
use tspp_mir as mir;

declare_lint! {
    /// Disallow dropping values without destructors.
    pub NO_DROP_OF_TRIVIAL_VALUE {
        id: "no-drop-of-trivial-value",
        summary: "Disallow dropping values without destructors",
        explanation: r#"
Calling `drop` for a value that requires no destruction performs no cleanup.
Instead, you SHOULD remove the call or use a narrower scope when ownership must end sooner.
"#,
        example: {
            reported: r#"
import { drop } from "tspp:memory";

function discard(value: int32): void {
    drop(value);
}
"#,
            accepted: r#"
import { Drop, drop } from "tspp:memory";

struct Subscription implements Drop {
    drop(&this): void {}
}

function unsubscribe(value: Subscription): void {
    drop(value);
}
"#,
        },
        provenance: [Clippy("drop_non_drop")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Report explicit drops of values that require no destruction.
fn check(module: &mut MirModule<'_>, lint: &Lint) -> LintResult {
    let tree = &module.lowered.tree;
    let mut output = LintOutput::default();

    // inspect calls to the drop function in every defined function
    for (_, function) in tree.iter_nodes::<mir::Function>() {
        let Some(body) = function.body() else {
            continue;
        };

        for block in body.blocks().iter().copied() {
            for instruction in tree.get(block).instructions.iter().copied() {
                let Some(call) =
                    module.language_item_call(instruction, dir::LanguageItem::MemoryDrop)?
                else {
                    continue;
                };
                let Some(value) = tree.get_values(call.arguments).first() else {
                    continue;
                };

                // require an authored source anchor
                if tree.source_span_by_id(instruction.id).is_none() {
                    continue;
                }

                // accept managed handles, releasing one destroys its storage
                let ty = function.expect_value_type(*value);
                if matches!(
                    tree.get(ty),
                    mir::Type::Reference {
                        kind: mir::Reference::Managed(_),
                        ..
                    }
                ) {
                    continue;
                }

                // keep values whose frame storage destructs
                if module
                    .lowered
                    .drops
                    .requires_destructor(ty, mir::Storage::Frame, tree)
                {
                    continue;
                }

                let anchor = module.anchor(instruction.into_any())?;
                let diagnostic = lint.diagnostic("value requires no destruction", anchor);
                output.report(diagnostic);
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an explicit drop of a primitive value.
    #[test]
    fn test_reports_primitive_value() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            NO_DROP_OF_TRIVIAL_VALUE.example.reported.source(),
        );

        session.assert_diagnostics(
            r#"
warning[no-drop-of-trivial-value]: value requires no destruction
 ──▶ main.tspp:4:5
  │
2 │
3 │ function discard(value: int32): void {
4 │     drop(value);
  │     ^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Report an explicit drop of a Copy value.
    #[test]
    fn test_reports_copy_value() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
import { drop } from "tspp:memory";

function discard(value: boolean): void {
    drop(value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-drop-of-trivial-value]: value requires no destruction
 ──▶ main.tspp:4:5
  │
2 │
3 │ function discard(value: boolean): void {
4 │     drop(value);
  │     ^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Report an explicit drop of a borrowed reference.
    #[test]
    fn test_reports_borrowed_reference() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
import { drop } from "tspp:memory";

function discard(value: &readonly int32): void {
    drop(value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-drop-of-trivial-value]: value requires no destruction
 ──▶ main.tspp:4:5
  │
2 │
3 │ function discard(value: &readonly int32): void {
4 │     drop(value);
  │     ^^^^^^^^^^^
5 │ }
  │
"#,
        );
    }

    /// Report an explicit drop of a trivial value type.
    #[test]
    fn test_reports_trivial_struct_drop() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
import { drop } from "tspp:memory";

struct Point {
    x: int32;
    y: int32;
}

function discard(value: Point): void {
    drop(value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-drop-of-trivial-value]: value requires no destruction
  ──▶ main.tspp:9:5
   │
 7 │
 8 │ function discard(value: Point): void {
 9 │     drop(value);
   │     ^^^^^^^^^^^
10 │ }
   │
"#,
        );
    }

    /// Accept an explicit drop of a value with a destructor.
    #[test]
    fn test_accepts_destructor_drop() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            NO_DROP_OF_TRIVIAL_VALUE.example.accepted.source(),
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicit drop of a plain class handle releasing its storage.
    #[test]
    fn test_accepts_plain_class_handle_drop() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
import { drop } from "tspp:memory";

class Plain {
    value: int32 = 0;
}

function release(value: Plain): void {
    drop(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicit drop of a value with an extension-declared destructor.
    #[test]
    fn test_accepts_extension_destructor_drop() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
import { Drop, drop } from "tspp:memory";

struct Subscription {
    handle: int32;
}

extension of Subscription implements Drop {
    drop(&this): void {}
}

function unsubscribe(value: Subscription): void {
    drop(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicit drop of a specialized value with a destructor.
    #[test]
    fn test_accepts_generic_destructor_drop() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
import { Drop, drop } from "tspp:memory";

struct Producer<T> implements Drop {
    value: T;

    drop(&this): void {}
}

function unsubscribe(producer: Producer<int32>): void {
    drop(producer);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicit drop of a class value with a destructor.
    #[test]
    fn test_accepts_class_destructor_drop() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
import { Drop, drop } from "tspp:memory";

class Session implements Drop {
    drop(&this): void {}
}

function close(value: Session): void {
    drop(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept ordinary use of a trivial value.
    #[test]
    fn test_accepts_ordinary_value_use() {
        let session = TestSession::dir(
            &NO_DROP_OF_TRIVIAL_VALUE,
            r#"
function identity(value: int32): int32 {
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
