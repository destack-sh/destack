use crate::tests::TestSession;

#[test]
fn test_lower_omitted_call_evaluates_the_callee_default() {
    let session = TestSession::single(
        r#"
function greet(count: int32 = 3): int32 {
    return count;
}

function main(): int32 {
    return greet();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): int32 {
    local l0: int32, readonly

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    variant.switch v0, 1 => b2, else b1

b1:
    v1: int32 = variant.payload v0, 0
    local.set l0, v1
    jump b3

b2:
    v2: int32 = 3
    local.set l0, v2
    jump b3

b3:
    v3: int32 = local.get l0
    return v3
}

function test.main.main(): int32 {
entry:
    v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v1: int32 = call test.main.greet(v0): (variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => int32
    return v1
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_provided_call_wraps_the_defaulted_parameter() {
    let session = TestSession::single(
        r#"
function greet(count: int32 = 3): int32 {
    return count;
}

function main(): int32 {
    return greet(7);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): int32 {
    local l0: int32, readonly

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    variant.switch v0, 1 => b2, else b1

b1:
    v1: int32 = variant.payload v0, 0
    local.set l0, v1
    jump b3

b2:
    v2: int32 = 3
    local.set l0, v2
    jump b3

b3:
    v3: int32 = local.get l0
    return v3
}

function test.main.main(): int32 {
entry:
    v0: int32 = 7
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v2: int32 = call test.main.greet(v1): (variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => int32
    return v2
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_undefined_argument_takes_the_callee_default() {
    let session = TestSession::single(
        r#"
function greet(count: int32 | undefined = 3): int32 {
    return count;
}

function main(): int32 {
    return greet(undefined);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }): int32 {
    local l0: int32, readonly

entry(v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }):
    variant.switch v0, 1 => b2, else b1

b1:
    v1: int32 = variant.payload v0, 0
    local.set l0, v1
    jump b3

b2:
    v2: int32 = 3
    local.set l0, v2
    jump b3

b3:
    v3: int32 = local.get l0
    return v3
}

function test.main.main(): int32 {
entry:
    v0: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    v1: int32 = call test.main.greet(v0): (variant<uint1> { 0uint1 = int32; 1uint1 = void; }) => int32
    return v1
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_rest_arguments_pack_an_array() {
    let session = TestSession::single(
        r#"
function total(...values: int32[]): int32 {
    return values.length as int32;
}

function main(): int32 {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Array<int32> {
    storage: slice<uninit<int32>, unique, exclusive, local>;
    count: isize;
    allocated: usize;
}

@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

constant string.0: String = "arrayFromSlice"

function test.main.total(v0: ref<Array<int32>, managed, mutable, local>): int32 {
entry(v0: ref<Array<int32>, managed, mutable, local>):
    v1: ref<Array<int32>, borrowed, 'frame, readonly, local> = cast.bit v0 -> ref<Array<int32>, borrowed, 'frame, readonly, local>
    v2: isize = call length<int32>(v1): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => isize
    v3: int32 = cast.truncate v2 -> int32
    return v3
}

function test.main.main(): int32 {
    local l0: [int32; 3], readonly

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    local.set l0, v3
    v4: ref<[int32; 3], borrowed, readonly, frame> = local.address l0
    v5: uint64 = 0
    v6: usize = 3
    v7: slice<int32, borrowed, readonly, frame> = slice.view v4, v5, v6
    v8: slice<int32, borrowed, 'l0, readonly, local> = cast.bit v7 -> slice<int32, borrowed, 'l0, readonly, local>
    v9: Array<int32> = call arrayFromSlice<int32>(v8): <'a>(slice<int32, borrowed, 'a, readonly, local>) => Array<int32>
    v10: ref<Array<int32>, managed, mutable, local> = cast.bit v9 -> ref<Array<int32>, managed, mutable, local>
    v11: int32 = call test.main.total(v10): (ref<Array<int32>, managed, mutable, local>) => int32
    return v11
}

function length<int32, 'a>(v0: ref<Array<int32>, borrowed, 'a, readonly, local>): isize {
entry(v0: ref<Array<int32>, borrowed, 'a, readonly, local>):
    v1: ref<isize, borrowed, readonly, local> = field.address v0, 1
    v2: isize = load v1
    return v2
}

function arrayFromSlice<int32, 'a>(v0: slice<int32, borrowed, 'a, readonly, local>): Array<int32> {
entry(v0: slice<int32, borrowed, 'a, readonly, local>):
    v1: ref<String, managed, mutable, local> = global.address string.0
    v2: ref<String, managed, mutable, undefined, local> = cast.bit v1 -> ref<String, managed, mutable, undefined, local>
    panic v2

b1:
    return
}

/// @layout.struct name=Array<int32> size=32 align=8
/// @layout.field owner=Array<int32> index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=Array<int32> index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=Array<int32> index=2 name=allocated offset=24 size=8 align=8
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}
