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

function drop.frame<Value, 'a>(v0: ref<Value, borrowed, 'a & frame, exclusive>): void {
entry(v0: ref<Value, borrowed, 'a & frame, exclusive>):
    v1: uint1 = variant.tag.load (*v0)
    v2: uint1 = 0
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: ref<ref<int32, unique, mutable, local>, borrowed, 'a & frame, exclusive> = address ((*v0) as 0)
    v5: ref<int32, unique, mutable, local> = load (*v4)
    release v5
    jump b2

b2:
    return
}
"#,
    );
}
