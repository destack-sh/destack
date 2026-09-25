use crate::tests::{DirRows, TestSession};

#[test]
fn test_global_binding_resolves_without_import() {
    let session = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}

"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
const value = answer;
"#,
        )
        .build();

    session.assert_dir_many(
        &["globals.tspp", "main.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== globals.tspp ===

=== annotated ===
global {
    const answer: int32 = 42;
}

=== dir ===
global {
    const answer: int32 = 42;
    /// @type.symbol symbol=answer source=answer type=int32
    /// @resolution.pattern source=answer kind=binding target=answer
    /// @type.node source=42 type=42

}

=== main.tspp ===

=== annotated ===
const value: int32 = answer;

=== dir ===
const value = answer;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=answer type=int32
/// @resolution.name source=answer target=globals.answer
/// @resolution.access source=answer root=globals.answer
"#,
    );
}

#[test]
fn test_reject_cyclic_inferred_bindings() {
    let session = TestSession::builder()
        .module(
            "first.tspp",
            r#"
import { second } from "./second.tspp";

export const first = second;
"#,
        )
        .module(
            "second.tspp",
            r#"
import { first } from "./first.tspp";

export const second = first;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "first.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { second } from "./second.tspp";

export const first = second;

=== dir ===
import { second } from "./second.tspp";

export const first = second;
/// @type.symbol symbol=first source=first type=<error>
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=second type=<error>
/// @resolution.name source=second target=second.second
/// @resolution.access source=second root=second.second
"#,
        r#"
/// @diagnostic.error id=missing-export-binding-type message="exported binding needs a written type"
/// @diagnostic.label line=4 column=14 span="first" line_source="export const first = second;"
/// @diagnostic.help message="state the type or initialize with a literal"
/// @diagnostic.error id=missing-export-binding-type message="exported binding needs a written type"
/// @diagnostic.label line=4 column=14 span="second" line_source="export const second = first;"
/// @diagnostic.help message="state the type or initialize with a literal"
"#,
    );
}
