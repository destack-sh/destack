use crate::tests::{DirRows, TestSession};

#[test]
fn test_static_true_namespace_export_is_available() {
    let session = TestSession::builder()
        .module(
            "dep.ds",
            r#"
@if(true)
export const value = 1;
"#,
        )
        .module(
            "main.ds",
            r#"
import * as dep from "./dep.ds";

const result = dep.value;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["dep.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== dep.ds ===

=== annotated ===
@if(true)
export const value: 1 = 1;

=== checked ===
@if(true)
export const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

=== main.ds ===

=== annotated ===
import * as dep from "./dep.ds";

const result: 1 = dep.value;

=== checked ===
import * as dep from "./dep.ds";

const result = dep.value;
/// @type.symbol symbol=result source=result type=1
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source=dep.value type=1
/// @resolution.name source=dep.value target=dep.value
/// @resolution.access source=dep.value root=dep.value
"#,
    );
}

#[test]
fn test_static_false_namespace_export_is_unavailable() {
    let session = TestSession::builder()
        .module(
            "dep.ds",
            r#"
@if(false)
export const value = 1;
"#,
        )
        .module(
            "main.ds",
            r#"
import * as dep from "./dep.ds";

dep.value;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import * as dep from "./dep.ds";

dep.value;

=== checked ===
import * as dep from "./dep.ds";

dep.value;
/// @type.node source=dep.value type=<error>
/// @resolution.unresolved source=dep.value path=dep.value
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'dep.value'"
/// @diagnostic.label line=4 column=5 span="value" line_source="dep.value;"
"#,
    );
}
