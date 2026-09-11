use crate::tests::{DirRows, TestSession};

/// Infer a union from literal and spread elements, converting both into that union.
#[test]
fn test_infer_union_elements_from_spreads() {
    let session = TestSession::single(
        r#"
function extend(values: int32[]): void {
    const mixed = [true, ...values];
    mixed satisfies (int32 | boolean)[];
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked().with_coercion(), r#"
=== annotated ===
function extend(values: int32[]): void {
    const mixed: (int32 | boolean)[] = [true as int32 | boolean, ...values];
    mixed satisfies (int32 | boolean)[];
}

=== dir ===
function extend(values: int32[]): void {
/// @type.symbol symbol=extend type=(int32[]) => void
/// @type.symbol symbol=extend.values source="values: int32[]" type=int32[]

    const mixed = [true, ...values];
    /// @type.symbol symbol=extend.mixed source=mixed type=int32 | boolean[]
    /// @resolution.pattern source=mixed kind=binding target=extend.mixed
    /// @resolution.call source=[true, ...values] parameters=(^Slice<int32 | boolean>) arguments=(rest(provided(true) as int32 | boolean, spread(provided(...values) as int32[], iterator=iterator#2(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>)) as int32 | boolean) as int32 | boolean) return=int32 | boolean[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<int32 | boolean>"
    /// @generic.instantiation id="arrayFromOwnedSlice<int32 | boolean>" template=arrayFromOwnedSlice arguments=(int32 | boolean)
    /// @generic.instantiation id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
    /// @coercion.node source=true from=true adjustments=[{ kind: union, target: int32 | boolean, cases: ({ source: true, target: boolean, adjustments: [{ kind: materialize, target: boolean }] }) }] origin=implicit
    /// @coercion.node source=...values from=int32 adjustments=[{ kind: union, target: int32 | boolean, cases: ({ source: int32, target: int32 }) }] origin=implicit
    /// @resolution.name source=values target=extend.values
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=extend.values

    mixed satisfies (int32 | boolean)[];
    /// @resolution.name source=mixed target=extend.mixed
    /// @resolution.place source=mixed placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=mixed root=extend.mixed

}
"#, r#""#);
}

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
/// @generic.instance id="elementSlot<int64, \"mutable\">" template=elementSlot arguments=(int64, "mutable")
/// @generic.instance id="initAsPointer<int64, \"mutable\">" template=initAsPointer arguments=(int64, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int64>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int64>, "mutable")
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=assumeInitDrop#1<int64> template=assumeInitDrop#1 arguments=(int64)
/// @generic.instance id=assumeInitDrop<int64> template=assumeInitDrop arguments=(int64)
/// @generic.instance id=clear<int64> template=clear arguments=(int64)
/// @generic.instance id=drop<int64> template=drop arguments=(int64)
/// @generic.instance id=dropInPlace<int64> template=dropInPlace arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @generic.instance id=truncate<int64> template=truncate arguments=(int64)
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=fromOwnedSlice<int64> template=fromOwnedSlice arguments=(int64)
/// @generic.instance id=intoUninit<int64> template=intoUninit arguments=(int64)
/// @generic.instance id=size<int64> template=size arguments=(int64)
/// @generic.instance id=sliceIntoUninit<int64> template=sliceIntoUninit arguments=(int64)
/// @generic.instance id=sliceLength<int64> template=sliceLength arguments=(int64)
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
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @resolution.call source=[1, 2] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
/// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
/// @generic.instance id=size<int32> template=size arguments=(int32)
/// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
/// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
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
const first: IteratorResult<int64, void> = [1].iterator<int64>().next<int64>();

=== dir ===
const first = [1].iterator().next();
/// @type.symbol symbol=first source=first type=IteratorResult<int64, void>
/// @resolution.pattern source=first kind=binding target=first
/// @generic.instance id="IteratorResult<int64, void>" template=IteratorResult arguments=(int64, void)
/// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
/// @generic.instance id=IteratorYield<int64> template=IteratorYield arguments=(int64)
/// @resolution.member source=[1].iterator receiver=int64[] type=<iterator#2.P0: Place>(this: Managed<int64[], iterator#2.P0>) => Iterator<int64> kind=symbol target_receiver=int64[] target=iterator#2
/// @resolution.member source=[1].iterator().next receiver=Iterator<int64> type=(this: Iterator<int64>) => IteratorResult<int64, void> kind=symbol target_receiver=Iterator<int64> dispatch=dynamic constraint=Iterator<int64> target=Iterator.next
/// @resolution.call source=[1] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @resolution.call source=[1].iterator() parameters=() return=Iterator<int64> kind=symbol target=iterator#2 receiver=int64[] instance="Array<int64>.<extension#4>.iterator#2<\"local\">"
/// @resolution.call source=[1].iterator().next() parameters=() return=IteratorResult<int64, void> kind=dynamic target=Iterator.next receiver=Iterator<int64> constraint=Iterator<int64> generic_arguments=(int64)
/// @generic.instantiation id="iterator#2<int64, \"local\">" template=iterator#2 arguments=(int64, "local")
/// @generic.instantiation id=Iterator.next<int64> template=Iterator.next arguments=(int64)
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instantiation id=iterator#2<int64> template=iterator#2 arguments=(int64)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="elementSlot<int64, \"mutable\">" template=elementSlot arguments=(int64, "mutable")
/// @generic.instance id="initAsPointer<int64, \"mutable\">" template=initAsPointer arguments=(int64, "mutable")
/// @generic.instance id="iterator#2<int64, \"local\">" template=iterator#2 arguments=(int64, "local")
/// @generic.instance id="sliceIndex<MaybeUninit<int64>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int64>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=Iterator<int64> template=Iterator arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=assumeInitDrop#1<int64> template=assumeInitDrop#1 arguments=(int64)
/// @generic.instance id=assumeInitDrop<int64> template=assumeInitDrop arguments=(int64)
/// @generic.instance id=clear<int64> template=clear arguments=(int64)
/// @generic.instance id=drop<int64> template=drop arguments=(int64)
/// @generic.instance id=dropInPlace<int64> template=dropInPlace arguments=(int64)
/// @generic.instance id=fromOwnedSlice<int64> template=fromOwnedSlice arguments=(int64)
/// @generic.instance id=intoUninit<int64> template=intoUninit arguments=(int64)
/// @generic.instance id=size<int64> template=size arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceIntoUninit<int64> template=sliceIntoUninit arguments=(int64)
/// @generic.instance id=sliceLength<int64> template=sliceLength arguments=(int64)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @generic.instance id=truncate<int64> template=truncate arguments=(int64)
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
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.symbol symbol=extend.values source="values: int32[]" type=int32[]

    [...values, 1]
    /// @resolution.call source=[...values, 1] parameters=(^Slice<int32>) arguments=(rest(spread(provided(...values) as int32[], iterator=iterator#2(parameters=(), arguments=(), return=Iterator<int32>), next=dynamic(Iterator<int32> as Iterator<int32>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<int32, void>)) as int32, provided(1) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="IteratorResult<int32, void>" template=IteratorResult arguments=(int32, void)
    /// @generic.instance id="iterator#2<int32, \"local\">" template=iterator#2 arguments=(int32, "local")
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=Iterator<int32> template=Iterator arguments=(int32)
    /// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
    /// @generic.instance id=IteratorYield<int32> template=IteratorYield arguments=(int32)
    /// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
    /// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
    /// @generic.instance id=size<int32> template=size arguments=(int32)
    /// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
    /// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
    /// @resolution.name source=values target=extend.values
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
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
/// @generic.instance id="assumeInitDrop#1<int64 | undefined>" template=assumeInitDrop#1 arguments=(int64 | undefined)
/// @generic.instance id="assumeInitDrop<int64 | undefined>" template=assumeInitDrop arguments=(int64 | undefined)
/// @generic.instance id="clear<int64 | undefined>" template=clear arguments=(int64 | undefined)
/// @generic.instance id="drop<int64 | undefined>" template=drop arguments=(int64 | undefined)
/// @generic.instance id="dropInPlace<int64 | undefined>" template=dropInPlace arguments=(int64 | undefined)
/// @generic.instance id="elementSlot<int64 | undefined, \"mutable\">" template=elementSlot arguments=(int64 | undefined, "mutable")
/// @generic.instance id="initAsPointer<int64 | undefined, \"mutable\">" template=initAsPointer arguments=(int64 | undefined, "mutable")
/// @generic.instance id="sliceAssumeInit<MaybeUninit<int64 | undefined>>" template=sliceAssumeInit arguments=(MaybeUninit<int64 | undefined>)
/// @generic.instance id="sliceIndex<MaybeUninit<int64 | undefined>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int64 | undefined>, "mutable")
/// @generic.instance id="sliceUninit<MaybeUninit<int64 | undefined>>" template=sliceUninit arguments=(MaybeUninit<int64 | undefined>)
/// @generic.instance id="truncate<int64 | undefined>" template=truncate arguments=(int64 | undefined)
/// @resolution.call source=[1, , 3] parameters=(^Slice<int64 | undefined>) arguments=(rest(provided(1) as int64 | undefined, omitted as int64 | undefined, provided(3) as int64 | undefined) as int64 | undefined) return=int64 | undefined[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<int64 | undefined>"
/// @generic.instantiation id="arrayFromOwnedSlice<int64 | undefined>" template=arrayFromOwnedSlice arguments=(int64 | undefined)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="arrayFromOwnedSlice<int64 | undefined>" template=arrayFromOwnedSlice arguments=(int64 | undefined)
/// @generic.instance id="fromOwnedSlice<int64 | undefined>" template=fromOwnedSlice arguments=(int64 | undefined)
/// @generic.instance id="intoUninit<int64 | undefined>" template=intoUninit arguments=(int64 | undefined)
/// @generic.instance id="size<int64 | undefined>" template=size arguments=(int64 | undefined)
/// @generic.instance id="sliceIntoUninit<int64 | undefined>" template=sliceIntoUninit arguments=(int64 | undefined)
/// @generic.instance id="sliceLength<int64 | undefined>" template=sliceLength arguments=(int64 | undefined)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
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
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64, provided(3) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="elementSlot<int64, \"mutable\">" template=elementSlot arguments=(int64, "mutable")
/// @generic.instance id="initAsPointer<int64, \"mutable\">" template=initAsPointer arguments=(int64, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int64>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int64>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=assumeInitDrop#1<int64> template=assumeInitDrop#1 arguments=(int64)
/// @generic.instance id=assumeInitDrop<int64> template=assumeInitDrop arguments=(int64)
/// @generic.instance id=clear<int64> template=clear arguments=(int64)
/// @generic.instance id=drop<int64> template=drop arguments=(int64)
/// @generic.instance id=dropInPlace<int64> template=dropInPlace arguments=(int64)
/// @generic.instance id=fromOwnedSlice<int64> template=fromOwnedSlice arguments=(int64)
/// @generic.instance id=intoUninit<int64> template=intoUninit arguments=(int64)
/// @generic.instance id=size<int64> template=size arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceIntoUninit<int64> template=sliceIntoUninit arguments=(int64)
/// @generic.instance id=sliceLength<int64> template=sliceLength arguments=(int64)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @generic.instance id=truncate<int64> template=truncate arguments=(int64)
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
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64, provided(3) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="elementSlot<int64, \"mutable\">" template=elementSlot arguments=(int64, "mutable")
/// @generic.instance id="initAsPointer<int64, \"mutable\">" template=initAsPointer arguments=(int64, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int64>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int64>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=assumeInitDrop#1<int64> template=assumeInitDrop#1 arguments=(int64)
/// @generic.instance id=assumeInitDrop<int64> template=assumeInitDrop arguments=(int64)
/// @generic.instance id=clear<int64> template=clear arguments=(int64)
/// @generic.instance id=drop<int64> template=drop arguments=(int64)
/// @generic.instance id=dropInPlace<int64> template=dropInPlace arguments=(int64)
/// @generic.instance id=fromOwnedSlice<int64> template=fromOwnedSlice arguments=(int64)
/// @generic.instance id=intoUninit<int64> template=intoUninit arguments=(int64)
/// @generic.instance id=size<int64> template=size arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceIntoUninit<int64> template=sliceIntoUninit arguments=(int64)
/// @generic.instance id=sliceLength<int64> template=sliceLength arguments=(int64)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @generic.instance id=truncate<int64> template=truncate arguments=(int64)
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

const values: [float64] = make();

=== dir ===
declare function make(): Slice<float64>;
/// @type.symbol symbol=make source="declare function make(): Slice<float64>" type=() => Slice<float64>
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
/// @generic.instance id=Iterable<int32> template=Iterable arguments=(int32)
/// @resolution.name source=Iterable target=Iterable
/// @resolution.call source=[1, 2] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
/// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
/// @generic.instance id=size<int32> template=size arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
/// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
"#,
    );
}
