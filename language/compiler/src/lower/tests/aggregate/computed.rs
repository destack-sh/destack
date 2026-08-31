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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Full {
    kept: int32;
    dropped: int64;
    tail: int32;
}

@copy
type Compact = newtype<ref<{ kept: int32, tail: int32 }, managed, mutable, local>>;

function test.main.shrink(v0: int32, v1: int32): Compact {
entry(v0: int32, v1: int32):
    v2: { kept: int32, tail: int32 } = aggregate (v0, v1)
    v3: ref<{ kept: int32, tail: int32 }, managed, mutable, local> = new.complete v2
    v4: Compact = aggregate (v3)
    return v4
}

/// @layout.struct name=Full size=16 align=8
/// @layout.field owner=Full index=0 name=kept offset=8 size=4 align=4
/// @layout.field owner=Full index=1 name=dropped offset=0 size=8 align=8
/// @layout.field owner=Full index=2 name=tail offset=12 size=4 align=4
/// @layout.struct name=type@8 size=8 align=4
/// @layout.field owner=type@8 index=0 name=kept offset=0 size=4 align=4
/// @layout.field owner=type@8 index=1 name=tail offset=4 size=4 align=4
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Full {
    kept: int32;
    dropped: int64;
}

type Kept {
    kept: int32;
}

@copy
type Holder {
    slice: ref<Kept, managed, mutable, local>;
}

function test.main.read(v0: Holder): int32 {
entry(v0: Holder):
    v1: ref<Kept, managed, mutable, local> = field.get v0, 0
    v2: ref<int32, borrowed, mutable, local> = field.address v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=Full size=16 align=8
/// @layout.field owner=Full index=0 name=kept offset=8 size=4 align=4
/// @layout.field owner=Full index=1 name=dropped offset=0 size=8 align=8
/// @layout.struct name=Kept size=4 align=4
/// @layout.field owner=Kept index=0 name=kept offset=0 size=4 align=4
/// @layout.struct name=Holder size=8 align=8
/// @layout.field owner=Holder index=0 name=slice offset=0 size=8 align=8
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Pair = newtype<ref<{ x: int32, y: int32 }, managed, mutable, local>>;

function test.main.diagonal(v0: int32): Pair {
entry(v0: int32):
    v1: { x: int32, y: int32 } = aggregate (v0, v0)
    v2: ref<{ x: int32, y: int32 }, managed, mutable, local> = new.complete v1
    v3: Pair = aggregate (v2)
    return v3
}

/// @layout.struct name=type@4 size=8 align=4
/// @layout.field owner=type@4 index=0 name=x offset=0 size=4 align=4
/// @layout.field owner=type@4 index=1 name=y offset=4 size=4 align=4
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Cell {
    value: int32;
}

function test.main.Cell.greet<'a>(v0: ref<Cell, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Cell, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function test.main.run(): int32 {
    local l0: Cell

entry:
    v0: int32 = 3
    v1: Cell = aggregate (v0)
    local.set l0, v1
    v2: ref<Cell, borrowed, 'frame, readonly, local> = local.address l0
    v3: int32 = call test.main.invoke<Cell>(v2): <'a>(ref<Cell, borrowed, 'a, readonly, local>) => int32
    return v3
}

function test.main.invoke<Cell, 'a>(v0: ref<Cell, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Cell, borrowed, 'a, readonly, local>):
    v1: int32 = call test.main.greet<Cell>(v0): <'a>(ref<Cell, borrowed, 'a, readonly, local>) => int32
    return v1
}

function test.main.greet<Cell, 'a>(v0: ref<Cell, borrowed, 'a, readonly, local>): int32 {
entry(v0: ref<Cell, borrowed, 'a, readonly, local>):
    v1: ref<int32, borrowed, readonly, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

/// @layout.struct name=Cell size=4 align=4
/// @layout.field owner=Cell index=0 name=value offset=0 size=4 align=4
"#,
    );
}
