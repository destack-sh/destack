use crate::tests::{DirRows, TestSession};

#[test]
fn test_struct_member_access_selects_field_symbol() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const x = point.x;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @resolution.pattern source=x kind=binding target=x
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @resolution.access source=point.x root=point keys=[x]
"#,
    );
}

#[test]
fn test_struct_member_call_selects_method_symbol() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;

    length(&readonly this): int32 {
        return this.x;
    }
}

const point = Point { x: 1 };
const length = point.length();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;

    length(&readonly this): int32 {
        return this.x;
    }
}

const point: Point = Point { x: 1 };
const length: int32 = point.length();

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.length slot=length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    length(&readonly this): int32 {
    /// @generic.template symbol=Point.length parameters=('a)
    /// @type.symbol symbol=Point.length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32
    /// @type.symbol symbol=Point.length.this source="&readonly this" type=&Point.length.'a readonly this

        return this.x;
        /// @type.node source=this type=&Point.length.'a readonly Point
        /// @type.node source=this.x type=int32
        /// @resolution.member source=this.x receiver=&Point.length.'a readonly Point type=int32 kind=field target_receiver=&Point.length.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.length.'a readonly Point
        /// @resolution.place source=this placement=Point.length.'a lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=Point.length.'a lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

const length = point.length();
/// @type.symbol symbol=length source=length type=int32
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=point type=Point
/// @type.node source=point.length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32
/// @type.node source=point.length() type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.length receiver=Point type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32 kind=symbol target_receiver=Point target=Point.length
/// @resolution.call source=point.length() parameters=() return=int32 kind=symbol target=Point.length receiver=Point adjustments=(borrow(&'static readonly constant Point))
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
"#,
    );
}

#[test]
fn test_imported_struct_member_access_selects_exported_field() {
    let compiler = TestSession::builder()
        .module(
            "geometry.ds",
            r#"
export struct Point {
    x: int32;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point } from "./geometry.ds";

const point = Point { x: 1 };
const x = point.x;
"#,
        )
        .build();

    compiler.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Point } from "./geometry.ds";

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== dir ===
import { Point } from "./geometry.ds";

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=geometry.Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=geometry.Point
/// @resolution.name source=Point target=geometry.Point
/// @type.node source=1 type=1

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @resolution.pattern source=x kind=binding target=x
/// @type.node source=point type=geometry.Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=geometry.Point type=int32 kind=field target_receiver=geometry.Point key=x target=geometry.Point.x target_type=int32
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @resolution.access source=point.x root=point keys=[x]
"#,
    );
}

#[test]
fn test_array_member_access_selects_length() {
    let session = TestSession::single(
        r#"
let values: int32[] = [];
const length = values.length;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
const length: isize = values.length;

=== dir ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="elementSlot<int32, \"exclusive\">" template=elementSlot arguments=(int32, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive int32[], usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive int32[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>)
/// @generic.instance id="initAsPointer<int32, \"exclusive\">" template=initAsPointer arguments=(int32, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<int32>) => Raw<int32>)
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<int32>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<int32>>, usize) => &sliceIndex.'a exclusive MaybeUninit<int32>)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<MaybeUninit<int32>> template=MaybeUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<int32>) => Raw<int32>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<int32>)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<int32>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive int32[], usize) => &truncate.'a exclusive MaybeUninit<int32>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive int32[])
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Slice<int32> template=Slice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
/// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
/// @generic.instance id=size<int32> template=size arguments=(int32)
/// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
/// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=int32[]
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=values placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instantiation id=length<int32> template=length arguments=(int32)
/// @generic.instance id=length<int32> template=length arguments=(int32)
"#,
    );
}

