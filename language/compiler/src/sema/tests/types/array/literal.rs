use crate::tests::{DirRows, TestSession};

#[test]
fn test_fixed_array_annotation_contextualizes_array_literal() {
    let session = TestSession::single(
        r#"
const pair: [int32; 2] = [1, 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const pair: [int32; 2] = [1, 2];

=== dir ===
const pair: [int32; 2] = [1, 2];
/// @type.symbol symbol=pair source=pair type=FixedArray<int32, 2>
/// @resolution.pattern source=pair kind=binding target=pair
/// @type.node source=[1, 2] type=FixedArray<int32, 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const pair: [int32; 2] = [1, 2, 3];

=== dir ===
const pair: [int32; 2] = [1, 2, 3];
/// @type.symbol symbol=pair source=pair type=FixedArray<int32, 2>
/// @resolution.pattern source=pair kind=binding target=pair
/// @type.node source=[1, 2, 3] type=FixedArray<int32, 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'FixedArray<int32, 3>' is not assignable to type 'FixedArray<int32, 2>'"
/// @diagnostic.label line=2 column=26 span="[1, 2, 3]" line_source="const pair: [int32; 2] = [1, 2, 3];"
/// @diagnostic.related line=2 column=13 span="[int32; 2]" line_source="const pair: [int32; 2] = [1, 2, 3];" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in the length: expected '2', found '3'"
"#,
    );
}

#[test]
fn test_fixed_array_length_hole_infers_literal_length() {
    let session = TestSession::single(
        r#"
function build(): void {
    const bytes: [uint8; _] = [1, 2, 3, 4];
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function build(): void {
    const bytes: [uint8; 4] = [1, 2, 3, 4];
}

=== dir ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    const bytes: [uint8; _] = [1, 2, 3, 4];
    /// @type.symbol symbol=build.bytes source=bytes type=FixedArray<uint8, 4>
    /// @resolution.pattern source=bytes kind=binding target=build.bytes
    /// @type.node source=[1, 2, 3, 4] type=FixedArray<uint8, 4>
    /// @type.node source=1 type=1
    /// @type.node source=2 type=2
    /// @type.node source=3 type=3
    /// @type.node source=4 type=4

}
"#,
    );
}

#[test]
fn test_fixed_array_holes_infer_complete_type() {
    let session = TestSession::single(
        r#"
function build(): void {
    const values: [_; _] = [1, 2, 3, 4];
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function build(): void {
    const values: [float64; 4] = [1, 2, 3, 4];
}

=== dir ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    const values: [_; _] = [1, 2, 3, 4];
    /// @type.symbol symbol=build.values source=values type=FixedArray<float64, 4>
    /// @resolution.pattern source=values kind=binding target=build.values
    /// @type.node source=[1, 2, 3, 4] type=FixedArray<float64, 4>
    /// @type.node source=1 type=1
    /// @type.node source=2 type=2
    /// @type.node source=3 type=3
    /// @type.node source=4 type=4

}
"#,
    );
}

#[test]
fn test_fixed_array_element_hole_infers_widened_element_type() {
    let session = TestSession::single(
        r#"
function build(): void {
    const values: [_; 3] = [1, 2, 3];
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function build(): void {
    const values: [float64; 3] = [1, 2, 3];
}

=== dir ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    const values: [_; 3] = [1, 2, 3];
    /// @type.symbol symbol=build.values source=values type=FixedArray<float64, 3>
    /// @resolution.pattern source=values kind=binding target=build.values
    /// @type.node source=[1, 2, 3] type=FixedArray<float64, 3>
    /// @type.node source=1 type=1
    /// @type.node source=2 type=2
    /// @type.node source=3 type=3

}
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: [1 | 2 | 3; 3] = [1 as 1 | 2 | 3, 2 as 1 | 2 | 3, 3 as 1 | 2 | 3];

=== dir ===
const values: [1 | 2 | 3; 3] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=FixedArray<1 | 2 | 3, 3>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] type=FixedArray<1 | 2 | 3, 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
    );
}

#[test]
fn test_fixed_array_cast_fills_type_holes() {
    let session = TestSession::single(
        r#"
const values = [1, 2, 3] as [_; _];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: [float64; 3] = [1, 2, 3] as [float64; 3];

=== dir ===
const values = [1, 2, 3] as [_; _];
/// @type.symbol symbol=values source=values type=FixedArray<float64, 3>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] as [_; _] type=FixedArray<float64, 3>
/// @type.node source=[1, 2, 3] type=FixedArray<float64, 3>
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: Slice<float64> = [1, 2, 3] as Slice<float64>;

=== dir ===
const values = [1, 2, 3] as Slice<_>;
/// @type.symbol symbol=values source=values type=Slice<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source="[1, 2, 3] as Slice<_>" type=Slice<float64>
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const values: [float64] = [1, 2, 3] as [float64];

=== dir ===
const values = [1, 2, 3] as [_];
/// @type.symbol symbol=values source=values type=Slice<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] as [_] type=Slice<float64>
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
    );
}

