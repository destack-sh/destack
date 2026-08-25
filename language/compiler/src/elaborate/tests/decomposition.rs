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

    program.assert_elaborated(
        r#"
type Pair {
    left: ref<int32, unique, mutable, local>;
    right: ref<int32, unique, mutable, local>;
}

function consume(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    free v0
    return
}

function test(v0: ref<int32, unique, mutable, local>, v1: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>, v1: ref<int32, unique, mutable, local>):
    v2: Pair = aggregate (v0, v1)
    v3: ref<int32, unique, mutable, local> = field.get v2, 0
    v4: ref<int32, unique, mutable, local> = field.get v2, 1
    free v4
    call consume(v3): (ref<int32, unique, mutable, local>) => void
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

    program.assert_elaborated(
        r#"
type Pair {
    left: ref<int32, unique, mutable, local>;
    right: ref<int32, unique, mutable, local>;
}

type Outer {
    pair: Pair;
    tail: ref<int32, unique, mutable, local>;
}

function consume(v0: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>):
    free v0
    return
}

function test(v0: ref<int32, unique, mutable, local>, v1: ref<int32, unique, mutable, local>, v2: ref<int32, unique, mutable, local>): void {
entry(v0: ref<int32, unique, mutable, local>, v1: ref<int32, unique, mutable, local>, v2: ref<int32, unique, mutable, local>):
    v3: Pair = aggregate (v0, v1)
    v4: Outer = aggregate (v3, v2)
    v5: Pair = field.get v4, 0
    v7: ref<int32, unique, mutable, local> = field.get v4, 1
    free v7
    v6: ref<int32, unique, mutable, local> = field.get v5, 0
    v8: ref<int32, unique, mutable, local> = field.get v5, 1
    free v8
    call consume(v6): (ref<int32, unique, mutable, local>) => void
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
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable> = field.get v0, 1
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable, local>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    v1: ref<int32, unique, mutable, local> = field.get v0, 1
    free v1
    return
}
"#,
    );
}