#[test]
fn test_imported_array_member_access_selects_length() {
    let session = TestSession::builder()
        .module(
            "values.ds",
            r#"
export const values: int32[] = [];
"#,
        )
        .module(
            "main.ds",
            r#"
import { values } from "./values.ds";

const length = values.length;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { values } from "./values.ds";

const length: isize = values.length;

=== dir ===
import { values } from "./values.ds";

const length = values.length;
/// @type.symbol symbol=length source=length type=isize
/// @resolution.pattern source=length kind=binding target=length
/// @type.node source=values type=int32[]
/// @type.node source=values.length type=isize
/// @resolution.name source=values target=values.values
/// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
/// @resolution.place source=values placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=values root=values.values
/// @generic.instantiation id=length<int32> template=length arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="elementSlot<int32, \"exclusive\">" template=elementSlot arguments=(int32, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive int32[], usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive int32[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>)
/// @generic.instance id="initAsPointer<int32, \"exclusive\">" template=initAsPointer arguments=(int32, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<int32>) => Raw<int32>)
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<int32>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<int32>>, usize) => &sliceIndex.'a exclusive MaybeUninit<int32>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<MaybeUninit<int32>> template=MaybeUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<int32>) => Raw<int32>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<int32>)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=length<int32> template=length arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<int32>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive int32[], usize) => &truncate.'a exclusive MaybeUninit<int32>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive int32[])
"#,
    );
}

#[test]
fn test_member_access_uses_later_inferred_binding() {
    let session = TestSession::single(
        r#"
struct State {
    value: int32;
}

function read(): int32 {
    return state.value;
}

const state = State { value: 1 };
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct State {
    value: int32;
}

function read(): int32 {
    return state.value;
}

const state: State = State { value: 1 };

=== dir ===
struct State {
/// @type.symbol symbol=State type=State
/// @definition.struct symbol=State
/// @definition.field symbol=State.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=State.value source="value: int32" type=int32

}

function read(): int32 {
/// @type.symbol symbol=read type=() => int32

    return state.value;
    /// @type.node source=state type=State
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=state
    /// @resolution.member source=state.value receiver=State type=int32 kind=field target_receiver=State key=value target=State.value target_type=int32
    /// @resolution.place source=state placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=state root=state
    /// @resolution.place source=state.value placement="constant" lifetime="static" access="readonly"
    /// @resolution.access source=state.value root=state keys=[value]

}

const state = State { value: 1 };
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @type.node source="State { value: 1 }" type=State
/// @resolution.name source=State target=State
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_array_member_call_selects_push_overload() {
    let session = TestSession::single(
        r#"
let values: int32[] = [];
values.push(1);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[] = [];
values.push<int32>(1);

=== dir ===
let values: int32[] = [];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="initAsPointer<int32, \"exclusive\">" template=initAsPointer arguments=(int32, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<int32>) => Raw<int32>)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<MaybeUninit<int32>> template=MaybeUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<int32>) => Raw<int32>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<int32>)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<int32>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive int32[], usize) => &truncate.'a exclusive MaybeUninit<int32>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive int32[])
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>

