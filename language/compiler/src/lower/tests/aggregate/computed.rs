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
        "main.ds",
        "test.main.shrink",
        r#"
@copy
type test.main.Compact = newtype<ref<{ kept: int32, tail: int32 }, managed, mutable, local>>;

function test.main.shrink(v0: int32, v1: int32): test.main.Compact {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: int32 = local.get l0
    v3: int32 = local.get l1
    v4: { kept: int32, tail: int32 } = aggregate (v2, v3)
    v5: ref<{ kept: int32, tail: int32 }, managed, mutable, local> = new.complete v4
    v6: test.main.Compact = aggregate (v5)
    return v6
}

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

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
type test.main.Kept {
    kept: int32;
}

@copy
type test.main.Holder {
    slice: ref<test.main.Kept, managed, mutable, local>;
}

function test.main.read(v0: test.main.Holder): int32 {
    local l0: test.main.Holder

entry(v0: test.main.Holder):
    local.set l0, v0
    v1: test.main.Holder = local.get l0
    v2: ref<test.main.Kept, managed, mutable, local> = field.get v1, 0
    v3: ref<int32, borrowed, 'managed, readonly, local> = field.project v2, 0
    v4: int32 = load v3
    return v4
}

/// @layout.struct name=test.main.Kept size=4 align=4
/// @layout.field owner=test.main.Kept index=0 name=kept offset=0 size=4 align=4
/// @layout.struct name=test.main.Holder size=8 align=8
/// @layout.field owner=test.main.Holder index=0 name=slice offset=0 size=8 align=8
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
        "main.ds",
        "test.main.diagonal",
        r#"
@copy
type test.main.Pair = newtype<ref<{ x: int32, y: int32 }, managed, mutable, local>>;

function test.main.diagonal(v0: int32): test.main.Pair {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: int32 = local.get l0
    v3: { x: int32, y: int32 } = aggregate (v1, v2)
    v4: ref<{ x: int32, y: int32 }, managed, mutable, local> = new.complete v3
    v5: test.main.Pair = aggregate (v4)
    return v5
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

    session.assert_mir_function(
        "main.ds",
        "test.main.Cell.Greet.greet",
        r#"
@copy
type test.main.Cell {
    value: int32;
}

function test.main.Cell.Greet.greet<'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly, local>): int32 {
    local l0: ref<test.main.Cell, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Cell, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Cell, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<int32, borrowed, 'a, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.run", r#"
@copy
type test.main.Cell {
    value: int32;
}

function test.main.run(): int32 {
    local l0: test.main.Cell

entry:
    v0: int32 = 3
    v1: test.main.Cell = aggregate (v0)
    local.set l0, v1
    v2: ref<test.main.Cell, borrowed, 'frame, readonly, local> = local.address l0
    v3: int32 = call test.main.invoke<test.main.Cell>(v2): <'a>(ref<test.main.Cell, borrowed, 'a, readonly, local>) => int32
    return v3
}

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.invoke<test.main.Cell>",
        r#"
@copy
type test.main.Cell {
    value: int32;
}

shared function test.main.invoke<test.main.Cell, 'a>(v0: ref<test.main.Cell, borrowed, 'a, readonly, local>): int32;

/// @layout.struct name=test.main.Cell size=4 align=4
/// @layout.field owner=test.main.Cell index=0 name=value offset=0 size=4 align=4
"#,
    );
}
