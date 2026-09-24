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
/// @resolution.place source=bytes placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bytes root=bytes
/// @resolution.subscript source=bytes[1..3] type=Slice<uint8> kind=call target="index#1(parameters=(Range<isize>), arguments=(provided(1..3) as Range<isize>), return=&'static immutable Slice<uint8>, regions=(\"static\" & \"local\"))"
/// @generic.instantiation id="index#1<uint8, 4, Range<isize>, \"immutable\", \"static\" & \"local\">" template=index#1 arguments=(uint8, 4, Range<isize>, "immutable", "static" & "local")
/// @generic.instance id="index#1<uint8, 4, Range<isize>, \"immutable\", \"bound0\" & \"local\">" template=index#1 arguments=(uint8, 4, Range<isize>, "immutable", "bound0" & "local")
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
/// @type.symbol symbol=values source=values type=&'static readonly Slice<float64>
/// @resolution.pattern source=values kind=binding target=values

const window = values[1..3];
/// @type.symbol symbol=window source=window type=Slice<float64>
/// @resolution.pattern source=window kind=binding target=window
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[1..3] type=Slice<float64> kind=call target="index#1(parameters=(Range<isize>), arguments=(provided(1..3) as Range<isize>), return=&'static readonly Slice<float64>, regions=(\"static\" & \"local\"))"
/// @generic.instantiation id="index#1<float64, Range<isize>, \"readonly\", \"static\" & \"local\">" template=index#1 arguments=(float64, Range<isize>, "readonly", "static" & "local")
/// @generic.instance id="index#1<float64, Range<isize>, \"readonly\", \"bound0\" & \"local\">" template=index#1 arguments=(float64, Range<isize>, "readonly", "bound0" & "local")
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
/// @type.symbol symbol=buffer source=buffer type=&'static Slice<uint8>
/// @resolution.pattern source=buffer kind=binding target=buffer

declare const header: &readonly [uint8];
/// @type.symbol symbol=header source=header type=&'static readonly Slice<uint8>
/// @resolution.pattern source=header kind=binding target=header

function stamp(): void {
/// @type.symbol symbol=stamp type=() => void

    buffer[0..2] = header;
    /// @resolution.name source=buffer target=buffer
    /// @resolution.place source=buffer placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=buffer root=buffer
    /// @resolution.pattern.assign source=buffer[0..2] kind=place
    /// @resolution.assignment source=buffer[0..2] write="indexSet#2(parameters=(Range<isize>, &'static readonly Slice<uint8>), arguments=(provided(0..2) as Range<isize>, supplied(0) as &'static readonly Slice<uint8>), return=void, regions=(\"frame\", \"static\" & \"local\", \"static\" & \"local\"))" type=&'static readonly Slice<uint8>
    /// @generic.instantiation id="indexSet#2<uint8, Range<isize>, \"frame\", \"static\" & \"local\", \"static\" & \"local\">" template=indexSet#2 arguments=(uint8, Range<isize>, "frame", "static" & "local", "static" & "local")
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="copyFrom<uint8, \"bound0\" & \"local\", \"bound1\" & \"local\">" template=copyFrom arguments=(uint8, "bound0" & "local", "bound1" & "local")
    /// @generic.instance id="fromReference<uint8, \"bound0\" & \"local\">" template=fromReference arguments=(uint8, "bound0" & "local")
    /// @generic.instance id="index#1<uint8, Range<isize>, \"mutable\", \"bound0\" & \"local\">" template=index#1 arguments=(uint8, Range<isize>, "mutable", "bound0" & "local")
    /// @generic.instance id="indexSet#2<uint8, Range<isize>, \"frame\", \"bound1\" & \"local\", \"bound2\" & \"local\">" template=indexSet#2 arguments=(uint8, Range<isize>, "frame", "bound1" & "local", "bound2" & "local")
    /// @generic.instance id="panic<\"bound0\" & \"local\">" template=panic arguments=("bound0" & "local")
    /// @generic.instance id="size<uint8, \"bound0\" & \"local\">" template=size arguments=(uint8, "bound0" & "local")
    /// @generic.instance id="sliceIndex<uint8, \"bound0\" & \"local\", \"mutable\">" template=sliceIndex arguments=(uint8, "bound0" & "local", "mutable")
    /// @generic.instance id="sliceIndex<uint8, \"bound0\" & \"local\", \"readonly\">" template=sliceIndex arguments=(uint8, "bound0" & "local", "readonly")
    /// @generic.instance id="sliceLength<uint8, \"bound0\" & \"local\">" template=sliceLength arguments=(uint8, "bound0" & "local")
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=Slice<uint8> template=Slice arguments=(uint8)
    /// @generic.instance id=copy<uint8> template=copy arguments=(uint8)
    /// @generic.instance id=Range<isize> template=Range arguments=(isize)
    /// @resolution.name source=header target=header
    /// @resolution.place source=header placement="local" lifetime="static" access="immutable"
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
    let (left, right) = values.splitAt<int32, "static", "mutable">(2);
    left.reverse<int32, "static", "mutable">();
    right.reverse<int32, "static", "mutable">();
}

