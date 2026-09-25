use crate::tests::TestSession;

#[test]
fn test_lower_omit_newtype_to_the_remaining_fields() {
    let session = TestSession::single(
        r#"
struct Full {
    kept: int32;
    dropped: int64;
    tail: int32;
}

newtype Compact = Omit<Full, "dropped">;

function shrink(kept: int32, tail: int32): Compact {
    return Compact({ kept, tail });
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.shrink",
        r#"
type test.main.Compact = newtype<{ kept: int32, tail: int32 }>;

function test.main.shrink(v0: int32, v1: int32): ref<test.main.Compact, managed, mutable, local> {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: int32 = load l1
    v4: { kept: int32, tail: int32 } = aggregate (v2, v3)
    v5: ref<{ kept: int32, tail: int32 }, managed, mutable, local> = new.complete v4
    v6: ref<test.main.Compact, managed, mutable, local> = cast.bit v5 -> ref<test.main.Compact, managed, mutable, local>
    return v6
}

/// @layout.struct name=type@6 size=8 align=4
/// @layout.field owner=type@6 index=0 name=kept offset=0 size=4 align=4
/// @layout.field owner=type@6 index=1 name=tail offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_pick_alias_field_to_the_named_alias_struct() {
    let session = TestSession::single(
        r#"
struct Full {
    kept: int32;
    dropped: int64;
}

type Kept = Pick<Full, "kept">;

struct Holder {
    slice: Kept;
}

function read(holder: Holder): int32 {
    return holder.slice.kept;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.read",
        r#"
type test.main.Holder {
    slice: ref<{ kept: int32 }, managed, mutable, local>;
}

function test.main.read(v0: test.main.Holder): int32 {
    local l0: test.main.Holder

entry(v0: test.main.Holder):
    store l0, v0
    v1: ref<{ kept: int32 }, managed, mutable, local> = load (l0).0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=test.main.Holder size=8 align=8
/// @layout.field owner=test.main.Holder index=0 name=slice offset=0 size=8 align=8
/// @layout.struct name=type@5 size=4 align=4
/// @layout.field owner=type@5 index=0 name=kept offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_record_newtype_to_keyed_fields() {
    let session = TestSession::single(
        r#"
newtype Pair = Record<"x" | "y", int32>;

function diagonal(value: int32): Pair {
    return Pair({ x: value, y: value });
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.diagonal",
        r#"
type test.main.Pair = newtype<{ x: int32, y: int32 }>;

function test.main.diagonal(v0: int32): ref<test.main.Pair, managed, mutable, local> {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = load l0
    v2: int32 = load l0
    v3: { x: int32, y: int32 } = aggregate (v1, v2)
    v4: ref<{ x: int32, y: int32 }, managed, mutable, local> = new.complete v3
    v5: ref<test.main.Pair, managed, mutable, local> = cast.bit v4 -> ref<test.main.Pair, managed, mutable, local>
    return v5
}

/// @layout.struct name=type@3 size=8 align=4
/// @layout.field owner=type@3 index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=type@3 index=1 name=y offset=4 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_a_bound_interface_call_through_its_conformance() {
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

    session.assert_mir_function(
        "main.tspp",
        "test.main.Cell.Greet.greet",
        r#"
type test.main.Cell {
    value: int32;
}

function test.main.Cell.Greet.greet<'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.Cell, borrowed, 'a, readonly>

entry(v0: ref<test.main.Cell, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.Cell, borrowed, 'a, readonly> = load l0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.run", r#"
type test.main.Cell {
    value: int32;
}

function test.main.run(): int32 {
    local l0: test.main.Cell

entry:
    v0: int32 = 3
    v1: test.main.Cell = aggregate (v0)
    store l0, v1
    v2: ref<test.main.Cell, borrowed, 'frame, readonly> = address l0
    v3: int32 = call test.main.invoke<test.main.Cell>(v2): (ref<test.main.Cell, borrowed, 'frame, readonly>) => int32
    return v3
}

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.tspp",
        "test.main.invoke<test.main.Cell>",
        r#"
type test.main.Cell {
    value: int32;
}

shared function test.main.invoke<test.main.Cell, 'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly>): int32;

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
"#,
    );
}
