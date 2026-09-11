use crate::tests::TestSession;

/// A constructor parameter invokes the supplied function value.
#[test]
fn test_call_constructor_parameter() {
    let session = TestSession::single(
        r#"
class Counter {}

function create(constructor: typeof Counter): Counter {
    return new constructor();
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.create", r#"
type test.main.Counter { }

function test.main.create(v0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>): ref<test.main.Counter, managed, mutable, local> {
    local l0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>):
    local.set l0, v0
    v1: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l0
    v2: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly, local> = cast.bit v1 -> function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly, local>
    v3: ref<test.main.Counter, managed, mutable, local> = call.indirect v2(): () => ref<test.main.Counter, managed, mutable, local>
    return v3
}

/// @layout.struct name=test.main.Counter size=0 align=1
"#);
}

/// A constructor function forwards its borrowed rest parameter to the initializer.
#[test]
fn test_assign_borrowed_rest_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    constructor(...values: &readonly [int32]) {}
}

function create(): new (...values: &readonly [int32]) => Counter {
    return Counter;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Counter { }

function test.main.Counter.constructor<'a, 'b>(v0: ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>, v1: slice<int32, borrowed, 'a, readonly, local>): void {
    local l0: slice<int32, borrowed, 'a, readonly, local>
    local l1: ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>, v1: slice<int32, borrowed, 'a, readonly, local>):
    local.set l0, v1
    local.set l1, v0
    return
}

function test.main.create(): function<(slice<int32, borrowed, 'managed, readonly, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(slice<int32, borrowed, 'managed, readonly, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v0
    return v1
}

function test.main.Counter.constructor.new(v0: slice<int32, borrowed, 'managed, readonly, local>): ref<test.main.Counter, managed, mutable, local> {
entry(v0: slice<int32, borrowed, 'managed, readonly, local>):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v2, v0): <'a, 'b>(ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>, slice<int32, borrowed, 'a, readonly, local>) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@24 size=8 align=8
/// @layout.discriminant owner=type@24 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@24 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@24 index=1 discriminant=1 payload_offset=0
"#);
}

/// A constructor function converts positional parameters and forwards its rest collection.
#[test]
fn test_assign_constructor_with_positional_and_rest_parameters() {
    let session = TestSession::single(
        r#"
class Counter {
    constructor(start: int32 = 0, ...values: ^[int32]) {}
}

function create(): new (start: int32, ...values: ^[int32]) => Counter {
    return Counter;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Counter { }

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }, v2: slice<int32, unique, mutable, local>): void {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    local l1: slice<int32, unique, mutable, local>
    local l2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>
    local l3: int32, readonly
    local l4: int32

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }, v2: slice<int32, unique, mutable, local>):
    local.set l0, v1
    local.set l1, v2
    local.set l2, v0
    v3: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l0
    variant.switch v3, 0 => b2, else b1

b1:
    v4: int32 = variant.payload v3, 1
    local.set l3, v4
    jump b3

b2:
    v5: int32 = 0
    local.set l3, v5
    jump b3

b3:
    v6: int32 = local.get l3
    local.set l4, v6
    return
}

function test.main.create(): function<(int32, slice<int32, unique, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(int32, slice<int32, unique, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v0
    return v1
}

function test.main.Counter.constructor.new(v0: int32, v1: slice<int32, unique, mutable, local>): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32, v1: slice<int32, unique, mutable, local>):
    v2: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v0
    v3: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v4: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v3 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v4, v2, v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, variant<uint1> { 0uint1 = void; 1uint1 = int32; }, slice<int32, unique, mutable, local>) => void
    return v3
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@41 size=8 align=8
/// @layout.discriminant owner=type@41 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@41 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@41 index=1 discriminant=1 payload_offset=0
"#);
}

/// A constructor function accepts borrowed arguments under its declared lifetimes.
#[test]
fn test_assign_borrowed_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    constructor(value: &readonly int32) {}
}