values.push(1);
/// @type.node source=values type=int32[]
/// @type.node source=values.push type=<push.'a>(this: &push.'a exclusive int32[], ...int32[]) => isize
/// @type.node source=values.push(1) type=isize
/// @resolution.name source=values target=values
/// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a exclusive int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
/// @resolution.call source=values.push(1) parameters=(int32[]) arguments=(rest(1) pack=arrayFromOwnedSlice as int32) return=isize kind=symbol target=push receiver=int32[] adjustments=(borrow(Borrowed<int32[], "managed" & "local", "exclusive">)) instance=Array<int32>.<extension#6>.push
/// @resolution.place source=values placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="elementSlot<int32, \"exclusive\">" template=elementSlot arguments=(int32, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive int32[], usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive int32[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>)
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<int32>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<int32>>, usize) => &sliceIndex.'a exclusive MaybeUninit<int32>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Slice<int32> template=Slice arguments=(int32)
/// @generic.instance id=append<int32> template=append arguments=(int32) evaluated=(WithAccess<&append.'b MaybeUninit<T#6>, "exclusive"> => &append.'b exclusive MaybeUninit<int32>, (WithAccess<&append.'b T#6[], "exclusive">, usize) => WithAccess<&append.'b MaybeUninit<T#6>, "exclusive"> => (&append.'b exclusive int32[], usize) => &append.'b exclusive MaybeUninit<int32>, WithAccess<&append.'b T#6[], "exclusive"> => &append.'b exclusive int32[], WithAccess<&append.'a MaybeUninit<T#6>, "exclusive"> => &append.'a exclusive MaybeUninit<int32>, (WithAccess<&append.'a T#6[], "exclusive">, usize) => WithAccess<&append.'a MaybeUninit<T#6>, "exclusive"> => (&append.'a exclusive int32[], usize) => &append.'a exclusive MaybeUninit<int32>, WithAccess<&append.'a T#6[], "exclusive"> => &append.'a exclusive int32[])
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=assumeInitRead#1<int32> template=assumeInitRead#1 arguments=(int32)
/// @generic.instance id=assumeInitRead<int32> template=assumeInitRead arguments=(int32)
/// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
/// @generic.instance id=initWrite<int32> template=initWrite arguments=(int32)
/// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
/// @generic.instance id=push<int32> template=push arguments=(int32)
/// @generic.instance id=reserve<int32> template=reserve arguments=(int32) evaluated=(WithAccess<&reserve.'a MaybeUninit<T#6>, "exclusive"> => &reserve.'a exclusive MaybeUninit<int32>, (WithAccess<&reserve.'a T#6[], "exclusive">, usize) => WithAccess<&reserve.'a MaybeUninit<T#6>, "exclusive"> => (&reserve.'a exclusive int32[], usize) => &reserve.'a exclusive MaybeUninit<int32>, WithAccess<&reserve.'a T#6[], "exclusive"> => &reserve.'a exclusive int32[], (WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive">, usize) => WithAccess<&'frame MaybeUninit<T#6>, "exclusive"> => (&'frame exclusive Slice<MaybeUninit<int32>>, usize) => &'frame exclusive MaybeUninit<int32>, WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive"> => &'frame exclusive Slice<MaybeUninit<int32>>, (WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive">, usize) => WithAccess<&'frame MaybeUninit<T#6>, "exclusive"> => (&'frame exclusive Slice<MaybeUninit<int32>>, usize) => &'frame exclusive MaybeUninit<int32>, WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive"> => &'frame exclusive Slice<MaybeUninit<int32>>)
/// @generic.instance id=size<int32> template=size arguments=(int32)
/// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
/// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
/// @generic.instance id=sliceUninit<int32> template=sliceUninit arguments=(int32)
/// @generic.instance id=uninit<int32> template=uninit arguments=(int32)
/// @generic.instance id=write<int32> template=write arguments=(int32)
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_member_on_never_reports_missing_member() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";

function pending(): int32 {
    let value = todo("later");
    return value.field;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

function pending(): int32 {
    let value: never = todo("later" as string | undefined);
    return value.field;
}

=== dir ===
import { todo } from "destack:error";

function pending(): int32 {
/// @type.symbol symbol=pending type=() => int32

    let value = todo("later");
    /// @type.symbol symbol=pending.value source=value type=never
    /// @resolution.pattern source=value kind=binding target=pending.value
    /// @resolution.name source=todo target=todo
    /// @resolution.call source="todo(\"later\")" parameters=(string | undefined) arguments=(provided("later") as string | undefined) return=never kind=symbol target=todo

    return value.field;
    /// @resolution.name source=value target=pending.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=pending.value
    /// @resolution.rejected source=value.field

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'field' does not exist on type 'never'"
/// @diagnostic.label line=6 column=18 span="field" line_source="return value.field;"
"#,
    );
}

