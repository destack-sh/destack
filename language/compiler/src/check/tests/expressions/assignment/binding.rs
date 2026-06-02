use crate::tests::{DirRows, TestSession};

#[test]
fn test_mutable_binding_accepts_assignment() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 terms=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_assignment_rejects_incompatible_value() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value = "text";
/// @type.node source="value = \"text\"" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @type.node source="\"text\"" type="text"

/// @check.stats.solve variables=0 terms=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=3 column=9 source="value = \"text\";"
"#,
    );
}

#[test]
fn test_mutable_binding_uses_widened_initializer_type() {
    let session = TestSession::single(
        r#"
let value = 1;
value = 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let value = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 terms=4 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_mutable_binding_rejects_assignment_outside_widened_type() {
    let session = TestSession::single(
        r#"
let value = 1;
value = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let value = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value = "text";
/// @type.node source="value = \"text\"" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @type.node source="\"text\"" type="text"

/// @check.stats.solve variables=0 terms=4 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=3 column=9 source="value = \"text\";"
"#,
    );
}

#[test]
fn test_assignment_definitely_assigns_annotated_binding() {
    let session = TestSession::single(
        r#"
let value: int32;
value = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let value: int32;
/// @type.symbol symbol=value source=value type=int32

value = 1;
/// @type.node source="value = 1" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 terms=3 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_array_assignment_accepts_literal_elements() {
    let session = TestSession::single(
        r#"
let values: int32[];
values = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

values = [1, 2];
/// @type.node source="values = [1, 2]" type=Array<int32>
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 terms=7 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_empty_array_assignment_uses_target_type() {
    let session = TestSession::single(
        r#"
let values: int32[];
values = [];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

values = [];
/// @type.node source="values = []" type=Array<int32>
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values
/// @type.node source=[] type=Array<int32>

/// @check.stats.solve variables=0 terms=4 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_fixed_array_assignment_accepts_matching_length() {
    let session = TestSession::single(
        r#"
let values: [int32; 2];
values = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let values: [int32; 2];
/// @type.symbol symbol=values source=values type=[int32; 2]

values = [1, 2];
/// @type.node source="values = [1, 2]" type=[int32; 2]
/// @type.node source=values type=[int32; 2]
/// @resolution.name source=values target=values
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 terms=8 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_fixed_array_assignment_rejects_wrong_length() {
    let session = TestSession::single(
        r#"
let values: [int32; 2];
values = [1, 2, 3];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let values: [int32; 2];
/// @type.symbol symbol=values source=values type=[int32; 2]

values = [1, 2, 3];
/// @type.node source="values = [1, 2, 3]" type=[int32; 2]
/// @type.node source=values type=[int32; 2]
/// @resolution.name source=values target=values
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

/// @check.stats.solve variables=0 terms=9 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1

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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let value: string;
/// @type.symbol symbol=value source=value type=string

value = "ready";
/// @type.node source="value = \"ready\"" type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @type.node source="\"ready\"" type="ready"

const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value

/// @check.stats.solve variables=0 terms=5 constraints=1 obligations=1 solutions=0 bounds=0 decisions=2
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let value: string;
/// @type.symbol symbol=value source=value type=string

const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value

/// @check.stats.solve variables=0 terms=4 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1

"#,
        r#"
/// @diagnostic.error code=EC405 message="value is used before assignment"
/// @diagnostic.label line=3 column=14 source="const copy = value;"
"#,
    );
}
