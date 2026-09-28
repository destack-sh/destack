use crate::tests::TestSession;

/// Instantiate one module's own template at the argument its call closes.
#[test]
fn test_instantiate_an_own_template_at_a_closed_call() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

function main(): int32 {
    return identity(1);
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
export function test.main.main(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call test.main.identity<int32>(v0): (int32) => int32
    return v1
}

export function test.main.identity<T>(v0: T): T;

shared function test.main.identity<int32>(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    return v1
}
"#,
    );
}

/// Collapse a union whose arms an instance makes identical.
#[test]
fn test_collapse_a_union_whose_arms_an_instance_makes_identical() {
    let session = TestSession::single(
        r#"
function either<E, F>(isLeft: boolean, left: E, right: F): E | F {
    if (isLeft) {
        left
    } else {
        right
    }
}

function main(): int32 {
    const left: int32 = 1;
    const right: int32 = 2;
    return either(true, left, right);
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.either<int32, int32>",
        r#"
shared function test.main.either<int32, int32>(v0: boolean, v1: int32, v2: int32): int32 {
    local l0: boolean
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: boolean, v1: int32, v2: int32):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: boolean = load l0
    branch v3 => b1 | b2

b1:
    v4: int32 = load l1
    v5: int32 = copy v4
    store l3, v5
    jump b3

b2:
    v6: int32 = load l2
    v7: int32 = copy v6
    store l3, v7
    jump b3

b3:
    v8: int32 = load l3
    return v8
}
"#,
    );
}

/// Instantiate an imported template by copying its body out of its own module.
#[test]
fn test_instantiate_an_imported_template_across_modules() {
    let session = TestSession::builder()
        .module(
            "identity.tspp",
            r#"
export function identity<T>(value: T): T {
    return value;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { identity } from "./identity";

function main(): int32 {
    return identity(1);
}
"#,
        )
        .build();

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
export function test.main.main(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call test.identity.identity<int32>(v0): (int32) => int32
    return v1
}

external function test.identity.identity<T>(T): T

shared function test.identity.identity<int32>(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    return v1
}
"#,
    );
}

/// A conversion intent over a parameter refines to the substituted representations.
#[test]
fn test_refine_a_truncate_intent_at_the_specialized_formats() {
    let session = TestSession::single(
        r#"
import { Integer } from "tspp:math";

@intrinsic("math.cast.int.truncate")
declare function truncateInt<T: Integer, U: Integer>(value: T): U;

function widen<T: Integer>(value: T): int32 {
    return truncateInt<T, int32>(value);
}

function main(): int32 {
    const small: int8 = 1;
    return widen(small);
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
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

export function test.main.main(): int32 {
    local l0: int8

entry:
    v0: int8 = 1
    store l0, v0
    v1: int8 = load l0
    v2: int32 = call test.main.widen<int8>(v1): (int8) => int32
    return v2
}

export function test.main.widen<T: Integer>(v0: T): int32;

shared function test.main.widen<int8>(v0: int8): int32 {
    local l0: int8

entry(v0: int8):
    store l0, v0
    v1: int8 = load l0
    v2: int32 = cast.intToInt v1 -> int32
    return v2
}

/// @layout.struct name=Concrete size=0 align=1
/// @layout.struct name=Copy size=0 align=1
/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=IntegerDomain size=0 align=1
/// @layout.struct name=Zero size=0 align=1
/// @layout.struct name=One size=0 align=1
/// @layout.struct name=type@4 size=0 align=1

/// @dispatch.shape constraint=type@2 function=clone function=cloneFrom function=zero function=one
/// @dispatch.shape constraint=type@7 function=clone function=cloneFrom
/// @dispatch.shape constraint=type@9 function=zero
/// @dispatch.shape constraint=type@10 function=one
"#,
    );
}

/// A singleton test specializes to a tag test at a tagged variant and a null compare at a niche.
#[test]
fn test_specialize_a_nullish_test_at_a_tagged_variant_and_a_niched_reference() {
    let session = TestSession::single(
        r#"
import { StrictEqual } from "tspp:ops";

class User {}

function isNull<T: StrictEqual<null>>(value: T): boolean {
    return value === null;
}

function main(): boolean {
    const count: int32 | null = null;
    const user: User | null = null;
    return isNull(count) && isNull(user);
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp", r#"
@nocopy
type test.main.User { }

@nocopy
@languageItem("ops.StrictEqual")
type StrictEqual<T> { }

export function test.main.main(): boolean {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = null; }
    local l1: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; }
    local l2: boolean

entry:
    v0: null = zeroed
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = null; } = variant.new 1
    store l0, v1
    v2: null = zeroed
    v3: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; } = variant.new 1
    store l1, v3
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = null; } = load l0
    v5: boolean = call test.main.isNull<variant<uint1> { 0uint1 = int32; 1uint1 = null; }>(v4): (variant<uint1> { 0uint1 = int32; 1uint1 = null; }) => boolean
    store l2, v5
    branch v5 => b1 | b2

