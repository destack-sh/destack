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

    session.assert_mir_function("main.tspp", "test.main.create", r#"
@nocopy
type test.main.Counter { }

function test.main.create(v0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>): ref<test.main.Counter, managed, mutable, local> {
    local l0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>):
    store l0, v0
    v1: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = load l0
    v2: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly> = cast.bit v1 -> function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly>
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Counter { }

constructor test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: slice<int32, borrowed, 'a, readonly>): void {
    local l0: slice<int32, borrowed, 'a, readonly>
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: slice<int32, borrowed, 'a, readonly>):
    store l0, v1
    store l1, v0
    return
}

function test.main.create(): function<<'a>(slice<int32, borrowed, 'a, readonly>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<<'a>(slice<int32, borrowed, 'a, readonly>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v0
    return v1
}

function test.main.Counter.constructor.new<'a>(v0: slice<int32, borrowed, 'a, readonly>): ref<test.main.Counter, managed, mutable, local> {
entry(v0: slice<int32, borrowed, 'a, readonly>):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v2, v0): <'a_1>(ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, slice<int32, borrowed, 'a_1, readonly>) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.struct name=type@2 size=0 align=1
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Counter { }

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, v2: slice<int32, unique, mutable>): void {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l1: slice<int32, unique, mutable>
    local l2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    local l3: int32, readonly
    local l4: int32

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, v2: slice<int32, unique, mutable>):
    store l0, v1
    store l1, v2
    store l2, v0
    v3: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l0
    variant.switch v3, 1 => b2, else b1

b1:
    v4: int32 = variant.payload v3, 0
    store l3, v4
    jump b3

b2:
    v5: int32 = 0
    store l3, v5
    jump b3

b3:
    v6: int32 = load l3
    store l4, v6
    return
}

function test.main.create(): function<(int32, slice<int32, unique, mutable>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(int32, slice<int32, unique, mutable>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v0
    return v1
}

function test.main.Counter.constructor.new(v0: int32, v1: slice<int32, unique, mutable>): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32, v1: slice<int32, unique, mutable>):
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v3: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v4: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v3 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v4, v2, v1): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, variant<uint1> { 0uint1 = int32; 1uint1 = void; }, slice<int32, unique, mutable>) => void
    return v3
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.struct name=type@2 size=0 align=1
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Counter { }

constructor test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: ref<int32, borrowed, 'a, readonly>): void {
    local l0: ref<int32, borrowed, 'a, readonly>
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: ref<int32, borrowed, 'a, readonly>):
    store l0, v1
    store l1, v0
    return
}

function test.main.create<'a>(v0: ref<int32, borrowed, 'a, readonly>): ref<test.main.Counter, managed, mutable, local> {
    local l0: ref<int32, borrowed, 'a, readonly>
    local l1: function<<'a_1>(ref<int32, borrowed, 'a_1, readonly>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: ref<int32, borrowed, 'a, readonly>):
    store l0, v0
    v1: ptr<void, readonly> = null
    v2: function<<'a_1>(ref<int32, borrowed, 'a_1, readonly>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    store l1, v2
    v3: function<<'a_1>(ref<int32, borrowed, 'a_1, readonly>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = load l1
    v4: ref<int32, borrowed, 'a, readonly> = load l0
    v5: ref<test.main.Counter, managed, mutable, local> = call.indirect v3(v4): <'a_1>(ref<int32, borrowed, 'a_1, readonly>) => ref<test.main.Counter, managed, mutable, local>
    return v5
}

function test.main.Counter.constructor.new<'a>(v0: ref<int32, borrowed, 'a, readonly>): ref<test.main.Counter, managed, mutable, local> {
entry(v0: ref<int32, borrowed, 'a, readonly>):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v2, v0): <'a_1>(ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, ref<int32, borrowed, 'a_1, readonly>) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.struct name=type@2 size=0 align=1
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Box<T> {
    value: T;
}

@nocopy
type test.main.Value<T> {
    value: T;
}

function test.main.first<T>(): function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<T>, v0
    return v1
}

function test.main.second<T, U>(): function<(U) => ref<test.main.Box<U>, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(U) => ref<test.main.Box<U>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<U>, v0
    return v1
}

function test.main.constrained<T: test.main.Value<U>, U>(): function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<T, U>, v0
    return v1
}

