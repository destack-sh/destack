use crate::tests::TestProgram;

#[test]
fn test_generate_variant_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Value = variant<uint8> { 0uint8 = ref<int32, unique, mutable, local>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    drop v0
    return
}

function drop.frame<Value>(v0: ref<Value, borrowed, exclusive, frame>): void {
entry(v0: ref<Value, borrowed, exclusive, frame>):
    v1: uint8 = variant.tag.load v0
    v2: uint8 = 0
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: ref<ref<int32, unique, mutable, local>, borrowed, exclusive, frame> = variant.payload.address v0, 0
    v5: ref<int32, unique, mutable, local> = load v4
    free v5
    jump b2

b2:
    return
}
"#,
    );
}
