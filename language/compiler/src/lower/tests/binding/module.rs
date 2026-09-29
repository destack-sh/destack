use crate::tests::TestSession;

/// Lower a module constant to a global read through its address.
#[test]
fn test_lower_module_constants_to_globals() {
    let session = TestSession::single(
        r#"
newtype Flags = uint32

const READ: Flags = Flags(4);

function pick(): Flags {
    return READ;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pick",
        r#"
type test.main.Flags = newtype<uint32>;

export function test.main.pick(): test.main.Flags {
entry:
    v0: test.main.Flags = load @test.main.READ
    return v0
}
"#,
    );
}

/// Store a constant with a runtime initializer from the module initializer.
#[test]
fn test_store_a_runtime_binding_from_the_module_initializer() {
    let session = TestSession::single(
        r#"
function seed(): int32 {
    return 41;
}

const start = seed() + 1;

function run(): int32 {
    return start;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.seed",
        r#"
export function test.main.seed(): int32 {
entry:
    v0: int32 = 41
    return v0
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.run",
        r#"
export function test.main.run(): int32 {
entry:
    v0: int32 = load @test.main.start
    return v0
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.@init",
        r#"
export park function test.main.@init(): void {
entry:
    v0: int32 = call test.main.seed(): () => int32
    v1: int32 = 1
    v2: int32 = add v0, v1
    store @test.main.start, v2
    return
}
"#,
    );
}

/// Store a struct constant from the module initializer until constant aggregates land.
#[test]
fn test_store_a_struct_constant_from_the_module_initializer() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const ORIGIN = Point { x: 1 };

function pick(): Point {
    return ORIGIN;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.pick",
        r#"
type test.main.Point {
    x: int32;
}

export function test.main.pick(): test.main.Point {
entry:
    v0: test.main.Point = load @test.main.ORIGIN
    return v0
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.@init",
        r#"
type test.main.Point {
    x: int32;
}

export park function test.main.@init(): void {
entry:
    v0: int32 = 1
    v1: test.main.Point = aggregate (v0)
    store @test.main.ORIGIN, v1
    return
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

/// Initialize a string constant with the address of its constant String object.
#[test]
fn test_lower_a_string_constant_to_the_address_of_its_object() {
    let session = TestSession::single(
        r#"
const NAME: string = "tspp";

function pick(): string {
    return NAME;
}
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
@nocopy
@languageItem("string.String")
type String = class { codeUnits: slice<uint16, unique, mutable> };

shared constant string.0: String = "tspp"
export constant test.main.NAME: ref<String, managed, mutable, local> = globalAddress string.0

export function test.main.pick(): ref<String, managed, mutable, local> {
entry:
    v0: ref<String, managed, mutable, local> = load @test.main.NAME
    return v0
}

/// @layout.class name=String size=24 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.class name=type@4 size=24 align=8
/// @layout.field owner=type@4 index=0 name=codeUnits offset=8 size=16 align=8
"#,
    );
}

/// Pair a generic extension's drop hook with each nominal specialization.
#[test]
fn test_pair_generic_extension_drop_hook() {
    let session = TestSession::single(
        r#"
import { Drop, drop } from "tspp:memory";

struct Guard<T> {
    value: T;
}

extension<T> of Guard<T> implements Drop {
    drop(&this): void {}
}

function consume(guard: Guard<int32>): void {
    drop(guard);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.consume",
        r#"
@nocopy
type test.main.Guard<T> {
    value: T;
}

export function test.main.consume(v0: test.main.Guard<int32>): void {
    local l0: test.main.Guard<int32>

entry(v0: test.main.Guard<int32>):
    store l0, v0
    v1: test.main.Guard<int32> = load l0
    call drop<test.main.Guard<int32>>(v1): (test.main.Guard<int32>) => void
    return
}

/// @layout.struct name=test.main.Guard<int32> size=4 align=4
/// @layout.field owner=test.main.Guard<int32> index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.Guard.Drop.drop",
        r#"
@nocopy
type test.main.Guard<T> {
    value: T;
}

export function test.main.Guard.Drop.drop<T, 'a>(v0: ref<test.main.Guard<T>, borrowed, 'a, mutable>): void {
    local l0: ref<test.main.Guard<T>, borrowed, 'a, mutable>

entry(v0: ref<test.main.Guard<T>, borrowed, 'a, mutable>):
    store l0, v0
    return
}
"#,
    );
}

/// Extension members take their target root and, for conformance members, their interface.
#[test]
fn test_name_extension_members_by_target_root_and_interface() {
    let session = TestSession::single(
        r#"
newtype interface Greet {
    greet(&readonly this): int32;
}

struct Cell {
    value: int32;
}

export extension of Cell implements Greet {
    greet(&readonly this): int32 {
        return this.value;
    }

    peek(&readonly this): int32 {
        return this.value;
    }
}

export extension<T: Greet> of T {
    twice(&readonly this): int32 {
        return this.greet() + this.greet();
    }
}
"#,
    );
    session.assert_mir_lowered(
        "main.tspp",
        r#"
type test.main.Cell {
    value: int32;
}

@nocopy
type test.main.Greet { }

export function test.main.Cell.Greet.greet<'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Cell, borrowed, 'a, readonly>

entry(v0: ref<test.main.Cell, borrowed, 'a, readonly>):
    store l0, v0
    v1: int32 = load (*l0).0
    return v1
}

export function test.main.Cell.peek<'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Cell, borrowed, 'a, readonly>

entry(v0: ref<test.main.Cell, borrowed, 'a, readonly>):
    store l0, v0
    v1: int32 = load (*l0).0
    return v1
}

export function test.main.Greet.twice<T: test.main.Greet, 'a>(v0: ref<?T, borrowed, 'a, readonly>): int32 {
    local l0: ref<?T, borrowed, 'a, readonly>

entry(v0: ref<?T, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<?T, borrowed, 'a, readonly> = load l0
    v2: int32 = call.witness T, test.main.Greet, test.main.Greet.greet(v1): (ref<?T, borrowed, 'a, readonly>) => int32
    v3: ref<?T, borrowed, 'a, readonly> = load l0
    v4: int32 = call.witness T, test.main.Greet, test.main.Greet.greet(v3): (ref<?T, borrowed, 'a, readonly>) => int32
    v5: int32 = add v2, v4
    return v5
}

external function test.main.Greet.greet<this: test.main.Greet, 'a>(ref<?this, borrowed, 'a, readonly>): int32

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@2 size=4 align=4
/// @layout.field owner=type@2 index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@10 size=0 align=1

/// @dispatch.shape constraint=type@7 function=greet
"#,
    );
}

/// Store a module-level let in a mutable global the module initializer writes and functions update.
#[test]
fn test_update_a_module_let_from_a_function_and_the_module_body() {
    let session = TestSession::single(
        r#"
let count: int32 = 0;

function bump(): void {
    count = count + 1;
}

bump();
count = count * 2;
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
export global test.main.count: int32 = zeroinit

export function test.main.bump(): void {
entry:
    v0: int32 = load @test.main.count
    v1: int32 = 1
    v2: int32 = add v0, v1
    store @test.main.count, v2
    return
}

export park function test.main.@init(): void {
entry:
    v0: int32 = 0
    store @test.main.count, v0
    call test.main.bump(): () => void
    v1: int32 = load @test.main.count
    v2: int32 = 2
    v3: int32 = mul v1, v2
    store @test.main.count, v3
    return
}
"#,
    );
}

/// Store a function value into a module binding and call it through the binding.
#[test]
fn test_call_a_function_value_stored_in_a_module_binding() {
    let session = TestSession::single(
        r#"
function double(value: int32): int32 {
    value * 2
}

const scale: (value: int32) => int32 = double;

const scaled: int32 = scale(21);
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
export global test.main.scale: function<(int32) => int32, repeatable, managed, mutable, local> = zeroinit
export global test.main.scaled: int32 = zeroinit

export function test.main.double(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = 2
    v3: int32 = mul v1, v2
    return v3
}

export park function test.main.@init(): void {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.double, v0
    store @test.main.scale, v1
    v2: function<(int32) => int32, repeatable, managed, mutable, local> = load @test.main.scale
    v3: int32 = 21
    v4: function<(int32) => int32, repeatable, borrowed, 'managed, mutable> = cast.bit v2 -> function<(int32) => int32, repeatable, borrowed, 'managed, mutable>
    v5: int32 = call.indirect v4(v3): (int32) => int32
    store @test.main.scaled, v5
    return
}
"#,
    );
}

/// Bind a generic function as a value at its specialization.
#[test]
fn test_bind_a_generic_function_value_to_its_specialization() {
    let session = TestSession::single(
        r#"
function same<T>(value: T): T {
    value
}

function apply(f: (value: int32) => int32): int32 {
    f(9)
}

const nine: int32 = apply(same);
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
export global test.main.nine: int32 = zeroinit

export function test.main.apply(v0: function<(int32) => int32, repeatable, managed, mutable, local>): int32 {
    local l0: function<(int32) => int32, repeatable, managed, mutable, local>

entry(v0: function<(int32) => int32, repeatable, managed, mutable, local>):
    store l0, v0
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = load l0
    v2: int32 = 9
    v3: function<(int32) => int32, repeatable, borrowed, 'managed, mutable> = cast.bit v1 -> function<(int32) => int32, repeatable, borrowed, 'managed, mutable>
    v4: int32 = call.indirect v3(v2): (int32) => int32
    return v4
}

export function test.main.same<T>(v0: T): T {
    local l0: T

entry(v0: T):
    store l0, v0
    v1: T = load l0
    return v1
}

export park function test.main.@init(): void {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32) => int32, repeatable, managed, mutable, local> = function.bind test.main.same<int32>, v0
    v2: int32 = call test.main.apply(v1): (function<(int32) => int32, repeatable, managed, mutable, local>) => int32
    store @test.main.nine, v2
    return
}

shared function test.main.same<int32>(v0: int32): int32;
"#,
    );
}

/// Borrow a computed module string into a readonly parameter without moving it.
#[test]
fn test_borrow_a_computed_module_string_into_a_readonly_parameter() {
    let session = TestSession::single(
        r#"
function measure(value: &readonly string): void {}

const name = "wor" + "ld";

measure(name);
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
@nocopy
@languageItem("string.String")
type String = class { codeUnits: slice<uint16, unique, mutable> };

export constant test.main.name: String = "world"

export function test.main.measure<'a>(v0: ref<String, borrowed, 'a, readonly>): void {
    local l0: ref<String, borrowed, 'a, readonly>

entry(v0: ref<String, borrowed, 'a, readonly>):
    store l0, v0
    return
}

export park function test.main.@init(): void {
entry:
    v0: ref<String, borrowed, 'static, readonly> = address @test.main.name
    call test.main.measure(v0): (ref<String, borrowed, 'static, readonly>) => void
    return
}

/// @layout.class name=String size=24 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=8 size=16 align=8
/// @layout.class name=type@4 size=24 align=8
/// @layout.field owner=type@4 index=0 name=codeUnits offset=8 size=16 align=8
"#,
    );
}
