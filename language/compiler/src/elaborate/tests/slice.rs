use crate::tests::TestProgram;

#[test]
fn test_generate_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: slice<Box, unique, mutable, local>): void {
entry(v0: slice<Box, unique, mutable, local>):
    drop v0
    return
}

function drop.frame<slice<Box, unique, mutable, local>>(v0: ref<slice<Box, unique, mutable, local>, borrowed, exclusive, frame>): void {
entry(v0: ref<slice<Box, unique, mutable, local>, borrowed, exclusive, frame>):
    v1: slice<Box, unique, mutable, local> = load v0
    v2: usize = slice.length v1
    v3: usize = 0
    jump b1(v2)

b1(v4: usize):
    v5: boolean = ne v4, v3
    branch v5 => b2 | b3

b2:
    v6: usize = 1
    v7: usize = sub v4, v6
    v8: ref<Box, unique, mutable, local> = element.address v1, v7
    v9: ref<Box, borrowed, exclusive, local> = cast.bit v8 -> ref<Box, borrowed, exclusive, local>
    call drop.local<Box>(v9): (ref<Box, borrowed, exclusive, local>) => void
    jump b1(v7)

b3:
    free v1
    return
}

function drop.local<Box>(v0: ref<Box, borrowed, exclusive, local>): void {
entry(v0: ref<Box, borrowed, exclusive, local>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, exclusive, local> = field.address v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_generate_nested_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<slice<Box, unique, mutable>, unique, mutable>): void {
entry(v0: slice<slice<Box, unique, mutable>, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Box {
    value: ref<int32, unique, mutable, local>;
}

function test(v0: slice<slice<Box, unique, mutable, local>, unique, mutable, local>): void {
entry(v0: slice<slice<Box, unique, mutable, local>, unique, mutable, local>):
    drop v0
    return
}

function drop.frame<slice<slice<Box, unique, mutable, local>, unique, mutable, local>>(v0: ref<slice<slice<Box, unique, mutable, local>, unique, mutable, local>, borrowed, exclusive, frame>): void {
entry(v0: ref<slice<slice<Box, unique, mutable, local>, unique, mutable, local>, borrowed, exclusive, frame>):
    v1: slice<slice<Box, unique, mutable, local>, unique, mutable, local> = load v0
    v2: usize = slice.length v1
    v3: usize = 0
    jump b1(v2)

b1(v4: usize):
    v5: boolean = ne v4, v3
    branch v5 => b2 | b3

b2:
    v6: usize = 1
    v7: usize = sub v4, v6
    v8: ref<slice<Box, unique, mutable, local>, unique, mutable, local> = element.address v1, v7
    v9: ref<slice<Box, unique, mutable, local>, borrowed, exclusive, local> = cast.bit v8 -> ref<slice<Box, unique, mutable, local>, borrowed, exclusive, local>
    call drop.local<slice<Box, unique, mutable, local>>(v9): (ref<slice<Box, unique, mutable, local>, borrowed, exclusive, local>) => void
    jump b1(v7)

b3:
    free v1
    return
}

function drop.local<slice<Box, unique, mutable, local>>(v0: ref<slice<Box, unique, mutable, local>, borrowed, exclusive, local>): void {
entry(v0: ref<slice<Box, unique, mutable, local>, borrowed, exclusive, local>):
    v1: slice<Box, unique, mutable, local> = load v0
    v2: usize = slice.length v1
    v3: usize = 0
    jump b1(v2)

b1(v4: usize):
    v5: boolean = ne v4, v3
    branch v5 => b2 | b3

b2:
    v6: usize = 1
    v7: usize = sub v4, v6
    v8: ref<Box, unique, mutable, local> = element.address v1, v7
    v9: ref<Box, borrowed, exclusive, local> = cast.bit v8 -> ref<Box, borrowed, exclusive, local>
    call drop.local<Box>(v9): (ref<Box, borrowed, exclusive, local>) => void
    jump b1(v7)

b3:
    free v1
    return
}

function drop.local<Box>(v0: ref<Box, borrowed, exclusive, local>): void {
entry(v0: ref<Box, borrowed, exclusive, local>):
    v1: ref<ref<int32, unique, mutable, local>, borrowed, exclusive, local> = field.address v0, 0
    v2: ref<int32, unique, mutable, local> = load v1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_free_unique_slice_with_dynamic_elements() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer, managed, mutable>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer, managed, mutable>, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable, local>): void {
entry(v0: slice<dynamic<Writer, managed, mutable, local>, unique, mutable, local>):
    free v0
    return
}
"#,
    );
}