#[test]
fn test_function_valued_field_supports_repeated_calls() {
    let session = TestSession::single(
        r#"
struct Handler {
    readonly run: Function<(int32,), int32, "readonly">;
}

const handler = Handler { run: (value) => value };
const first = handler.run(1);
const second = handler.run(2);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Handler {
    readonly run: Function<(int32,), int32, "readonly">;
}

const handler: Handler = Handler { run: (value: int32): int32 => value };
const first: int32 = handler.run(1);
const second: int32 = handler.run(2);

=== dir ===
struct Handler {
/// @type.symbol symbol=Handler type=Handler
/// @generic.instance id="Function<(int32,), int32, \"readonly\">" template=Function arguments=((int32,), int32, "readonly")
/// @definition.struct symbol=Handler
/// @definition.field symbol=Handler.run source="readonly run: Function<(int32,), int32, \"readonly\">" key=run type=Function<(int32,), int32, "readonly">

    readonly run: Function<(int32,), int32, "readonly">;
    /// @type.symbol symbol=Handler.run source="readonly run: Function<(int32,), int32, \"readonly\">" type=Function<(int32,), int32, "readonly">
    /// @resolution.name source=Function target=Function

}

const handler = Handler { run: (value) => value };
/// @type.symbol symbol=handler source=handler type=Handler
/// @resolution.pattern source=handler kind=binding target=handler
/// @type.node source="Handler { run: (value) => value }" type=Handler
/// @resolution.name source=Handler target=Handler
/// @type.symbol symbol=symbol4 source="(value) => value" type=Function<(int32,), int32, "readonly">
/// @type.node source="(value) => value" type=Function<(int32,), int32, "readonly">
/// @type.symbol symbol=symbol4.value source=value type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=symbol4.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol4.value

const first = handler.run(1);
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=handler type=Handler
/// @type.node source=handler.run type=Function<(int32,), int32, "readonly">
/// @type.node source=handler.run(1) type=int32
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver=Handler type=Function<(int32,), int32, "readonly"> kind=field target_receiver=Handler key=run target=Handler.run target_type=Function<(int32,), int32, "readonly">
/// @resolution.call source=handler.run(1) parameters=(int32) arguments=(provided(1) as int32) return=int32 kind=expression target=expression
/// @resolution.place source=handler placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=handler root=handler
/// @resolution.place source=handler.run placement="local" lifetime="static" access="readonly"
/// @resolution.access source=handler.run root=handler keys=[run]
/// @type.node source=1 type=1

const second = handler.run(2);
/// @type.symbol symbol=second source=second type=int32
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=handler type=Handler
/// @type.node source=handler.run type=Function<(int32,), int32, "readonly">
/// @type.node source=handler.run(2) type=int32
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver=Handler type=Function<(int32,), int32, "readonly"> kind=field target_receiver=Handler key=run target=Handler.run target_type=Function<(int32,), int32, "readonly">
/// @resolution.call source=handler.run(2) parameters=(int32) arguments=(provided(2) as int32) return=int32 kind=expression target=expression
/// @resolution.place source=handler placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=handler root=handler
/// @resolution.place source=handler.run placement="local" lifetime="static" access="readonly"
/// @resolution.access source=handler.run root=handler keys=[run]
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_struct_member_access_selects_declared_field() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const point = Point { x: 1 };
const x = point.x;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const point: Point = Point { x: 1 };
const x: int32 = point.x;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

const x = point.x;
/// @type.symbol symbol=x source=x type=int32
/// @resolution.pattern source=x kind=binding target=x
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @resolution.access source=point.x root=point keys=[x]
"#,
    );
}

#[test]
fn test_reject_instance_method_read_outside_call_position() {
    let session = TestSession::single(
        r#"
class Logger {
    log(message: string): void {}
}

declare const logger: Logger;

const log = logger.log;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Logger {
    log(message: string): void {}
}

declare const logger: Logger;

const log = logger.log;

=== dir ===
class Logger {
/// @type.symbol symbol=Logger type=Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.log source="log(message: string): void {}" slot=log type=<Logger.log.P0: Place>(this: Managed<this, Logger.log.P0>, string) => void

    log(message: string): void {}
    /// @generic.template symbol=Logger.log parameters=(P0: Place)
    /// @type.symbol symbol=Logger.log source="log(message: string): void {}" type=<Logger.log.P0: Place>(this: Managed<this, Logger.log.P0>, string) => void
    /// @type.symbol symbol=Logger.log.this type=Managed<Logger, Logger.log.P0>
    /// @type.symbol symbol=Logger.log.message source="message: string" type=string

}

declare const logger: Logger;
/// @type.symbol symbol=logger source=logger type=Logger
/// @resolution.pattern source=logger kind=binding target=logger
/// @resolution.name source=Logger target=Logger

const log = logger.log;
/// @type.symbol symbol=log source=log type=<error>
/// @resolution.pattern source=log kind=binding target=log
/// @resolution.name source=logger target=logger
/// @resolution.place source=logger placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=logger root=logger
/// @resolution.rejected source=logger.log
"#,
        r#"
/// @diagnostic.error id=cannot-extract-bound-method message="method 'log' cannot be read as a value"
/// @diagnostic.label line=8 column=20 span="log" line_source="const log = logger.log;"
/// @diagnostic.help message="wrap the read in a closure to make its receiver capture explicit"
"#,
    );
}