b1:
    v6: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; } = load l1
    v7: boolean = call test.main.isNull<variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; }>(v6): (variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; }) => boolean
    store l2, v7
    jump b2

b2:
    v8: boolean = load l2
    return v8
}

export function test.main.isNull<T: StrictEqual<null>>(v0: T): boolean;

shared function test.main.isNull<variant<uint1> { 0uint1 = int32; 1uint1 = null; }>(v0: variant<uint1> { 0uint1 = int32; 1uint1 = null; }): boolean {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = null; }

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = null; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = null; } = load l0
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = null; } = variant.new 1
    v3: boolean = eq v1, v2
    return v3
}

shared function test.main.isNull<variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; }>(v0: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; }): boolean {
    local l0: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; }

entry(v0: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; } = load l0
    v2: variant<uint1> { 0uint1 = ref<test.main.User, managed, mutable, local>; 1uint1 = null; } = variant.new 1
    v3: boolean = eq v1, v2
    return v3
}

/// @layout.struct name=test.main.User size=0 align=1
/// @layout.struct name=type@2 size=0 align=1
/// @layout.variant name=type@12 size=8 align=4
/// @layout.discriminant owner=type@12 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@12 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@12 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@13 size=8 align=8
/// @layout.discriminant owner=type@13 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@13 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@13 index=1 discriminant=1 payload_offset=0

/// @dispatch.shape constraint=type@6
"#,
    );
}

/// A const ordering argument substitutes into the specialized atomic instruction.
#[test]
fn test_specialize_an_atomic_ordering_parameter() {
    let session = TestSession::single(
        r#"
import { MemoryOrdering, AtomicScope, MemoryScope, MemoryRegionSet, atomicLoad } from "tspp:sync";

function load<
    const Order:
        | MemoryOrdering.Relaxed
        | MemoryOrdering.Acquire
        | MemoryOrdering.SequentiallyConsistent = MemoryOrdering.SequentiallyConsistent,
>(
    ptr: *int32,
    order?: Order,
): int32 {
    atomicLoad(ptr, Order, AtomicScope.Device, MemoryScope.Device, MemoryRegionSet.Any, false, false, false)
}

export function acquire(ptr: *int32): int32 {
    load(ptr, MemoryOrdering.Acquire)
}

export function relaxed(ptr: *int32): int32 {
    load(ptr, MemoryOrdering.Relaxed)
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp", r#"
@languageItem("sync.MemoryOrdering")
type MemoryOrdering = variant<uint8> { 0uint8 = void; 1uint8 = void; 2uint8 = void; 3uint8 = void; 4uint8 = void; };

@nocopy
@languageItem("memory.Clone")
type Clone { }

export function test.main.acquire(v0: ptr<int32, mutable>): int32 {
    local l0: ptr<int32, mutable>

entry(v0: ptr<int32, mutable>):
    store l0, v0
    v1: ptr<int32, mutable> = load l0
    v2: MemoryOrdering = variant.new 1
    v3: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; } = variant.new 0, v2
    v4: int32 = call test.main.load<1>(v1, v3): (ptr<int32, mutable>, variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }) => int32
    return v4
}

export function test.main.relaxed(v0: ptr<int32, mutable>): int32 {
    local l0: ptr<int32, mutable>

entry(v0: ptr<int32, mutable>):
    store l0, v0
    v1: ptr<int32, mutable> = load l0
    v2: MemoryOrdering = variant.new 0
    v3: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; } = variant.new 0, v2
    v4: int32 = call test.main.load<0>(v1, v3): (ptr<int32, mutable>, variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }) => int32
    return v4
}

export function test.main.load<const Order: MemoryOrdering>(v0: ptr<int32, mutable>, v1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }): int32;

shared function test.main.load<1>(v0: ptr<int32, mutable>, v1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }): int32 {
    local l0: ptr<int32, mutable>
    local l1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }

