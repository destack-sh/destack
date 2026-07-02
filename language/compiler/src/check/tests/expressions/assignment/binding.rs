use crate::tests::{DirRows, TestSession};

#[test]
fn test_initializer_rejects_incompatible_value() {
    let session = TestSession::single(
        r#"
const value: int32 = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: int32 = "text";

=== checked ===
const value: int32 = "text";
/// @type.symbol symbol=value source=value type=int32
/// @type.node source="\"text\"" type="text"

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"text\"' is not assignable to type 'int32'"
/// @diagnostic.label line=2 column=22 span="\"text\"" line_source="const value: int32 = \"text\";"
"#,
    );
}

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
=== annotated ===
let value: int32 = 1;
value = 2;

=== checked ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=2
/// @type.node source=value type=int32
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=1
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
=== annotated ===
let value: int32 = 1;
value = "text";

=== checked ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value = "text";
/// @type.node source="value = \"text\"" type="text"
/// @type.node source=value type=int32
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source="\"text\"" type="text"

/// @check.stats.solve variables=0 types=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"text\"' is not assignable to type 'int32'"
/// @diagnostic.label line=3 column=9 span="\"text\"" line_source="value = \"text\";"
"#,
    );
}

#[test]
fn test_mutable_binding_accepts_compound_assignment() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value += 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: int32 = 1;
value += 2;

=== checked ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=1 type=1

value += 2;
/// @type.node source="value += 2" type=int32
/// @type.node source=value type=int32
/// @resolution.call source="value += 2" parameters=() return=int32 kind=builtin builtin=binary.add
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=2
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
=== annotated ===
let value: float64 = 1;
value = 2;

=== checked ===
let value = 1;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=2
/// @type.node source=value type=float64
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=float64
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=4 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
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
=== annotated ===
let value: float64 = 1;
value = "text";

=== checked ===
let value = 1;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source=1 type=1

value = "text";
/// @type.node source="value = \"text\"" type="text"
/// @type.node source=value type=float64
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=float64
/// @type.node source="\"text\"" type="text"

/// @check.stats.solve variables=0 types=4 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"text\"' is not assignable to type 'float64'"
/// @diagnostic.label line=3 column=9 span="\"text\"" line_source="value = \"text\";"
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
=== annotated ===
let value: int32;
value = 1;

=== checked ===
let value: int32;
/// @type.symbol symbol=value source=value type=int32

value = 1;
/// @type.node source="value = 1" type=1
/// @type.node source=value type=int32
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
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
=== annotated ===
let values: int32[];
values = [1, 2];

=== checked ===
let values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

values = [1, 2];
/// @type.node source="values = [1, 2]" type=Array<int32>
/// @type.node source=values type=Array<int32>
/// @resolution.pattern.assign source=values kind=place place=binding(values) type=Array<int32>
/// @type.node source=[1, 2] type=Array<int32>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=5 constraints=2 obligations=1 solutions=0 bounds=0 decisions=1
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
=== annotated ===
let values: int32[];
values = [];

=== checked ===
let values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

values = [];
/// @type.node source="values = []" type=Array<int32>
/// @type.node source=values type=Array<int32>
/// @resolution.pattern.assign source=values kind=place place=binding(values) type=Array<int32>
/// @type.node source=[] type=Array<int32>

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=1 solutions=0 bounds=0 decisions=1
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
=== annotated ===
let values: [int32; 2];
values = [1, 2];

