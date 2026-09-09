use crate::tests::TestSession;

/// Dispatch a requirement call in a template through the receiver's witness at the instance.
#[test]
fn test_instantiate_dispatches_a_requirement_through_the_witness() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
}

function duplicate<T: Clone>(value: &readonly T): T {
    return value.clone();
}

function main(): Path {
    const path = Path { steps: [] };
    return duplicate(path);
}
"#,
    );

    session.assert_mir_elaborated(
        "main.ds", r#"
@languageItem("collections.Array")
type Array<T> {
    storage: slice<uninit<T>, unique, mutable, local>;
    count: isize;
    allocated: usize;
}

@copy
type test.main.Path {
    steps: Array<int32>;
}

@languageItem("memory.Clone")
type Clone { }

@languageItem("memory.Drop")
type Drop { }

@languageItem("memory.Concrete")
type Concrete { }

@languageItem("memory.Copy")
type Copy extends Clone { }

@languageItem("math.IntegerDomain")
type IntegerDomain { }

@languageItem("math.Zero")
type Zero { }

@languageItem("math.One")
type One { }

@languageItem("math.Integer")
type Integer extends Concrete, Copy, IntegerDomain, Zero, One { }

@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable, local>;
}

external constant string.1: String

function test.main.main(): test.main.Path {
    local l0: test.main.Path

entry:
    v0: usize = 0
    v1: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v0
    v2: slice<int32, unique, mutable, local> = new.complete v1
    v3: Array<int32> = call arrayFromOwnedSlice<int32>(v2): (slice<int32, unique, mutable, local>) => Array<int32>
    v4: test.main.Path = aggregate (v3)
    local.set l0, v4
    v5: ref<test.main.Path, borrowed, 'frame, readonly, local> = local.address l0
    v6: test.main.Path = call test.main.duplicate<test.main.Path>(v5): <'a>(ref<test.main.Path, borrowed, 'a, readonly, local>) => test.main.Path
    v7: test.main.Path = local.get l0
    drop v7
    return v6
}

function test.main.duplicate<T: Clone, 'a>(v0: ref<T, borrowed, 'a, readonly, local>): T;

external function Drop.drop<this: Drop, 'a>(ref<this, borrowed, 'a, mutable, local>): void

external function Array.Drop.drop<T, 'a>(ref<Array<T>, borrowed, 'a, mutable, local>): void

shared function Array.Drop.drop<int32, 'a>(v0: ref<Array<int32>, borrowed, 'a, mutable, local>): void {
    local l0: ref<Array<int32>, borrowed, 'a, mutable, local>

entry(v0: ref<Array<int32>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, borrowed, 'a, mutable, local> = local.get l0
    call Array.clear<int32>(v1): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>) => void
    return
}

external function Clone.clone<this: Clone, 'a>(ref<this, borrowed, 'a, readonly, local>): this

function test.main.Clone.clone<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, readonly, local>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Path, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Path, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<Array<int32>, borrowed, 'a, readonly, local> = field.address v1, 0
    v3: Array<int32> = call Array.Clone.clone<int32>(v2): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => Array<int32>
    v4: test.main.Path = aggregate (v3)
    return v4
}

external function Integer.Clone.clone<T: Integer, 'a>(ref<T, borrowed, 'a, readonly, local>): T

shared function Integer.Clone.clone<int32, 'a>(v0: ref<int32, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v2: int32 = load v1
    return v2
}

@languageItem("collections.array.fromOwnedSlice")
external function arrayFromOwnedSlice<T>(slice<T, unique, mutable, local>): Array<T>

@languageItem("collections.array.fromOwnedSlice")
shared function arrayFromOwnedSlice<int32>(v0: slice<int32, unique, mutable, local>): Array<int32> {
    local l0: slice<int32, unique, mutable, local>

entry(v0: slice<int32, unique, mutable, local>):
    local.set l0, v0
    v1: slice<int32, unique, mutable, local> = local.get l0
    v2: Array<int32> = call Array.fromOwnedSlice<int32>(v1): (slice<int32, unique, mutable, local>) => Array<int32>
    return v2
}

