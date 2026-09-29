use crate::tests::TestSession;

/// Lower a shared pinned class into shared reference storage and a shared allocation.
#[test]
fn test_lower_shared_class_into_shared_storage() {
    let session = TestSession::single(
        r#"
export shared class Counter {
    value: int32 = 0;
}

export function make(): Counter {
    return new Counter();
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter = class { value: int32 };

export constructor test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.class name=test.main.Counter size=8 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=4 size=4 align=4
"#);

    session.assert_mir_function("main.tspp", "test.main.make", r#"
@nocopy
type test.main.Counter = class { value: int32 };

export function test.main.make(): ref<test.main.Counter, managed, mutable, shared> {
entry:
    v0: ref<test.main.Counter, managed, mutable, shared> = new.zeroed test.main.Counter, shared
    v1: ref<uninit<test.main.Counter>, borrowed, 'managed, exclusive> = cast.bit v0 -> ref<uninit<test.main.Counter>, borrowed, 'managed, exclusive>
    call test.main.Counter.constructor(v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>) => void
    return v0
}

/// @layout.class name=test.main.Counter size=8 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=4 size=4 align=4
"#);
}

/// Lower explicitly placed fields into their written spaces' reference storage.
#[test]
fn test_lower_placed_fields() {
    let session = TestSession::single(
        r#"
export class User {}

export shared class SharedUser {}

export struct Cache {
    localUser: User;
    sharedUser: SharedUser;
}
"#,
    );

    session.assert_mir_lowered(
        "main.tspp",
        r#"
@nocopy
type test.main.User = class {  };

@nocopy
type test.main.SharedUser = class {  };

type test.main.Cache {
    localUser: ref<test.main.User, managed, mutable, local>;
    sharedUser: ref<test.main.SharedUser, managed, mutable, shared>;
}

/// @layout.class name=test.main.User size=4 align=4
/// @layout.class name=test.main.SharedUser size=4 align=4
/// @layout.class name=type@2 size=4 align=4
/// @layout.struct name=type@6 size=16 align=8
/// @layout.field owner=type@6 index=0 name=localUser offset=0 size=8 align=8
/// @layout.field owner=type@6 index=1 name=sharedUser offset=8 size=8 align=8

/// @dispatch.virtual concrete=type@0 value=type@1
/// @dispatch.virtual concrete=type@3 value=type@4
"#,
    );
}

/// Lower a shared class method body over its pinned receiver and fields.
#[test]
fn test_lower_shared_class_method_body() {
    let session = TestSession::single(
        r#"
export shared class Counter {
    value: int32 = 0;

    bump(): int32 {
        return this.value + 1;
    }
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter = class { value: int32 };

export constructor test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.class name=test.main.Counter size=8 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=4 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.Counter.bump",
        r#"
@nocopy
type test.main.Counter = class { value: int32 };

export function test.main.Counter.bump(v0: ref<test.main.Counter, managed, mutable, shared>): int32 {
    local l0: ref<test.main.Counter, managed, mutable, shared>

entry(v0: ref<test.main.Counter, managed, mutable, shared>):
    store l0, v0
    v1: ref<test.main.Counter, managed, mutable, shared> = load l0
    v2: int32 = load (*v1).0
    v3: int32 = 1
    v4: int32 = add v2, v3
    return v4
}

/// @layout.class name=test.main.Counter size=8 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=4 size=4 align=4
"#,
    );
}

/// Lower a shared struct's field read from another module.
#[test]
fn test_lower_shared_field_read_across_modules() {
    let session = TestSession::builder()
        .module(
            "state.tspp",
            r#"
export shared class User {}

export shared struct Holder {
    user: User;
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Holder } from "./state.tspp";

export function read(holder: Holder): void {
    const user = holder.user;
}
"#,
        )
        .build();

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
type test.state.Holder;

@nocopy
type test.state.User;

export function test.main.read(v0: test.state.Holder): void {
    local l0: test.state.Holder
    local l1: ref<test.state.User, managed, mutable, shared>

entry(v0: test.state.Holder):
    store l0, v0
    v1: ref<test.state.User, managed, mutable, shared> = load (l0).0
    store l1, v1
    return
}
"#,
    );
}

/// A callee's induced place lowers to the caller's place parameter inside a place-generic body.
#[test]
fn test_lower_a_callee_place_from_a_borrowed_handle_receiver() {
    let session = TestSession::single(
        r#"
function grow(items: &int64[]): void {
    items.push(1);
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.grow",
        r#"
@nocopy
@languageItem("collections.Array")
type Array<T>;

export function test.main.grow<'a>(v0: ref<Array<int64>, borrowed, 'a, mutable>): void {
    local l0: ref<Array<int64>, borrowed, 'a, mutable>

entry(v0: ref<Array<int64>, borrowed, 'a, mutable>):
    store l0, v0
    v1: ref<Array<int64>, borrowed, 'a, mutable> = load l0
    v2: int64 = 1
    v3: usize = 1
    v4: slice<uninit<int64>, unique, mutable> = new.slice.uninit uninit<int64>, v3, local
    v5: usize = 0
    store (*v4)[v5], v2
    v6: slice<int64, unique, mutable> = new.complete v4
    v7: Array<int64> = call arrayFromOwnedSlice<int64>(v6): (slice<int64, unique, mutable>) => Array<int64>
    v8: uninit<ref<Array<int64>, managed, mutable, local>> = new.uninit Array<int64>, local
    store (*v8), v7
    v9: ref<Array<int64>, managed, mutable, local> = new.complete v8
    v10: isize = call Array.push<int64>(v1, v9): (ref<Array<int64>, borrowed, 'a, mutable>, ref<Array<int64>, managed, mutable, local>) => isize
    return
}
"#,
    );
}

/// A borrowed receiver at a generic place lowers the callee's induced place to that place.
#[test]
fn test_lower_a_callee_place_from_a_generic_borrowed_receiver() {
    let session = TestSession::single(
        r#"
import { Clone } from "tspp:memory";

function duplicate<T: Clone>(value: &immutable T): T {
    return value.clone();
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.duplicate",
        r#"
@nocopy
@languageItem("memory.Clone")
type Clone;

export function test.main.duplicate<T: Clone, 'a>(v0: ref<?T, borrowed, 'a, immutable>): T {
    local l0: ref<?T, borrowed, 'a, immutable>

entry(v0: ref<?T, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<?T, borrowed, 'a, immutable> = load l0
    v2: ?T = call.witness T, Clone, Clone.clone(v1): (ref<?T, borrowed, 'a, immutable>) => ?T
    v3: T = new.complete v2
    return v3
}
"#,
    );
}
