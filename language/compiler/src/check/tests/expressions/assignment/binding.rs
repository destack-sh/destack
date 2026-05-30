use crate::tests::{DirRows, TestSession};

#[test]
fn test_mutable_binding_allows_assignment() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value: int32 = 1;
/// @type.symbol symbol=value type=int32
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_assignment_expression_mismatch_reports_error() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value: int32 = 1;
/// @type.symbol symbol=value type=int32
/// @type.node source=1 type=1

value = "text";
/// @type.node source="value = \"text\"" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @type.node source="\"text\"" type="text"
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=3 column=9 source="value = \"text\";"
"#,
    );
}

#[test]
fn test_array_assignment_checks_element_type_without_retyping_literal() {
    let session = TestSession::single(
        r#"
let values: int32[];
values = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let values: int32[];
/// @type.symbol symbol=values type=Array<int32>

values = [1, 2];
/// @type.node source="values = [1, 2]" type=Array<int32>
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_fixed_array_assignment_checks_length_without_retyping_literal() {
    let session = TestSession::single(
        r#"
let values: [int32; 2];
values = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let values: [int32; 2];
/// @type.symbol symbol=values type=[int32; 2]

values = [1, 2];
/// @type.node source="values = [1, 2]" type=[int32; 2]
/// @type.node source=values type=[int32; 2]
/// @resolution.name source=values target=values
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_fixed_array_assignment_rejects_length_mismatch() {
    let session = TestSession::single(
        r#"
let values: [int32; 2];
values = [1, 2, 3];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let values: [int32; 2];
/// @type.symbol symbol=values type=[int32; 2]

values = [1, 2, 3];
/// @type.node source="values = [1, 2, 3]" type=[int32; 2]
/// @type.node source=values type=[int32; 2]
/// @resolution.name source=values target=values
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=3 column=10 source="values = [1, 2, 3];"
"#,
    );
}

#[test]
fn test_write_definitely_assigns_binding() {
    let session = TestSession::single(
        r#"
let value: string;
value = "ready";
const copy = value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value: string;
/// @type.symbol symbol=value type=string

value = "ready";
/// @type.node source="value = \"ready\"" type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @type.node source="\"ready\"" type="ready"

const copy = value;
/// @type.symbol symbol=copy type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_read_before_definite_assignment_reports_error() {
    let session = TestSession::single(
        r#"
let value: string;
const copy = value;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value: string;
/// @type.symbol symbol=value type=string

const copy = value;
/// @type.symbol symbol=copy type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value

"#,
        r#"
/// @diagnostic.error code=EC405 message="value is used before assignment"
/// @diagnostic.label line=3 column=14 source="const copy = value;"
"#,
    );
}
