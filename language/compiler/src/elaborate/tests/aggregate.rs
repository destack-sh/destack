use crate::tests::TestProgram;

#[test]
fn test_generate_struct_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    drop v2
    return
}

function drop.frame<Pair>(v0: ref<Pair, borrowed, exclusive, frame>): void {
entry(v0: ref<Pair, borrowed, exclusive, frame>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 1
    v2: ref<int32, unique, mutable> = load v1
    free v2
    v3: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v4: ref<int32, unique, mutable> = load v3
    free v4
    return
}
"#,
    );
}

#[test]
fn test_skip_destructor_for_dynamic_field() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

type Entry {
    writer: dynamic<Writer, managed, mutable>;
}

function test(v0: dynamic<Writer, managed, mutable>): void {
entry(v0: dynamic<Writer, managed, mutable>):
    v1: Entry = aggregate (v0)
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

type Entry {
    writer: dynamic<Writer, managed, mutable>;
}

function test(v0: dynamic<Writer, managed, mutable>): void {
entry(v0: dynamic<Writer, managed, mutable>):
    v1: Entry = aggregate (v0)
    return
}
"#,
    );
}

#[test]
fn test_generate_struct_destructor_with_unique_slice_field() {
    let mut program = TestProgram::mir(
        r#"
type Buffer {
    items: slice<int32, unique, mutable>;
}

function test(v0: slice<int32, unique, mutable>): void {
entry(v0: slice<int32, unique, mutable>):
    v1: Buffer = aggregate (v0)
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Buffer {
    items: slice<int32, unique, mutable>;
}

function test(v0: slice<int32, unique, mutable>): void {
entry(v0: slice<int32, unique, mutable>):
    v1: Buffer = aggregate (v0)
    drop v1
    return
}

function drop.frame<Buffer>(v0: ref<Buffer, borrowed, exclusive, frame>): void {
entry(v0: ref<Buffer, borrowed, exclusive, frame>):
    v1: ref<slice<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v2: slice<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_generate_struct_destructor_with_unique_tensor_view_field() {
    let mut program = TestProgram::mir(
        r#"
type Buffer {
    items: tensorView<int32, unique, mutable, (2, 2)>;
}

function test(v0: tensorView<int32, unique, mutable, (2, 2)>): void {
entry(v0: tensorView<int32, unique, mutable, (2, 2)>):
    v1: Buffer = aggregate (v0)
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Buffer {
    items: tensorView<int32, unique, mutable, (2, 2)>;
}

function test(v0: tensorView<int32, unique, mutable, (2, 2)>): void {
entry(v0: tensorView<int32, unique, mutable, (2, 2)>):
    v1: Buffer = aggregate (v0)
    drop v1
    return
}

function drop.frame<Buffer>(v0: ref<Buffer, borrowed, exclusive, frame>): void {
entry(v0: ref<Buffer, borrowed, exclusive, frame>):
    v1: ref<tensorView<int32, unique, mutable, (2, 2)>, borrowed, exclusive, frame> = field.address v0, 0
    v2: tensorView<int32, unique, mutable, (2, 2)> = load v1
    free v2
    return
}
"#,
    );
}
