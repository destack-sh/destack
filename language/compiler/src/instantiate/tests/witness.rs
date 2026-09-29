use crate::tests::TestSession;

/// Dispatch a requirement call in a template through the receiver's witness at the instance.
#[test]
fn test_instantiate_dispatches_a_requirement_through_the_witness() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
}

function duplicate<T: Clone>(value: &immutable T): T {
    return value.clone();
}

function main(): Path {
    const path = Path { steps: [] };
    return duplicate(path);
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.duplicate<test.main.Path>",
        r#"
type test.main.Path {
    steps: Array<int32>;
}

shared function test.main.duplicate<test.main.Path, 'a>(v0: ref<test.main.Path, borrowed, 'a, immutable>): test.main.Path {
    local l0: ref<test.main.Path, borrowed, 'a, immutable>

entry(v0: ref<test.main.Path, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Path, borrowed, 'a, immutable> = load l0
    v2: test.main.Path = call test.main.Clone.clone<test.main.Path>(v1): (ref<test.main.Path, borrowed, 'a, immutable>) => test.main.Path
    v3: test.main.Path = copy v2
    drop v2
    return v3
}

/// @layout.struct name=test.main.Path size=40 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=40 align=8
"#,
    );
}

/// Dispatch each closed receiver of one generic base through its own witness.
#[test]
fn test_instantiate_dispatches_each_receiver_of_one_base_through_its_own_witness() {
    let session = TestSession::single(
        r#"
import { rc } from "tspp:memory";

function duplicate<T: Clone>(value: &immutable T): T {
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
        "main.tspp",
        "test.main.duplicate<Rc<int32>>",
        r#"
@nocopy
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int32>, 'a>(v0: ref<Rc<int32>, borrowed, 'a, immutable>): Rc<int32> {
    local l0: ref<Rc<int32>, borrowed, 'a, immutable>

entry(v0: ref<Rc<int32>, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<Rc<int32>, borrowed, 'a, immutable> = load l0
    v2: Rc<int32> = call Rc.Clone.clone<int32>(v1): (ref<Rc<int32>, borrowed, 'a, immutable>) => Rc<int32>
    v3: Rc<int32> = copy v2
    drop v2
    return v3
}
"#,
    );
    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.duplicate<Rc<int64>>",
        r#"
@nocopy
@languageItem("memory.rc.Rc")
type Rc<T>;

shared function test.main.duplicate<Rc<int64>, 'a>(v0: ref<Rc<int64>, borrowed, 'a, immutable>): Rc<int64> {
    local l0: ref<Rc<int64>, borrowed, 'a, immutable>

entry(v0: ref<Rc<int64>, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<Rc<int64>, borrowed, 'a, immutable> = load l0
    v2: Rc<int64> = call Rc.Clone.clone<int64>(v1): (ref<Rc<int64>, borrowed, 'a, immutable>) => Rc<int64>
    v3: Rc<int64> = copy v2
    drop v2
    return v3
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
        "main.tspp",
        "test.main.tagOf<test.main.Point>",
        r#"
type test.main.Point {
    x: int32;
}

shared function test.main.tagOf<test.main.Point>(): int32 {
entry:
    v0: int32 = load @test.main.Point.Tag
    return v0
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

/// A specialization closing an erasure records the dynamic table of the closed payload.
#[test]
fn test_instantiate_the_dynamic_table_of_a_closed_erasure() {
    let session = TestSession::single(
        r#"
interface Greeter {
    greet(): int32;
}

class Console implements Greeter {
    greet(): int32 {
        return 1;
    }
}

function erase<T: Greeter>(value: T): Greeter {
    return value;
}

function run(): int32 {
    return erase(new Console()).greet();
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp", r#"
@nocopy
type test.main.Console = class {  };

@nocopy
type test.main.Greeter { }

export function test.main.Console.greet(v0: ref<test.main.Console, managed, mutable, local>): int32 {
    local l0: ref<test.main.Console, managed, mutable, local>

entry(v0: ref<test.main.Console, managed, mutable, local>):
    store l0, v0
    v1: int32 = 1
    return v1
}

export function test.main.run(): int32 {
entry:
    v0: ref<test.main.Console, managed, mutable, local> = new.zeroed test.main.Console, local
    v1: dynamic<test.main.Greeter, managed, mutable, local> = call test.main.erase<ref<test.main.Console, managed, mutable, local>>(v0): (ref<test.main.Console, managed, mutable, local>) => dynamic<test.main.Greeter, managed, mutable, local>
    v2: ref<test.main.Greeter, managed, mutable, local> = dynamic.payload v1
    v3: int32 = call.dynamic v1, test.main.Greeter, 0(v2): (ref<test.main.Greeter, managed, mutable, local>) => int32
    return v3
}

export function test.main.erase<T: test.main.Greeter>(v0: T): dynamic<test.main.Greeter, managed, mutable, local>;

external function test.main.Greeter.greet<this: test.main.Greeter>(this): int32

shared function test.main.erase<ref<test.main.Console, managed, mutable, local>>(v0: ref<test.main.Console, managed, mutable, local>): dynamic<test.main.Greeter, managed, mutable, local> {
    local l0: ref<test.main.Console, managed, mutable, local>

entry(v0: ref<test.main.Console, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Console, managed, mutable, local> = load l0
    v2: dynamic<test.main.Greeter, managed, mutable, local> = dynamic.bind v1, ref<test.main.Console, managed, mutable, local>[0]
    return v2
}

/// @layout.class name=test.main.Console size=4 align=4
/// @layout.struct name=test.main.Greeter size=0 align=1
/// @layout.class name=type@2 size=4 align=4
/// @layout.struct name=type@8 size=0 align=1

/// @dispatch.virtual concrete=type@0 value=type@1 conformance=type@5
/// @dispatch.table concrete=type@1 constraint=type@5 function=test.main.Console.greet
"#,
    );
}

/// An erased function value answers its call slot through a shim loading it from the payload.
#[test]
fn test_instantiate_a_payload_shim_for_an_erased_function_value() {
    let session = TestSession::single(
        r#"
interface Adder {
    (value: int32): int32;
}

function add(adder: Adder): int32 {
    adder(1)
}

function run(): int32 {
    add((value: int32): int32 => value + 1)
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp", r#"
@nocopy
type test.main.Adder { }

export function test.main.add(v0: dynamic<test.main.Adder, managed, mutable, local>): int32 {
    local l0: dynamic<test.main.Adder, managed, mutable, local>

entry(v0: dynamic<test.main.Adder, managed, mutable, local>):
    store l0, v0
    v1: dynamic<test.main.Adder, managed, mutable, local> = load l0
    v2: int32 = 1
    v3: ref<test.main.Adder, managed, mutable, local> = dynamic.payload v1
    v4: int32 = call.dynamic v1, test.main.Adder, 0(v3, v2): (ref<test.main.Adder, managed, mutable, local>, int32) => int32
    return v4
}

export function test.main.run(): int32 {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.run.closure#0, v0
    v2: uninit<ref<function<(int32) => int32, repeatable, managed, mutable, local>, managed, mutable, local>> = new.uninit function<(int32) => int32, repeatable, managed, mutable, local>, local
    store (*v2), v1
    v3: ref<function<(int32) => int32, repeatable, managed, mutable, local>, managed, mutable, local> = new.complete v2
    v4: dynamic<test.main.Adder, managed, mutable, local> = dynamic.bind v3, function<(int32) => int32, repeatable, managed, mutable, local>
    v5: int32 = call test.main.add(v4): (dynamic<test.main.Adder, managed, mutable, local>) => int32
    return v5
}

external function test.main.Adder.()<this: test.main.Adder>(this, int32): int32

export function test.main.Adder.()<function<(int32) => int32, repeatable, managed, mutable, local>>(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32): int32 {
    local l0: int32
    local l1: function<(int32) => int32, repeatable, managed, mutable, local>

entry(v0: function<(int32) => int32, repeatable, managed, mutable, local>, v1: int32):
    store l0, v1
    store l1, v0
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = load l1
    v3: int32 = load l0
    v4: function<(int32) => int32, repeatable, borrowed, 'managed, readonly> = cast.bit v2 -> function<(int32) => int32, repeatable, borrowed, 'managed, readonly>
    v5: int32 = call.indirect v4(v3): (int32) => int32
    return v5
}

export function test.main.run.closure#0(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = 1
    v3: int32 = add v1, v2
    return v3
}

shared function test.main.Adder.().shim<'payload>(v0: ref<function<(int32) => int32, repeatable, managed, mutable, local>, borrowed, 'payload, readonly>, v1: int32): int32 {
entry(v0: ref<function<(int32) => int32, repeatable, managed, mutable, local>, borrowed, 'payload, readonly>, v1: int32):
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = load (*v0)
    v3: int32 = call test.main.Adder.()<function<(int32) => int32, repeatable, managed, mutable, local>>(v2, v1): (function<(int32) => int32, repeatable, managed, mutable, local>, int32) => int32
    return v3
}

/// @layout.struct name=test.main.Adder size=0 align=1
/// @layout.struct name=type@4 size=0 align=1

/// @dispatch.table concrete=type@5 constraint=type@0 function=test.main.Adder.().shim
"#,
    );
}

/// A class extending a generic base inherits the base's virtual methods at its heritage arguments.
#[test]
fn test_instantiate_the_virtual_table_of_a_class_extending_a_generic_base() {
    let session = TestSession::single(
        r#"
class Base<T: Copy> {
    item: T;

    constructor(item: T) {
        this.item = item;
    }

    virtual get(): T {
        this.item
    }
}

class Derived extends Base<int32> {
    constructor() {
        super(1);
    }
}

class Wrapper<T: Copy> extends Base<T> {
    constructor(item: T) {
        super(item);
    }

    override get(): T {
        this.item
    }
}

function read(base: Base<int32>): int32 {
    base.get()
}

function run(): int32 {
    read(new Derived()) + read(new Wrapper<int32>(2))
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp", r#"
@nocopy
type test.main.Derived = class<test.main.Base<int32>> { item: int32 };

@nocopy
type test.main.Base<T: Copy> = class { item: T };

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

@nocopy
type test.main.Wrapper<T: Copy> = class<test.main.Base<T>> { item: T };

export constructor test.main.Derived.constructor(v0: ref<uninit<test.main.Derived>, borrowed, 'managed, mutable>): void {
    local l0: ref<uninit<test.main.Derived>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Derived>, borrowed, 'managed, mutable>):
    store l0, v0
    v1: ref<uninit<test.main.Derived>, borrowed, 'managed, mutable> = address (*l0)
    v2: ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable>
    v3: int32 = 1
    call test.main.Base.constructor<int32>(v2, v3): (ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable>, int32) => void
    v4: void = zeroed
    return
}

export function test.main.read(v0: ref<test.main.Base<int32>, managed, mutable, local>): int32 {
    local l0: ref<test.main.Base<int32>, managed, mutable, local>

entry(v0: ref<test.main.Base<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Base<int32>, managed, mutable, local> = load l0
    v2: int32 = call.virtual test.main.Base<int32>, 0(v1): (ref<test.main.Base<int32>, managed, mutable, local>) => int32
    return v2
}

export function test.main.run(): int32 {
entry:
    v0: ref<test.main.Derived, managed, mutable, local> = new.zeroed test.main.Derived, local
    v1: ref<uninit<test.main.Derived>, borrowed, 'managed, mutable> = cast.bit v0 -> ref<uninit<test.main.Derived>, borrowed, 'managed, mutable>
    call test.main.Derived.constructor(v1): (ref<uninit<test.main.Derived>, borrowed, 'managed, mutable>) => void
    v2: ref<test.main.Base<int32>, managed, mutable, local> = cast.bit v0 -> ref<test.main.Base<int32>, managed, mutable, local>
    v3: int32 = call test.main.read(v2): (ref<test.main.Base<int32>, managed, mutable, local>) => int32
    v4: int32 = 2
    v5: ref<test.main.Wrapper<int32>, managed, mutable, local> = new.zeroed test.main.Wrapper<int32>, local
    v6: ref<uninit<test.main.Wrapper<int32>>, borrowed, 'managed, mutable> = cast.bit v5 -> ref<uninit<test.main.Wrapper<int32>>, borrowed, 'managed, mutable>
    call test.main.Wrapper.constructor<int32>(v6, v4): (ref<uninit<test.main.Wrapper<int32>>, borrowed, 'managed, mutable>, int32) => void
    v7: ref<test.main.Base<int32>, managed, mutable, local> = cast.bit v5 -> ref<test.main.Base<int32>, managed, mutable, local>
    v8: int32 = call test.main.read(v7): (ref<test.main.Base<int32>, managed, mutable, local>) => int32
    v9: int32 = add v3, v8
    return v9
}

export constructor test.main.Base.constructor<T: Copy>(v0: ref<uninit<test.main.Base<T>>, borrowed, 'managed, mutable>, v1: T): void;

export function test.main.Base.get<T: Copy>(v0: ref<test.main.Base<T>, managed, mutable, local>): T;

export constructor test.main.Wrapper.constructor<T: Copy>(v0: ref<uninit<test.main.Wrapper<T>>, borrowed, 'managed, mutable>, v1: T): void;

export function test.main.Wrapper.get<T: Copy>(v0: ref<test.main.Wrapper<T>, managed, mutable, local>): T;

shared constructor test.main.Base.constructor<int32>(v0: ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

shared constructor test.main.Wrapper.constructor<int32>(v0: ref<uninit<test.main.Wrapper<int32>>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Wrapper<int32>>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Wrapper<int32>>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Wrapper<int32>>, borrowed, 'managed, mutable> = address (*l1)
    v3: ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable>
    v4: int32 = load l0
    call test.main.Base.constructor<int32>(v3, v4): (ref<uninit<test.main.Base<int32>>, borrowed, 'managed, mutable>, int32) => void
    v5: void = zeroed
    return
}

shared function test.main.Base.get<int32>(v0: ref<test.main.Base<int32>, managed, mutable, local>): int32 {
    local l0: ref<test.main.Base<int32>, managed, mutable, local>

entry(v0: ref<test.main.Base<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Base<int32>, managed, mutable, local> = load l0
    v2: int32 = load (*v1).0
    return v2
}

shared function test.main.Wrapper.get<int32>(v0: ref<test.main.Wrapper<int32>, managed, mutable, local>): int32 {
    local l0: ref<test.main.Wrapper<int32>, managed, mutable, local>

entry(v0: ref<test.main.Wrapper<int32>, managed, mutable, local>):
    store l0, v0
    v1: ref<test.main.Wrapper<int32>, managed, mutable, local> = load l0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.class name=test.main.Derived size=8 align=4
/// @layout.field owner=test.main.Derived index=0 name=item offset=4 size=4 align=4
/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=type@7 size=0 align=1
/// @layout.class name=test.main.Base<int32> size=8 align=4
/// @layout.field owner=test.main.Base<int32> index=0 name=item offset=4 size=4 align=4
/// @layout.class name=type@18 size=8 align=4
/// @layout.field owner=type@18 index=0 name=item offset=4 size=4 align=4
/// @layout.class name=type@19 size=8 align=4
/// @layout.field owner=type@19 index=0 name=item offset=4 size=4 align=4
/// @layout.class name=test.main.Wrapper<int32> size=8 align=4
/// @layout.field owner=test.main.Wrapper<int32> index=0 name=item offset=4 size=4 align=4

/// @dispatch.virtual concrete=type@0 value=type@1 method=test.main.Base.get<int32>
/// @dispatch.virtual concrete=type@16 value=type@17 method=test.main.Base.get<int32>
/// @dispatch.virtual concrete=type@47 value=type@51 method=test.main.Wrapper.get<int32>
"#,
    );
}

/// Borrow a whole receiver for a struct implementation of an interface method.
#[test]
fn test_instantiate_an_elided_receiver_call_at_a_struct_implementation() {
    let session = TestSession::single(
        r#"
interface Shape {
    area(): float64;
}

struct Square implements Shape {
    side: float64;

    area(): float64 {
        this.side * this.side
    }
}

function total<T: Shape>(shape: T): float64 {
    shape.area()
}

function main(): float64 {
    total(Square { side: 2.0 })
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.total<test.main.Square>",
        r#"
type test.main.Square {
    side: float64;
}

shared function test.main.total<test.main.Square>(v0: test.main.Square): float64 {
    local l0: test.main.Square
    local l1: test.main.Square

entry(v0: test.main.Square):
    store l0, v0
    v1: test.main.Square = load l0
    store l1, v1
    v3: ref<test.main.Square, borrowed, '_, readonly> = address l1
    v2: float64 = call test.main.Square.area(v3): <'a>(ref<test.main.Square, borrowed, 'a, readonly>) => float64
    return v2
}

/// @layout.struct name=test.main.Square size=8 align=8
/// @layout.field owner=test.main.Square index=0 name=side offset=0 size=8 align=8
"#,
    );
}

/// Select an interface field by name at each instance, through a class handle and inline in a struct.
#[test]
fn test_instantiate_an_interface_field_read_by_name() {
    let session = TestSession::single(
        r#"
interface Named {
    name: string;
}

class Person implements Named {
    age: int32 = 0;
    name: string = "ada";
}

struct Pet implements Named {
    name: string;
}

function label<T: Named>(value: &readonly T): string {
    value.name
}

function main(): void {
    const person = new Person();
    label(&readonly person);
    const pet = Pet { name: "rex" };
    label(&readonly pet);
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.label<ref<test.main.Person, managed, mutable, local>>",
        r#"
@nocopy
type test.main.Person = class { age: int32, name: ref<String, managed, mutable, local> };

@nocopy
@languageItem("string.String")
type String;

shared function test.main.label<ref<test.main.Person, managed, mutable, local>, 'a>(v0: ref<test.main.Person, borrowed, 'a, readonly>): ref<String, managed, mutable, local> {
    local l0: ref<test.main.Person, borrowed, 'a, readonly>

entry(v0: ref<test.main.Person, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Person, borrowed, 'a, readonly> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v1).1
    v3: ref<String, managed, mutable, local> = load (*v2)
    return v3
}

/// @layout.class name=test.main.Person size=24 align=8
/// @layout.field owner=test.main.Person index=0 name=age offset=16 size=4 align=4
/// @layout.field owner=test.main.Person index=1 name=name offset=8 size=8 align=8
"#,
    );
    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.label<test.main.Pet>",
        r#"
@nocopy
@languageItem("string.String")
type String;

type test.main.Pet {
    name: ref<String, managed, mutable, local>;
}

shared function test.main.label<test.main.Pet, 'a>(v0: ref<test.main.Pet, borrowed, 'a, readonly>): ref<String, managed, mutable, local> {
    local l0: ref<test.main.Pet, borrowed, 'a, readonly>

entry(v0: ref<test.main.Pet, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Pet, borrowed, 'a, readonly> = load l0
    v2: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v1).0
    v3: ref<String, managed, mutable, local> = load (*v2)
    return v3
}

/// @layout.struct name=test.main.Pet size=8 align=8
/// @layout.field owner=test.main.Pet index=0 name=name offset=0 size=8 align=8
"#,
    );
}

/// Display an enum case through the display of its declared value.
#[test]
fn test_instantiate_an_enum_display_through_its_declared_value() {
    let session = TestSession::single(
        r#"
enum Level {
    Info = "info",
}

function main(): void {
    const level = Level.Info;
    const text = level.toString();
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
type test.main.Level = newtype<ref<String, managed, mutable, local>>;

@nocopy
@languageItem("string.String")
type String = class { codeUnits: slice<uint16, unique, mutable> };

@nocopy
@languageItem("ops.Display")
type Display { }

shared constant string.0: String = "info"

export function test.main.main(): void {
    local l0: test.main.Level
    local l1: String

entry:
    v0: ref<String, managed, mutable, local> = address @string.0
    v1: test.main.Level = aggregate (v0)
    store l0, v1
    v2: ref<test.main.Level, borrowed, 'frame, immutable> = address l0
    v3: String = call Display.toString<test.main.Level>(v2): (ref<test.main.Level, borrowed, 'frame, immutable>) => String
    store l1, v3
    v4: String = load l1
    drop v4
    return
}

external function Display.display<this: Display, 'a>(ref<?this, borrowed, 'a, immutable>): String

export function test.main.Display.display<test.main.Level, 'a>(v0: ref<test.main.Level, borrowed, 'a, immutable>): String {
    local l0: ref<test.main.Level, borrowed, 'a, immutable>

entry(v0: ref<test.main.Level, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Level, borrowed, 'a, immutable> = load l0
    v2: test.main.Level = load (*v1)
    v3: ref<String, managed, mutable, local> = field.get v2, 0
    v4: ref<String, borrowed, 'a, immutable> = cast.bit v3 -> ref<String, borrowed, 'a, immutable>
    v5: String = call String.Display.display(v4): (ref<String, borrowed, 'a, immutable>) => String
    return v5
}

external function Display.toString<T: Display, 'a>(ref<?T, borrowed, 'a, immutable>): String

shared function Display.toString<test.main.Level, 'a>(v0: ref<test.main.Level, borrowed, 'a, immutable>): String {
    local l0: ref<test.main.Level, borrowed, 'a, immutable>

entry(v0: ref<test.main.Level, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Level, borrowed, 'a, immutable> = load l0
    v2: String = call test.main.Display.display<test.main.Level>(v1): (ref<test.main.Level, borrowed, 'a, immutable>) => String
    return v2
}

external function String.Display.display<'a>(ref<String, borrowed, 'a, immutable>): String

shared function drop.frame<String, 'a>(v0: ref<String, borrowed, 'a, exclusive>): void {
entry(v0: ref<String, borrowed, 'a, exclusive>):
    v1: ref<slice<uint16, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: slice<uint16, unique, mutable> = load (*v1)
    release v2
    return
}

/// @layout.class name=String size=24 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.class name=type@5 size=24 align=8
/// @layout.field owner=type@5 index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.struct name=type@10 size=0 align=1
"#,
    );
}

/// Read an enum case's declared value through a cast, a module initializer included.
#[test]
fn test_instantiate_an_enum_value_as_its_declared_value() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function code(mode: Mode): int64 {
    mode as int64
}

const mode: Mode = Mode.Write;
const initial: int64 = mode as int64;
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
type test.main.Mode = variant<uint8> { 1uint8 = void; 2uint8 = void; };

export constant test.main.mode: test.main.Mode = variant 1
export global test.main.initial: int64 = zeroinit

export function test.main.code(v0: test.main.Mode): int64 {
    local l0: test.main.Mode

entry(v0: test.main.Mode):
    store l0, v0
    v1: test.main.Mode = load l0
    v2: uint8 = variant.tag v1
    v3: int64 = cast.intToInt v2 -> int64
    return v3
}

export park function test.main.@init(): void {
entry:
    v0: test.main.Mode = load @test.main.mode
    v1: uint8 = variant.tag v0
    v2: int64 = cast.intToInt v1 -> int64
    store @test.main.initial, v2
    return
}

/// @layout.variant name=test.main.Mode size=1 align=1
/// @layout.discriminant owner=test.main.Mode kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=test.main.Mode index=0 discriminant=1 payload_offset=1
/// @layout.case owner=test.main.Mode index=1 discriminant=2 payload_offset=1
/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=1 payload_offset=1
/// @layout.case owner=type@3 index=1 discriminant=2 payload_offset=1
"#,
    );
}

/// Dispatch a requirement at a primitive through the blanket implementation over its family.
#[test]
fn test_instantiate_a_primitive_witness_from_a_blanket_implementation() {
    let session = TestSession::single(
        r#"
function show<T: Display>(value: &immutable T): ^string {
    value.display()
}

function same<T: Equal>(left: &immutable T, right: &immutable T): boolean {
    left.equal(right)
}

function main(value: int32): void {
    show(&immutable value);
    same(&immutable value, &immutable value);
    const flag = true;
    flag.toString();
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
@nocopy
@languageItem("string.String")
type String = class { codeUnits: slice<uint16, unique, mutable> };

@nocopy
@languageItem("ops.Display")
type Display { }

@nocopy
@languageItem("ops.Equal")
type Equal<T> extends PartialEqual<T> { }

@nocopy
@languageItem("ops.PartialEqual")
type PartialEqual<T> { }

@nocopy
@languageItem("math.Integer")
type Integer extends Concrete, Copy, IntegerDomain, Zero, One { }

@nocopy
@languageItem("memory.Concrete")
type Concrete { }

@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

@nocopy
@languageItem("math.IntegerDomain")
type IntegerDomain { }

@nocopy
@languageItem("math.Zero")
type Zero { }

@nocopy
@languageItem("math.One")
type One { }

type literal.boolean.true { }

external constant string.2: String

export function test.main.main(v0: int32): void {
    local l0: int32
    local l1: literal.boolean.true
    local l2: boolean

entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'frame, immutable> = address l0
    v2: String = call test.main.show<int32>(v1): (ref<int32, borrowed, 'frame, immutable>) => String
    drop v2
    v3: ref<int32, borrowed, 'frame, immutable> = address l0
    v4: ref<int32, borrowed, 'frame, immutable> = address l0
    v5: boolean = call test.main.same<int32>(v3, v4): (ref<int32, borrowed, 'frame, immutable>, ref<int32, borrowed, 'frame, immutable>) => boolean
    v6: literal.boolean.true = zeroed
    store l1, v6
    v7: literal.boolean.true = load l1
    v8: literal.boolean.true = zeroed
    v9: boolean = true
    store l2, v9
    v10: ref<boolean, borrowed, 'frame, immutable> = address l2
    v11: String = call Display.toString<boolean>(v10): (ref<boolean, borrowed, 'frame, immutable>) => String
    drop v11
    return
}

export function test.main.show<T: Display, 'a>(v0: ref<?T, borrowed, 'a, immutable>): String;

export function test.main.same<T: Equal<T>, 'a, 'b>(v0: ref<?T, borrowed, 'a, immutable>, v1: ref<?T, borrowed, 'b, immutable>): boolean;

external function Display.display<this: Display, 'a>(ref<?this, borrowed, 'a, immutable>): String

external function Integer.Display.display<T: Integer, 'a>(ref<?T, borrowed, 'a, immutable>): String

shared function Integer.Display.display<int32, 'a>(v0: ref<int32, borrowed, 'a, immutable>): String {
    local l0: ref<int32, borrowed, 'a, immutable>

entry(v0: ref<int32, borrowed, 'a, immutable>):
    store l0, v0
    v1: int32 = load (*l0)
    v2: variant<uint1> { 0uint1 = float64; 1uint1 = void; } = variant.new 1
    v3: String = call Integer.toString<int32>(v1, v2): (int32, variant<uint1> { 0uint1 = float64; 1uint1 = void; }) => String
    return v3
}

external function PartialEqual.equal<T, this: PartialEqual<T>, 'a, 'b>(ref<?this, borrowed, 'a, immutable>, ref<?T, borrowed, 'b, immutable>): boolean

external function Integer.PartialEqual.equal<T: Integer, 'a, 'b>(ref<?T, borrowed, 'a, immutable>, ref<?T, borrowed, 'b, immutable>): boolean

shared function Integer.PartialEqual.equal<int32, 'a, 'b>(v0: ref<int32, borrowed, 'a, immutable>, v1: ref<int32, borrowed, 'b, immutable>): boolean {
    local l0: ref<int32, borrowed, 'b, immutable>
    local l1: ref<int32, borrowed, 'a, immutable>

entry(v0: ref<int32, borrowed, 'a, immutable>, v1: ref<int32, borrowed, 'b, immutable>):
    store l0, v1
    store l1, v0
    v2: int32 = load (*l1)
    v3: int32 = load (*l0)
    v4: boolean = eq v2, v3
    return v4
}

external function Boolean.Display.display<'a>(ref<boolean, borrowed, 'a, immutable>): String

shared function test.main.show<int32, 'a>(v0: ref<int32, borrowed, 'a, immutable>): String {
    local l0: ref<int32, borrowed, 'a, immutable>

entry(v0: ref<int32, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<int32, borrowed, 'a, immutable> = load l0
    v2: String = call Integer.Display.display<int32>(v1): (ref<int32, borrowed, 'a, immutable>) => String
    return v2
}

shared function test.main.same<int32, 'a, 'b>(v0: ref<int32, borrowed, 'a, immutable>, v1: ref<int32, borrowed, 'b, immutable>): boolean {
    local l0: ref<int32, borrowed, 'a, immutable>
    local l1: ref<int32, borrowed, 'b, immutable>

entry(v0: ref<int32, borrowed, 'a, immutable>, v1: ref<int32, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<int32, borrowed, 'a, immutable> = load l0
    v3: ref<int32, borrowed, 'b, immutable> = load l1
    v4: boolean = call Integer.PartialEqual.equal<int32>(v2, v3): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    return v4
}

external function Display.toString<T: Display, 'a>(ref<?T, borrowed, 'a, immutable>): String

shared function Display.toString<boolean, 'a>(v0: ref<boolean, borrowed, 'a, immutable>): String {
    local l0: ref<boolean, borrowed, 'a, immutable>

entry(v0: ref<boolean, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<boolean, borrowed, 'a, immutable> = load l0
    v2: String = call Boolean.Display.display(v1): (ref<boolean, borrowed, 'a, immutable>) => String
    return v2
}

external function Integer.toString<T: Integer>(T, variant<uint1> { 0uint1 = float64; 1uint1 = void; }): String

shared function Integer.toString<int32>(v0: int32, v1: variant<uint1> { 0uint1 = float64; 1uint1 = void; }): String {
    local l0: variant<uint1> { 0uint1 = float64; 1uint1 = void; }
    local l1: int32

entry(v0: int32, v1: variant<uint1> { 0uint1 = float64; 1uint1 = void; }):
    store l0, v1
    store l1, v0
    v2: ref<String, managed, mutable, local> = address @string.2
    v3: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v2
    v4: never = call todo(v3): (variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }) => never
    unreachable

b1:
    return
}

external function todo(variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }): never

shared function drop.frame<String, 'a>(v0: ref<String, borrowed, 'a, exclusive>): void {
entry(v0: ref<String, borrowed, 'a, exclusive>):
    v1: ref<slice<uint16, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: slice<uint16, unique, mutable> = load (*v1)
    release v2
    return
}

/// @layout.class name=String size=24 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.struct name=Concrete size=0 align=1
/// @layout.struct name=Copy size=0 align=1
/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=IntegerDomain size=0 align=1
/// @layout.struct name=Zero size=0 align=1
/// @layout.struct name=One size=0 align=1
/// @layout.struct name=literal.boolean.true size=0 align=1
/// @layout.class name=type@9 size=24 align=8
/// @layout.field owner=type@9 index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.struct name=type@12 size=0 align=1
/// @layout.variant name=type@64 size=16 align=8
/// @layout.discriminant owner=type@64 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@64 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@64 index=1 discriminant=1 payload_offset=8
/// @layout.variant name=type@67 size=8 align=8
/// @layout.discriminant owner=type@67 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@67 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@67 index=1 discriminant=1 payload_offset=0
"#,
    );
}

/// Select a generic implementation at a closed type reached only inside another instance.
#[test]
fn test_instantiate_a_generic_implementation_selected_inside_an_instance() {
    let session = TestSession::single(
        r#"
import { Deque } from "tspp:collections";

function fresh<T: Default>(): T {
    T.default()
}

function wrap<U>(): Deque<U> {
    fresh<Deque<U>>()
}

function main(): void {
    wrap<int32>();
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
@nocopy
@languageItem("memory.Default")
type Default { }

@nocopy
@languageItem("collections.Deque")
type Deque<T> = class { storage: slice<uninit<T>, unique, mutable>, start: usize, capacity: usize, count: isize };

@nocopy
@languageItem("string.String")
type String = class { codeUnits: slice<uint16, unique, mutable> };

external constant string.18: String

export function test.main.main(): void {
entry:
    v0: ref<Deque<int32>, managed, mutable, local> = call test.main.wrap<int32>(): () => ref<Deque<int32>, managed, mutable, local>
    return
}

export function test.main.fresh<T: Default>(): T;

export function test.main.wrap<U>(): ref<Deque<U>, managed, mutable, local>;

external function Default.default<this: Default>(): ?this

external function Deque.Default.default<T>(): Deque<T>

shared function Deque.Default.default<int32>(): Deque<int32> {
entry:
    v0: Deque<int32> = call Deque.new<int32>(): () => Deque<int32>
    return v0
}

shared function test.main.wrap<int32>(): ref<Deque<int32>, managed, mutable, local> {
entry:
    v0: ref<Deque<int32>, managed, mutable, local> = call test.main.fresh<ref<Deque<int32>, managed, mutable, local>>(): () => ref<Deque<int32>, managed, mutable, local>
    return v0
}

shared function test.main.fresh<ref<Deque<int32>, managed, mutable, local>>(): ref<Deque<int32>, managed, mutable, local> {
entry:
    v0: Deque<int32> = call Deque.Default.default<int32>(): () => Deque<int32>
    v1: ref<Deque<int32>, managed, mutable, local> = new.complete v0
    return v1
}

external function Deque.new<T>(): Deque<T>

shared function Deque.new<int32>(): Deque<int32> {
entry:
    v0: ref<String, managed, mutable, local> = address @string.18
    v1: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v0
    v2: never = call todo(v1): (variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }) => never
    unreachable

b1:
    return
}

external function todo(variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }): never

/// @layout.class name=String size=24 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.struct name=type@4 size=0 align=1
/// @layout.class name=Deque<int32> size=48 align=8
/// @layout.field owner=Deque<int32> index=0 name=storage offset=8 size=16 align=8
/// @layout.field owner=Deque<int32> index=1 name=start offset=24 size=8 align=8
/// @layout.field owner=Deque<int32> index=2 name=capacity offset=32 size=8 align=8
/// @layout.field owner=Deque<int32> index=3 name=count offset=40 size=8 align=8
/// @layout.class name=type@24 size=48 align=8
/// @layout.field owner=type@24 index=0 name=storage offset=8 size=16 align=8
/// @layout.field owner=type@24 index=1 name=start offset=24 size=8 align=8
/// @layout.field owner=type@24 index=2 name=capacity offset=32 size=8 align=8
/// @layout.field owner=type@24 index=3 name=count offset=40 size=8 align=8
/// @layout.class name=type@30 size=24 align=8
/// @layout.field owner=type@30 index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.variant name=type@33 size=8 align=8
/// @layout.discriminant owner=type@33 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@33 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@33 index=1 discriminant=1 payload_offset=0

/// @dispatch.virtual concrete=type@15 value=type@16
"#,
    );
}
