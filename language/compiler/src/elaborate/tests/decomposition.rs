use crate::tests::TestProgram;

/// Free the unconsumed fields once a struct is fully decomposed.
#[test]
fn test_insert_drop_after_complete_struct_decomposition() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable> = field.get v2, 0
    v4: ref<int32, unique, mutable> = field.get v2, 1
    release v4
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );
}

/// Free the unconsumed fields once a nested struct is fully decomposed.
#[test]
fn test_insert_drop_after_complete_nested_decomposition() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

type Outer {
    pair: Pair;
    tail: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>):
    v3: Pair = aggregate (v0, v1)
    v4: Outer = aggregate (v3, v2)
    v5: Pair = field.get v4, 0
    v7: ref<int32, unique, mutable> = field.get v4, 1
    v6: ref<int32, unique, mutable> = field.get v5, 0
    v8: ref<int32, unique, mutable> = field.get v5, 1
    call consume(v6): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

type Outer {
    pair: Pair;
    tail: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: ref<int32, unique, mutable>):
    v3: Pair = aggregate (v0, v1)
    v4: Outer = aggregate (v3, v2)
    v5: Pair = field.get v4, 0
    v7: ref<int32, unique, mutable> = field.get v4, 1
    release v7
    v6: ref<int32, unique, mutable> = field.get v5, 0
    v8: ref<int32, unique, mutable> = field.get v5, 1
    release v8
    call consume(v6): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );
}

/// Free the moved payload of a variant once it is read out.
#[test]
fn test_variant_payload_move_consumes_variant() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint1> { 0uint1 = ref<int32, unique, mutable>; 1uint1 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Value = variant<uint1> { 0uint1 = ref<int32, unique, mutable>; 1uint1 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = variant.payload v0, 0
    release v1
    return
}
"#,
    );
}
