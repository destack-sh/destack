use crate::tests::{DirRows, TestSession};

#[test]
fn test_widen_array_literal_elements_at_the_binding() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
let values: int64[] = [1, 2];

=== dir ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
/// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>
/// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
"#,
    );
}

#[test]
fn test_adopt_the_annotated_element_type() {
    let session = TestSession::single(
        r#"
const values: int32[] = [1, 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const values: int32[] = [1, 2];

=== dir ===
const values: int32[] = [1, 2];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as int32) return=int32[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int32>
/// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
/// @generic.instance id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
"#,
    );
}

#[test]
fn test_settle_an_array_literal_receiver_eagerly() {
    let session = TestSession::single(
        r#"
const first = [1].iterator().next();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const first: IteratorResult<int64, Iterator<int64>.Return> = [1].iterator<int64>().next<int64>();

=== dir ===
const first = [1].iterator().next();
/// @type.symbol symbol=first source=first type=IteratorResult<int64, Iterator<int64>.Return>
/// @resolution.pattern source=first kind=binding target=first
/// @generic.instance id="DropIterator<Iterator<int64>, int64>" template=DropIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="DropWhileIterator<Iterator<int64>, int64>" template=DropWhileIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="EnumeratedIterator<Iterator<int64>, int64>" template=EnumeratedIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="FilterIterator<Iterator<int64>, int64>" template=FilterIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="InspectIterator<Iterator<int64>, int64>" template=InspectIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="Iterator.collect<Iterator<int64>, int64, Owned<int64[]>>" template=Iterator.collect arguments=(int64, Owned<int64[]>)
/// @generic.instance id="IteratorResult<int64, Iterator<int64>.Return>" template=IteratorResult arguments=(int64, Iterator<int64>.Return)
/// @generic.instance id="IteratorResult<int64, void>" template=IteratorResult arguments=(int64, void)
/// @generic.instance id="PeekableIterator<Iterator<int64>, int64>" template=PeekableIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="TakeIterator<Iterator<int64>, int64>" template=TakeIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="TakeWhileIterator<Iterator<int64>, int64>" template=TakeWhileIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id=FromIterator.fromIterator<int64> template=FromIterator.fromIterator arguments=(int64)
/// @generic.instance id=Iterator<int64> template=Iterator arguments=(int64)
/// @generic.instance id=IteratorReturn<Iterator<int64>.Return> template=IteratorReturn arguments=(Iterator<int64>.Return)
/// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
/// @generic.instance id=IteratorYield<int64> template=IteratorYield arguments=(int64)
/// @resolution.member source=[1].iterator receiver=int64[] type=<iterator#2.P0: Place>(this: Managed<int64[], iterator#2.P0>) => Iterator<int64> kind=symbol target_receiver=int64[] target=iterator#2
/// @resolution.member source=[1].iterator().next receiver=Iterator<int64> type=(this: Iterator<int64>) => IteratorResult<int64, Iterator<int64>.Return> kind=symbol target_receiver=Iterator<int64> dispatch=dynamic constraint=Iterator<int64> target=Iterator.next
/// @resolution.call source=[1] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>
/// @resolution.call source=[1].iterator() parameters=() return=Iterator<int64> kind=symbol target=iterator#2 receiver=int64[] instance="Array<int64>.<extension#3>.iterator#2<\"local\">"
/// @resolution.call source=[1].iterator().next() parameters=() return=IteratorResult<int64, Iterator<int64>.Return> kind=dynamic target=Iterator.next receiver=Iterator<int64> constraint=Iterator<int64> generic_arguments=(int64)
/// @generic.instantiation id="iterator#2<int64, \"local\">" template=iterator#2 arguments=(int64, "local")
/// @generic.instantiation id=Iterator.next<int64> template=Iterator.next arguments=(int64)
/// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instantiation id=iterator#2<int64> template=iterator#2 arguments=(int64)
/// @generic.instance id="iterator#2<int64, \"local\">" template=iterator#2 arguments=(int64, "local")
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=Iterator.next<int64> template=Iterator.next arguments=(int64)
/// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
/// @generic.instance id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
"#,
    );
}

#[test]
fn test_infer_the_element_type_from_a_spread_and_a_literal_element() {
    let session = TestSession::single(
        r#"
function extend(values: int32[]): int32[] {
    [...values, 1]
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function extend(values: int32[]): int32[] {
    [...values, 1]
}

=== dir ===
function extend(values: int32[]): int32[] {
/// @type.symbol symbol=extend type=(int32[]) => int32[]
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=extend.values source="values: int32[]" type=int32[]

    [...values, 1]
    /// @resolution.call source=[...values, 1] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1) as int32) return=int32[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int32>
    /// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
    /// @generic.instance id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
    /// @resolution.name source=values target=extend.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=extend.values

}
"#,
    );
}

#[test]
fn test_include_undefined_for_an_elided_array_element() {
    let session = TestSession::single(
        r#"
let values = [1, , 3];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
let values: (int64 | undefined)[] = [1 as int64 | undefined, , 3 as int64 | undefined];

=== dir ===
let values = [1, , 3];
/// @type.symbol symbol=values source=values type=int64 | undefined[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<int64 | undefined>" template=Array arguments=(int64 | undefined)
/// @generic.instance id="MaybeUninit<int64 | undefined>" template=MaybeUninit arguments=(int64 | undefined)
/// @generic.instance id="new<MaybeUninit<int64 | undefined>>" template=new arguments=(MaybeUninit<int64 | undefined>)
/// @resolution.call source=[1, , 3] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 3) as int64 | undefined) return=int64 | undefined[] kind=symbol target=arrayFromSlice instance="arrayFromSlice<int64 | undefined>"
/// @generic.instantiation id="arrayFromSlice<int64 | undefined>" template=arrayFromSlice arguments=(int64 | undefined)
/// @generic.instance id="arrayFromSlice<int64 | undefined>" template=arrayFromSlice arguments=(int64 | undefined)
"#,
    );
}

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
    const values: [int64; 4] = [1, 2, 3, 4];
}

=== dir ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    const values: [_; _] = [1, 2, 3, 4];
    /// @type.symbol symbol=build.values source=values type=FixedArray<int64, 4>
    /// @resolution.pattern source=values kind=binding target=build.values
    /// @type.node source=[1, 2, 3, 4] type=FixedArray<int64, 4>
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
    const values: [int64; 3] = [1, 2, 3];
}