#[test]
fn test_instance_method_call_through_explicit_application_selects_method() {
    let session = TestSession::single(
        r#"
class Store {
    pick<T>(value: T): T {
        return value;
    }
}

declare const store: Store;

const chosen = store.pick<int32>(3);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Store {
    pick<T>(value: T): T {
        return value;
    }
}

declare const store: Store;

const chosen: int32 = store.pick<int32>(3);

=== dir ===
class Store {
/// @type.symbol symbol=Store type=Store
/// @definition.class symbol=Store
/// @definition.method symbol=Store.pick slot=pick type=<T, Store.pick.P1: Place>(this: Managed<this, Store.pick.P1>, T) => T

    pick<T>(value: T): T {
    /// @generic.template symbol=Store.pick parameters=(T, P1: Place)
    /// @type.symbol symbol=Store.pick type=<T, Store.pick.P1: Place>(this: Managed<this, Store.pick.P1>, T) => T
    /// @type.symbol symbol=Store.pick.this type=Managed<Store, Store.pick.P1>
    /// @type.symbol symbol=Store.pick.T source=T type=T
    /// @type.symbol symbol=Store.pick.value source="value: T" type=T
    /// @resolution.name source=T target=Store.pick.T
    /// @resolution.name source=T target=Store.pick.T

        return value;
        /// @resolution.name source=value target=Store.pick.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Store.pick.value

    }
}

declare const store: Store;
/// @type.symbol symbol=store source=store type=Store
/// @resolution.pattern source=store kind=binding target=store
/// @resolution.name source=Store target=Store

const chosen = store.pick<int32>(3);
/// @type.symbol symbol=chosen source=chosen type=int32
/// @resolution.pattern source=chosen kind=binding target=chosen
/// @resolution.name source=store target=store
/// @resolution.member source=store.pick receiver=Store type=<T, Store.pick.P1: Place>(this: Managed<Store, Store.pick.P1>, T) => T kind=symbol target_receiver=Store target=Store.pick
/// @resolution.call source=store.pick<int32>(3) parameters=(int32) arguments=(provided(3) as int32) return=int32 kind=symbol target=Store.pick receiver=Store instance="Store.pick<int32, \"local\">"
/// @resolution.place source=store placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=store root=store
/// @generic.instantiation id="Store.pick<int32, \"local\">" template=Store.pick arguments=(int32, "local")
/// @generic.instance id="Store.pick<int32, \"local\">" template=Store.pick arguments=(int32, "local")
"#,
    );
}