shared function test.main.duplicate<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, readonly, local>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Path, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Path, borrowed, 'a, readonly, local> = local.get l0
    v2: test.main.Path = call test.main.Clone.clone<test.main.Path>(v1): <'a>(ref<test.main.Path, borrowed, 'a, readonly, local>) => test.main.Path
    return v2
}

external function Array.Clone.clone<T: Clone, 'a>(ref<Array<T>, borrowed, 'a, readonly, local>): Array<T>

shared function Array.Clone.clone<int32, 'a>(v0: ref<Array<int32>, borrowed, 'a, readonly, local>): Array<int32> {
    local l0: ref<Array<int32>, borrowed, 'a, readonly, local>

entry(v0: ref<Array<int32>, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<String, managed, mutable, local> = global.address string.1
    v2: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v1
    panic v2

b1:
    return
}

external function Array.fromOwnedSlice<T>(slice<T, unique, mutable, local>): Array<T>

shared function Array.fromOwnedSlice<int32>(v0: slice<int32, unique, mutable, local>): Array<int32> {
    local l0: slice<int32, unique, mutable, local>
    local l1: isize
    local l2: Array<int32>
    local l3: Array<int32>

entry(v0: slice<int32, unique, mutable, local>):
    local.set l0, v0
    v1: ref<slice<int32, unique, mutable, local>, borrowed, 'frame, readonly, frame> = local.project l0
    v2: slice<int32, borrowed, 'frame, readonly, local> = load v1
    v3: isize = call Slice.size.get<int32>(v2): <'a>(slice<int32, borrowed, 'a, readonly, local>) => isize
    local.set l1, v3
    v4: ref<Array<int32>, borrowed, 'frame, mutable, frame> = local.address l2
    v5: ref<uninit<Array<int32>>, borrowed, 'frame, mutable, frame> = cast.bit v4 -> ref<uninit<Array<int32>>, borrowed, 'frame, mutable, frame>
    call Array.constructor<int32>(v5): <'a>(ref<uninit<Array<int32>>, borrowed, 'a, mutable, local>) => void
    v6: Array<int32> = local.get l2
    local.set l3, v6
    v7: slice<int32, unique, mutable, local> = local.get l0
    v8: slice<uninit<int32>, unique, mutable, local> = call Slice.intoUninit<int32>(v7): (slice<int32, unique, mutable, local>) => slice<uninit<int32>, unique, mutable, local>
    v9: ref<Array<int32>, borrowed, 'frame, mutable, frame> = local.project l3
    v10: ref<slice<uninit<int32>, unique, mutable, local>, borrowed, 'frame, mutable, frame> = field.project v9, 0
    v19: ref<Array<int32>, borrowed, 'frame, mutable, frame> = local.project l3
    v20: ref<slice<uninit<int32>, unique, mutable, local>, borrowed, 'frame, mutable, frame> = field.project v19, 0
    v21: slice<uninit<int32>, unique, mutable, local> = load v20
    release v21
    store v10, v8
    v11: isize = local.get l1
    v12: ref<Array<int32>, borrowed, 'frame, mutable, frame> = local.project l3
    v13: ref<isize, borrowed, 'frame, mutable, frame> = field.project v12, 1
    v22: ref<Array<int32>, borrowed, 'frame, mutable, frame> = local.project l3
    v23: ref<isize, borrowed, 'frame, mutable, frame> = field.project v22, 1
    v24: isize = load v23
    store v13, v11
    v14: isize = local.get l1
    v15: usize = call Cast.truncate<isize, usize>(v14): (isize) => usize
    v16: ref<Array<int32>, borrowed, 'frame, mutable, frame> = local.project l3
    v17: ref<usize, borrowed, 'frame, mutable, frame> = field.project v16, 2
    v25: ref<Array<int32>, borrowed, 'frame, mutable, frame> = local.project l3
    v26: ref<usize, borrowed, 'frame, mutable, frame> = field.project v25, 2
    v27: usize = load v26
    store v17, v15
    v18: Array<int32> = local.get l3
    return v18
}

external function Slice.size.get<T, 'a>(slice<T, borrowed, 'a, readonly, local>): isize

