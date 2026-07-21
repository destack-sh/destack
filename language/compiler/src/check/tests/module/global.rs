use crate::tests::{DirRows, TestSession};

#[test]
fn test_global_binding_resolves_without_import() {
    let session = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "compiler": {
        "globals": ["globals.ds"],
        "emitCheckedTypes": true
    }
}

"#,
        )
        .module(
            "globals.ds",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .module(
            "main.ds",
            r#"
const value = answer;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["globals.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== globals.ds ===

=== annotated ===
global {
    const answer: int32 = 42;
}

=== checked ===
global {
    const answer: int32 = 42;
    /// @type.symbol symbol=answer source=answer type=int32
    /// @resolution.pattern source=answer kind=binding target=answer
    /// @type.node source=42 type=42

}

=== main.ds ===

=== annotated ===
const value: int32 = answer;

=== checked ===
const value = answer;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=answer type=int32
/// @resolution.name source=answer target=globals.answer
"#,
    );
}

#[test]
fn test_reject_cyclic_inferred_bindings() {
    let session = TestSession::builder()
        .module(
            "first.ds",
            r#"
import { second } from "./second.ds";

export const first = second;
"#,
        )
        .module(
            "second.ds",
            r#"
import { first } from "./first.ds";

export const second = first;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "first.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { second } from "./second.ds";

export const first = second;

=== checked ===
import { second } from "./second.ds";

export const first = second;
/// @type.symbol symbol=first source=first type=<error>
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=second type=<error>
/// @resolution.name source=second target=second.second
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=4 column=14 span="first" line_source="export const first = second;"
/// @diagnostic.related file="second.ds" line=4 column=14 span="second" line_source="export const second = first;" message="it must equal '_' here"
/// @diagnostic.help message="annotate the type explicitly"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=4 column=14 span="second" line_source="export const second = first;"
/// @diagnostic.related file="first.ds" line=4 column=14 span="first" line_source="export const first = second;" message="it must equal '_' here"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}