=== dir ===
declare const values: &[int32];
/// @type.symbol symbol=values source=values type=&'static Slice<int32>
/// @resolution.pattern source=values kind=binding target=values

function halves(): void {
/// @type.symbol symbol=halves type=() => void

    let (left, right) = values.splitAt(2);
    /// @resolution.pattern source=(left, right) kind=tuple fields=(halves.left, halves.right)
    /// @type.symbol symbol=halves.left source=left type=&'static Slice<int32>
    /// @resolution.pattern source=left kind=binding target=halves.left
    /// @type.symbol symbol=halves.right source=right type=&'static Slice<int32>
    /// @resolution.pattern source=right kind=binding target=halves.right
    /// @resolution.name source=values target=values
    /// @resolution.member source=values.splitAt receiver=&'static Slice<int32> type=<const splitAt.R, const splitAt.A: Access>(this: Borrowed<Slice<int32>, splitAt.R, splitAt.A>, isize) => (Borrowed<Slice<int32>, splitAt.R, splitAt.A>, Borrowed<Slice<int32>, splitAt.R, splitAt.A>) kind=symbol target_receiver=&'static Slice<int32> target=splitAt
    /// @resolution.call source=values.splitAt(2) parameters=(isize) arguments=(provided(2) as isize) return=(&'static Slice<int32>, &'static Slice<int32>) regions=("static" & "local") kind=symbol target=splitAt receiver=&'static Slice<int32> instance="Slice<int32>.<extension#1>.splitAt<\"static\" & \"local\", \"mutable\">"
    /// @resolution.place source=values placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=values root=values
    /// @generic.instantiation id="splitAt<int32, \"static\" & \"local\", \"mutable\">" template=splitAt arguments=(int32, "static" & "local", "mutable")
    /// @generic.instantiation id=splitAt<int32> template=splitAt arguments=(int32)
    /// @generic.instance id="splitAt<int32, \"bound0\" & \"local\", \"mutable\">" template=splitAt arguments=(int32, "bound0" & "local", "mutable")

    left.reverse();
    /// @resolution.name source=left target=halves.left
    /// @resolution.member source=left.reverse receiver=&'static Slice<int32> type=<const reverse.R, const reverse.A: "mutable" | "exclusive">(this: Borrowed<Slice<int32>, reverse.R, reverse.A>) => void kind=symbol target_receiver=&'static Slice<int32> target=reverse
    /// @resolution.call source=left.reverse() parameters=() return=void regions=("static" & "local") kind=symbol target=reverse receiver=&'static Slice<int32> instance="Slice<int32>.<extension#1>.reverse<\"static\" & \"local\", \"mutable\">"
    /// @resolution.place source=left placement="local" lifetime="static" access="mutable"
    /// @resolution.access source=left root=halves.left
    /// @generic.instantiation id="reverse<int32, \"static\" & \"local\", \"mutable\">" template=reverse arguments=(int32, "static" & "local", "mutable")
    /// @generic.instantiation id=reverse<int32> template=reverse arguments=(int32)
    /// @generic.instance id="reverse<int32, \"bound0\" & \"local\", \"mutable\">" template=reverse arguments=(int32, "bound0" & "local", "mutable")

    right.reverse();
    /// @resolution.name source=right target=halves.right
    /// @resolution.member source=right.reverse receiver=&'static Slice<int32> type=<const reverse.R, const reverse.A: "mutable" | "exclusive">(this: Borrowed<Slice<int32>, reverse.R, reverse.A>) => void kind=symbol target_receiver=&'static Slice<int32> target=reverse
    /// @resolution.call source=right.reverse() parameters=() return=void regions=("static" & "local") kind=symbol target=reverse receiver=&'static Slice<int32> instance="Slice<int32>.<extension#1>.reverse<\"static\" & \"local\", \"mutable\">"
    /// @resolution.place source=right placement="local" lifetime="static" access="mutable"
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
/// @type.symbol symbol=values source=values type=&'static readonly Slice<float64>
/// @resolution.pattern source=values kind=binding target=values

const head = values[..2];
/// @type.symbol symbol=head source=head type=Slice<float64>
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[..2] type=Slice<float64> kind=call target="index#1(parameters=(RangeTo<isize>), arguments=(provided(..2) as RangeTo<isize>), return=&'static readonly Slice<float64>, regions=(\"static\" & \"local\"))"
/// @generic.instantiation id="index#1<float64, RangeTo<isize>, \"readonly\", \"static\" & \"local\">" template=index#1 arguments=(float64, RangeTo<isize>, "readonly", "static" & "local")
/// @generic.instance id="index#1<float64, RangeTo<isize>, \"readonly\", \"bound0\" & \"local\">" template=index#1 arguments=(float64, RangeTo<isize>, "readonly", "bound0" & "local")
/// @generic.instance id=RangeTo<isize> template=RangeTo arguments=(isize)

const tail = values[1..];
/// @type.symbol symbol=tail source=tail type=Slice<float64>
/// @resolution.pattern source=tail kind=binding target=tail
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[1..] type=Slice<float64> kind=call target="index#1(parameters=(RangeFrom<isize>), arguments=(provided(1..) as RangeFrom<isize>), return=&'static readonly Slice<float64>, regions=(\"static\" & \"local\"))"
/// @generic.instantiation id="index#1<float64, RangeFrom<isize>, \"readonly\", \"static\" & \"local\">" template=index#1 arguments=(float64, RangeFrom<isize>, "readonly", "static" & "local")
/// @generic.instance id="index#1<float64, RangeFrom<isize>, \"readonly\", \"bound0\" & \"local\">" template=index#1 arguments=(float64, RangeFrom<isize>, "readonly", "bound0" & "local")
/// @generic.instance id=RangeFrom<isize> template=RangeFrom arguments=(isize)

const whole = values[..];
/// @type.symbol symbol=whole source=whole type=Slice<float64>
/// @resolution.pattern source=whole kind=binding target=whole
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[..] type=Slice<float64> kind=call target="index#1(parameters=(RangeFull), arguments=(provided(..) as RangeFull), return=&'static readonly Slice<float64>, regions=(\"static\" & \"local\"))"
/// @generic.instantiation id="index#1<float64, RangeFull, \"readonly\", \"static\" & \"local\">" template=index#1 arguments=(float64, RangeFull, "readonly", "static" & "local")
/// @generic.instance id="index#1<float64, RangeFull, \"readonly\", \"bound0\" & \"local\">" template=index#1 arguments=(float64, RangeFull, "readonly", "bound0" & "local")
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
/// @type.symbol symbol=values source=values type=&'static Slice<float64>
/// @resolution.pattern source=values kind=binding target=values

function set(): void {
/// @type.symbol symbol=set type=() => void

    values[0] = 1.0;
    /// @resolution.name source=values target=values
    /// @resolution.place source=values placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=values root=values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, float64), arguments=(provided(0) as isize, supplied(0) as float64), return=void, regions=(\"static\" & \"local\"))" type=float64
    /// @generic.instantiation id="indexSet#1<float64, \"static\" & \"local\">" template=indexSet#1 arguments=(float64, "static" & "local")
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="indexSet#1<float64, \"bound0\" & \"local\">" template=indexSet#1 arguments=(float64, "bound0" & "local")
    /// @generic.instance id="panic<\"bound0\" & \"local\">" template=panic arguments=("bound0" & "local")
    /// @generic.instance id="size<float64, \"bound0\" & \"local\">" template=size arguments=(float64, "bound0" & "local")
    /// @generic.instance id="sliceLength<float64, \"bound0\" & \"local\">" template=sliceLength arguments=(float64, "bound0" & "local")
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id="unsafeSet<float64, \"bound0\" & \"local\">" template=unsafeSet arguments=(float64, "bound0" & "local")
    /// @generic.instance id=Slice<float64> template=Slice arguments=(float64)

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
/// @generic.instance id=Array<float64> template=Array arguments=(float64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<float64>> template=sliceAssumeInit arguments=(MaybeUninit<float64>)
/// @generic.instance id=sliceUninit<MaybeUninit<float64>> template=sliceUninit arguments=(MaybeUninit<float64>)

declare const source: &readonly [float64];
/// @type.symbol symbol=source source=source type=&'static readonly Slice<float64>
/// @resolution.pattern source=source kind=binding target=source

function overwrite(): void {
/// @type.symbol symbol=overwrite type=() => void

    values[0..2] = source;
    /// @resolution.name source=values target=values
    /// @resolution.place source=values placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=values root=values
    /// @resolution.pattern.assign source=values[0..2] kind=place
    /// @resolution.assignment source=values[0..2] write="indexSet#2(parameters=(Range<isize>, &'static readonly Slice<float64>), arguments=(provided(0..2) as Range<isize>, supplied(0) as &'static readonly Slice<float64>), return=void, regions=(\"frame\", \"managed\" & \"local\", \"static\" & \"local\"))" type=&'static readonly Slice<float64>
    /// @generic.instantiation id="indexSet#2<float64, Range<isize>, \"frame\", \"managed\" & \"local\", \"static\" & \"local\">" template=indexSet#2 arguments=(float64, Range<isize>, "frame", "managed" & "local", "static" & "local")
    /// @generic.instance id="indexSet#2<float64, Range<isize>, \"frame\", \"bound1\" & \"local\", \"bound2\" & \"local\">" template=indexSet#2 arguments=(float64, Range<isize>, "frame", "bound1" & "local", "bound2" & "local")
    /// @generic.instance id=Range<isize> template=Range arguments=(isize)
    /// @resolution.name source=source target=source
    /// @resolution.place source=source placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=source root=source

}
"#,
    );
}