constructor test.main.Box.constructor<T>(v0: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>, v1: T): void {
    local l0: T
    local l1: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>, v1: T):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable> = load l1
    v3: T = load l0
    store (*v2).0, v3
    return
}

function test.main.Box.constructor.new<T>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
entry(v0: T):
    v1: ref<test.main.Box<T>, managed, mutable, local> = new.zeroed test.main.Box<T>, local
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>
    call test.main.Box.constructor<T>(v2, v0): (ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>, T) => void
    return v1
}

function test.main.Box.constructor.new<U>(v0: U): ref<test.main.Box<U>, managed, mutable, local> {
entry(v0: U):
    v1: ref<test.main.Box<U>, managed, mutable, local> = new.zeroed test.main.Box<U>, local
    v2: ref<uninit<test.main.Box<U>>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Box<U>>, borrowed, 'managed, mutable>
    call test.main.Box.constructor<U>(v2, v0): (ref<uninit<test.main.Box<U>>, borrowed, 'managed, mutable>, U) => void
    return v1
}

function test.main.Box.constructor.new<T: test.main.Value<U>, U>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
entry(v0: T):
    v1: ref<test.main.Box<T>, managed, mutable, local> = new.zeroed test.main.Box<T>, local
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>
    call test.main.Box.constructor<T>(v2, v0): (ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>, T) => void
    return v1
}

/// @dispatch.shape constraint=type@13 field=value
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.User { }

function test.main.second(): function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.User.new, v0
    return v1
}

function test.main.first<T>(): function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.User.new, v0
    return v1
}

function test.main.third<T, U>(): function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> {
entry:
    v0: ptr<void, readonly> = null
    v1: function<() => ref<test.main.User, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.User.new, v0
    return v1
}

function test.main.User.new(): ref<test.main.User, managed, mutable, local> {
entry:
    v0: ref<test.main.User, managed, mutable, local> = new.zeroed test.main.User, local
    return v0
}