function create(value: &readonly int32): Counter {
    const create: new (value: &readonly int32) => Counter = Counter;

    return new create(value);
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Counter { }

function test.main.Counter.constructor<'a, 'b>(v0: ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>, v1: ref<int32, borrowed, 'a, readonly, local>): void {
    local l0: ref<int32, borrowed, 'a, readonly, local>
    local l1: ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>, v1: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v1
    local.set l1, v0
    return
}

function test.main.create<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): ref<test.main.Counter, managed, mutable, local> {
    local l0: ref<int32, borrowed, 'a, readonly, local>
    local l1: function<<'a>(ref<int32, borrowed, 'a, readonly, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v2: function<<'a>(ref<int32, borrowed, 'a, readonly, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    local.set l1, v2
    v3: function<<'a>(ref<int32, borrowed, 'a, readonly, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l1
    v4: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v5: ref<test.main.Counter, managed, mutable, local> = call.indirect v3(v4): <'a>(ref<int32, borrowed, 'a, readonly, local>) => ref<test.main.Counter, managed, mutable, local>
    return v5
}

function test.main.Counter.constructor.new<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): ref<test.main.Counter, managed, mutable, local> {
entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v2, v0): <'a, 'b>(ref<uninit<test.main.Counter>, borrowed, 'b, mutable, local>, ref<int32, borrowed, 'a, readonly, local>) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@25 size=8 align=8
/// @layout.discriminant owner=type@25 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@25 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@25 index=1 discriminant=1 payload_offset=0
"#);
}

/// Generic constructors retain required arguments and bounds across function scopes.
#[test]
fn test_return_generic_class_constructor() {
    let session = TestSession::single(
        r#"
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function first<T>(): new (value: T) => Box<T> {
    return Box<T>;
}

function second<T, U>(): new (value: U) => Box<U> {
    return Box<U>;
}

interface Value<T> {
    value: T;
}

function constrained<T: Value<U>, U>(): new (value: T) => Box<T> {
    return Box<T>;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Box<T> {
    value: T;
}

type test.main.Value<T> {
    value: T;
}

function test.main.first<T>(): function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<T>, v0
    return v1
}

function test.main.second<T, U>(): function<(U) => ref<test.main.Box<U>, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(U) => ref<test.main.Box<U>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<U>, v0
    return v1
}

function test.main.constrained<T: test.main.Value<U>, U>(): function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<T, U>, v0
    return v1
}

function test.main.Box.constructor<T, 'a>(v0: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>, v1: T): void {
    local l0: T
    local l1: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>, v1: T):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local> = local.get l1
    v3: T = local.get l0
    v4: ref<uninit<T>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

function test.main.Box.constructor.new<T>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
entry(v0: T):
    v1: ref<test.main.Box<T>, managed, mutable, local> = new.zeroed test.main.Box<T>
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable, local>
    call test.main.Box.constructor<T>(v2, v0): <'a>(ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>, T) => void
    return v1
}

function test.main.Box.constructor.new<T: test.main.Value<U>, U>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
entry(v0: T):
    v1: ref<test.main.Box<T>, managed, mutable, local> = new.zeroed test.main.Box<T>
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable, local>
    call test.main.Box.constructor<T>(v2, v0): <'a>(ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>, T) => void
    return v1
}

/// @layout.variant name=type@34 size=8 align=8
/// @layout.discriminant owner=type@34 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@34 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@34 index=1 discriminant=1 payload_offset=0

/// @dispatch.shape constraint=type@19 field=value
"#);
}

/// Generic and nongeneric functions return the same class constructor.
#[test]
fn test_return_class_constructor_from_generic_functions() {
    let session = TestSession::single(
        r#"
class User {}

function first<T>(): new () => User {
    return User;
}

function second(): new () => User {
    return User;
}

function third<T, U>(): new () => User {
    return User;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.User { }

function test.main.second(): function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.User.new, v0
    return v1
}

function test.main.first<T>(): function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.User.new, v0
    return v1
}

function test.main.third<T, U>(): function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.User.new, v0
    return v1
}

function test.main.User.new(): ref<test.main.User, managed, mutable, local> {
entry:
    v0: ref<test.main.User, managed, mutable, local> = new.zeroed test.main.User
    return v0
}

/// @layout.struct name=test.main.User size=0 align=1
/// @layout.variant name=type@14 size=8 align=8
/// @layout.discriminant owner=type@14 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@14 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@14 index=1 discriminant=1 payload_offset=0
"#);
}

