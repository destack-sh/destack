use crate::tests::{DirRows, TestSession};

#[test]
fn test_fixed_array_annotation_contextualizes_array_literal() {
    let session = TestSession::single(
        r#"
const pair: [int32; 2] = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const pair: [int32; 2] = [1, 2];

=== checked ===
const pair: [int32; 2] = [1, 2];
/// @type.symbol symbol=pair source=pair type=FixedArray<int32, 2>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=1 types=9 constraints=6 obligations=0 solutions=1 bounds=3 decisions=0
"#,
    );
}

#[test]
fn test_fixed_array_annotation_rejects_mismatched_literal_length() {
    let session = TestSession::single(
        r#"
const pair: [int32; 2] = [1, 2, 3];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const pair: [int32; 2] = [1, 2, 3];

=== checked ===
const pair: [int32; 2] = [1, 2, 3];
/// @type.symbol symbol=pair source=pair type=FixedArray<int32, 2>
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Array<1 | 2 | 3>' is not assignable to type 'FixedArray<int32, 2>'"
/// @diagnostic.label line=2 column=26 source="const pair: [int32; 2] = [1, 2, 3];"
"#,
    );
}

#[test]
fn test_fixed_array_length_hole_infers_literal_length() {
    let session = TestSession::single(
        r#"
const bytes: [uint8; _] = [1, 2, 3, 4];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const bytes: [uint8; 4] = [1, 2, 3, 4];

=== checked ===
const bytes: [uint8; _] = [1, 2, 3, 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @type.node source=[1, 2, 3, 4] type=Array<1 | 2 | 3 | 4>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
/// @type.node source=4 type=4

/// @check.stats.solve variables=2 types=12 constraints=11 obligations=0 solutions=2 bounds=7 decisions=0
"#,
    );
}

#[test]
fn test_fixed_array_holes_infer_widened_element_type_and_length() {
    let session = TestSession::single(
        r#"
const values: [_; _] = [1, 2, 3, 4];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: [float64; 4] = [1, 2, 3, 4];

=== checked ===
const values: [_; _] = [1, 2, 3, 4];
/// @type.symbol symbol=values source=values type=FixedArray<float64, 4>
/// @type.node source=[1, 2, 3, 4] type=Array<1 | 2 | 3 | 4>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
/// @type.node source=4 type=4
"#,
    );
}

#[test]
fn test_fixed_array_element_hole_infers_widened_element_type() {
    let session = TestSession::single(
        r#"
const values: [_; 3] = [1, 2, 3];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values: [float64; 3] = [1, 2, 3];

=== checked ===
const values: [_; 3] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=FixedArray<float64, 3>
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

/// @check.stats.solve variables=2 types=13 constraints=7 obligations=0 solutions=2 bounds=4 decisions=0
"#,
    );
}

#[test]
fn test_fixed_array_literal_union_element_context_preserves_literal_union() {
    let session = TestSession::single(
        r#"
const values: [1 | 2 | 3; 3] = [1, 2, 3];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: [1 | 2 | 3; 3] = [1, 2, 3];

=== checked ===
const values: [1 | 2 | 3; 3] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=FixedArray<1 | 2 | 3, 3>
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
    );
}

#[test]
fn test_fixed_array_cast_infers_element_and_length() {
    let session = TestSession::single(
        r#"
const values = [1, 2, 3] as [_; _];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: [float64; 3] = [1, 2, 3] as [float64; 3];

=== checked ===
const values = [1, 2, 3] as [_; _];
/// @type.symbol symbol=values source=values type=FixedArray<float64, 3>
/// @type.node source=[1, 2, 3] as [_; _] type=FixedArray<float64, 3>
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
    );
}

#[test]
fn test_slice_cast_infers_widened_element_type() {
    let session = TestSession::single(
        r#"
const values = [1, 2, 3] as Slice<_>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: Slice<float64> = [1 as float64, 2 as float64, 3 as float64] as Slice<float64>;

=== checked ===
const values = [1, 2, 3] as Slice<_>;
/// @type.symbol symbol=values source=values type=collections.slice.Slice<float64>
/// @type.node source="[1, 2, 3] as Slice<_>" type=collections.slice.Slice<float64>
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 to=float64 origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 to=float64 origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 to=float64 origin=implicit
/// @resolution.name source=Slice target=collections.slice.Slice
"#,
    );
}

#[test]
fn test_bracket_slice_cast_infers_widened_element_type() {
    let session = TestSession::single(
        r#"
const values = [1, 2, 3] as [_];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: [float64] = [1 as float64, 2 as float64, 3 as float64] as [float64];

=== checked ===
const values = [1, 2, 3] as [_];
/// @type.symbol symbol=values source=values type=Slice<float64>
/// @type.node source=[1, 2, 3] as [_] type=Slice<float64>
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 to=float64 origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 to=float64 origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 to=float64 origin=implicit
"#,
    );
}
