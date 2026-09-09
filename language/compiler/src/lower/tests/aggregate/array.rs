use crate::tests::TestSession;

#[test]
fn test_lower_array_literals_construct_arrays() {
    let session = TestSession::single(
        r#"
function build(): ^int32[] {
    return [1, 2, 3];
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.build", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.build(): Array<int32> {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: usize = 3
    v4: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v3
    v5: usize = 0
    v6: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v4, v5
    store v6, v0
    v7: usize = 1
    v8: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v4, v7
    store v8, v1
    v9: usize = 2
    v10: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v4, v9
    store v10, v2
    v11: slice<int32, unique, mutable, local> = new.complete v4
    v12: Array<int32> = call arrayFromOwnedSlice<int32>(v11): (slice<int32, unique, mutable, local>) => Array<int32>
    return v12
}
"#);
}

#[test]
fn test_lower_pushes_onto_an_owned_array() {
    let session = TestSession::single(
        r#"
import { Array } from "destack:collections";

function fill(): int32 {
    let values: ^Array<int32> = Array.new();

    values.push(1);

    return values.pop() ?? 0;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.fill", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.fill(): int32 {
    local l0: Array<int32>
    local l1: int32, readonly

entry:
    v0: Array<int32> = call Array.new<int32>(): () => Array<int32>
    local.set l0, v0
    v1: int32 = 1
    v2: usize = 1
    v3: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v2
    v4: usize = 0
    v5: ref<uninit<int32>, borrowed, 'frame, mutable, local> = element.address v3, v4
    store v5, v1
    v6: slice<int32, unique, mutable, local> = new.complete v3
    v7: Array<int32> = call arrayFromOwnedSlice<int32>(v6): (slice<int32, unique, mutable, local>) => Array<int32>
    v8: ref<Array<int32>, borrowed, 'frame, mutable, local> = local.address l0
    v9: isize = call Array.push<int32>(v8, v7): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>, ref<Array<int32>, managed, mutable, local>) => isize
    v10: ref<Array<int32>, borrowed, 'frame, mutable, local> = local.address l0
    v11: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = call Array.pop<int32>(v10): <'a>(ref<Array<int32>, borrowed, 'a, mutable, local>) => variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    variant.switch v11, 0 => b2, else b1

b1:
    v12: int32 = variant.payload v11, 1
    local.set l1, v12
    jump b3

b2:
    v13: int32 = 0
    local.set l1, v13
    jump b3

b3:
    v14: int32 = local.get l1
    return v14
}

/// @layout.variant name=type@76 size=8 align=4
/// @layout.discriminant owner=type@76 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@76 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@76 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_a_repeated_fixed_array() {
    let session = TestSession::single(
        r#"
function zeros(): [int32; 4] {
    return [0; 4];
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.zeros",
        r#"
function test.main.zeros(): [int32; 4] {
entry:
    v0: int32 = 0
    v1: [int32; 4] = aggregate (v0, v0, v0, v0)
    return v1
}
"#,
    );
}

#[test]
fn test_lower_a_range_to_its_language_family() {
    let session = TestSession::single(
        r#"
import { Range } from "destack:range";

function span(start: isize, end: isize): Range<isize> {
    return start..end;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.span",
        r#"
@copy
@languageItem("range.Range")
type Range<T>;

function test.main.span(v0: isize, v1: isize): Range<isize> {
    local l0: isize
    local l1: isize

entry(v0: isize, v1: isize):
    local.set l0, v0
    local.set l1, v1
    v2: isize = local.get l0
    v3: isize = local.get l1
    v4: Range<isize> = aggregate (v2, v3)
    return v4
}
"#,
    );
}

#[test]
fn test_lower_an_empty_array_literal_into_fresh_owned_storage() {
    let session = TestSession::single(
        r#"
struct Path {
    steps: ^Array<int32>;
}

function main(): Path {
    const path = Path { steps: [] };
    return path;
}
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.main",
        r#"
@languageItem("collections.Array")
type Array<T>;

@copy
type test.main.Path {
    steps: Array<int32>;
}

function test.main.main(): test.main.Path {
    local l0: test.main.Path

entry:
    v0: usize = 0
    v1: slice<uninit<int32>, unique, mutable, local> = new.slice.uninit uninit<int32>, v0
    v2: slice<int32, unique, mutable, local> = new.complete v1
    v3: Array<int32> = call arrayFromOwnedSlice<int32>(v2): (slice<int32, unique, mutable, local>) => Array<int32>
    v4: test.main.Path = aggregate (v3)
    local.set l0, v4
    v5: test.main.Path = local.get l0
    return v5
}

/// @layout.struct name=test.main.Path size=32 align=8
/// @layout.field owner=test.main.Path index=0 name=steps offset=0 size=32 align=8
"#,
    );
}