/// Constructor functions pack their positional parameters into the declared rest array.
#[test]
fn test_assign_positional_arguments_to_rest_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    constructor(...counts: (int32 | undefined)[]) {}
}

function pair(first: int32, second: int32): Counter {
    const create: new (first: int32, second: int32) => Counter = Counter;

    return new create(first, second);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.pair", r#"
type test.main.Counter { }

function test.main.pair(v0: int32, v1: int32): ref<test.main.Counter, managed, mutable, local> {
    local l0: int32
    local l1: int32
    local l2: function<(int32, int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: int32, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v3: function<(int32, int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v2
    local.set l2, v3
    v4: function<(int32, int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l2
    v5: int32 = local.get l0
    v6: int32 = local.get l1
    v7: ref<test.main.Counter, managed, mutable, local> = call.indirect v4(v5, v6): (int32, int32) => ref<test.main.Counter, managed, mutable, local>
    return v7
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@70 size=8 align=8
/// @layout.discriminant owner=type@70 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@70 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@70 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.ds", "test.main.Counter.constructor.new", r#"
type test.main.Counter { }

@languageItem("collections.Array")
type Array<T>;

function test.main.Counter.constructor.new(v0: int32, v1: int32): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32, v1: int32):
    v2: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v0
    v3: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v1
    v4: usize = 2
    v5: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, v4
    v6: usize = 0
    v7: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v5, v6
    store v7, v2
    v8: usize = 1
    v9: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v5, v8
    store v9, v3
    v10: slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local> = new.complete v5
    v11: Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>(v10): (slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local>) => Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>
    v12: ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local> = new.complete v11
    v13: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v14: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v13 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v14, v12): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>) => void
    return v13
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#);
}

/// Direct construction packs positional arguments into the declared rest array.
#[test]
fn test_construct_class_with_rest_arguments() {
    let session = TestSession::single(
        r#"
class Counter {
    constructor(...counts: (int32 | undefined)[]) {}
}

function direct(first: int32, second: int32): Counter {
    return new Counter(first, second);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.direct", r#"
type test.main.Counter { }

@languageItem("collections.Array")
type Array<T>;

function test.main.direct(v0: int32, v1: int32): ref<test.main.Counter, managed, mutable, local> {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: int32 = local.get l0
    v3: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v2
    v4: int32 = local.get l1
    v5: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v4
    v6: usize = 2
    v7: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, v6
    v8: usize = 0
    v9: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v7, v8
    store v9, v3
    v10: usize = 1
    v11: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v7, v10
    store v11, v5
    v12: slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local> = new.complete v7
    v13: Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>(v12): (slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local>) => Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>
    v14: ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local> = new.complete v13
    v15: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v16: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v15 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v16, v14): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>) => void
    return v15
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#);
}

/// Constructor calls iterate spread arrays into fresh rest storage.
#[test]
fn test_spread_array_into_constructor_function() {
    let session = TestSession::single(
        r#"
class Counter {
    constructor(...counts: (int32 | undefined)[]) {}
}

function spread(counts: (int32 | undefined)[]): Counter {
    const create: new (...counts: (int32 | undefined)[]) => Counter = Counter;

    return new create(...counts);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.spread", r#"
type test.main.Counter { }

@languageItem("collections.Array")
type Array<T>;

@languageItem("iter.Iterator")
type Iterator<T>;

@copy
@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

@copy
@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@copy
@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

function test.main.spread(v0: ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>): ref<test.main.Counter, managed, mutable, local> {
    local l0: ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>
    local l1: function<(ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>
    local l2: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local>
    local l3: usize
    local l4: dynamic<Iterator<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>
    local l5: usize
    local l6: usize

entry(v0: ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v2: function<(ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    local.set l1, v2
    v3: function<(ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l1
    v4: usize = 0
    v5: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, v4
    local.set l2, v5
    local.set l3, v4
    v6: ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local> = local.get l0
    v7: dynamic<Iterator<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local> = call Array.Iterable.iterator<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>(v6): (ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>) => dynamic<Iterator<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>
    local.set l4, v7
    jump b1

b1:
    v8: dynamic<Iterator<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local> = local.get l4
    v9: IteratorResult<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, void> = call.dynamic v8, Iterator<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, 0(): () => IteratorResult<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, void>
    v10: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>; } = field.get v9, 0
    variant.switch v10, 1 => b2, 0 => b3

b2:
    v11: IteratorYield<variant<uint1> { 0uint1 = void; 1uint1 = int32; }> = variant.payload v10, 1
    v12: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = field.get v11, 1
    v13: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = local.get l2
    v14: usize = slice.length v13
    v15: usize = local.get l3
    v16: boolean = eq v15, v14
    branch v16 => b4 | b5

b3:
    v35: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = local.get l2
    v36: usize = slice.length v35
    v37: usize = local.get l3
    v38: boolean = eq v37, v36
    branch v38 => b10 | b9

b4:
    v17: usize = add v14, v14
    v18: usize = 1
    v19: usize = add v17, v18
    v20: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = local.get l2
    v21: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, v19
    v22: usize = local.get l3
    v23: usize = 0
    local.set l5, v23
    jump b6

b5:
    v31: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = local.get l2
    v32: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v31, v15
    store v32, v12
    v33: usize = 1
    v34: usize = add v15, v33
    local.set l3, v34
    jump b1

b6:
    v24: usize = local.get l5
    v25: boolean = lt v24, v22
    branch v25 => b7 | b8

b7:
    v26: ref<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, borrowed, 'frame, mutable, local> = element.address v20, v24
    v27: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = load v26
    v28: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v21, v24
    store v28, v27
    v29: usize = 1
    v30: usize = add v24, v29
    local.set l5, v30
    jump b6

b8:
    release v20
    local.set l2, v21
    jump b5

b9:
    v39: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = local.get l2
    v40: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = new.slice.uninit uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, v37
    v41: usize = local.get l3
    v42: usize = 0
    local.set l6, v42
    jump b11

b10:
    v50: slice<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, unique, mutable, local> = local.get l2
    v51: slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local> = new.complete v50
    v52: Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>(v51): (slice<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, unique, mutable, local>) => Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>
    v53: ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local> = new.complete v52
    v54: ref<test.main.Counter, managed, mutable, local> = call.indirect v3(v53): (ref<Array<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>
    return v54

b11:
    v43: usize = local.get l6
    v44: boolean = lt v43, v41
    branch v44 => b12 | b13

b12:
    v45: ref<variant<uint1> { 0uint1 = void; 1uint1 = int32; }, borrowed, 'frame, mutable, local> = element.address v39, v43
    v46: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = load v45
    v47: ref<uninit<variant<uint1> { 0uint1 = void; 1uint1 = int32; }>, borrowed, 'frame, mutable, local> = element.address v40, v43
    store v47, v46
    v48: usize = 1
    v49: usize = add v43, v48
    local.set l6, v49
    jump b11

b13:
    release v39
    local.set l2, v40
    jump b10
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@68 size=8 align=8
/// @layout.discriminant owner=type@68 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@68 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@68 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@128 size=12 align=4
/// @layout.discriminant owner=type@128 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@128 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@128 index=1 discriminant=1 payload_offset=4
"#);
}

/// A namespace-qualified generic class constructs through its selected declaration.
#[test]
fn test_construct_qualified_class() {
    let session = TestSession::builder()
        .module(
            "counter.ds",
            r#"
export class Counter<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import * as counter from "./counter.ds";

function create(value: int32): counter.Counter<int32> {
    return new counter.Counter<int32>(value);
}
"#,
        )
        .build();

    session.assert_mir_lowered("main.ds", r#"
type test.counter.Counter<T> {
    value: T;
}

function test.main.create(v0: int32): ref<test.counter.Counter<int32>, managed, mutable, local> {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: ref<test.counter.Counter<int32>, managed, mutable, local> = new.zeroed test.counter.Counter<int32>
    v3: ref<uninit<test.counter.Counter<int32>>, borrowed, 'managed, mutable, local> = cast.bit v2 -> ref<uninit<test.counter.Counter<int32>>, borrowed, 'managed, mutable, local>
    call test.counter.Counter.constructor<int32>(v3, v1): <'a>(ref<uninit<test.counter.Counter<int32>>, borrowed, 'a, mutable, local>, int32) => void
    return v2
}

external function test.counter.Counter.constructor<T, 'a>(ref<uninit<test.counter.Counter<T>>, borrowed, 'a, mutable, local>, T): void

shared function test.counter.Counter.constructor<int32, 'a>(v0: ref<uninit<test.counter.Counter<int32>>, borrowed, 'a, mutable, local>, v1: int32): void;

/// @layout.struct name=test.counter.Counter<int32> size=4 align=4
/// @layout.field owner=test.counter.Counter<int32> index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@8 size=4 align=4
/// @layout.field owner=type@8 index=0 name=value offset=0 size=4 align=4
"#);
}

/// Class names bind ordinary constructor functions that allocate and return instances.
#[test]
fn test_assign_class_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(count: int32) {
        this.count = count;
    }
}

function read(count: int32): int32 {
    const create = Counter;
    const counter = new create(count);

    return counter.count;
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

function test.main.read(v0: int32): int32 {
    local l0: int32
    local l1: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>
    local l2: ref<test.main.Counter, managed, mutable, local>

entry(v0: int32):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v2: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    local.set l1, v2
    v3: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l1
    v4: int32 = local.get l0
    v5: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly, local> = cast.bit v3 -> function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly, local>
    v6: ref<test.main.Counter, managed, mutable, local> = call.indirect v5(v4): (int32) => ref<test.main.Counter, managed, mutable, local>
    local.set l2, v6
    v7: ref<test.main.Counter, managed, mutable, local> = local.get l2
    v8: ref<int32, borrowed, 'managed, readonly, local> = field.project v7, 0
    v9: int32 = load v8
    return v9
}

function test.main.Counter.constructor.new(v0: int32): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v2, v0): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, int32) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@31 size=8 align=8
/// @layout.discriminant owner=type@31 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@31 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@31 index=1 discriminant=1 payload_offset=0
"#);
}

/// A constructor operand runs once before its arguments.
#[test]
fn test_evaluate_constructor_operands() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(count: int32) {
        this.count = count;
    }
}

declare function choose(): typeof Counter;

declare function count(): int32;

function create(): Counter {
    return new (choose())(count());
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.create", r#"
type test.main.Counter {
    count: int32;
}

function test.main.create(): ref<test.main.Counter, managed, mutable, local> {
entry:
    v0: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = call test.main.choose(): () => function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>
    v1: int32 = call test.main.count(): () => int32
    v2: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly, local> = cast.bit v0 -> function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly, local>
    v3: ref<test.main.Counter, managed, mutable, local> = call.indirect v2(v1): (int32) => ref<test.main.Counter, managed, mutable, local>
    return v3
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
"#);
}

/// A constructor function supplies omitted arguments to declared defaults.
#[test]
fn test_assign_defaulted_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(count: int32 = 1) {
        this.count = count;
    }
}

function create(): Counter {
    const create: new () => Counter = Counter;

    return new create();
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }): void {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>
    local l2: int32, readonly
    local l3: int32

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }):
    local.set l0, v1
    local.set l1, v0
    v2: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l0
    variant.switch v2, 0 => b2, else b1

b1:
    v3: int32 = variant.payload v2, 1
    local.set l2, v3
    jump b3

b2:
    v4: int32 = 1
    local.set l2, v4
    jump b3

b3:
    v5: int32 = local.get l2
    local.set l3, v5
    v6: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v7: int32 = local.get l3
    v8: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v6, 0
    store v8, v7
    return
}

