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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { counter } from "./counter.ds";

counter = 1;

=== dir ===
import { counter } from "./counter.ds";

counter = 1;
/// @type.node source="counter = 1" type=1
/// @type.node source=counter type=int32
/// @resolution.name source=counter target=counter.counter
/// @resolution.pattern.assign source=counter kind=place
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter.counter
/// @resolution.assignment source=counter write=binding(counter.counter) type=int32
/// @type.node source=1 type=1
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { counter as localCounter } from "./counter.ds";

localCounter = 1;

=== dir ===
import { counter as localCounter } from "./counter.ds";

localCounter = 1;
/// @type.node source="localCounter = 1" type=1
/// @type.node source=localCounter type=int32
/// @resolution.name source=localCounter target=counter.counter
/// @resolution.pattern.assign source=localCounter kind=place
/// @resolution.place source=localCounter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localCounter root=counter.counter
/// @resolution.assignment source=localCounter write=binding(counter.counter) type=int32
/// @type.node source=1 type=1
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import * as counter from "./counter.ds";

counter = counter;

=== dir ===
import * as counter from "./counter.ds";

counter = counter;
/// @type.node source="counter = counter" type=<error>
/// @type.node source=counter type=<error>
/// @resolution.rejected source=counter
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import * as namespaceCounter from "./counter.ds";

namespaceCounter.counter = 1;

=== dir ===
import * as namespaceCounter from "./counter.ds";

namespaceCounter.counter = 1;
/// @type.node source="namespaceCounter.counter = 1" type=1
/// @type.node source=namespaceCounter.counter type=int32
/// @resolution.name source=namespaceCounter.counter target=counter.counter
/// @resolution.pattern.assign source=namespaceCounter.counter kind=place
/// @resolution.access source=namespaceCounter.counter root=counter.counter
/// @resolution.assignment source=namespaceCounter.counter write=binding(counter.counter) type=int32
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=cannot-assign-imported-binding message="cannot assign to imported binding 'namespaceCounter.counter'"
/// @diagnostic.label line=4 column=18 span="counter" line_source="namespaceCounter.counter = 1;"
/// @diagnostic.related file="counter.ds" line=2 column=12 span="counter" line_source="export let counter: int32 = 0;" message="declared here"
"#,
    );
}
