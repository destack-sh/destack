use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_reports_assignment_mismatch() {
    let session = TestSession::single(
        r#"
const value: int32 = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
const value: int32 = "text";
/// @type.node source="\"text\"" type=string
/// @type.symbol symbol=value type=int32

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=22 source="const value: int32 = \"text\";"
"#,
    );
}

#[test]
fn test_check_records_mutable_assignment() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
let value: int32 = 1;
/// @type.symbol symbol=value type=int32

value = 2;
/// @resolution.name source=value target=value
/// @type.node source=2 type=int32
/// @type.node source="value = 2" type=int32
"#,
    );
}

#[test]
fn test_check_reports_assignment_expression_mismatch() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
let value: int32 = 1;
/// @type.symbol symbol=value type=int32

value = "text";
/// @resolution.name source=value target=value
/// @type.node source="\"text\"" type=string
/// @type.node source="value = \"text\"" type=int32
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=3 column=9 source="value = \"text\";"
"#,
    );
}

#[test]
fn test_check_reports_const_binding_assignment() {
    let session = TestSession::single(
        r#"
const value: int32 = 1;
value = 2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
const value: int32 = 1;
/// @type.symbol symbol=value type=int32

value = 2;
/// @resolution.name source=value target=value
/// @type.node source=2 type=int32
"#,
        r#"
/// @diagnostic.error code=EC204 message="assignment target is not writable"
/// @diagnostic.label line=3 column=7 source="value = 2;"
"#,
    );
}

#[test]
fn test_check_allows_member_assignment_through_const_bindings() {
    let session = TestSession::single(
        r#"
const state: { count: int32 } = { count: 0 };
state.count = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
const state: { count: int32 } = { count: 0 };
/// @type.symbol symbol=state type={ count: int32 }

state.count = 1;
/// @resolution.name source=state target=state
/// @resolution.member source=state.count receiver={ count: int32 } kind=direct target=state.count
/// @type.node source=1 type=int32
/// @type.node source="state.count = 1" type=int32
"#,
    );
}

#[test]
fn test_check_reports_readonly_member_assignment() {
    let session = TestSession::single(
        r#"
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
const state: { readonly count: int32 } = { count: 0 };
/// @type.symbol symbol=state type={ readonly count: int32 }

state.count = 1;
/// @resolution.name source=state target=state
/// @resolution.member source=state.count receiver={ readonly count: int32 } kind=direct target=state.count
/// @type.node source=1 type=int32
"#,
        r#"
/// @diagnostic.error code=EC204 message="assignment target is not writable"
/// @diagnostic.label line=3 column=13 source="state.count = 1;"
"#,
    );
}

#[test]
fn test_check_records_definite_assignment_after_write() {
    let session = TestSession::single(
        r#"
let value: string;
value = "ready";
const copy = value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
let value: string;
/// @type.symbol symbol=value type=string

value = "ready";
/// @resolution.name source=value target=value
/// @type.node source="\"ready\"" type=string
/// @type.node source="value = \"ready\"" type=string

const copy = value;
/// @resolution.name source=value target=value
/// @type.symbol symbol=copy type=string
"#,
    );
}

#[test]
fn test_check_reports_read_before_definite_assignment() {
    let session = TestSession::single(
        r#"
let value: string;
const copy = value;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
let value: string;
/// @type.symbol symbol=value type=string

const copy = value;
/// @resolution.name source=value target=value

"#,
        r#"
/// @diagnostic.error code=EC405 message="value is used before assignment"
/// @diagnostic.label line=3 column=14 source="const copy = value;"
"#,
    );
}

#[test]
fn test_check_reports_import_binding_assignment() {
    let session = TestSession::new()
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
        DirRows::checked(),
        r#"
import { counter } from "./counter.ds";
/// @resolution.name source=counter target=counter.counter

counter = 1;
/// @resolution.name source=counter target=counter.counter
/// @type.node source=1 type=int32
"#,
        r#"
/// @diagnostic.error code=EC204 message="assignment target is not writable"
/// @diagnostic.label line=4 column=9 source="counter = 1;"
"#,
    );
}
