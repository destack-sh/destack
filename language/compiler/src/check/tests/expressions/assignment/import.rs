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
/// @type.node source="counter = 1" type=1
/// @type.node source=counter type=int32
/// @resolution.pattern.assign source=counter kind=place place=binding(counter.counter) type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=2 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error id=cannot-assign-imported-binding message="cannot assign to imported binding 'counter'"
/// @diagnostic.label line=4 column=1 span="counter" line_source="counter = 1;"
/// @diagnostic.related file="counter.ds" line=2 column=12 span="counter" line_source="export let counter: int32 = 0;" message="declared here"
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
/// @type.node source="localCounter = 1" type=1
/// @type.node source=localCounter type=int32
/// @resolution.pattern.assign source=localCounter kind=place place=binding(counter.counter) type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=2 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error id=cannot-assign-imported-binding message="cannot assign to imported binding 'localCounter'"
/// @diagnostic.label line=4 column=1 span="localCounter" line_source="localCounter = 1;"
/// @diagnostic.related file="counter.ds" line=2 column=12 span="counter" line_source="export let counter: int32 = 0;" message="declared here"
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
/// @type.node source="counter = counter" type=<error>
/// @type.node source=counter type=<error>
/// @type.node source=counter type=<error>

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error id=non-writable-assignment-target message="assignment target is not a writable place"
/// @diagnostic.label line=4 column=1 span="counter" line_source="counter = counter;"
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
/// @type.node source="namespaceCounter.counter = 1" type=1
/// @type.node source=namespaceCounter.counter type=int32
/// @resolution.pattern.assign source=namespaceCounter.counter kind=place place=binding(counter.counter) type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=2 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error id=cannot-assign-imported-binding message="cannot assign to imported binding 'namespaceCounter.counter'"
/// @diagnostic.label line=4 column=18 span="counter" line_source="namespaceCounter.counter = 1;"
/// @diagnostic.related file="counter.ds" line=2 column=12 span="counter" line_source="export let counter: int32 = 0;" message="declared here"
"#,
    );
}
