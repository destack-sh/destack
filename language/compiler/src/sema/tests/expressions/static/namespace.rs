use crate::tests::{DirRows, TestSession};

#[test]
fn test_static_true_namespace_export_is_available() {
    let session = TestSession::builder()
        .module(
            "dep.tspp",
            r#"
@if(true)
export const value = 1;
"#,
        )
        .module(
            "main.tspp",
            r#"
import * as dep from "./dep.tspp";

const result = dep.value;
"#,
        )
        .build();

    session.assert_dir_many(
        &["dep.tspp", "main.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== dep.tspp ===

=== annotated ===
@if(true)
export const value: 1 = 1;

=== dir ===
@if(true)
export const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

=== main.tspp ===

=== annotated ===
import * as dep from "./dep.tspp";

const result: 1 = dep.value;

=== dir ===
import * as dep from "./dep.tspp";

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
            "dep.tspp",
            r#"
@if(false)
export const value = 1;
"#,
        )
        .module(
            "main.tspp",
            r#"
import * as dep from "./dep.tspp";

dep.value;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import * as dep from "./dep.tspp";

dep.value;

=== dir ===
import * as dep from "./dep.tspp";

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
