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
const pair: [int32; 2] = [1, 2];
/// @type.symbol symbol=pair type=[int32; 2]
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @check.stats.solve variables=0 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
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
const pair: [int32; 2] = [1, 2, 3];
/// @type.symbol symbol=pair type=[int32; 2]
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
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
const bytes: [uint8; _] = [1, 2, 3, 4];
/// @type.symbol symbol=bytes type=[uint8; 4]
/// @type.node source=[1, 2, 3, 4] type=Array<1 | 2 | 3 | 4>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
/// @type.node source=4 type=4
/// @check.stats.solve variables=1 constraints=1 obligations=0 solutions=1 bounds=0 decisions=0
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
const values: [_; _] = [1, 2, 3, 4];
/// @type.symbol symbol=values type=[int32; 4]
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
const values: [_; 3] = [1, 2, 3];
/// @type.symbol symbol=values type=[int32; 3]
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
/// @check.stats.solve variables=1 constraints=1 obligations=0 solutions=1 bounds=0 decisions=0
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
const values: [1 | 2 | 3; 3] = [1, 2, 3];
/// @type.symbol symbol=values type=[1 | 2 | 3; 3]
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
const values = [1, 2, 3] as [_; _];
/// @type.symbol symbol=values type=[int32; 3]
/// @type.node source=[1, 2, 3] as [_; _] type=[int32; 3]
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @coercion.node source=[1, 2, 3] as [_; _] from=Array<1 | 2 | 3> to=[int32; 3] origin=explicit
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
const values = [1, 2, 3] as Slice<_>;
/// @type.symbol symbol=values type=[int32]
/// @type.node source="[1, 2, 3] as Slice<_>" type=[int32]
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @coercion.node source="[1, 2, 3] as Slice<_>" from=Array<1 | 2 | 3> to=[int32] origin=explicit
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
/// @generic.instance source=Slice<_> id=collections.slice.Slice<int32>
/// @resolution.name source=Slice target=collections.slice.Slice
/// @generic.instance id=collections.slice.Slice<int32> symbol=collections.slice.Slice arguments=[int32]
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
const values = [1, 2, 3] as [_];
/// @type.symbol symbol=values type=[int32]
/// @type.node source=[1, 2, 3] as [_] type=[int32]
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @coercion.node source=[1, 2, 3] as [_] from=Array<1 | 2 | 3> to=[int32] origin=explicit
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
    );
}