function test.main.create(): ref<test.main.Counter, managed, mutable, local> {
    local l0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v1: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v0
    local.set l0, v1
    v2: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l0
    v3: ref<test.main.Counter, managed, mutable, local> = call.indirect v2(): () => ref<test.main.Counter, managed, mutable, local>
    return v3
}

function test.main.Counter.constructor.new(): ref<test.main.Counter, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 0
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v2, v0): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, variant<uint1> { 0uint1 = void; 1uint1 = int32; }) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@7 size=8 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@45 size=8 align=8
/// @layout.discriminant owner=type@45 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@45 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@45 index=1 discriminant=1 payload_offset=0
"#);
}

/// A constructor function preserves its enclosing type parameter.
#[test]
fn test_assign_generic_constructor() {
    let session = TestSession::single(
        r#"
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function create<T>(value: T): Box<T> {
    const create: new (value: T) => Box<T> = Box<T>;

    return new create(value);
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Box<T> {
    value: T;
}

function test.main.create<T>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
    local l0: T
    local l1: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: T):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v2: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<T>, v1
    local.set l1, v2
    v3: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l1
    v4: T = local.get l0
    v5: ref<test.main.Box<T>, managed, mutable, local> = call.indirect v3(v4): (T) => ref<test.main.Box<T>, managed, mutable, local>
    return v5
}

function test.main.Box.constructor<T, 'a>(v0: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>, v1: T): void {
    local l0: T
    local l1: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>, v1: T):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local> = local.get l1
    v3: T = local.get l0
    v4: ref<uninit<T>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

