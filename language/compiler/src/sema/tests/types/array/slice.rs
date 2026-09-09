use crate::tests::{DirRows, TestSession};

#[test]
fn test_range_subscript_selects_slice_type() {
    let session = TestSession::single(
        r#"
declare const bytes: [uint8; 4];
const slice = bytes[1..3];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_witnesses().with_reference_types(),
        r#"
=== annotated ===
declare const bytes: [uint8; 4];
const slice: [uint8] = bytes[1..3];

=== dir ===
declare const bytes: [uint8; 4];
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @resolution.pattern source=bytes kind=binding target=bytes

const slice = bytes[1..3];
/// @type.symbol symbol=slice source=slice type=Slice<uint8>
/// @resolution.pattern source=slice kind=binding target=slice
/// @type.node source=bytes type=FixedArray<uint8, 4>
/// @type.node source=bytes[1..3] type=Slice<uint8>
/// @resolution.name source=bytes target=bytes
/// @resolution.place source=bytes placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=bytes root=bytes
/// @resolution.subscript source=bytes[1..3] type=Slice<uint8> kind=call target="index#2(parameters=(Range<isize>), arguments=(provided(1..3) as Range<isize>), return=&'static readonly constant Slice<uint8>, regions=(\"static\" & \"constant\"))"
/// @generic.instantiation id="index#2<uint8, 4, Range<isize>, \"readonly\">" template=index#2 arguments=(uint8, 4, Range<isize>, "readonly")
/// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
/// @generic.instance id="FixedArray<uint8, 4>" template=FixedArray arguments=(uint8, 4)
/// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
/// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
/// @generic.instance id="index#2<uint8, 4, Range<isize>, \"readonly\">" template=index#2 arguments=(uint8, 4, Range<isize>, "readonly")
/// @generic.instance id=endBound#1<isize> template=endBound#1 arguments=(isize)
/// @generic.instance id=startBound#1<isize> template=startBound#1 arguments=(isize)
/// @type.node source=1 type=1
/// @type.node source=1..3 type=Range<isize>
/// @generic.instance id=Range<isize> template=Range arguments=(isize)
/// @type.node source=3 type=3

/// @generic.witness type=Range<isize> interface=RangeBounds<isize> functions=(RangeBounds.startBound: startBound#1<isize>, RangeBounds.endBound: endBound#1<isize>)
"#,
    );
}

#[test]
fn test_borrow_a_range_view_from_a_readonly_slice() {
    let session = TestSession::single(
        r#"
declare const values: &readonly [float64];
const window = values[1..3];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: &'static readonly [float64];
const window: [float64] = values[1..3];

=== dir ===
declare const values: &readonly [float64];
/// @type.symbol symbol=values source=values type=&'static readonly constant Slice<float64>
/// @resolution.pattern source=values kind=binding target=values

const window = values[1..3];
/// @type.symbol symbol=window source=window type=Slice<float64>
/// @resolution.pattern source=window kind=binding target=window
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[1..3] type=Slice<float64> kind=call target="index#2(parameters=(Range<isize>), arguments=(provided(1..3) as Range<isize>), return=WithAccess<&'static constant Slice<float64>, \"readonly\">, regions=(\"static\" & \"constant\"))"
/// @generic.instantiation id="index#2<float64, Range<isize>, \"readonly\">" template=index#2 arguments=(float64, Range<isize>, "readonly")
/// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="WithAccess<&'bound0 Slice<float64>, \"readonly\">" template=WithAccess arguments=(&'bound0 Slice<float64>, "readonly")
/// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
/// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
/// @generic.instance id="index#2<float64, Range<isize>, \"readonly\">" template=index#2 arguments=(float64, Range<isize>, "readonly")
/// @generic.instance id="rangeSpan<float64, Range<isize>, \"readonly\" | \"mutable\">" template=rangeSpan arguments=(float64, Range<isize>, "readonly" | "mutable")
/// @generic.instance id="sliceView<float64, \"readonly\">" template=sliceView arguments=(float64, "readonly")
/// @generic.instance id="subslice<float64, \"readonly\">" template=subslice arguments=(float64, "readonly")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=endBound#1<isize> template=endBound#1 arguments=(isize)
/// @generic.instance id=size<float64> template=size arguments=(float64)
/// @generic.instance id=sliceLength<float64> template=sliceLength arguments=(float64)
/// @generic.instance id=startBound#1<isize> template=startBound#1 arguments=(isize)
/// @generic.instance id=Range<isize> template=Range arguments=(isize)
"#,
    );
}