shared function Slice.size.get<int32, 'a>(v0: slice<int32, borrowed, 'a, readonly, local>): isize {
    local l0: slice<int32, borrowed, 'a, readonly, local>

entry(v0: slice<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: slice<int32, borrowed, 'a, readonly, local> = local.get l0
    v2: usize = slice.length v1
    v3: isize = call Cast.truncate<usize, isize>(v2): (usize) => isize
    return v3
}

external function Array.constructor<T, 'a>(ref<uninit<Array<T>>, borrowed, 'a, mutable, local>): void

shared function Array.constructor<int32, 'a>(v0: ref<uninit<Array<int32>>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<Array<int32>>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<Array<int32>>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<Array<int32>>, borrowed, 'a, mutable, local> = local.get l0
    v2: slice<uninit<int32>, unique, mutable, local> = call Slice.new<uninit<int32>>(): () => slice<uninit<int32>, unique, mutable, local>
    v3: ref<uninit<slice<uninit<int32>, unique, mutable, local>>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    v8: usize = 0
    v9: usize = 16
    barrier.write v3, v8, v9
    v4: isize = 0
    v5: ref<uninit<isize>, borrowed, 'a, mutable, local> = field.project v1, 1
    store v5, v4
    v6: usize = 0
    v7: ref<uninit<usize>, borrowed, 'a, mutable, local> = field.project v1, 2
    store v7, v6
    return
}

external function Slice.intoUninit<T>(slice<T, unique, mutable, local>): slice<uninit<T>, unique, mutable, local>

shared function Slice.intoUninit<int32>(v0: slice<int32, unique, mutable, local>): slice<uninit<int32>, unique, mutable, local> {
    local l0: slice<int32, unique, mutable, local>

entry(v0: slice<int32, unique, mutable, local>):
    local.set l0, v0
    v1: slice<int32, unique, mutable, local> = local.get l0
    v2: slice<uninit<int32>, unique, mutable, local> = intrinsic.memory.raw.transmute(v1)
    return v2
}

external function Cast.truncate<isize, usize>(isize): usize

external function Slice.new<T>(): slice<T, unique, mutable, local>

shared function Slice.new<uninit<int32>>(): slice<uninit<int32>, unique, mutable, local> {
entry:
    v0: usize = 0
    v1: slice<uninit<uninit<int32>>, unique, mutable, local> = new.slice.uninit uninit<uninit<int32>>, v0
    v2: slice<uninit<int32>, unique, mutable, local> = new.complete v1
    return v2
}

external function Cast.truncate<usize, isize>(usize): isize

external function Array.clear<T, 'a>(ref<Array<T>, borrowed, 'a, mutable, local>): void

shared function Array.clear<int32, 'a>(v0: ref<Array<int32>, borrowed, 'a, mutable, local>): void {
    local l0: ref<Array<int32>, borrowed, 'a, mutable, local>

entry(v0: ref<Array<int32>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, borrowed, 'a, mutable, local> = local.get l0
    v2: isize = 0
    call Array.truncate<int32>(v1, v2): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>, isize) => void
    return
}

external function Array.truncate<T, 'a>(ref<Array<T>, borrowed, 'a, mutable, local>, isize): void