entry(v0: ptr<int32, mutable>, v1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }):
    store l0, v0
    store l1, v1
    v2: ptr<int32, mutable> = load l0
    v3: int32 = atomic.load (*v2), acquire, scope(device)
    return v3
}

shared function test.main.load<0>(v0: ptr<int32, mutable>, v1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }): int32 {
    local l0: ptr<int32, mutable>
    local l1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }

entry(v0: ptr<int32, mutable>, v1: variant<uint1> { 0uint1 = MemoryOrdering; 1uint1 = void; }):
    store l0, v0
    store l1, v1
    v2: ptr<int32, mutable> = load l0
    v3: int32 = atomic.load (*v2), relaxed, scope(device)
    return v3
}

/// @layout.variant name=MemoryOrdering size=1 align=1
/// @layout.discriminant owner=MemoryOrdering kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=MemoryOrdering index=0 discriminant=0 payload_offset=1
/// @layout.case owner=MemoryOrdering index=1 discriminant=1 payload_offset=1
/// @layout.case owner=MemoryOrdering index=2 discriminant=2 payload_offset=1
/// @layout.case owner=MemoryOrdering index=3 discriminant=3 payload_offset=1
/// @layout.case owner=MemoryOrdering index=4 discriminant=4 payload_offset=1
/// @layout.variant name=type@5 size=1 align=1
/// @layout.discriminant owner=type@5 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=1
/// @layout.case owner=type@5 index=2 discriminant=2 payload_offset=1
/// @layout.case owner=type@5 index=3 discriminant=3 payload_offset=1
/// @layout.case owner=type@5 index=4 discriminant=4 payload_offset=1
/// @layout.variant name=type@7 size=1 align=1
/// @layout.discriminant owner=type@7 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=5
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@10 size=0 align=1

/// @dispatch.shape constraint=type@8 function=clone function=cloneFrom
"#,
    );
}

/// Compare strict-equal values an instance closes at string by content.
#[test]
fn test_instantiate_strict_equality_at_string_as_content_comparison() {
    let session = TestSession::single(
        r#"
import { StrictEqual } from "tspp:ops";

function same<T: StrictEqual<T>>(left: T, right: T): boolean {
    left === right
}

function main(left: string, right: string): boolean {
    same(left, right)
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

@nocopy
@languageItem("ops.StrictEqual")
type StrictEqual<T> { }

export function test.main.main(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>): boolean {
    local l0: ref<String, managed, mutable, local>
    local l1: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: ref<String, managed, mutable, local> = load l0
    v3: ref<String, managed, mutable, local> = load l1
    v4: boolean = call test.main.same<ref<String, managed, mutable, local>>(v2, v3): (ref<String, managed, mutable, local>, ref<String, managed, mutable, local>) => boolean
    return v4
}

export function test.main.same<T: StrictEqual<T>>(v0: T, v1: T): boolean;

shared function test.main.same<ref<String, managed, mutable, local>>(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>): boolean {
    local l0: ref<String, managed, mutable, local>
    local l1: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: ref<String, managed, mutable, local> = load l0
    v3: ref<String, managed, mutable, local> = load l1
    v4: boolean = call stringEqual(v2, v3): (ref<String, managed, mutable, local>, ref<String, managed, mutable, local>) => boolean
    return v4
}

external function stringEqual(ref<String, managed, mutable, local>, ref<String, managed, mutable, local>): boolean

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@4 size=16 align=8
/// @layout.field owner=type@4 index=0 name=codeUnits offset=0 size=16 align=8

/// @dispatch.shape constraint=type@7
"#,
    );
}

/// Compare strict-unequal values an instance closes at string by negated content.
#[test]
fn test_instantiate_strict_inequality_at_string_as_content_comparison() {
    let session = TestSession::single(
        r#"
import { StrictEqual } from "tspp:ops";

function differ<T: StrictEqual<T>>(left: T, right: T): boolean {
    left !== right
}

function main(left: string, right: string): boolean {
    differ(left, right)
}
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

@nocopy
@languageItem("ops.StrictEqual")
type StrictEqual<T> { }

export function test.main.main(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>): boolean {
    local l0: ref<String, managed, mutable, local>
    local l1: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: ref<String, managed, mutable, local> = load l0
    v3: ref<String, managed, mutable, local> = load l1
    v4: boolean = call test.main.differ<ref<String, managed, mutable, local>>(v2, v3): (ref<String, managed, mutable, local>, ref<String, managed, mutable, local>) => boolean
    return v4
}

export function test.main.differ<T: StrictEqual<T>>(v0: T, v1: T): boolean;

shared function test.main.differ<ref<String, managed, mutable, local>>(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>): boolean {
    local l0: ref<String, managed, mutable, local>
    local l1: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>, v1: ref<String, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: ref<String, managed, mutable, local> = load l0
    v3: ref<String, managed, mutable, local> = load l1
    v4: boolean = call stringEqual(v2, v3): (ref<String, managed, mutable, local>, ref<String, managed, mutable, local>) => boolean
    v5: boolean = not v4
    return v5
}

external function stringEqual(ref<String, managed, mutable, local>, ref<String, managed, mutable, local>): boolean

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@4 size=16 align=8
/// @layout.field owner=type@4 index=0 name=codeUnits offset=0 size=16 align=8

/// @dispatch.shape constraint=type@7
"#,
    );
}

/// Instantiate the park call an awaited async function result lowers to.
#[test]
fn test_instantiate_the_park_call_of_an_await() {
    let session = TestSession::single(
        r#"
async function answer(): Promise<int32> {
    42
}

async function main(): Promise<int32> {
    await answer()
}
"#,
    );

    session.assert_mir_elaborated_function(
        "main.tspp",
        "test.main.main.body",
        r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{  }, unique, mutable>)
park function test.main.main.body(): int32 {
entry:
    v0: ref<{  }, unique, mutable> = function.environment.current
    v1: {  } = load (*v0)
    v2: ref<uninit<{  }>, unique, mutable> = cast.bit v0 -> ref<uninit<{  }>, unique, mutable>
    release v2
    v3: ref<Promise<int32>, managed, mutable, local> = call test.main.answer(): () => ref<Promise<int32>, managed, mutable, local>
    v4: int32 = call Promise.park<int32>(v3): park (ref<Promise<int32>, managed, mutable, local>) => int32
    return v4
}

/// @layout.struct name=type@6 size=0 align=1
"#,
    );
}

