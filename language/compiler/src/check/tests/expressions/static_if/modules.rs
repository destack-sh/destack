use crate::tests::{DirRows, TestSession};

#[test]
fn test_static_if_true_namespace_export_is_available() {
    let session = TestSession::new()
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
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
=== dep.ds ===
@if(true)
export const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 terms=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0

=== main.ds ===
import * as dep from "./dep.ds";

const result = dep.value;
/// @type.symbol symbol=result source=result type=1
/// @type.node source=dep.value type=1
/// @resolution.name source=dep.value target=dep.value

/// @check.stats.solve variables=0 terms=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_static_if_false_namespace_export_is_unavailable() {
    let session = TestSession::new()
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
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
import * as dep from "./dep.ds";

dep.value;

/// @check.stats.solve variables=0 terms=1 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
        r#"
/// @diagnostic.error code=EC308 message="unresolved reference 'dep.value'"
/// @diagnostic.label line=4 column=1 source="dep.value;"
"#,
    );
}
