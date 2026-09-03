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
        DirRows::checked().with_reference_types(),
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
/// @resolution.subscript source=bytes[1..3] type=Slice<uint8> kind=call target="index#2(parameters=(Range<isize>), arguments=(provided(1..3) as Range<isize>), return=&'static readonly constant Slice<uint8>)"
/// @generic.instantiation id="index#2<uint8, 4, Range<isize>, \"readonly\">" template=index#2 arguments=(uint8, 4, Range<isize>, "readonly")
/// @generic.instance id="FixedArray<uint8, 4>" template=FixedArray arguments=(uint8, 4)
/// @generic.instance id="index#2<uint8, 4, Range<isize>, \"readonly\">" template=index#2 arguments=(uint8, 4, Range<isize>, "readonly")
/// @type.node source=1 type=1
/// @type.node source=1..3 type=Range<isize>
/// @generic.instance id=Range<isize> template=Range arguments=(isize)
/// @type.node source=3 type=3
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
/// @resolution.subscript source=values[1..3] type=Slice<float64> kind=call target="index#2(parameters=(Range<isize>), arguments=(provided(1..3) as Range<isize>), return=WithAccess<&'static constant Slice<float64>, \"readonly\">)"
/// @generic.instantiation id="index#2<float64, Range<isize>, \"readonly\">" template=index#2 arguments=(float64, Range<isize>, "readonly")
/// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="endBound#1<Range<isize>, isize>" template=endBound#1 arguments=(isize)
/// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
/// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
/// @generic.instance id="index#2<float64, Range<isize>, \"readonly\">" template=index#2 arguments=(float64, Range<isize>, "readonly") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: &index#2.'a readonly Slice<float64>, Range<isize>) => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: &index#2.'a readonly Slice<float64>, usize, usize) => &index#2.'a readonly Slice<float64>)
/// @generic.instance id="rangeSpan<float64, Range<isize>, \"readonly\" | \"exclusive\" | \"mutable\">" template=rangeSpan arguments=(float64, Range<isize>, "readonly" | "exclusive" | "mutable")
/// @generic.instance id="sliceView<float64, \"readonly\">" template=sliceView arguments=(float64, "readonly") evaluated=(<sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A>, usize, usize) => WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A> => <sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(&sliceView.'a readonly Slice<float64>, usize, usize) => &sliceView.'a readonly Slice<float64>)
/// @generic.instance id="startBound#1<Range<isize>, isize>" template=startBound#1 arguments=(isize)
/// @generic.instance id="subslice<float64, \"readonly\">" template=subslice arguments=(float64, "readonly") evaluated=(<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => <const subslice.A: Access = "readonly", subslice.'a>(this: &subslice.'a readonly Slice<float64>, usize, usize) => &subslice.'a readonly Slice<float64>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a readonly Slice<float64>, (WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => (&subslice.'a readonly Slice<float64>, usize, usize) => &subslice.'a readonly Slice<float64>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a readonly Slice<float64>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Slice<float64> template=Slice arguments=(float64)
/// @generic.instance id=size<float64> template=size arguments=(float64)
/// @generic.instance id=sliceLength<float64> template=sliceLength arguments=(float64)
/// @generic.instance id=Range<isize> template=Range arguments=(isize)
"#,
    );
}