/// Bind a closure of a generic function at the specialization of its enclosing function.
#[test]
fn test_bind_a_generic_closure_at_its_enclosing_specialization() {
    let session = TestSession::single(
        r#"
function wrap<T: Copy>(value: T): () => T {
    () => value
}

const make = wrap<int32>(1);
"#,
    );

    session.assert_mir_elaborated(
        "main.tspp",
        r#"
@nocopy
@languageItem("memory.Copy")
type Copy extends Clone { }

@nocopy
@languageItem("memory.Clone")
type Clone { }

export global test.main.make: function<() => int32, repeatable, managed, mutable, local> = zeroinit

export function test.main.wrap<T: Copy>(v0: T): function<() => T, repeatable, managed, mutable, local>;

export park function test.main.@init(): void {
entry:
    v0: int32 = 1
    v1: function<() => int32, repeatable, managed, mutable, local> = call test.main.wrap<int32>(v0): (int32) => function<() => int32, repeatable, managed, mutable, local>
    store @test.main.make, v1
    return
}

shared function test.main.wrap<int32>(v0: int32): function<() => int32, repeatable, managed, mutable, local> {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: ref<{ value: int32 }, managed, mutable, local> = new.zeroed { value: int32 }, local
    store (*v2).0, v1
    v3: { ref<{ value: int32 }, managed, mutable, local> } = aggregate (v2)
    v4: uninit<ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local>> = new.uninit { ref<{ value: int32 }, managed, mutable, local> }, local
    store (*v4), v3
    v5: ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local> = new.complete v4
    v6: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.wrap.closure#0<int32>, v5
    return v6
}

@environment(ref<{ ref<{ value: T }, managed, mutable, local> }, managed, mutable, local>)
export function test.main.wrap.closure#0<T: Copy>(): T;

@environment(ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local>)
shared function test.main.wrap.closure#0<int32>(): int32 {
entry:
    v0: ref<{ ref<{ value: int32 }, managed, mutable, local> }, managed, mutable, local> = function.environment.current
    v1: ref<{ value: int32 }, managed, mutable, local> = load (*v0).0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=type@8 size=0 align=1
/// @layout.struct name=type@26 size=4 align=4
/// @layout.field owner=type@26 index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@28 size=8 align=8
/// @layout.field owner=type@28 index=0 offset=0 size=8 align=8

/// @dispatch.shape constraint=type@6 function=clone function=cloneFrom
/// @dispatch.shape constraint=type@9 function=clone function=cloneFrom
"#,
    );
}