=== dir ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    const values: [_; 3] = [1, 2, 3];
    /// @type.symbol symbol=build.values source=values type=FixedArray<int64, 3>
    /// @resolution.pattern source=values kind=binding target=build.values
    /// @type.node source=[1, 2, 3] type=FixedArray<int64, 3>
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
const values: [1 | 2 | 3; 3] = [1, 2, 3];

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
const values: [int64; 3] = [1, 2, 3] as [int64; 3];

=== dir ===
const values = [1, 2, 3] as [_; _];
/// @type.symbol symbol=values source=values type=FixedArray<int64, 3>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] as [_; _] type=FixedArray<int64, 3>
/// @type.node source=[1, 2, 3] type=FixedArray<int64, 3>
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
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
const values: Slice<int64> = [1, 2, 3] as Slice<int64>;

=== dir ===
const values = [1, 2, 3] as Slice<_>;
/// @type.symbol symbol=values source=values type=Slice<int64>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Slice<int64> template=Slice arguments=(int64)
/// @type.node source="[1, 2, 3] as Slice<_>" type=Slice<int64>
/// @type.node source=[1, 2, 3] type=int64[]
/// @resolution.call source=[1, 2, 3] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2, 3) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>
/// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
/// @generic.instance id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @resolution.name source=Slice target=Slice
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
const values: [int64] = [1, 2, 3] as [int64];

=== dir ===
const values = [1, 2, 3] as [_];
/// @type.symbol symbol=values source=values type=Slice<int64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] as [_] type=Slice<int64>
/// @type.node source=[1, 2, 3] type=int64[]
/// @resolution.call source=[1, 2, 3] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2, 3) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>
/// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
/// @generic.instance id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=2 type=2
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
/// @type.node source=3 type=3
/// @coercion.node source=3 from=3 adjustments=[{ kind: materialize, target: int64 }] origin=implicit
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
/// @resolution.place source=matrix placement="constant" lifetime="static" access="readonly"
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
/// @generic.instance id=Slice<float64> template=Slice arguments=(float64)
/// @resolution.name source=Slice target=Slice

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
const items: Iterable<int32> = [1, 2] as Iterable<int32>;

=== dir ===
const items: Iterable<int32> = [1, 2];
/// @type.symbol symbol=items source=items type=Iterable<int32>
/// @resolution.pattern source=items kind=binding target=items
/// @generic.instance id="DropIterator<Iterator<int32>, int32>" template=DropIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id="DropWhileIterator<Iterator<int32>, int32>" template=DropWhileIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id="EnumeratedIterator<Iterator<int32>, int32>" template=EnumeratedIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id="FilterIterator<Iterator<int32>, int32>" template=FilterIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id="InspectIterator<Iterator<int32>, int32>" template=InspectIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id="Iterator.collect<Iterator<int32>, int32, Owned<int32[]>>" template=Iterator.collect arguments=(int32, Owned<int32[]>)
/// @generic.instance id="IteratorResult<int32, Iterator<int32>.Return>" template=IteratorResult arguments=(int32, Iterator<int32>.Return)
/// @generic.instance id="IteratorResult<int32, void>" template=IteratorResult arguments=(int32, void)
/// @generic.instance id="PeekableIterator<Iterator<int32>, int32>" template=PeekableIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id="TakeIterator<Iterator<int32>, int32>" template=TakeIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id="TakeWhileIterator<Iterator<int32>, int32>" template=TakeWhileIterator arguments=(Iterator<int32>, int32)
/// @generic.instance id=FromIterator.fromIterator<int32> template=FromIterator.fromIterator arguments=(int32)
/// @generic.instance id=Iterable<int32> template=Iterable arguments=(int32)
/// @generic.instance id=Iterator<int32> template=Iterator arguments=(int32)
/// @generic.instance id=IteratorReturn<Iterator<int32>.Return> template=IteratorReturn arguments=(Iterator<int32>.Return)
/// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
/// @generic.instance id=IteratorYield<int32> template=IteratorYield arguments=(int32)
/// @resolution.name source=Iterable target=Iterable
/// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as int32) return=int32[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int32>
/// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
"#,
    );
}