function test.main.Box.constructor.new<T>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
entry(v0: T):
    v1: ref<test.main.Box<T>, managed, mutable, local> = new.zeroed test.main.Box<T>
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable, local>
    call test.main.Box.constructor<T>(v2, v0): <'a>(ref<uninit<test.main.Box<T>>, borrowed, 'a, mutable, local>, T) => void
    return v1
}

/// @layout.variant name=type@22 size=8 align=8
/// @layout.discriminant owner=type@22 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@22 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@22 index=1 discriminant=1 payload_offset=0
"#);
}

/// A required constructor argument converts to an optional initializer parameter.
#[test]
fn test_assign_optional_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(count: int32 = 1) {
        this.count = count;
    }
}

function create(count: int32): Counter {
    const create: new (count: int32) => Counter = Counter;

    return new create(count);
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }): void {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>
    local l2: int32, readonly
    local l3: int32

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; }):
    local.set l0, v1
    local.set l1, v0
    v2: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l0
    variant.switch v2, 0 => b2, else b1

b1:
    v3: int32 = variant.payload v2, 1
    local.set l2, v3
    jump b3

b2:
    v4: int32 = 1
    local.set l2, v4
    jump b3

b3:
    v5: int32 = local.get l2
    local.set l3, v5
    v6: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v7: int32 = local.get l3
    v8: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v6, 0
    store v8, v7
    return
}