shared function Array.truncate<int32, 'a>(v0: ref<Array<int32>, borrowed, 'a, mutable, local>, v1: isize): void {
    local l0: isize
    local l1: ref<Array<int32>, borrowed, 'a, mutable, local>
    local l2: isize
    local l3: isize
    local l4: isize
    local l5: isize

entry(v0: ref<Array<int32>, borrowed, 'a, mutable, local>, v1: isize):
    local.set l0, v1
    local.set l1, v0
    v2: isize = local.get l0
    v3: isize = 0
    v4: boolean = lt v2, v3
    branch v4 => b1 | b2

b1:
    v5: isize = 0
    local.set l2, v5
    jump b3

b2:
    v6: isize = local.get l0
    local.set l2, v6
    jump b3

b3:
    v7: isize = local.get l2
    local.set l3, v7
    v8: ref<Array<int32>, borrowed, 'a, mutable, local> = local.get l1
    v9: ref<isize, borrowed, 'a, readonly, local> = field.project v8, 1
    v10: isize = load v9
    local.set l4, v10
    v11: isize = local.get l3
    v12: isize = local.get l4
    v13: boolean = ge v11, v12
    branch v13 => b4 | b5

b4:
    return

b5:
    v14: ref<Array<int32>, borrowed, 'a, mutable, local> = local.get l1
    v15: isize = local.get l3
    v16: ref<isize, borrowed, 'a, mutable, local> = field.project v14, 1
    store v16, v15
    v17: isize = local.get l3
    local.set l5, v17
    jump b6

b6:
    poll
    v18: isize = local.get l5
    v19: isize = local.get l4
    v20: boolean = lt v18, v19
    branch v20 => b7 | b9

b7:
    v21: ref<Array<int32>, borrowed, 'a, mutable, local> = local.get l1
    v22: isize = local.get l5
    v23: usize = call Cast.truncate<isize, usize>(v22): (isize) => usize
    v24: ref<int32, borrowed, 'a, mutable, local> = call elementSlot<int32, mutable>(v21, v23): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>, usize) => ref<int32, borrowed, 'a, mutable, local>
    call MaybeUninit.assumeInitDrop<int32>(v24): <'a>(ref<int32, borrowed, 'a, mutable, local>) => void
    jump b8

b8:
    v25: isize = local.get l5
    v26: isize = 1
    v27: isize = add v25, v26
    local.set l5, v27
    jump b6

b9:
    return
}

external function elementSlot<T, access A, 'a>(ref<Array<T>, borrowed, 'a, A, local>, usize): ref<T, borrowed, 'a, A, local>

shared function elementSlot<int32, mutable, 'a>(v0: ref<Array<int32>, borrowed, 'a, mutable, local>, v1: usize): ref<int32, borrowed, 'a, mutable, local> {
    local l0: ref<Array<int32>, borrowed, 'a, mutable, local>
    local l1: usize

entry(v0: ref<Array<int32>, borrowed, 'a, mutable, local>, v1: usize):
    local.set l0, v0
    local.set l1, v1
    v2: ref<Array<int32>, borrowed, 'a, mutable, local> = local.get l0
    v3: ref<slice<uninit<int32>, unique, mutable, local>, borrowed, 'a, readonly, local> = field.project v2, 0
    v4: slice<uninit<int32>, borrowed, 'a, mutable, local> = load v3
    v5: usize = local.get l1
    v6: ref<int32, borrowed, 'a, mutable, local> = element.address v4, v5
    return v6
}

external function MaybeUninit.assumeInitDrop<T, 'a>(ref<T, borrowed, 'a, mutable, local>): void

shared function MaybeUninit.assumeInitDrop<int32, 'a>(v0: ref<int32, borrowed, 'a, mutable, local>): void {
    local l0: ref<int32, borrowed, 'a, mutable, local>

entry(v0: ref<int32, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<int32, borrowed, 'a, mutable, local> = local.get l0
    call assumeInitDrop<int32>(v1): <'a>(ref<int32, borrowed, 'a, mutable, local>) => void
    return
}

external function assumeInitDrop<T, 'a>(ref<T, borrowed, 'a, mutable, local>): void

shared function assumeInitDrop<int32, 'a>(v0: ref<int32, borrowed, 'a, mutable, local>): void {
    local l0: ref<int32, borrowed, 'a, mutable, local>

entry(v0: ref<int32, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<int32, borrowed, 'a, mutable, local> = local.get l0
    v2: ptr<int32, mutable> = intrinsic.memory.raw.transmute(v1)
    v3: int32 = load v2
    drop v3
    return
}

function drop.frame<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<test.main.Path, borrowed, 'a, mutable, frame>):
    v1: ref<Array<int32>, borrowed, 'a, mutable, frame> = field.project v0, 0
    call drop.frame<Array<int32>>(v1): <'a>(ref<Array<int32>, borrowed, 'a, mutable, frame>) => void
    return
}

