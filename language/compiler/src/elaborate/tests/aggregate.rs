use crate::tests::TestProgram;

/// Generate a destructor freeing each unique field of an aggregated struct in reverse order.
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

    program.assert_optimized(
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

function drop.frame<Pair, 'a>(v0: ref<Pair, borrowed, 'a, exclusive>): void {
entry(v0: ref<Pair, borrowed, 'a, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).1
    v2: ref<int32, unique, mutable> = load (*v1)
    release v2
    v3: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v4: ref<int32, unique, mutable> = load (*v3)
    release v4
    return
}
"#,
    );
}

/// Leave a struct whose only field is a managed dynamic value without a destructor.
#[test]
fn test_skip_destructor_for_dynamic_field() {
    let mut program = TestProgram::mir(
        r#"
type Writer {
    write: fn() => uint32;
}

type Entry {
    writer: dynamic<Writer, managed, mutable, local>;
}

function test(v0: dynamic<Writer, managed, mutable, local>): void {
entry(v0: dynamic<Writer, managed, mutable, local>):
    v1: Entry = aggregate (v0)
    return
}
"#,
    );

    program.assert_optimized(
        r#"
type Writer {
    write: fn() => uint32;
}

type Entry {
    writer: dynamic<Writer, managed, mutable, local>;
}

function test(v0: dynamic<Writer, managed, mutable, local>): void {
entry(v0: dynamic<Writer, managed, mutable, local>):
    v1: Entry = aggregate (v0)
    return
}
"#,
    );
}

/// Generate a destructor freeing the unique slice field of an aggregated struct.
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

    program.assert_optimized(
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

function drop.frame<Buffer, 'a>(v0: ref<Buffer, borrowed, 'a, exclusive>): void {
entry(v0: ref<Buffer, borrowed, 'a, exclusive>):
    v1: ref<slice<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v2: slice<int32, unique, mutable> = load (*v1)
    release v2
    return
}
"#,
    );
}