/// @layout.struct name=test.main.User size=0 align=1
/// @layout.struct name=type@2 size=0 align=1
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

    session.assert_mir_function("main.tspp", "test.main.pair", r#"
@nocopy
type test.main.Counter { }

function test.main.pair(v0: int32, v1: int32): ref<test.main.Counter, managed, mutable, local> {
    local l0: int32
    local l1: int32
    local l2: function<(int32, int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: ptr<void, readonly> = null
    v3: function<(int32, int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v2
    store l2, v3
    v4: function<(int32, int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = load l2
    v5: int32 = load l0
    v6: int32 = load l1
    v7: ref<test.main.Counter, managed, mutable, local> = call.indirect v4(v5, v6): (int32, int32) => ref<test.main.Counter, managed, mutable, local>
    return v7
}

/// @layout.struct name=test.main.Counter size=0 align=1
"#);

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor.new", r#"
@nocopy
type test.main.Counter { }

@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.Counter.constructor.new(v0: int32, v1: int32): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32, v1: int32):
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v3: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v1
    v4: usize = 2
    v5: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, v4, local
    v6: usize = 0
    store (*v5)[v6], v2
    v7: usize = 1
    store (*v5)[v7], v3
    v8: slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable> = new.complete v5
    v9: Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>(v8): (slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable>) => Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>
    v10: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local> = new.complete v9
    v11: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v12: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v11 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v12, v10): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>) => void
    return v11
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

    session.assert_mir_function("main.tspp", "test.main.direct", r#"
@nocopy
type test.main.Counter { }

@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.direct(v0: int32, v1: int32): ref<test.main.Counter, managed, mutable, local> {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v2
    v4: int32 = load l1
    v5: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v4
    v6: usize = 2
    v7: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, v6, local
    v8: usize = 0
    store (*v7)[v8], v3
    v9: usize = 1
    store (*v7)[v9], v5
    v10: slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable> = new.complete v7
    v11: Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>(v10): (slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable>) => Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>
    v12: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local> = new.complete v11
    v13: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v14: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v13 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v14, v12): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>) => void
    return v13
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

    session.assert_mir_function("main.tspp", "test.main.spread", r#"
@nocopy
type test.main.Counter { }

@nocopy
@languageItem("collections.Array")
type Array<T>;

@nocopy
@languageItem("iter.Iterator")
type Iterator<T>;

@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

function test.main.spread(v0: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>): ref<test.main.Counter, managed, mutable, local> {
    local l0: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>
    local l1: function<(ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>
    local l2: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable>
    local l3: usize
    local l4: dynamic<Iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>
    local l5: usize
    local l6: usize

entry(v0: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>):
    store l0, v0
    v1: ptr<void, readonly> = null
    v2: function<(ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    store l1, v2
    v3: function<(ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = load l1
    v4: usize = 0
    v5: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, v4, local
    store l2, v5
    store l3, v4
    v6: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local> = load l0
    v7: dynamic<Iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local> = call Array.Iterable.iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>(v6): (ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>) => dynamic<Iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>
    store l4, v7
    jump b1

b1:
    v8: dynamic<Iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local> = load l4
    v9: dynamic<Iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, borrowed, 'managed, mutable> = cast.bit v8 -> dynamic<Iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, borrowed, 'managed, mutable>
    v10: IteratorResult<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, void> = call.dynamic v9, Iterator<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, 0(): () => IteratorResult<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, void>
    v11: variant<uint1> { 0uint1 = IteratorYield<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>; 1uint1 = IteratorReturn<void>; } = field.get v10, 0
    variant.switch v11, 0 => b2, 1 => b3

b2:
    v12: IteratorYield<variant<uint1> { 0uint1 = int32; 1uint1 = void; }> = variant.payload v11, 0
    v13: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = field.get v12, 1
    v14: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, borrowed, 'frame, readonly> = address (*l2)
    v15: usize = slice.length v14
    v16: usize = load l3
    v17: boolean = eq v16, v15
    branch v17 => b4 | b5

b3:
    v32: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, borrowed, 'frame, readonly> = address (*l2)
    v33: usize = slice.length v32
    v34: usize = load l3
    v35: boolean = eq v34, v33
    branch v35 => b10 | b9

b4:
    v18: usize = add v15, v15
    v19: usize = 1
    v20: usize = add v18, v19
    v21: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = load l2
    v22: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, v20, local
    v23: usize = load l3
    v24: usize = 0
    store l5, v24
    jump b6

b5:
    store (*l2)[v16], v13
    v30: usize = 1
    v31: usize = add v16, v30
    store l3, v31
    jump b1

b6:
    v25: usize = load l5
    v26: boolean = lt v25, v23
    branch v26 => b7 | b8

b7:
    v27: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load (*v21)[v25]
    store (*v22)[v25], v27
    v28: usize = 1
    v29: usize = add v25, v28
    store l5, v29
    jump b6

b8:
    release v21
    store l2, v22
    jump b5

b9:
    v36: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = load l2
    v37: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = new.slice.uninit uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, v34, local
    v38: usize = load l3
    v39: usize = 0
    store l6, v39
    jump b11

b10:
    v45: slice<uninit<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, unique, mutable> = load l2
    v46: slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable> = new.complete v45
    v47: Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }> = call arrayFromOwnedSlice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>(v46): (slice<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, unique, mutable>) => Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>
    v48: ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local> = new.complete v47
    v49: ref<test.main.Counter, managed, mutable, local> = call.indirect v3(v48): (ref<Array<variant<uint1> { 0uint1 = int32; 1uint1 = void; }>, managed, mutable, local>) => ref<test.main.Counter, managed, mutable, local>
    return v49

b11:
    v40: usize = load l6
    v41: boolean = lt v40, v38
    branch v41 => b12 | b13

b12:
    v42: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load (*v36)[v40]
    store (*v37)[v40], v42
    v43: usize = 1
    v44: usize = add v40, v43
    store l6, v44
    jump b11

b13:
    release v36
    store l2, v37
    jump b10
}

/// @layout.struct name=test.main.Counter size=0 align=1
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@133 size=12 align=4
/// @layout.discriminant owner=type@133 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@133 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@133 index=1 discriminant=1 payload_offset=4
"#);
}

/// A namespace-qualified generic class constructs through its selected declaration.
#[test]
fn test_construct_qualified_class() {
    let session = TestSession::builder()
        .module(
            "counter.tspp",
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
            "main.tspp",
            r#"
import * as counter from "./counter.tspp";

function create(value: int32): counter.Counter<int32> {
    return new counter.Counter<int32>(value);
}
"#,
        )
        .build();

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.counter.Counter<T> {
    value: T;
}

function test.main.create(v0: int32): ref<test.counter.Counter<int32>, managed, mutable, local> {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: ref<test.counter.Counter<int32>, managed, mutable, local> = new.zeroed test.counter.Counter<int32>, local
    v3: ref<uninit<test.counter.Counter<int32>>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.counter.Counter<int32>>, borrowed, 'managed, mutable>
    call test.counter.Counter.constructor<int32>(v3, v1): (ref<uninit<test.counter.Counter<int32>>, borrowed, 'managed, mutable>, int32) => void
    return v2
}

external constructor test.counter.Counter.constructor<T>(ref<uninit<test.counter.Counter<T>>, borrowed, 'managed, mutable>, T): void

shared constructor test.counter.Counter.constructor<int32>(v0: ref<uninit<test.counter.Counter<int32>>, borrowed, 'managed, mutable>, v1: int32): void;

/// @layout.struct name=test.counter.Counter<int32> size=4 align=4
/// @layout.field owner=test.counter.Counter<int32> index=0 name=value offset=0 size=4 align=4
/// @layout.struct name=type@14 size=4 align=4
/// @layout.field owner=type@14 index=0 name=value offset=0 size=4 align=4
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

function test.main.read(v0: int32): int32 {
    local l0: int32
    local l1: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>
    local l2: ref<test.main.Counter, managed, mutable, local>

entry(v0: int32):
    store l0, v0
    v1: ptr<void, readonly> = null
    v2: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    store l1, v2
    v3: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = load l1
    v4: int32 = load l0
    v5: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly> = cast.bit v3 -> function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly>
    v6: ref<test.main.Counter, managed, mutable, local> = call.indirect v5(v4): (int32) => ref<test.main.Counter, managed, mutable, local>
    store l2, v6
    v7: ref<test.main.Counter, managed, mutable, local> = load l2
    v8: int32 = load (*v7).0
    return v8
}

function test.main.Counter.constructor.new(v0: int32): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v2, v0): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, int32) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=count offset=0 size=4 align=4
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

    session.assert_mir_function("main.tspp", "test.main.create", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

function test.main.create(): ref<test.main.Counter, managed, mutable, local> {
entry:
    v0: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = call test.main.choose(): () => function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>
    v1: int32 = call test.main.count(): () => int32
    v2: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly> = cast.bit v0 -> function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, borrowed, 'managed, readonly>
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): void {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    local l2: int32, readonly
    local l3: int32

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    store l0, v1
    store l1, v0
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l0
    variant.switch v2, 1 => b2, else b1

b1:
    v3: int32 = variant.payload v2, 0
    store l2, v3
    jump b3

b2:
    v4: int32 = 1
    store l2, v4
    jump b3

b3:
    v5: int32 = load l2
    store l3, v5
    v6: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v7: int32 = load l3
    store (*v6).0, v7
    return
}

function test.main.create(): ref<test.main.Counter, managed, mutable, local> {
    local l0: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry:
    v0: ptr<void, readonly> = null
    v1: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v0
    store l0, v1
    v2: function<() => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = load l0
    v3: ref<test.main.Counter, managed, mutable, local> = call.indirect v2(): () => ref<test.main.Counter, managed, mutable, local>
    return v3
}

function test.main.Counter.constructor.new(): ref<test.main.Counter, managed, mutable, local> {
entry:
    v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v2, v0): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => void
    return v1
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Box<T> {
    value: T;
}

function test.main.create<T>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
    local l0: T
    local l1: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: T):
    store l0, v0
    v1: ptr<void, readonly> = null
    v2: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Box.constructor.new<T>, v1
    store l1, v2
    v3: function<(T) => ref<test.main.Box<T>, managed, mutable, local>, repeatable, managed, mutable, local> = load l1
    v4: T = load l0
    v5: ref<test.main.Box<T>, managed, mutable, local> = call.indirect v3(v4): (T) => ref<test.main.Box<T>, managed, mutable, local>
    return v5
}

constructor test.main.Box.constructor<T>(v0: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>, v1: T): void {
    local l0: T
    local l1: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>, v1: T):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable> = load l1
    v3: T = load l0
    store (*v2).0, v3
    return
}

function test.main.Box.constructor.new<T>(v0: T): ref<test.main.Box<T>, managed, mutable, local> {
entry(v0: T):
    v1: ref<test.main.Box<T>, managed, mutable, local> = new.zeroed test.main.Box<T>, local
    v2: ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>
    call test.main.Box.constructor<T>(v2, v0): (ref<uninit<test.main.Box<T>>, borrowed, 'managed, mutable>, T) => void
    return v1
}
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): void {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    local l2: int32, readonly
    local l3: int32

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    store l0, v1
    store l1, v0
    v2: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = load l0
    variant.switch v2, 1 => b2, else b1

b1:
    v3: int32 = variant.payload v2, 0
    store l2, v3
    jump b3

b2:
    v4: int32 = 1
    store l2, v4
    jump b3

b3:
    v5: int32 = load l2
    store l3, v5
    v6: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v7: int32 = load l3
    store (*v6).0, v7
    return
}

function test.main.create(v0: int32): ref<test.main.Counter, managed, mutable, local> {
    local l0: int32
    local l1: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local>

entry(v0: int32):
    store l0, v0
    v1: ptr<void, readonly> = null
    v2: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    store l1, v2
    v3: function<(int32) => ref<test.main.Counter, managed, mutable, local>, repeatable, managed, mutable, local> = load l1
    v4: int32 = load l0
    v5: ref<test.main.Counter, managed, mutable, local> = call.indirect v3(v4): (int32) => ref<test.main.Counter, managed, mutable, local>
    return v5
}

function test.main.Counter.constructor.new(v0: int32): ref<test.main.Counter, managed, mutable, local> {
entry(v0: int32):
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v2: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v3: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v2 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v3, v1): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => void
    return v2
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_lowered("main.tspp", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    return
}

function test.main.create(v0: int32): variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; } {
    local l0: int32
    local l1: function<(int32) => variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; }, repeatable, managed, mutable, local>

entry(v0: int32):
    store l0, v0
    v1: ptr<void, readonly> = null
    v2: function<(int32) => variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; }, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.new, v1
    store l1, v2
    v3: function<(int32) => variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; }, repeatable, managed, mutable, local> = load l1
    v4: int32 = load l0
    v5: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; } = call.indirect v3(v4): (int32) => variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; }
    return v5
}

function test.main.Counter.constructor.new(v0: int32): variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; } {
entry(v0: int32):
    v1: ref<test.main.Counter, managed, mutable, local> = new.zeroed test.main.Counter, local
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = cast.bit v1 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    call test.main.Counter.constructor(v2, v0): (ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, int32) => void
    v3: variant<uint1> { 0uint1 = ref<test.main.Counter, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v1
    return v3
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@3 size=4 align=4
/// @layout.field owner=type@3 index=0 name=count offset=0 size=4 align=4
/// @layout.variant name=type@9 size=8 align=8
/// @layout.discriminant owner=type@9 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@9 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@9 index=1 discriminant=1 payload_offset=0
"#);
}