function drop.frame<Array<int32>, 'a>(v0: ref<Array<int32>, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Array<int32>, borrowed, 'a, mutable, frame>):
    v1: ref<Array<int32>, borrowed, 'a, mutable, local> = cast.bit v0 -> ref<Array<int32>, borrowed, 'a, mutable, local>
    call Array.Drop.drop<int32>(v1): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>) => void
    v2: ref<usize, borrowed, 'a, mutable, frame> = field.project v0, 2
    v3: ref<isize, borrowed, 'a, mutable, frame> = field.project v0, 1
    v4: ref<slice<uninit<int32>, unique, mutable, local>, borrowed, 'a, mutable, frame> = field.project v0, 0
    v5: slice<uninit<int32>, unique, mutable, local> = load v4
    release v5
    return
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=Drop size=0 align=1
/// @layout.struct name=Concrete size=0 align=1
/// @layout.struct name=Copy size=0 align=1
/// @layout.struct name=IntegerDomain size=0 align=1
/// @layout.struct name=Zero size=0 align=1
/// @layout.struct name=One size=0 align=1
/// @layout.struct name=Integer size=0 align=1
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=Array<int32> size=32 align=8
/// @layout.field owner=Array<int32> index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=Array<int32> index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=Array<int32> index=2 name=allocated offset=24 size=8 align=8
/// @layout.struct name=type@17 size=32 align=8
/// @layout.field owner=type@17 index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=type@17 index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=type@17 index=2 name=allocated offset=24 size=8 align=8
/// @layout.variant name=type@120 size=8 align=8
/// @layout.discriminant owner=type@120 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@120 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@120 index=1 discriminant=1 payload_offset=0

/// @dispatch.shape constraint=type@23 function=clone function=cloneFrom
/// @dispatch.shape constraint=type@31 function=drop
/// @dispatch.shape constraint=type@53 function=zero
/// @dispatch.shape constraint=type@55 function=one
"#,
    );
}

/// Dispatch each closed receiver of one generic base through its own witness.
#[test]
fn test_instantiate_dispatches_each_receiver_of_one_base_through_its_own_witness() {
    let session = TestSession::single(
        r#"
import { rc } from "destack:memory";

function duplicate<T: Clone>(value: &readonly T): T {
    return value.clone();
}

function main(): (rc.Rc<int32>, rc.Rc<int64>) {
    const first = rc.Rc.new(1 as int32);
    const second = rc.Rc.new(2 as int64);
    return (duplicate(first), duplicate(second));
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.ds",
        "test.main.duplicate<Rc<int32>>",
        r#"
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int32>, 'a>(v0: ref<Rc<int32>, borrowed, 'a, readonly, local>): Rc<int32> {
    local l0: ref<Rc<int32>, borrowed, 'a, readonly, local>

entry(v0: ref<Rc<int32>, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<Rc<int32>, borrowed, 'a, readonly, local> = local.get l0
    v2: Rc<int32> = call Rc.Clone.clone<int32>(v1): <'a>(ref<Rc<int32>, borrowed, 'a, readonly, local>) => Rc<int32>
    return v2
}
"#,
    );
    session.assert_mir_elaborated_function(
        "main.ds",
        "test.main.duplicate<Rc<int64>>",
        r#"
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int64>, 'a>(v0: ref<Rc<int64>, borrowed, 'a, readonly, local>): Rc<int64> {
    local l0: ref<Rc<int64>, borrowed, 'a, readonly, local>

entry(v0: ref<Rc<int64>, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<Rc<int64>, borrowed, 'a, readonly, local> = local.get l0
    v2: Rc<int64> = call Rc.Clone.clone<int64>(v1): <'a>(ref<Rc<int64>, borrowed, 'a, readonly, local>) => Rc<int64>
    return v2
}
"#,
    );
}

/// Instantiate an associated const read at a closed receiver as a load of the witness's global.
#[test]
fn test_instantiate_resolves_an_associated_const_through_the_witness() {
    let session = TestSession::single(
        r#"
interface Tagged {
    const Tag: int32;
}

struct Point implements Tagged {
    x: int32;
    const Tag: int32 = 7;
}

function tagOf<T: Tagged>(): int32 {
    return T.Tag;
}

function main(): int32 {
    return tagOf<Point>();
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.ds",
        "test.main.tagOf<test.main.Point>",
        r#"
@copy
type test.main.Point {
    x: int32;
}

shared function test.main.tagOf<test.main.Point>(): int32 {
entry:
    v1: ref<int32, borrowed, readonly, constant> = global.project test.main.Point.Tag
    v0: int32 = load v1
    return v0
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}