/// A member observation settles a widened integer receiver at its family form.
#[test]
fn test_default_an_integer_binding_to_int64_at_a_method_call() {
    let session = TestSession::single(
        r#"
function render(): void {
    let x = 5;
    x.toFixed(1);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
=== annotated ===
function render(): void {
    let x: int64 = 5;
    x.toFixed(1 as float64 | undefined);
}

=== dir ===
function render(): void {
/// @type.symbol symbol=render type=() => void

    let x = 5;
    /// @type.symbol symbol=render.x source=x type=int64
    /// @resolution.pattern source=x kind=binding target=render.x
    /// @type.node source=5 type=5

    x.toFixed(1);
    /// @type.node source=x.toFixed type=<Number.toFixed.'a>(this: &Number.toFixed.'a readonly int64, float64 | undefined?) => MaybeOwned<Number.toFixed.'a, string>
    /// @type.node source=x.toFixed(1) type=MaybeOwned<"frame" & "local", string>
    /// @resolution.name source=x target=render.x
    /// @resolution.member source=x.toFixed receiver=int64 type=<Number.toFixed.'a>(this: &Number.toFixed.'a readonly int64, float64 | undefined?) => MaybeOwned<Number.toFixed.'a, string> kind=symbol target_receiver=int64 target=Number.toFixed
    /// @resolution.call source=x.toFixed(1) parameters=(float64 | undefined) arguments=(provided(1) as float64 | undefined) return=MaybeOwned<"frame" & "local", string> kind=symbol target=Number.toFixed receiver=int64 adjustments=(borrow(&'frame readonly int64))
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=render.x
    /// @generic.instance id="CowBorrowed<&'bound0 readonly string>" template=CowBorrowed arguments=(&'bound0 readonly string)
    /// @generic.instance id=Cow<string> template=Cow arguments=(string)
    /// @generic.instance id=CowOwned<^string> template=CowOwned arguments=(^string)
    /// @generic.instance id=MaybeOwned<string> template=MaybeOwned arguments=(string)
    /// @type.node source=1 type=1

}
"#,
    );
}

/// A member observation settles a widened float receiver at its family form.
#[test]
fn test_default_a_float_binding_to_float64_at_a_method_call() {
    let session = TestSession::single(
        r#"
function render(): void {
    let x = 1.5;
    x.toFixed(1);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
=== annotated ===
function render(): void {
    let x: float64 = 1.5;
    x.toFixed(1 as float64 | undefined);
}

=== dir ===
function render(): void {
/// @type.symbol symbol=render type=() => void

    let x = 1.5;
    /// @type.symbol symbol=render.x source=x type=float64
    /// @resolution.pattern source=x kind=binding target=render.x
    /// @type.node source=1.5 type=1.5

    x.toFixed(1);
    /// @type.node source=x.toFixed type=<Number.toFixed.'a>(this: &Number.toFixed.'a readonly float64, float64 | undefined?) => MaybeOwned<Number.toFixed.'a, string>
    /// @type.node source=x.toFixed(1) type=MaybeOwned<"frame" & "local", string>
    /// @resolution.name source=x target=render.x
    /// @resolution.member source=x.toFixed receiver=float64 type=<Number.toFixed.'a>(this: &Number.toFixed.'a readonly float64, float64 | undefined?) => MaybeOwned<Number.toFixed.'a, string> kind=symbol target_receiver=float64 target=Number.toFixed
    /// @resolution.call source=x.toFixed(1) parameters=(float64 | undefined) arguments=(provided(1) as float64 | undefined) return=MaybeOwned<"frame" & "local", string> kind=symbol target=Number.toFixed receiver=float64 adjustments=(borrow(&'frame readonly float64))
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=render.x
    /// @generic.instance id="CowBorrowed<&'bound0 readonly string>" template=CowBorrowed arguments=(&'bound0 readonly string)
    /// @generic.instance id=Cow<string> template=Cow arguments=(string)
    /// @generic.instance id=CowOwned<^string> template=CowOwned arguments=(^string)
    /// @generic.instance id=MaybeOwned<string> template=MaybeOwned arguments=(string)
    /// @type.node source=1 type=1

}
"#,
    );
}

/// A widened integer stays open for wider uses without a member observation.
#[test]
fn test_infer_an_integer_binding_from_a_wider_parameter() {
    let session = TestSession::single(
        r#"
function wide(value: int64): int64 {
    return value;
}

function feed(): int64 {
    let x = 5;
    return wide(x);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types().without_reference_types(),
        r#"
=== annotated ===
function wide(value: int64): int64 {
    return value;
}

function feed(): int64 {
    let x: int64 = 5;
    return wide(x);
}

=== dir ===
function wide(value: int64): int64 {
/// @type.symbol symbol=wide type=(int64) => int64
/// @type.symbol symbol=wide.value source="value: int64" type=int64

    return value;
    /// @resolution.name source=value target=wide.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=wide.value

}

function feed(): int64 {
/// @type.symbol symbol=feed type=() => int64

    let x = 5;
    /// @type.symbol symbol=feed.x source=x type=int64
    /// @resolution.pattern source=x kind=binding target=feed.x
    /// @type.node source=5 type=5

    return wide(x);
    /// @type.node source=wide(x) type=int64
    /// @resolution.name source=wide target=wide
    /// @resolution.call source=wide(x) parameters=(int64) arguments=(provided(x) as int64) return=int64 kind=symbol target=wide
    /// @resolution.name source=x target=feed.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=feed.x

}
"#,
    );
}

/// A misspelled member on a widened receiver reports with the closest key.
#[test]
fn test_suggest_the_closest_member_on_an_integer_binding() {
    let session = TestSession::single(
        r#"
function render(): void {
    let x = 5;
    x.toFixd(1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function render(): void {
    let x: int64 = 5;
    x.toFixd(1);
}

=== dir ===
function render(): void {
/// @type.symbol symbol=render type=() => void

    let x = 5;
    /// @type.symbol symbol=render.x source=x type=int64
    /// @resolution.pattern source=x kind=binding target=render.x

    x.toFixd(1);
    /// @resolution.name source=x target=render.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=render.x
    /// @resolution.rejected source=x.toFixd
    /// @resolution.rejected source=x.toFixd(1)

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'toFixd' does not exist on type 'int64'; did you mean 'toFixed'?"
/// @diagnostic.label line=4 column=7 span="toFixd" line_source="x.toFixd(1);"
/// @diagnostic.suggestion message="rename to 'toFixed'" applicability=dangerous patched="x.toFixed(1);"
"#,
    );
}