#[test]
fn test_nested_fixed_array_annotation_accepts_matching_lengths() {
    let session = TestSession::single(
        r#"
const matrix: [[int32; 2]; 2] = [
    [1, 2],
    [3, 4],
];

matrix satisfies [[int32; 2]; 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const matrix: [[int32; 2]; 2] = [
    [1, 2],
    [3, 4],
];

matrix satisfies [[int32; 2]; 2];

=== dir ===
const matrix: [[int32; 2]; 2] = [
/// @type.symbol symbol=matrix source=matrix type=FixedArray<FixedArray<int32, 2>, 2>
/// @resolution.pattern source=matrix kind=binding target=matrix
/// @type.node type=FixedArray<FixedArray<int32, 2>, 2>

    [1, 2],
    /// @type.node source=[1, 2] type=FixedArray<int32, 2>
    /// @type.node source=1 type=1
    /// @type.node source=2 type=2

    [3, 4],
    /// @type.node source=[3, 4] type=FixedArray<int32, 2>
    /// @type.node source=3 type=3
    /// @type.node source=4 type=4

];

matrix satisfies [[int32; 2]; 2];
/// @type.node source="matrix satisfies [[int32; 2]; 2]" type=FixedArray<FixedArray<int32, 2>, 2>
/// @type.node source=matrix type=FixedArray<FixedArray<int32, 2>, 2>
/// @resolution.name source=matrix target=matrix
/// @resolution.place source=matrix placement="local" lifetime="static" access="readonly"
/// @resolution.access source=matrix root=matrix
"#,
    );
}

#[test]
fn test_nested_fixed_array_annotation_rejects_mismatched_inner_length() {
    let session = TestSession::single(
        r#"
const matrix: [[int32; 2]; 2] = [[1, 2], [3]];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const matrix: [[int32; 2]; 2] = [[1, 2], [3]];

=== dir ===
const matrix: [[int32; 2]; 2] = [[1, 2], [3]];
/// @type.symbol symbol=matrix source=matrix type=FixedArray<FixedArray<int32, 2>, 2>
/// @resolution.pattern source=matrix kind=binding target=matrix
/// @type.node source=[[1, 2], [3]] type=FixedArray<FixedArray<int32, 2>, 2>
/// @type.node source=[1, 2] type=FixedArray<int32, 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=[3] type=FixedArray<int32, 1>
/// @type.node source=3 type=3
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'FixedArray<int32, 1>' is not assignable to type 'FixedArray<int32, 2>'"
/// @diagnostic.label line=2 column=42 span="[3]" line_source="const matrix: [[int32; 2]; 2] = [[1, 2], [3]];"
/// @diagnostic.related line=2 column=15 span="[[int32; 2]; 2]" line_source="const matrix: [[int32; 2]; 2] = [[1, 2], [3]];" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in the length of element 1: expected '2', found '1'"
"#,
    );
}

#[test]
fn test_inferred_binding_keeps_alias_spelling() {
    let session = TestSession::single(
        r#"
declare function make(): Slice<float64>;

const values = make();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function make(): Slice<float64>;

const values: Slice<float64> = make();

=== dir ===
declare function make(): Slice<float64>;
/// @type.symbol symbol=make source="declare function make(): Slice<float64>" type=() => Slice<float64>
/// @resolution.name source=Slice target=collections.slice.Slice

const values = make();
/// @type.symbol symbol=values source=values type=Slice<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=make type=() => Slice<float64>
/// @type.node source=make() type=Slice<float64>
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=Slice<float64> kind=symbol target=make
"#,
    );
}

#[test]
fn test_assign_array_literal_to_iterable_interface() {
    let session = TestSession::single(
        r#"
const items: Iterable<int32> = [1, 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const items: Dynamic<Iterable<int32, void>> = [1, 2];

=== dir ===
const items: Iterable<int32> = [1, 2];
/// @type.symbol symbol=items source=items type=Dynamic<Iterable<int32, void>>
/// @resolution.pattern source=items kind=binding target=items
/// @resolution.name source=Iterable target=iter.iterator.Iterable
"#,
    );
}