function test.main.create(v0: int32): ref<test.main.Counter, managed, mutable, local> {
    local l0: int32
    local l1: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: int32):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v2: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    local.set l1, v2
    v3: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = local.get l1
    v4: int32 = local.get l0
    v5: ref<test.main.Counter, managed, mutable, local> = call.indirect v3(v4): (int32) => ref<test.main.Counter, managed, mutable, local>
    return v5
}

function test.main.Counter.constructor.new(v0: int32): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32):
    v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 1, v0
    v2: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v3, v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, variant<uint1> { 0uint1 = void; 1uint1 = int32; }) => void
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@7 size=8 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@47 size=8 align=8
/// @layout.discriminant owner=type@47 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@47 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@47 index=1 discriminant=1 payload_offset=0
"#);
}

/// A constructor result converts to the required function's union result.
#[test]
fn test_assign_constructor_result() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(count: int32) {
        this.count = count;
    }
}

function create(count: int32): Counter | undefined {
    const create: new (count: int32) => Counter | undefined = Counter;

    return new create(count);
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

function test.main.create(v0: int32): variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } {
    local l0: int32
    local l1: function<(int32) => variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }, repeatable, managed, mutable, local>

entry(v0: int32):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<void, managed, mutable, local>; } = variant.new 0
    v2: function<(int32) => variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    local.set l1, v2
    v3: function<(int32) => variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }, repeatable, managed, mutable, local> = local.get l1
    v4: int32 = local.get l0
    v5: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } = call.indirect v3(v4): (int32) => variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; }
    return v5
}

function test.main.Counter.constructor.new(v0: int32): variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } {
entry(v0: int32):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, local>
    call test.main.Counter.constructor(v2, v0): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, int32) => void
    v3: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Counter, managed, mutable, local>; } = variant.new 1, v1
    return v3
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@10 size=8 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@32 size=8 align=8
/// @layout.discriminant owner=type@32 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@32 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@32 index=1 discriminant=1 payload_offset=0
"#);
}