=== checked ===
let values: [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>

values = [1, 2];
/// @type.node source="values = [1, 2]" type=FixedArray<int32, 2>
/// @type.node source=values type=FixedArray<int32, 2>
/// @resolution.pattern.assign source=values kind=place place=binding(values) type=FixedArray<int32, 2>
/// @type.node source=[1, 2] type=FixedArray<int32, 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=5 constraints=3 obligations=1 solutions=0 bounds=0 decisions=1
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
=== annotated ===
let values: [int32; 2];
values = [1, 2, 3];

=== checked ===
let values: [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>

values = [1, 2, 3];
/// @type.node source="values = [1, 2, 3]" type=FixedArray<int32, 3>
/// @type.node source=values type=FixedArray<int32, 2>
/// @resolution.pattern.assign source=values kind=place place=binding(values) type=FixedArray<int32, 2>
/// @type.node source=[1, 2, 3] type=FixedArray<int32, 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

/// @check.stats.solve variables=0 types=7 constraints=4 obligations=1 solutions=0 bounds=0 decisions=1

"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'FixedArray<int32, 3>' is not assignable to type 'FixedArray<int32, 2>'"
/// @diagnostic.label line=3 column=10 span="[1, 2, 3]" line_source="values = [1, 2, 3];"
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
=== annotated ===
let value: string;
value = "ready";
const copy: string = value;

=== checked ===
let value: string;
/// @type.symbol symbol=value source=value type=string

value = "ready";
/// @type.node source="value = \"ready\"" type="ready"
/// @type.node source=value type=string
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=string
/// @type.node source="\"ready\"" type="ready"

const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=1 solutions=0 bounds=0 decisions=2
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
=== annotated ===
let value: string;
const copy: string = value;

=== checked ===
let value: string;
/// @type.symbol symbol=value source=value type=string

const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1

"#,
        r#"
/// @diagnostic.error code=EC405 message="'value' is used before being assigned"
/// @diagnostic.label line=3 column=14 span="value" line_source="const copy = value;"
"#,
    );
}

#[test]
fn test_branch_assignment_requires_all_paths() {
    let session = TestSession::single(
        r#"
declare const condition: boolean;

let value: string;
if (condition) {
    value = "ready";
}
const copy = value;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
declare const condition: boolean;

let value: string;
if (condition) {
    value = "ready";
}
const copy: string = value;

=== checked ===
declare const condition: boolean;
/// @type.symbol symbol=condition source=condition type=boolean

let value: string;
/// @type.symbol symbol=value source=value type=string

if (condition) {
/// @type.node source=condition type=boolean
/// @resolution.name source=condition target=condition

    value = "ready";
    /// @type.node source="value = \"ready\"" type="ready"
    /// @type.node source=value type=string
    /// @resolution.pattern.assign source=value kind=place place=binding(value) type=string
    /// @type.node source="\"ready\"" type="ready"

}
const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @type.node source=value type=string
/// @resolution.name source=value target=value

/// @check.stats.solve variables=0 types=4 constraints=2 obligations=1 solutions=0 bounds=0 decisions=3

"#,
        r#"
/// @diagnostic.error code=EC405 message="'value' is used before being assigned"
/// @diagnostic.label line=8 column=14 span="value" line_source="const copy = value;"
"#,
    );
}

#[test]
fn test_undefined_initializer_models_optional_state() {
    let session = TestSession::single(
        r#"
declare const condition: boolean;

let value: string | undefined = undefined;
if (condition) {
    value = "ready";
}
const copy = value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
declare const condition: boolean;

let value: string | undefined = undefined as string | undefined;
if (condition) {
    value = "ready" as string | undefined;
}
const copy: string | undefined = value;

=== checked ===
declare const condition: boolean;
/// @type.symbol symbol=condition source=condition type=boolean

let value: string | undefined = undefined;
/// @type.symbol symbol=value source=value type=string | undefined
/// @type.node source=undefined type=undefined

if (condition) {
/// @type.node source=condition type=boolean
/// @resolution.name source=condition target=condition

    value = "ready";
    /// @type.node source="value = \"ready\"" type="ready"
    /// @type.node source=value type=string | undefined
    /// @resolution.pattern.assign source=value kind=place place=binding(value) type=string | undefined
    /// @type.node source="\"ready\"" type="ready"

}
const copy = value;
/// @type.symbol symbol=copy source=copy type=string | undefined
/// @type.node source=value type=string | undefined
/// @resolution.name source=value target=value

/// @check.stats.solve variables=0 types=6 constraints=3 obligations=1 solutions=0 bounds=0 decisions=3
"#,
    );
}