/// Write scalar and enum elements through a mutable slice view.
#[test]
fn test_write_an_enum_element_through_a_mutable_view() {
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
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, int32), arguments=(provided(0) as isize, supplied(0) as int32), return=void, regions=(updateNumbers.'a))" type=int32
    /// @generic.instantiation id="indexSet#1<int32, updateNumbers.'a>" template=indexSet#1 arguments=(int32, updateNumbers.'a)

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
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, Status), arguments=(provided(0) as isize, supplied(0) as Status), return=void, regions=(updateStatuses.'a))" type=Status
    /// @generic.instantiation id="indexSet#1<Status, updateStatuses.'a>" template=indexSet#1 arguments=(Status, updateStatuses.'a)
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
    return value.greet<'a>();
}

export function run(): int32 {
    const cell: Cell = Cell { value: 3 };
    return invoke<Cell, "frame">(&readonly cell);
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
    /// @type.symbol symbol=greet.this source="&readonly this" type=&greet.'a readonly Cell

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
    /// @resolution.call source=value.greet() parameters=() return=int32 regions=(invoke.'a) kind=symbol target=Greet.greet receiver=&invoke.'a readonly T instance=Greet.greet<invoke.'a>
    /// @resolution.place source=value placement=invoke.'a lifetime=invoke.'a access="readonly"
    /// @resolution.access source=value root=invoke.value
    /// @generic.instantiation id="Greet.greet<T, invoke.'a>" template=Greet.greet arguments=(invoke.'a) owner=invoke
    /// @generic.instance id="Greet.greet<T, invoke.'a>" template=Greet.greet arguments=(invoke.'a)

}

export function run(): int32 {
/// @type.symbol symbol=run type=() => int32

    const cell = Cell { value: 3 };
    /// @type.symbol symbol=run.cell source=cell type=Cell
    /// @resolution.pattern source=cell kind=binding target=run.cell
    /// @resolution.name source=Cell target=Cell

    return invoke(&readonly cell);
    /// @resolution.name source=invoke target=invoke
    /// @resolution.call source="invoke(&readonly cell)" parameters=(&'frame readonly Cell) arguments=(provided(&readonly cell) as &'frame readonly Cell) return=int32 regions=("frame" & "local") kind=symbol target=invoke instance="invoke<Cell, \"frame\" & \"local\">"
    /// @generic.instantiation id="invoke<Cell, \"frame\" & \"local\">" template=invoke arguments=(Cell, "frame" & "local")
    /// @generic.instance id="invoke<Cell, \"bound0\" & \"local\">" template=invoke arguments=(Cell, "bound0" & "local")
    /// @resolution.name source=cell target=run.cell
    /// @resolution.place source=cell placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=cell root=run.cell

}
"#,
    );
}
