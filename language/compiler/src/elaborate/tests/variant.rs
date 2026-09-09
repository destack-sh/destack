use crate::tests::TestProgram;

#[test]
fn test_generate_variant_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint1> { 0uint1 = ref<int32, unique, mutable, local>; 1uint1 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Value = variant<uint1> { 0uint1 = ref<int32, unique, mutable, local>; 1uint1 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    drop v0
    return
}

function drop.frame<Value, 'a>(v0: ref<Value, borrowed, 'a, mutable, frame>): void {
entry(v0: ref<Value, borrowed, 'a, mutable, frame>):
    v1: uint1 = variant.tag.load v0
    v2: uint1 = 0
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: ref<ref<int32, unique, mutable, local>, borrowed, 'a, mutable, frame> = variant.payload.project v0, 0
    v5: ref<int32, unique, mutable, local> = load v4
    release v5
    jump b2

b2:
    return
}
"#,
    );
}

/// A boxed case moves its payload into a box on construction, takes it out and frees the shell
/// on extraction, and borrows through the box on projection.
#[test]
fn test_box_the_payload_of_a_boxed_variant_case() {
    let mut program = TestProgram::mir(
        r#"
type Buffer {
    steps: ref<int32, unique, mutable, local>;
}

type Cell {
    slot: variant<uint1> { 0uint1 = Buffer; 1uint1 = void; };
}

function wrap(v0: Buffer): Cell {
entry(v0: Buffer):
    v1: variant<uint1> { 0uint1 = Buffer; 1uint1 = void; } = variant.new 0, v0
    v2: Cell = aggregate (v1)
    return v2
}

function unwrap(v0: Cell): Buffer {
entry(v0: Cell):
    v1: variant<uint1> { 0uint1 = Buffer; 1uint1 = void; } = field.get v0, 0
    v2: Buffer = variant.payload v1, 0
    return v2
}

function peek<'a>(v0: ref<Cell, borrowed, 'a, readonly, local>): void {
entry(v0: ref<Cell, borrowed, 'a, readonly, local>):
    v1: ref<variant<uint1> { 0uint1 = Buffer; 1uint1 = void; }, borrowed, 'a, readonly, local> = field.project v0, 0
    v2: ref<Buffer, borrowed, 'a, readonly, local> = variant.payload.project v1, 0
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Buffer {
    steps: ref<int32, unique, mutable, local>;
}

type Cell {
    slot: variant<uint1> { 0uint1 = boxed ref<Buffer, unique, mutable, local>; 1uint1 = void; };
}

function wrap(v0: Buffer): Cell {
entry(v0: Buffer):
    v3: ref<Buffer, unique, mutable, local> = new.complete v0
    v1: variant<uint1> { 0uint1 = boxed ref<Buffer, unique, mutable, local>; 1uint1 = void; } = variant.new 0, v3
    v2: Cell = aggregate (v1)
    return v2
}

function unwrap(v0: Cell): Buffer {
entry(v0: Cell):
    v1: variant<uint1> { 0uint1 = boxed ref<Buffer, unique, mutable, local>; 1uint1 = void; } = field.get v0, 0
    v3: ref<Buffer, unique, mutable, local> = variant.payload v1, 0
    v2: Buffer = load v3
    release v3
    return v2
}

function peek<'a>(v0: ref<Cell, borrowed, 'a, readonly, local>): void {
entry(v0: ref<Cell, borrowed, 'a, readonly, local>):
    v1: ref<variant<uint1> { 0uint1 = boxed ref<Buffer, unique, mutable, local>; 1uint1 = void; }, borrowed, 'a, readonly, local> = field.project v0, 0
    v3: ref<ref<Buffer, unique, mutable, local>, borrowed, 'a, readonly, local> = variant.payload.project v1, 0
    v2: ref<Buffer, borrowed, 'a, readonly, local> = load v3
    return
}
"#,
    );
}
