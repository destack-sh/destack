use crate::tests::TestProgram;

#[test]
fn test_insert_drop_calls_known_function_glue() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

external function dropBox(Box): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = struct Box (v0)
    return
}
"#,
    );
    program.mark_function_drop("Box", "dropBox");

    program.assert_drop_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

external function dropBox(Box): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = struct Box (v0)
    call dropBox(v1)
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_inside_custom_drop_function() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function dropBox(v0: Box): void {
entry(v0: Box):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = struct Box (v0)
    return
}
"#,
    );
    program.mark_function_drop("Box", "dropBox");

    program.assert_drop_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function dropBox(v0: Box): void {
entry(v0: Box):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    free v1
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = struct Box (v0)
    call dropBox(v1)
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_calls_dynamic_glue() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: dynamic<Writer>): void {
entry(v0: dynamic<Writer>):
    return
}
"#,
    );
    program.mark_dynamic_drop_for_constraint("Writer");

    program.assert_drop_mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: dynamic<Writer>): void {
entry(v0: dynamic<Writer>):
    call.dynamic v0, Writer, 0(): (dynamic<Writer>) => void
    return
}
"#,
    );
}

#[test]
fn test_generate_struct_drop_glue() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = struct Pair (v0, v1)
    return
}
"#,
    );

    program.assert_verified_mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = struct Pair (v0, v1)
    call Pair.drop(v2)
    return
}

function Pair.drop(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    free v1
    v2: ref<int32, unique, mutable> = field.get v0, 1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_generate_struct_drop_glue_with_dynamic_field() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

type Entry {
    writer: dynamic<Writer>;
}

function test(v0: dynamic<Writer>): void {
entry(v0: dynamic<Writer>):
    v1: Entry = struct Entry (v0)
    return
}
"#,
    );
    program.mark_dynamic_drop_for_constraint("Writer");

    program.assert_verified_mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

type Entry {
    writer: dynamic<Writer>;
}

function test(v0: dynamic<Writer>): void {
entry(v0: dynamic<Writer>):
    v1: Entry = struct Entry (v0)
    call Entry.drop(v1)
    return
}

function Entry.drop(v0: Entry): void {
entry(v0: Entry):
    v1: dynamic<Writer> = field.get v0, 0
    call.dynamic v1, Writer, 0(): (dynamic<Writer>) => void
    return
}
"#,
    );
}

#[test]
fn test_generate_variant_drop_glue() {
    let mut program = TestProgram::mir(
        r#"
@copy
type ValueStorage = [usize; 1];

type Value = variant<uint8, ValueStorage> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    return
}
"#,
    );

    program.assert_verified_mir(
        r#"
@copy
type ValueStorage = [usize; 1];

type Value = variant<uint8, ValueStorage> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    call Value.drop(v0)
    return
}

function Value.drop(v0: Value): void {
entry(v0: Value):
    v1: uint8 = variant.tag v0
    check variant.tag v1, 0uint8 => b1, b2

b1:
    v2: ref<int32, unique, mutable> = variant.payload v0, 0uint8
    free v2
    jump b2

b2:
    return
}
"#,
    );
}

#[test]
fn test_generate_unique_slice_drop_glue() {
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

    program.assert_verified_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    call slice.Box.drop(v0)
    free v0
    return
}

function slice.Box.drop(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    v1: usize = slice.length v0
    v2: usize = 0
    jump b1(v2)

b1(v3: usize):
    v4: boolean = int.lt.u v3, v1
    branch v4, b2, b3

b2:
    v5: ref<Box, unique, mutable> = element.address v0, v3
    v6: Box = load v5
    call Box.drop(v6)
    v7: usize = 1
    v8: usize = int.add v3, v7
    jump b1(v8)

b3:
    return
}

function Box.drop(v0: Box): void {
entry(v0: Box):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    free v1
    return
}
"#,
    );
}

#[test]
fn test_generate_nested_unique_slice_drop_glue() {
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

    program.assert_verified_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<slice<Box, unique, mutable>, unique, mutable>): void {
entry(v0: slice<slice<Box, unique, mutable>, unique, mutable>):
    call slice.slice.Box.unique.drop(v0)
    free v0
    return
}

function slice.slice.Box.unique.drop(v0: slice<slice<Box, unique, mutable>, unique, mutable>): void {
entry(v0: slice<slice<Box, unique, mutable>, unique, mutable>):
    v1: usize = slice.length v0
    v2: usize = 0
    jump b1(v2)

b1(v3: usize):
    v4: boolean = int.lt.u v3, v1
    branch v4, b2, b3

b2:
    v5: ref<slice<Box, unique, mutable>, unique, mutable> = element.address v0, v3
    v6: slice<Box, unique, mutable> = load v5
    call slice.Box.drop(v6)
    free v6
    v7: usize = 1
    v8: usize = int.add v3, v7
    jump b1(v8)

b3:
    return
}

function slice.Box.drop(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    v1: usize = slice.length v0
    v2: usize = 0
    jump b1(v2)

b1(v3: usize):
    v4: boolean = int.lt.u v3, v1
    branch v4, b2, b3

b2:
    v5: ref<Box, unique, mutable> = element.address v0, v3
    v6: Box = load v5
    call Box.drop(v6)
    v7: usize = 1
    v8: usize = int.add v3, v7
    jump b1(v8)

b3:
    return
}

function Box.drop(v0: Box): void {
entry(v0: Box):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    free v1
    return
}
"#,
    );
}

#[test]
fn test_generate_unique_slice_dynamic_element_drop_glue() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer>, unique, mutable>):
    return
}
"#,
    );
    program.mark_dynamic_drop_for_constraint("Writer");

    program.assert_verified_mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer>, unique, mutable>):
    call slice.dynamic.Writer.drop(v0)
    free v0
    return
}

function slice.dynamic.Writer.drop(v0: slice<dynamic<Writer>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer>, unique, mutable>):
    v1: usize = slice.length v0
    v2: usize = 0
    jump b1(v2)

b1(v3: usize):
    v4: boolean = int.lt.u v3, v1
    branch v4, b2, b3

b2:
    v5: ref<dynamic<Writer>, unique, mutable> = element.address v0, v3
    v6: dynamic<Writer> = load v5
    call.dynamic v6, Writer, 0(): (dynamic<Writer>) => void
    v7: usize = 1
    v8: usize = int.add v3, v7
    jump b1(v8)

b3:
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_drops_unique_pointee_before_free() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    return
}
"#,
    );

    program.assert_verified_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: Box = load v0
    call Box.drop(v1)
    free v0
    return
}

function Box.drop(v0: Box): void {
entry(v0: Box):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    free v1
    return
}
"#,
    );
}
