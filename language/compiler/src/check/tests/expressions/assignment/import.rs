use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_binding_rejects_assignment() {
    let session = TestSession::builder()
        .module(
            "counter.ds",
            r#"
export let counter: int32 = 0;
"#,
        )
        .module(
            "main.ds",
            r#"
import { counter } from "./counter.ds";

counter = 1;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
import { counter } from "./counter.ds";

counter = 1;

=== checked ===
import { counter } from "./counter.ds";

counter = 1;
/// @type.node source="counter = 1" type=int32
/// @type.node source=counter type=int32
/// @resolution.name source=counter target=counter.counter
/// @type.node source=1 type=int32

/// @check.stats.solve variables=0 types=2 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC213 message="cannot assign to imported binding 'counter'"
/// @diagnostic.label line=4 column=1 source="counter = 1;"
"#,
    );
}

#[test]
fn test_import_alias_rejects_assignment() {
    let session = TestSession::builder()
        .module(
            "counter.ds",
            r#"
export let counter: int32 = 0;
"#,
        )
        .module(
            "main.ds",
            r#"
import { counter as localCounter } from "./counter.ds";

localCounter = 1;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
import { counter as localCounter } from "./counter.ds";

localCounter = 1;

=== checked ===
import { counter as localCounter } from "./counter.ds";

localCounter = 1;
/// @type.node source="localCounter = 1" type=int32
/// @type.node source=localCounter type=int32
/// @resolution.name source=localCounter target=counter.counter
/// @type.node source=1 type=int32

/// @check.stats.solve variables=0 types=2 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC213 message="cannot assign to imported binding 'localCounter'"
/// @diagnostic.label line=4 column=1 source="localCounter = 1;"
"#,
    );
}

#[test]
fn test_namespace_import_binding_rejects_assignment() {
    let session = TestSession::builder()
        .module(
            "counter.ds",
            r#"
export let counter: int32 = 0;
"#,
        )
        .module(
            "main.ds",
            r#"
import * as counter from "./counter.ds";

counter = counter;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
import * as counter from "./counter.ds";

counter = counter;

=== checked ===
import * as counter from "./counter.ds";

counter = counter;
/// @type.node source="counter = counter" type=typeof import("./counter.ds")
/// @type.node source=counter type=typeof import("./counter.ds")
/// @resolution.name source=counter target=counter
/// @type.node source=counter type=typeof import("./counter.ds")
/// @resolution.name source=counter target=counter

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC213 message="cannot assign to imported binding 'counter'"
/// @diagnostic.label line=4 column=1 source="counter = counter;"
"#,
    );
}

#[test]
fn test_namespace_import_member_rejects_assignment() {
    let session = TestSession::builder()
        .module(
            "counter.ds",
            r#"
export let counter: int32 = 0;
"#,
        )
        .module(
            "main.ds",
            r#"
import * as namespaceCounter from "./counter.ds";

namespaceCounter.counter = 1;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
import * as namespaceCounter from "./counter.ds";

namespaceCounter.counter = 1;

=== checked ===
import * as namespaceCounter from "./counter.ds";

namespaceCounter.counter = 1;
/// @type.node source="namespaceCounter.counter = 1" type=int32
/// @type.node source=namespaceCounter.counter type=int32
/// @resolution.name source=namespaceCounter.counter target=counter.counter
/// @type.node source=1 type=int32

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC213 message="cannot assign to imported binding 'namespaceCounter.counter'"
/// @diagnostic.label line=4 column=1 source="namespaceCounter.counter = 1;"
"#,
    );
}