#[test]
fn test_copy_into_a_range_of_a_mutable_slice() {
    let session = TestSession::single(
        r#"
declare const buffer: &[uint8];
declare const header: &readonly [uint8];

function stamp(): void {
    buffer[0..2] = header;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const buffer: &'static [uint8];
declare const header: &'static readonly [uint8];

function stamp(): void {
    buffer[0..2] = header;
}

=== dir ===
declare const buffer: &[uint8];
/// @type.symbol symbol=buffer source=buffer type=&'static constant Slice<uint8>
/// @resolution.pattern source=buffer kind=binding target=buffer

declare const header: &readonly [uint8];
/// @type.symbol symbol=header source=header type=&'static readonly constant Slice<uint8>
/// @resolution.pattern source=header kind=binding target=header

function stamp(): void {
/// @type.symbol symbol=stamp type=() => void

    buffer[0..2] = header;
    /// @resolution.name source=buffer target=buffer
    /// @resolution.place source=buffer placement="constant" lifetime="static" access="mutable"
    /// @resolution.access source=buffer root=buffer
    /// @resolution.pattern.assign source=buffer[0..2] kind=place
    /// @resolution.assignment source=buffer[0..2] write="indexSet#2(parameters=(Range<isize>, &'static readonly constant Slice<uint8>), arguments=(provided(0..2) as Range<isize>, supplied as &'static readonly constant Slice<uint8>), return=void, regions=(\"frame\", \"static\" & \"constant\", \"static\" & \"constant\"))" type=&'static readonly constant Slice<uint8>
    /// @generic.instantiation id="indexSet#2<uint8, Range<isize>>" template=indexSet#2 arguments=(uint8, Range<isize>)
    /// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
    /// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
    /// @generic.instance id="index#2<uint8, Range<isize>, \"mutable\">" template=index#2 arguments=(uint8, Range<isize>, "mutable")
    /// @generic.instance id="indexSet#2<uint8, Range<isize>>" template=indexSet#2 arguments=(uint8, Range<isize>)
    /// @generic.instance id="rangeSpan<uint8, Range<isize>, \"mutable\" | \"readonly\">" template=rangeSpan arguments=(uint8, Range<isize>, "mutable" | "readonly")
    /// @generic.instance id="sliceView<uint8, \"mutable\">" template=sliceView arguments=(uint8, "mutable")
    /// @generic.instance id="subslice<uint8, \"mutable\">" template=subslice arguments=(uint8, "mutable")
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=Slice<uint8> template=Slice arguments=(uint8)
    /// @generic.instance id=copyFrom<uint8> template=copyFrom arguments=(uint8)
    /// @generic.instance id=endBound#1<isize> template=endBound#1 arguments=(isize)
    /// @generic.instance id=size<uint8> template=size arguments=(uint8)
    /// @generic.instance id=sliceGet<uint8> template=sliceGet arguments=(uint8)
    /// @generic.instance id=sliceLength<uint8> template=sliceLength arguments=(uint8)
    /// @generic.instance id=sliceSet<uint8> template=sliceSet arguments=(uint8)
    /// @generic.instance id=startBound#1<isize> template=startBound#1 arguments=(isize)
    /// @generic.instance id=unsafeGet<uint8> template=unsafeGet arguments=(uint8)
    /// @generic.instance id=unsafeSet<uint8> template=unsafeSet arguments=(uint8)
    /// @generic.instance id=Range<isize> template=Range arguments=(isize)
    /// @resolution.name source=header target=header
    /// @resolution.place source=header placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=header root=header

}
"#,
    );
}

#[test]
fn test_split_a_mutable_slice_into_disjoint_views() {
    let session = TestSession::single(
        r#"
declare const values: &[int32];

function halves(): void {
    let (left, right) = values.splitAt(2);
    left.reverse();
    right.reverse();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: &'static [int32];

function halves(): void {
    let (left, right) = values.splitAt<int32, "mutable">(2);
    left.reverse<int32>();
    right.reverse<int32>();
}

=== dir ===
declare const values: &[int32];
/// @type.symbol symbol=values source=values type=&'static constant Slice<int32>
/// @resolution.pattern source=values kind=binding target=values

function halves(): void {
/// @type.symbol symbol=halves type=() => void

    let (left, right) = values.splitAt(2);
    /// @resolution.pattern source=(left, right) kind=tuple fields=(halves.left, halves.right)
    /// @type.symbol symbol=halves.left source=left type=WithAccess<&'static Slice<int32>, "mutable">
    /// @resolution.pattern source=left kind=binding target=halves.left
    /// @generic.instance id="WithAccess<&'bound0 Slice<int32>, \"mutable\">" template=WithAccess arguments=(&'bound0 Slice<int32>, "mutable")
    /// @type.symbol symbol=halves.right source=right type=WithAccess<&'static Slice<int32>, "mutable">
    /// @resolution.pattern source=right kind=binding target=halves.right
    /// @resolution.name source=values target=values
    /// @resolution.member source=values.splitAt receiver=&'static constant Slice<int32> type=<const splitAt.A: Access = "readonly", splitAt.'a>(this: WithAccess<&splitAt.'a Slice<int32>, splitAt.A>, isize) => (WithAccess<&splitAt.'a Slice<int32>, splitAt.A>, WithAccess<&splitAt.'a Slice<int32>, splitAt.A>) kind=symbol target_receiver=&'static constant Slice<int32> target=splitAt
    /// @resolution.call source=values.splitAt(2) parameters=(isize) arguments=(provided(2) as isize) return=(WithAccess<&'static Slice<int32>, "mutable">, WithAccess<&'static Slice<int32>, "mutable">) regions=("static") kind=symbol target=splitAt receiver=&'static constant Slice<int32> instance="Slice<int32>.<extension#1>.splitAt<\"mutable\">"
    /// @resolution.place source=values placement="constant" lifetime="static" access="mutable"
    /// @resolution.access source=values root=values
    /// @generic.instantiation id="splitAt<int32, \"mutable\">" template=splitAt arguments=(int32, "mutable")
    /// @generic.instantiation id=splitAt<int32> template=splitAt arguments=(int32)
    /// @generic.instance id="splitAt<int32, \"mutable\">" template=splitAt arguments=(int32, "mutable")

    left.reverse();
    /// @resolution.name source=left target=halves.left
    /// @resolution.member source=left.reverse receiver=WithAccess<&'static Slice<int32>, "mutable"> type=<reverse.'a>(this: &reverse.'a Slice<int32>) => void kind=symbol target_receiver=WithAccess<&'static Slice<int32>, "mutable"> target=reverse
    /// @resolution.call source=left.reverse() parameters=() return=void regions=("static") kind=symbol target=reverse receiver=WithAccess<&'static Slice<int32>, "mutable"> instance=Slice<int32>.<extension#1>.reverse
    /// @resolution.place source=left placement="static" lifetime="static" access="mutable"
    /// @resolution.access source=left root=halves.left
    /// @generic.instantiation id=reverse<int32> template=reverse arguments=(int32)
    /// @generic.instance id=Slice<int32> template=Slice arguments=(int32)
    /// @generic.instance id=reverse<int32> template=reverse arguments=(int32)

    right.reverse();
    /// @resolution.name source=right target=halves.right
    /// @resolution.member source=right.reverse receiver=WithAccess<&'static Slice<int32>, "mutable"> type=<reverse.'a>(this: &reverse.'a Slice<int32>) => void kind=symbol target_receiver=WithAccess<&'static Slice<int32>, "mutable"> target=reverse
    /// @resolution.call source=right.reverse() parameters=() return=void regions=("static") kind=symbol target=reverse receiver=WithAccess<&'static Slice<int32>, "mutable"> instance=Slice<int32>.<extension#1>.reverse
    /// @resolution.place source=right placement="static" lifetime="static" access="mutable"
    /// @resolution.access source=right root=halves.right

}
"#,
    );
}

#[test]
fn test_borrow_half_open_and_full_range_views() {
    let session = TestSession::single(
        r#"
declare const values: &readonly [float64];
const head = values[..2];
const tail = values[1..];
const whole = values[..];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: &'static readonly [float64];
const head: [float64] = values[..2];
const tail: [float64] = values[1..];
const whole: [float64] = values[..];

=== dir ===
declare const values: &readonly [float64];
/// @type.symbol symbol=values source=values type=&'static readonly constant Slice<float64>
/// @resolution.pattern source=values kind=binding target=values

const head = values[..2];
/// @type.symbol symbol=head source=head type=Slice<float64>
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[..2] type=Slice<float64> kind=call target="index#2(parameters=(RangeTo<isize>), arguments=(provided(..2) as RangeTo<isize>), return=WithAccess<&'static constant Slice<float64>, \"readonly\">, regions=(\"static\" & \"constant\"))"
/// @generic.instantiation id="index#2<float64, RangeTo<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeTo<isize>, "readonly")
/// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="WithAccess<&'bound0 Slice<float64>, \"readonly\">" template=WithAccess arguments=(&'bound0 Slice<float64>, "readonly")
/// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
/// @generic.instance id="index#2<float64, RangeTo<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeTo<isize>, "readonly")
/// @generic.instance id="rangeSpan<float64, RangeTo<isize>, \"readonly\" | \"mutable\">" template=rangeSpan arguments=(float64, RangeTo<isize>, "readonly" | "mutable")
/// @generic.instance id="sliceView<float64, \"readonly\">" template=sliceView arguments=(float64, "readonly")
/// @generic.instance id="subslice<float64, \"readonly\">" template=subslice arguments=(float64, "readonly")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id="unbounded<&'bound0 readonly isize>" template=unbounded arguments=(&'bound0 readonly isize)
/// @generic.instance id=endBound#4<isize> template=endBound#4 arguments=(isize)
/// @generic.instance id=size<float64> template=size arguments=(float64)
/// @generic.instance id=sliceLength<float64> template=sliceLength arguments=(float64)
/// @generic.instance id=startBound#4<isize> template=startBound#4 arguments=(isize)
/// @generic.instance id=RangeTo<isize> template=RangeTo arguments=(isize)

const tail = values[1..];
/// @type.symbol symbol=tail source=tail type=Slice<float64>
/// @resolution.pattern source=tail kind=binding target=tail
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[1..] type=Slice<float64> kind=call target="index#2(parameters=(RangeFrom<isize>), arguments=(provided(1..) as RangeFrom<isize>), return=WithAccess<&'static constant Slice<float64>, \"readonly\">, regions=(\"static\" & \"constant\"))"
/// @generic.instantiation id="index#2<float64, RangeFrom<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeFrom<isize>, "readonly")
/// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
/// @generic.instance id="index#2<float64, RangeFrom<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeFrom<isize>, "readonly")
/// @generic.instance id="rangeSpan<float64, RangeFrom<isize>, \"readonly\" | \"mutable\">" template=rangeSpan arguments=(float64, RangeFrom<isize>, "readonly" | "mutable")
/// @generic.instance id=endBound#3<isize> template=endBound#3 arguments=(isize)
/// @generic.instance id=startBound#3<isize> template=startBound#3 arguments=(isize)
/// @generic.instance id=RangeFrom<isize> template=RangeFrom arguments=(isize)

const whole = values[..];
/// @type.symbol symbol=whole source=whole type=Slice<float64>
/// @resolution.pattern source=whole kind=binding target=whole
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[..] type=Slice<float64> kind=call target="index#2(parameters=(RangeFull), arguments=(provided(..) as RangeFull), return=WithAccess<&'static constant Slice<float64>, \"readonly\">, regions=(\"static\" & \"constant\"))"
/// @generic.instantiation id="index#2<float64, RangeFull, \"readonly\">" template=index#2 arguments=(float64, RangeFull, "readonly")
/// @generic.instance id="index#2<float64, RangeFull, \"readonly\">" template=index#2 arguments=(float64, RangeFull, "readonly")
/// @generic.instance id="rangeSpan<float64, RangeFull, \"readonly\" | \"mutable\">" template=rangeSpan arguments=(float64, RangeFull, "readonly" | "mutable")
/// @generic.instance id=endBound#6<isize> template=endBound#6 arguments=(isize)
/// @generic.instance id=startBound#6<isize> template=startBound#6 arguments=(isize)
"#,
    );
}

#[test]
fn test_replace_an_element_of_a_mutable_slice() {
    let session = TestSession::single(
        r#"
declare const values: &[float64];

function set(): void {
    values[0] = 1.0;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: &'static [float64];

function set(): void {
    values[0] = 1.0;
}

=== dir ===
declare const values: &[float64];
/// @type.symbol symbol=values source=values type=&'static constant Slice<float64>
/// @resolution.pattern source=values kind=binding target=values

function set(): void {
/// @type.symbol symbol=set type=() => void

    values[0] = 1.0;
    /// @resolution.name source=values target=values
    /// @resolution.place source=values placement="constant" lifetime="static" access="mutable"
    /// @resolution.access source=values root=values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, float64), arguments=(provided(0) as isize, supplied as float64), return=void, regions=(\"static\" & \"constant\"))" type=float64
    /// @generic.instantiation id=indexSet#1<float64> template=indexSet#1 arguments=(float64)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="index#1<float64, \"mutable\">" template=index#1 arguments=(float64, "mutable")
    /// @generic.instance id="index#2<float64, RangeBounds<isize>, \"mutable\" | \"readonly\">" template=index#2 arguments=(float64, RangeBounds<isize>, "mutable" | "readonly")
    /// @generic.instance id="rangeSpan<float64, RangeBounds<isize>, \"mutable\" | \"readonly\">" template=rangeSpan arguments=(float64, RangeBounds<isize>, "mutable" | "readonly")
    /// @generic.instance id="sliceIndex<float64, \"mutable\">" template=sliceIndex arguments=(float64, "mutable")
    /// @generic.instance id="sliceView<float64, \"mutable\" | \"readonly\">" template=sliceView arguments=(float64, "mutable" | "readonly")
    /// @generic.instance id="subslice<float64, \"mutable\" | \"readonly\">" template=subslice arguments=(float64, "mutable" | "readonly")
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=Slice<float64> template=Slice arguments=(float64)
    /// @generic.instance id=indexSet#1<float64> template=indexSet#1 arguments=(float64)
    /// @generic.instance id=size<float64> template=size arguments=(float64)
    /// @generic.instance id=sliceLength<float64> template=sliceLength arguments=(float64)
    /// @generic.instance id=symbol2<float64> template=symbol2 arguments=(float64)

}
"#,
    );
}

#[test]
fn test_copy_into_a_range_of_an_array() {
    let session = TestSession::single(
        r#"
declare const values: float64[];
declare const source: &readonly [float64];

function overwrite(): void {
    values[0..2] = source;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: float64[];
declare const source: &'static readonly [float64];

function overwrite(): void {
    values[0..2] = source;
}

=== dir ===
declare const values: float64[];
/// @type.symbol symbol=values source=values type=float64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="elementSlot<float64, \"mutable\">" template=elementSlot arguments=(float64, "mutable")
/// @generic.instance id="initAsPointer<float64, \"mutable\">" template=initAsPointer arguments=(float64, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<float64>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<float64>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=assumeInitDrop#1<float64> template=assumeInitDrop#1 arguments=(float64)
/// @generic.instance id=assumeInitDrop<float64> template=assumeInitDrop arguments=(float64)
/// @generic.instance id=clear<float64> template=clear arguments=(float64)
/// @generic.instance id=drop<float64> template=drop arguments=(float64)
/// @generic.instance id=dropInPlace<float64> template=dropInPlace arguments=(float64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<float64>> template=sliceAssumeInit arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceUninit<MaybeUninit<float64>> template=sliceUninit arguments=(MaybeUninit<float64>)
/// @generic.instance id=truncate<float64> template=truncate arguments=(float64)

declare const source: &readonly [float64];
/// @type.symbol symbol=source source=source type=&'static readonly constant Slice<float64>
/// @resolution.pattern source=source kind=binding target=source

function overwrite(): void {
/// @type.symbol symbol=overwrite type=() => void

    values[0..2] = source;
    /// @resolution.name source=values target=values
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=values
    /// @resolution.pattern.assign source=values[0..2] kind=place
    /// @resolution.assignment source=values[0..2] write="indexSet#2(parameters=(Range<isize>, &'static readonly constant Slice<float64>), arguments=(provided(0..2) as Range<isize>, supplied as &'static readonly constant Slice<float64>), return=void, regions=(\"frame\", \"managed\" & \"local\", \"static\" & \"constant\"))" type=&'static readonly constant Slice<float64>
    /// @generic.instantiation id="indexSet#2<float64, Range<isize>>" template=indexSet#2 arguments=(float64, Range<isize>)
    /// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="as<float64, \"mutable\">" template=as arguments=(float64, "mutable")
    /// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
    /// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
    /// @generic.instance id="index#2<float64, Range<isize>, \"mutable\">" template=index#2 arguments=(float64, Range<isize>, "mutable")
    /// @generic.instance id="indexSet#2<float64, Range<isize>>" template=indexSet#2 arguments=(float64, Range<isize>)
    /// @generic.instance id="rangeSpan<float64, Range<isize>, \"readonly\" | \"mutable\">" template=rangeSpan arguments=(float64, Range<isize>, "readonly" | "mutable")
    /// @generic.instance id="sliceView<float64, \"mutable\">" template=sliceView arguments=(float64, "mutable")
    /// @generic.instance id="subslice<float64, \"mutable\">" template=subslice arguments=(float64, "mutable")
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=copyFrom<float64> template=copyFrom arguments=(float64)
    /// @generic.instance id=endBound#1<isize> template=endBound#1 arguments=(isize)
    /// @generic.instance id=size<float64> template=size arguments=(float64)
    /// @generic.instance id=sliceGet<float64> template=sliceGet arguments=(float64)
    /// @generic.instance id=sliceLength<float64> template=sliceLength arguments=(float64)
    /// @generic.instance id=sliceSet<float64> template=sliceSet arguments=(float64)
    /// @generic.instance id=startBound#1<isize> template=startBound#1 arguments=(isize)
    /// @generic.instance id=unsafeGet<float64> template=unsafeGet arguments=(float64)
    /// @generic.instance id=unsafeSet<float64> template=unsafeSet arguments=(float64)
    /// @generic.instance id=Range<isize> template=Range arguments=(isize)
    /// @resolution.name source=source target=source
    /// @resolution.place source=source placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=source root=source

}
"#,
    );
}

#[test]
fn test_reject_an_unstable_element_write_through_a_mutable_view() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

function updateNumbers(values: &[int32]): void {
    values[0] = 1;
}

function updateStatuses(values: &[Status]): void {
    values[0] = Status.Busy;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}

function updateNumbers<'a>(values: &'a [int32]): void {
    values[0] = 1;
}

function updateStatuses<'a>(values: &'a [Status]): void {
    values[0] = Status.Busy;
}

=== dir ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

function updateNumbers(values: &[int32]): void {
/// @generic.template symbol=updateNumbers parameters=('a)
/// @type.symbol symbol=updateNumbers type=<updateNumbers.'a>(&updateNumbers.'a Slice<int32>) => void
/// @type.symbol symbol=updateNumbers.values source="values: &[int32]" type=&updateNumbers.'a Slice<int32>

    values[0] = 1;
    /// @resolution.name source=values target=updateNumbers.values
    /// @resolution.place source=values placement=updateNumbers.'a lifetime=updateNumbers.'a access="mutable"
    /// @resolution.access source=values root=updateNumbers.values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, int32), arguments=(provided(0) as isize, supplied as int32), return=void, regions=(updateNumbers.'a))" type=int32
    /// @generic.instantiation id=indexSet#1<int32> template=indexSet#1 arguments=(int32)

}

function updateStatuses(values: &[Status]): void {
/// @generic.template symbol=updateStatuses parameters=('a)
/// @type.symbol symbol=updateStatuses type=<updateStatuses.'a>(&updateStatuses.'a Slice<Status>) => void
/// @type.symbol symbol=updateStatuses.values source="values: &[Status]" type=&updateStatuses.'a Slice<Status>
/// @resolution.name source=Status target=Status

    values[0] = Status.Busy;
    /// @resolution.name source=values target=updateStatuses.values
    /// @resolution.place source=values placement=updateStatuses.'a lifetime=updateStatuses.'a access="mutable"
    /// @resolution.access source=values root=updateStatuses.values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, Status), arguments=(provided(0) as isize, supplied as Status), return=void, regions=(updateStatuses.'a))" type=Status
    /// @generic.instantiation id=indexSet#1<Status> template=indexSet#1 arguments=(Status)
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_select_the_conformance_implementation_for_a_bound_call() {
    let session = TestSession::single(
        r#"
interface Greet {
    greet(&readonly this): int32;
}

struct Cell {
    value: int32;
}

export extension of Cell implements Greet {
    greet(&readonly this): int32 {
        this.value
    }
}

function invoke<T: Greet>(value: &readonly T): int32 {
    return value.greet();
}

export function run(): int32 {
    const cell = Cell { value: 3 };
    return invoke(&readonly cell);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Greet {
    greet(&readonly this): int32;
}

struct Cell {
    value: int32;
}

export extension of Cell implements Greet {
    greet(&readonly this): int32 {
        this.value
    }
}

function invoke<T: Greet, 'a>(value: &'a readonly T): int32 {
    return value.greet();
}

export function run(): int32 {
    const cell: Cell = Cell { value: 3 };
    return invoke<Cell>(&readonly cell);
}

=== dir ===
interface Greet {
/// @generic.template symbol=Greet parameters=(this: Greet)
/// @type.symbol symbol=Greet type=Greet
/// @definition.interface symbol=Greet template=(this: Greet)
/// @definition.where symbol=Greet relation=satisfies left=this right=Greet
/// @definition.method symbol=Greet.greet source="greet(&readonly this): int32" slot=greet type=<Greet.greet.'a>(this: &Greet.greet.'a readonly this) => int32

    greet(&readonly this): int32;
    /// @generic.template symbol=Greet.greet parent=template#0 parameters=('a)
    /// @type.symbol symbol=Greet.greet source="greet(&readonly this): int32" type=<Greet.greet.'a>(this: &Greet.greet.'a readonly this) => int32
    /// @type.symbol symbol=Greet.greet.this source="&readonly this" type=&Greet.greet.'a readonly this

}

struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

export extension of Cell implements Greet {
/// @definition.extension symbol=<module>#2 form=exported target=Cell
/// @definition.implements symbol=<module>#2 source=Greet target=Greet
/// @definition.method symbol=greet slot=greet type=<greet.'a>(this: &greet.'a readonly Cell) => int32
/// @definition.conformance symbol=<module>#2 member=greet requirement=Greet.greet
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Greet target=Greet

    greet(&readonly this): int32 {
    /// @generic.template symbol=greet parent=template#1 parameters=('a)
    /// @type.symbol symbol=greet type=<greet.'a>(this: &greet.'a readonly Cell) => int32
    /// @type.symbol symbol=greet.this source="&readonly this" type=&greet.'a readonly this

        this.value
        /// @resolution.member source=this.value receiver=&greet.'a readonly Cell type=int32 kind=field target_receiver=&greet.'a readonly Cell key=value target=Cell.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&greet.'a readonly Cell
        /// @resolution.place source=this placement=greet.'a lifetime=greet.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=greet.'a lifetime=greet.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

function invoke<T: Greet>(value: &readonly T): int32 {
/// @generic.template symbol=invoke parameters=(T: Greet, 'a)
/// @type.symbol symbol=invoke type=<T: Greet, invoke.'a>(&invoke.'a readonly T) => int32
/// @type.symbol symbol=invoke.T source="T: Greet" type=T
/// @resolution.name source=Greet target=Greet
/// @type.symbol symbol=invoke.value source="value: &readonly T" type=&invoke.'a readonly T
/// @resolution.name source=T target=invoke.T

    return value.greet();
    /// @resolution.name source=value target=invoke.value
    /// @resolution.member source=value.greet receiver=&invoke.'a readonly T type=<Greet.greet.'a>(this: &Greet.greet.'a readonly T) => int32 kind=symbol target_receiver=&invoke.'a readonly T target=Greet.greet
    /// @resolution.call source=value.greet() parameters=() return=int32 regions=(invoke.'a) kind=symbol target=Greet.greet receiver=&invoke.'a readonly T
    /// @resolution.place source=value placement=invoke.'a lifetime=invoke.'a access="readonly"
    /// @resolution.access source=value root=invoke.value
    /// @generic.instantiation id=Greet.greet<T> template=Greet.greet arguments=() owner=invoke
    /// @generic.instance id=Greet.greet<T> template=Greet.greet arguments=()

}

export function run(): int32 {
/// @type.symbol symbol=run type=() => int32

    const cell = Cell { value: 3 };
    /// @type.symbol symbol=run.cell source=cell type=Cell
    /// @resolution.pattern source=cell kind=binding target=run.cell
    /// @resolution.name source=Cell target=Cell

    return invoke(&readonly cell);
    /// @resolution.name source=invoke target=invoke
    /// @resolution.call source="invoke(&readonly cell)" parameters=(&'frame readonly Cell) arguments=(provided(&readonly cell) as &'frame readonly Cell) return=int32 regions=("frame" & "local") kind=symbol target=invoke instance=invoke<Cell>
    /// @generic.instantiation id=invoke<Cell> template=invoke arguments=(Cell)
    /// @generic.instance id=invoke<Cell> template=invoke arguments=(Cell)
    /// @resolution.name source=cell target=run.cell
    /// @resolution.place source=cell placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=cell root=run.cell

}
"#,
    );
}