#[test]
fn test_copy_into_a_range_of_an_exclusive_slice() {
    let session = TestSession::single(
        r#"
declare const buffer: &exclusive [uint8];
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
declare const buffer: &'static exclusive [uint8];
declare const header: &'static readonly [uint8];

function stamp(): void {
    buffer[0..2] = header;
}

=== dir ===
declare const buffer: &exclusive [uint8];
/// @type.symbol symbol=buffer source=buffer type=&'static exclusive constant Slice<uint8>
/// @resolution.pattern source=buffer kind=binding target=buffer

declare const header: &readonly [uint8];
/// @type.symbol symbol=header source=header type=&'static readonly constant Slice<uint8>
/// @resolution.pattern source=header kind=binding target=header

function stamp(): void {
/// @type.symbol symbol=stamp type=() => void

    buffer[0..2] = header;
    /// @resolution.name source=buffer target=buffer
    /// @resolution.place source=buffer placement="constant" lifetime="static" access="exclusive"
    /// @resolution.access source=buffer root=buffer
    /// @resolution.pattern.assign source=buffer[0..2] kind=place
    /// @resolution.assignment source=buffer[0..2] write="indexSet#2(parameters=(Range<isize>, &'static readonly constant Slice<uint8>), arguments=(provided(0..2) as Range<isize>, supplied as &'static readonly constant Slice<uint8>), return=void)" type=&'static readonly constant Slice<uint8>
    /// @generic.instantiation id="indexSet#2<uint8, Range<isize>>" template=indexSet#2 arguments=(uint8, Range<isize>)
    /// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="endBound#1<Range<isize>, isize>" template=endBound#1 arguments=(isize)
    /// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
    /// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
    /// @generic.instance id="index#2<uint8, Range<isize>, \"mutable\">" template=index#2 arguments=(uint8, Range<isize>, "mutable") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: &index#2.'a Slice<uint8>, Range<isize>) => &index#2.'a Slice<uint8>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a Slice<uint8>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a Slice<uint8>, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: &index#2.'a Slice<uint8>, usize, usize) => &index#2.'a Slice<uint8>)
    /// @generic.instance id="indexSet#2<uint8, Range<isize>>" template=indexSet#2 arguments=(uint8, Range<isize>) evaluated=(WithAccess<&indexSet#2.'a Slice<T#7>, "mutable"> => &indexSet#2.'a Slice<uint8>, (this: WithAccess<&indexSet#2.'a Slice<T#7>, "mutable">, R#2) => WithAccess<&indexSet#2.'a Slice<T#7>, "mutable"> => (this: &indexSet#2.'a Slice<uint8>, Range<isize>) => &indexSet#2.'a Slice<uint8>, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a Slice<T#7>, index.A>, isize) => WithAccess<&index#1.'a T#7, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a Slice<T#7>, "mutable">, R#2) => WithAccess<&index#2.'a Slice<T#7>, "mutable"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a Slice<uint8>, index.A>, isize) => WithAccess<&index#1.'a uint8, index.A> & <index#2.'a>(this: &index#2.'a Slice<uint8>, Range<isize>) => &index#2.'a Slice<uint8>, <index#2.'a>(this: WithAccess<&index#2.'a Slice<T#7>, "mutable">, R#2) => WithAccess<&index#2.'a Slice<T#7>, "mutable"> => <index#2.'a>(this: &index#2.'a Slice<uint8>, Range<isize>) => &index#2.'a Slice<uint8>)
    /// @generic.instance id="rangeSpan<uint8, Range<isize>, \"exclusive\" | \"readonly\" | \"mutable\">" template=rangeSpan arguments=(uint8, Range<isize>, "exclusive" | "readonly" | "mutable")
    /// @generic.instance id="sliceView<uint8, \"mutable\">" template=sliceView arguments=(uint8, "mutable") evaluated=(<sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A>, usize, usize) => WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A> => <sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(&sliceView.'a Slice<uint8>, usize, usize) => &sliceView.'a Slice<uint8>)
    /// @generic.instance id="startBound#1<Range<isize>, isize>" template=startBound#1 arguments=(isize)
    /// @generic.instance id="subslice<uint8, \"mutable\">" template=subslice arguments=(uint8, "mutable") evaluated=(<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => <const subslice.A: Access = "readonly", subslice.'a>(this: &subslice.'a Slice<uint8>, usize, usize) => &subslice.'a Slice<uint8>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a Slice<uint8>, (WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => (&subslice.'a Slice<uint8>, usize, usize) => &subslice.'a Slice<uint8>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a Slice<uint8>)
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=Slice<uint8> template=Slice arguments=(uint8)
    /// @generic.instance id=copyFrom<uint8> template=copyFrom arguments=(uint8)
    /// @generic.instance id=size<uint8> template=size arguments=(uint8)
    /// @generic.instance id=sliceGet<uint8> template=sliceGet arguments=(uint8)
    /// @generic.instance id=sliceLength<uint8> template=sliceLength arguments=(uint8)
    /// @generic.instance id=sliceSet<uint8> template=sliceSet arguments=(uint8)
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
fn test_split_an_exclusive_slice_into_disjoint_views() {
    let session = TestSession::single(
        r#"
declare const values: &exclusive [int32];

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
declare const values: &'static exclusive [int32];

function halves(): void {
    let (left, right) = values.splitAt<int32, "exclusive">(2);
    left.reverse<int32>();
    right.reverse<int32>();
}

=== dir ===
declare const values: &exclusive [int32];
/// @type.symbol symbol=values source=values type=&'static exclusive constant Slice<int32>
/// @resolution.pattern source=values kind=binding target=values

function halves(): void {
/// @type.symbol symbol=halves type=() => void

    let (left, right) = values.splitAt(2);
    /// @resolution.pattern source=(left, right) kind=tuple fields=(halves.left, halves.right)
    /// @type.symbol symbol=halves.left source=left type=&'static exclusive constant Slice<int32>
    /// @resolution.pattern source=left kind=binding target=halves.left
    /// @type.symbol symbol=halves.right source=right type=&'static exclusive constant Slice<int32>
    /// @resolution.pattern source=right kind=binding target=halves.right
    /// @resolution.name source=values target=values
    /// @resolution.member source=values.splitAt receiver=&'static exclusive constant Slice<int32> type=<const splitAt.A: Access = "readonly", splitAt.'a>(this: WithAccess<&splitAt.'a Slice<int32>, splitAt.A>, isize) => (WithAccess<&splitAt.'a Slice<int32>, splitAt.A>, WithAccess<&splitAt.'a Slice<int32>, splitAt.A>) kind=symbol target_receiver=&'static exclusive constant Slice<int32> target=splitAt
    /// @resolution.call source=values.splitAt(2) parameters=(isize) arguments=(provided(2) as isize) return=(WithAccess<&'static constant Slice<int32>, "exclusive">, WithAccess<&'static constant Slice<int32>, "exclusive">) kind=symbol target=splitAt receiver=&'static exclusive constant Slice<int32> instance="Slice<int32>.<extension#1>.splitAt<\"exclusive\">"
    /// @resolution.place source=values placement="constant" lifetime="static" access="exclusive"
    /// @resolution.access source=values root=values
    /// @generic.instantiation id="splitAt<int32, \"exclusive\">" template=splitAt arguments=(int32, "exclusive")
    /// @generic.instantiation id=splitAt<int32> template=splitAt arguments=(int32)
    /// @generic.instance id="splitAt<int32, \"exclusive\">" template=splitAt arguments=(int32, "exclusive") evaluated=(<const splitAt.A: Access = "readonly", splitAt.'a>(this: WithAccess<&splitAt.'a Slice<T#1>, splitAt.A>, isize) => (WithAccess<&splitAt.'a Slice<T#1>, splitAt.A>, WithAccess<&splitAt.'a Slice<T#1>, splitAt.A>) => <const splitAt.A: Access = "readonly", splitAt.'a>(this: &splitAt.'a exclusive Slice<int32>, isize) => (&splitAt.'a exclusive Slice<int32>, &splitAt.'a exclusive Slice<int32>))

    left.reverse();
    /// @resolution.name source=left target=halves.left
    /// @resolution.member source=left.reverse receiver=WithAccess<&'static constant Slice<int32>, "exclusive"> type=<reverse.'a>(this: &reverse.'a exclusive Slice<int32>) => void kind=symbol target_receiver=WithAccess<&'static constant Slice<int32>, "exclusive"> target=reverse
    /// @resolution.call source=left.reverse() parameters=() return=void kind=symbol target=reverse receiver=WithAccess<&'static constant Slice<int32>, "exclusive"> instance=Slice<int32>.<extension#1>.reverse
    /// @resolution.place source=left placement="static" & "constant" lifetime="static" & "constant" access="exclusive"
    /// @resolution.access source=left root=halves.left
    /// @generic.instantiation id=reverse<int32> template=reverse arguments=(int32)
    /// @generic.instance id=Slice<int32> template=Slice arguments=(int32)
    /// @generic.instance id=reverse<int32> template=reverse arguments=(int32)

    right.reverse();
    /// @resolution.name source=right target=halves.right
    /// @resolution.member source=right.reverse receiver=WithAccess<&'static constant Slice<int32>, "exclusive"> type=<reverse.'a>(this: &reverse.'a exclusive Slice<int32>) => void kind=symbol target_receiver=WithAccess<&'static constant Slice<int32>, "exclusive"> target=reverse
    /// @resolution.call source=right.reverse() parameters=() return=void kind=symbol target=reverse receiver=WithAccess<&'static constant Slice<int32>, "exclusive"> instance=Slice<int32>.<extension#1>.reverse
    /// @resolution.place source=right placement="static" & "constant" lifetime="static" & "constant" access="exclusive"
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
/// @resolution.subscript source=values[..2] type=Slice<float64> kind=call target="index#2(parameters=(RangeTo<isize>), arguments=(provided(..2) as RangeTo<isize>), return=WithAccess<&'static constant Slice<float64>, \"readonly\">)"
/// @generic.instantiation id="index#2<float64, RangeTo<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeTo<isize>, "readonly")
/// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="endBound#4<RangeTo<isize>, isize>" template=endBound#4 arguments=(isize)
/// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
/// @generic.instance id="index#2<float64, RangeTo<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeTo<isize>, "readonly") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: &index#2.'a readonly Slice<float64>, RangeTo<isize>) => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: &index#2.'a readonly Slice<float64>, usize, usize) => &index#2.'a readonly Slice<float64>)
/// @generic.instance id="rangeSpan<float64, RangeTo<isize>, \"readonly\" | \"exclusive\" | \"mutable\">" template=rangeSpan arguments=(float64, RangeTo<isize>, "readonly" | "exclusive" | "mutable")
/// @generic.instance id="sliceView<float64, \"readonly\">" template=sliceView arguments=(float64, "readonly") evaluated=(<sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A>, usize, usize) => WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A> => <sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(&sliceView.'a readonly Slice<float64>, usize, usize) => &sliceView.'a readonly Slice<float64>)
/// @generic.instance id="startBound#4<RangeTo<isize>, isize>" template=startBound#4 arguments=(isize)
/// @generic.instance id="subslice<float64, \"readonly\">" template=subslice arguments=(float64, "readonly") evaluated=(<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => <const subslice.A: Access = "readonly", subslice.'a>(this: &subslice.'a readonly Slice<float64>, usize, usize) => &subslice.'a readonly Slice<float64>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a readonly Slice<float64>, (WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => (&subslice.'a readonly Slice<float64>, usize, usize) => &subslice.'a readonly Slice<float64>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a readonly Slice<float64>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id="unbounded<&'bound0 readonly isize>" template=unbounded arguments=(&'bound0 readonly isize)
/// @generic.instance id=Slice<float64> template=Slice arguments=(float64)
/// @generic.instance id=size<float64> template=size arguments=(float64)
/// @generic.instance id=sliceLength<float64> template=sliceLength arguments=(float64)
/// @generic.instance id=RangeTo<isize> template=RangeTo arguments=(isize)

const tail = values[1..];
/// @type.symbol symbol=tail source=tail type=Slice<float64>
/// @resolution.pattern source=tail kind=binding target=tail
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[1..] type=Slice<float64> kind=call target="index#2(parameters=(RangeFrom<isize>), arguments=(provided(1..) as RangeFrom<isize>), return=WithAccess<&'static constant Slice<float64>, \"readonly\">)"
/// @generic.instantiation id="index#2<float64, RangeFrom<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeFrom<isize>, "readonly")
/// @generic.instance id="endBound#3<RangeFrom<isize>, isize>" template=endBound#3 arguments=(isize)
/// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
/// @generic.instance id="index#2<float64, RangeFrom<isize>, \"readonly\">" template=index#2 arguments=(float64, RangeFrom<isize>, "readonly") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: &index#2.'a readonly Slice<float64>, RangeFrom<isize>) => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: &index#2.'a readonly Slice<float64>, usize, usize) => &index#2.'a readonly Slice<float64>)
/// @generic.instance id="rangeSpan<float64, RangeFrom<isize>, \"readonly\" | \"exclusive\" | \"mutable\">" template=rangeSpan arguments=(float64, RangeFrom<isize>, "readonly" | "exclusive" | "mutable")
/// @generic.instance id="startBound#3<RangeFrom<isize>, isize>" template=startBound#3 arguments=(isize)
/// @generic.instance id=RangeFrom<isize> template=RangeFrom arguments=(isize)

const whole = values[..];
/// @type.symbol symbol=whole source=whole type=Slice<float64>
/// @resolution.pattern source=whole kind=binding target=whole
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[..] type=Slice<float64> kind=call target="index#2(parameters=(RangeFull), arguments=(provided(..) as RangeFull), return=WithAccess<&'static constant Slice<float64>, \"readonly\">)"
/// @generic.instantiation id="index#2<float64, RangeFull, \"readonly\">" template=index#2 arguments=(float64, RangeFull, "readonly")
/// @generic.instance id="RangeBounds.endBound<RangeFull, isize>" template=RangeBounds.endBound arguments=(isize)
/// @generic.instance id="RangeBounds.startBound<RangeFull, isize>" template=RangeBounds.startBound arguments=(isize)
/// @generic.instance id="index#2<float64, RangeFull, \"readonly\">" template=index#2 arguments=(float64, RangeFull, "readonly") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: &index#2.'a readonly Slice<float64>, RangeFull) => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a readonly Slice<float64>, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: &index#2.'a readonly Slice<float64>, usize, usize) => &index#2.'a readonly Slice<float64>)
/// @generic.instance id="rangeSpan<float64, RangeFull, \"readonly\" | \"exclusive\" | \"mutable\">" template=rangeSpan arguments=(float64, RangeFull, "readonly" | "exclusive" | "mutable")
"#,
    );
}

#[test]
fn test_replace_an_element_of_an_exclusive_slice() {
    let session = TestSession::single(
        r#"
declare const values: &exclusive [float64];

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
declare const values: &'static exclusive [float64];

function set(): void {
    values[0] = 1.0;
}

=== dir ===
declare const values: &exclusive [float64];
/// @type.symbol symbol=values source=values type=&'static exclusive constant Slice<float64>
/// @resolution.pattern source=values kind=binding target=values

function set(): void {
/// @type.symbol symbol=set type=() => void

    values[0] = 1.0;
    /// @resolution.name source=values target=values
    /// @resolution.place source=values placement="constant" lifetime="static" access="exclusive"
    /// @resolution.access source=values root=values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, float64), arguments=(provided(0) as isize, supplied as float64), return=void)" type=float64
    /// @generic.instantiation id=indexSet#1<float64> template=indexSet#1 arguments=(float64)
    /// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="RangeBounds.endBound<RangeBounds<isize>, isize>" template=RangeBounds.endBound arguments=(isize)
    /// @generic.instance id="RangeBounds.startBound<RangeBounds<isize>, isize>" template=RangeBounds.startBound arguments=(isize)
    /// @generic.instance id="index#1<float64, \"exclusive\">" template=index#1 arguments=(float64, "exclusive") evaluated=(<const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a Slice<T#5>, index.A>, isize) => WithAccess<&index#1.'a T#5, index.A> => <const index.A: Access = "readonly", index#1.'a>(this: &index#1.'a exclusive Slice<float64>, isize) => &index#1.'a exclusive float64, WithAccess<&index#1.'a T#5, index.A> => &index#1.'a exclusive float64, WithAccess<&index#1.'a Slice<T#5>, index.A> => &index#1.'a exclusive Slice<float64>, (WithAccess<&index#1.'a Slice<T#5>, index.A>, usize) => WithAccess<&index#1.'a T#5, index.A> => (&index#1.'a exclusive Slice<float64>, usize) => &index#1.'a exclusive float64, WithAccess<&index#1.'a Slice<T#5>, index.A> => &index#1.'a exclusive Slice<float64>)
    /// @generic.instance id="index#2<float64, RangeBounds<isize>, \"exclusive\" | \"readonly\" | \"mutable\">" template=index#2 arguments=(float64, RangeBounds<isize>, "exclusive" | "readonly" | "mutable") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: Borrowed<Slice<float64>, index#2.'a, "exclusive" | "readonly" | "mutable">, RangeBounds<isize>) => Borrowed<Slice<float64>, index#2.'a, "exclusive" | "readonly" | "mutable">, WithAccess<&index#2.'a Slice<T#6>, A#2> => Borrowed<Slice<float64>, index#2.'a, "exclusive" | "readonly" | "mutable">, WithAccess<&index#2.'a Slice<T#6>, A#2> => Borrowed<Slice<float64>, index#2.'a, "exclusive" | "readonly" | "mutable">, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: Borrowed<Slice<float64>, index#2.'a, "exclusive" | "readonly" | "mutable">, usize, usize) => Borrowed<Slice<float64>, index#2.'a, "exclusive" | "readonly" | "mutable">)
    /// @generic.instance id="rangeSpan<float64, RangeBounds<isize>, \"exclusive\" | \"readonly\" | \"mutable\">" template=rangeSpan arguments=(float64, RangeBounds<isize>, "exclusive" | "readonly" | "mutable")
    /// @generic.instance id="sliceIndex<float64, \"exclusive\">" template=sliceIndex arguments=(float64, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<float64>, usize) => &sliceIndex.'a exclusive float64)
    /// @generic.instance id="sliceView<float64, \"exclusive\" | \"readonly\" | \"mutable\">" template=sliceView arguments=(float64, "exclusive" | "readonly" | "mutable") evaluated=(<sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A>, usize, usize) => WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A> => <sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(Borrowed<Slice<float64>, sliceView.'a, "exclusive" | "readonly" | "mutable">, usize, usize) => Borrowed<Slice<float64>, sliceView.'a, "exclusive" | "readonly" | "mutable">)
    /// @generic.instance id="subslice<float64, \"exclusive\" | \"readonly\" | \"mutable\">" template=subslice arguments=(float64, "exclusive" | "readonly" | "mutable") evaluated=(<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => <const subslice.A: Access = "readonly", subslice.'a>(this: Borrowed<Slice<float64>, subslice.'a, "exclusive" | "readonly" | "mutable">, usize, usize) => Borrowed<Slice<float64>, subslice.'a, "exclusive" | "readonly" | "mutable">, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => Borrowed<Slice<float64>, subslice.'a, "exclusive" | "readonly" | "mutable">, (WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => (Borrowed<Slice<float64>, subslice.'a, "exclusive" | "readonly" | "mutable">, usize, usize) => Borrowed<Slice<float64>, subslice.'a, "exclusive" | "readonly" | "mutable">, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => Borrowed<Slice<float64>, subslice.'a, "exclusive" | "readonly" | "mutable">)
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=RangeBounds<isize> template=RangeBounds arguments=(isize)
    /// @generic.instance id=Slice<float64> template=Slice arguments=(float64)
    /// @generic.instance id=indexSet#1<float64> template=indexSet#1 arguments=(float64) evaluated=(WithAccess<&indexSet#1.'a T#5, "exclusive"> => &indexSet#1.'a exclusive float64, (this: WithAccess<&indexSet#1.'a Slice<T#5>, "exclusive">, isize) => WithAccess<&indexSet#1.'a T#5, "exclusive"> => (this: &indexSet#1.'a exclusive Slice<float64>, isize) => &indexSet#1.'a exclusive float64, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a Slice<T#5>, index.A>, isize) => WithAccess<&index#1.'a T#5, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a Slice<T#5>, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#5>, "readonly" | "mutable" | "exclusive"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a Slice<float64>, index.A>, isize) => WithAccess<&index#1.'a float64, index.A> & <index#2.'a>(this: Borrowed<Slice<float64>, index#2.'a, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => Borrowed<Slice<float64>, index#2.'a, "readonly" | "mutable" | "exclusive">, <index#2.'a>(this: WithAccess<&index#2.'a Slice<T#5>, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#5>, "readonly" | "mutable" | "exclusive"> => <index#2.'a>(this: Borrowed<Slice<float64>, index#2.'a, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => Borrowed<Slice<float64>, index#2.'a, "readonly" | "mutable" | "exclusive">, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a Slice<T#5>, index.A>, isize) => WithAccess<&index#1.'a T#5, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a Slice<T#5>, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => WithAccess<&index#2.'a Slice<T#5>, "readonly" | "mutable" | "exclusive"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a Slice<float64>, index.A>, isize) => WithAccess<&index#1.'a float64, index.A> & <index#2.'a>(this: Borrowed<Slice<float64>, index#2.'a, "readonly" | "mutable" | "exclusive">, RangeBounds<isize>) => Borrowed<Slice<float64>, index#2.'a, "readonly" | "mutable" | "exclusive">)
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
/// @generic.instance id="elementSlot<float64, \"exclusive\">" template=elementSlot arguments=(float64, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive float64[], usize) => &elementSlot.'a exclusive MaybeUninit<float64>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<float64>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<float64>>, usize) => &elementSlot.'a exclusive MaybeUninit<float64>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<float64>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<float64>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<float64>>, usize) => &elementSlot.'a exclusive MaybeUninit<float64>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive float64[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<float64>>)
/// @generic.instance id="initAsPointer<float64, \"exclusive\">" template=initAsPointer arguments=(float64, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<float64>) => Raw<float64>)
/// @generic.instance id="sliceIndex<MaybeUninit<float64>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<float64>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<float64>>, usize) => &sliceIndex.'a exclusive MaybeUninit<float64>)
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=MaybeUninit<MaybeUninit<float64>> template=MaybeUninit arguments=(MaybeUninit<float64>)
/// @generic.instance id=MaybeUninit<float64> template=MaybeUninit arguments=(float64)
/// @generic.instance id=assumeInitDrop#1<float64> template=assumeInitDrop#1 arguments=(float64)
/// @generic.instance id=assumeInitDrop<float64> template=assumeInitDrop arguments=(float64) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<float64>) => Raw<float64>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<float64>)
/// @generic.instance id=clear<float64> template=clear arguments=(float64)
/// @generic.instance id=drop<float64> template=drop arguments=(float64)
/// @generic.instance id=dropInPlace<float64> template=dropInPlace arguments=(float64)
/// @generic.instance id=new<MaybeUninit<float64>> template=new arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<float64>> template=sliceAssumeInit arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceUninit<MaybeUninit<float64>> template=sliceUninit arguments=(MaybeUninit<float64>)
/// @generic.instance id=truncate<float64> template=truncate arguments=(float64) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<float64>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive float64[], usize) => &truncate.'a exclusive MaybeUninit<float64>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive float64[])

declare const source: &readonly [float64];
/// @type.symbol symbol=source source=source type=&'static readonly constant Slice<float64>
/// @resolution.pattern source=source kind=binding target=source

function overwrite(): void {
/// @type.symbol symbol=overwrite type=() => void

    values[0..2] = source;
    /// @resolution.name source=values target=values
    /// @resolution.place source=values placement="local" lifetime="managed" access="exclusive"
    /// @resolution.access source=values root=values
    /// @resolution.pattern.assign source=values[0..2] kind=place
    /// @resolution.assignment source=values[0..2] write="indexSet#2(parameters=(Range<isize>, &'static readonly constant Slice<float64>), arguments=(provided(0..2) as Range<isize>, supplied as &'static readonly constant Slice<float64>), return=void)" type=&'static readonly constant Slice<float64>
    /// @generic.instantiation id="indexSet#2<float64, Range<isize>>" template=indexSet#2 arguments=(float64, Range<isize>)
    /// @generic.instance id="Bound<&'bound0 readonly isize>" template=Bound arguments=(&'bound0 readonly isize)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="as<float64, \"mutable\">" template=as arguments=(float64, "mutable") evaluated=(<as.'a>(this: WithAccess<&as.'a T#7[], A#2>) => WithAccess<&as.'a Slice<T#7>, A#2> => <as.'a>(this: &as.'a float64[]) => &as.'a Slice<float64>)
    /// @generic.instance id="endBound#1<Range<isize>, isize>" template=endBound#1 arguments=(isize)
    /// @generic.instance id="excluded<&'bound0 readonly isize>" template=excluded arguments=(&'bound0 readonly isize)
    /// @generic.instance id="included<&'bound0 readonly isize>" template=included arguments=(&'bound0 readonly isize)
    /// @generic.instance id="index#2<float64, Range<isize>, \"mutable\">" template=index#2 arguments=(float64, Range<isize>, "mutable") evaluated=(<index#2.'a>(this: WithAccess<&index#2.'a Slice<T#6>, A#2>, R#1) => WithAccess<&index#2.'a Slice<T#6>, A#2> => <index#2.'a>(this: &index#2.'a Slice<float64>, Range<isize>) => &index#2.'a Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a Slice<float64>, WithAccess<&index#2.'a Slice<T#6>, A#2> => &index#2.'a Slice<float64>, (this: WithAccess<&index#2.'a Slice<T#6>, A#2>, usize, usize) => WithAccess<&index#2.'a Slice<T#6>, A#2> => (this: &index#2.'a Slice<float64>, usize, usize) => &index#2.'a Slice<float64>)
    /// @generic.instance id="indexSet#2<float64, Range<isize>>" template=indexSet#2 arguments=(float64, Range<isize>) evaluated=(WithAccess<&indexSet#2.'a Slice<T#9>, "mutable"> => &indexSet#2.'a Slice<float64>, (this: WithAccess<&indexSet#2.'a T#9[], "mutable">, R#2) => WithAccess<&indexSet#2.'a Slice<T#9>, "mutable"> => (this: &indexSet#2.'a float64[], Range<isize>) => &indexSet#2.'a Slice<float64>, <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a T#9[], index.A>, isize) => WithAccess<&index#1.'a T#9, index.A> & <index#2.'a>(this: WithAccess<&index#2.'a T#9[], "mutable">, R#2) => WithAccess<&index#2.'a Slice<T#9>, "mutable"> => <const index.A: Access = "readonly", index#1.'a>(this: WithAccess<&index#1.'a float64[], index.A>, isize) => WithAccess<&index#1.'a float64, index.A> & <index#2.'a>(this: &index#2.'a float64[], Range<isize>) => &index#2.'a Slice<float64>, <index#2.'a>(this: WithAccess<&index#2.'a T#9[], "mutable">, R#2) => WithAccess<&index#2.'a Slice<T#9>, "mutable"> => <index#2.'a>(this: &index#2.'a float64[], Range<isize>) => &index#2.'a Slice<float64>)
    /// @generic.instance id="rangeSpan<float64, Range<isize>, \"readonly\" | \"exclusive\" | \"mutable\">" template=rangeSpan arguments=(float64, Range<isize>, "readonly" | "exclusive" | "mutable")
    /// @generic.instance id="sliceView<float64, \"mutable\">" template=sliceView arguments=(float64, "mutable") evaluated=(<sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A>, usize, usize) => WithAccess<&sliceView.'a Slice<sliceView.T>, sliceView.A> => <sliceView.T, const sliceView.A: Access = "readonly", sliceView.'a>(&sliceView.'a Slice<float64>, usize, usize) => &sliceView.'a Slice<float64>)
    /// @generic.instance id="startBound#1<Range<isize>, isize>" template=startBound#1 arguments=(isize)
    /// @generic.instance id="subslice<float64, \"mutable\">" template=subslice arguments=(float64, "mutable") evaluated=(<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => <const subslice.A: Access = "readonly", subslice.'a>(this: &subslice.'a Slice<float64>, usize, usize) => &subslice.'a Slice<float64>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a Slice<float64>, (WithAccess<&subslice.'a Slice<T#1>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T#1>, subslice.A> => (&subslice.'a Slice<float64>, usize, usize) => &subslice.'a Slice<float64>, WithAccess<&subslice.'a Slice<T#1>, subslice.A> => &subslice.'a Slice<float64>)
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=Slice<float64> template=Slice arguments=(float64)
    /// @generic.instance id=copyFrom<float64> template=copyFrom arguments=(float64)
    /// @generic.instance id=size<float64> template=size arguments=(float64)
    /// @generic.instance id=sliceGet<float64> template=sliceGet arguments=(float64)
    /// @generic.instance id=sliceLength<float64> template=sliceLength arguments=(float64)
    /// @generic.instance id=sliceSet<float64> template=sliceSet arguments=(float64)
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
    /// @resolution.assignment source=values[0] write="indexSet#2(parameters=(int64, &'frame readonly Slice<int32>), arguments=(provided(0) as int64, supplied as &'frame readonly Slice<int32>), return=void)" type=&'frame readonly Slice<int32>
    /// @generic.instantiation id="indexSet#2<int32, int64>" template=indexSet#2 arguments=(int32, int64)

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
    /// @resolution.assignment source=values[0] write="indexSet#2(parameters=(int64, &'frame readonly Slice<Status>), arguments=(provided(0) as int64, supplied as &'frame readonly Slice<Status>), return=void)" type=&'frame readonly Slice<Status>
    /// @generic.instantiation id="indexSet#2<Status, int64>" template=indexSet#2 arguments=(Status, int64)
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

}
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int64' does not satisfy 'RangeBounds<isize>'"
/// @diagnostic.label line=5 column=11 span="[" line_source="values[0] = 1;"
/// @diagnostic.related file="slice.ds" line=311 column=21 span="R" line_source="export extension<T, R: RangeBounds<isize>> of Slice<T>" message="required by this bound on 'R'"
/// @diagnostic.error id=not-assignable message="type '1' is not assignable to type '&readonly Slice<int32>'"
/// @diagnostic.label line=5 column=17 span="1" line_source="values[0] = 1;"
/// @diagnostic.related line=5 column=11 span="[" line_source="values[0] = 1;" message="expected due to the type of this target"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int64' does not satisfy 'RangeBounds<isize>'"
/// @diagnostic.label line=9 column=11 span="[" line_source="values[0] = Status.Busy;"
/// @diagnostic.related file="slice.ds" line=311 column=21 span="R" line_source="export extension<T, R: RangeBounds<isize>> of Slice<T>" message="required by this bound on 'R'"
/// @diagnostic.error id=not-assignable message="type 'Status.Busy' is not assignable to type '&readonly Slice<Status>'"
/// @diagnostic.label line=9 column=17 span="Status.Busy" line_source="values[0] = Status.Busy;"
/// @diagnostic.related line=9 column=11 span="[" line_source="values[0] = Status.Busy;" message="expected due to the type of this target"
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
/// @type.symbol symbol=Greet type=Greet
/// @definition.interface symbol=Greet
/// @definition.where symbol=Greet relation=satisfies left=this right=Greet
/// @definition.method symbol=Greet.greet source="greet(&readonly this): int32" slot=greet type=<Greet.greet.'a>(this: &Greet.greet.'a readonly Greet) => int32

    greet(&readonly this): int32;
    /// @generic.template symbol=Greet.greet parent=template#0 parameters=('a)
    /// @type.symbol symbol=Greet.greet source="greet(&readonly this): int32" type=<Greet.greet.'a>(this: &Greet.greet.'a readonly Greet) => int32
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
    /// @resolution.call source=value.greet() parameters=() return=int32 kind=symbol target=Greet.greet receiver=&invoke.'a readonly T
    /// @resolution.place source=value placement=invoke.'a lifetime=invoke.'a access="readonly"
    /// @resolution.access source=value root=invoke.value
    /// @generic.instantiation id=Greet.greet<T> template=Greet.greet arguments=() owner=invoke

}

export function run(): int32 {
/// @type.symbol symbol=run type=() => int32

    const cell = Cell { value: 3 };
    /// @type.symbol symbol=run.cell source=cell type=Cell
    /// @resolution.pattern source=cell kind=binding target=run.cell
    /// @resolution.name source=Cell target=Cell

    return invoke(&readonly cell);
    /// @resolution.name source=invoke target=invoke
    /// @resolution.call source="invoke(&readonly cell)" parameters=(&'frame readonly Cell) arguments=(provided(&readonly cell) as &'frame readonly Cell) return=int32 kind=symbol target=invoke instance=invoke<Cell>
    /// @generic.instantiation id=invoke<Cell> template=invoke arguments=(Cell)
    /// @generic.instance id=greet<Cell> template=greet arguments=()
    /// @generic.instance id=invoke<Cell> template=invoke arguments=(Cell)
    /// @resolution.name source=cell target=run.cell
    /// @resolution.place source=cell placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=cell root=run.cell

}
"#,
    );
}
