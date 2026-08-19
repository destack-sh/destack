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
type destack.collections.array.Array<int32> {
    storage: slice<uninit<int32>, unique, exclusive>;
    count: isize;
    allocated: usize;
}

type destack.string.string.String {
    codeUnits: slice<uint16, unique, exclusive>;
}

immortal constant string.3441301661858404811.codeUnits: [uint16; 14] = b"a\x00r\x00r\x00a\x00y\x00F\x00r\x00o\x00m\x00S\x00l\x00i\x00c\x00e\x00"

immortal constant string.3441301661858404811: destack.string.string.String = {{globalAddress string.3441301661858404811.codeUnits, 14uint64}}

function test.main.total(v0: ref<destack.collections.array.Array<int32>, managed, mutable>): int32 {
entry(v0: ref<destack.collections.array.Array<int32>, managed, mutable>):
    v1: isize = call destack.collections.array.length<int32>(v0): (ref<destack.collections.array.Array<int32>, managed, readonly>) => isize
    v2: int32 = cast.truncate v1 -> int32
    return v2
}

function test.main.main(): int32 {
    local l0: [int32; 3], readonly

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    local.set l0, v3
    v4: ref<[int32; 3], borrowed, readonly> = local.address l0
    v5: uint64 = 0
    v6: usize = 3
    v7: slice<int32, borrowed, readonly> = slice.view v4, v5, v6
    v8: destack.collections.array.Array<int32> = call destack.collections.array.arrayFromSlice<int32>(v7): <'a>(slice<int32, borrowed, 'a, readonly>) => destack.collections.array.Array<int32>
    v9: ref<destack.collections.array.Array<int32>, managed, mutable> = cast.bit v8 -> ref<destack.collections.array.Array<int32>, managed, mutable>
    v10: int32 = call test.main.total(v9): (ref<destack.collections.array.Array<int32>, managed, mutable>) => int32
    return v10
}

function destack.collections.array.length<int32>(v0: ref<destack.collections.array.Array<int32>, managed, readonly>): isize {
entry(v0: ref<destack.collections.array.Array<int32>, managed, readonly>):
    v1: ref<isize, borrowed, readonly> = field.address v0, 1
    v2: isize = load v1
    return v2
}

function destack.collections.array.arrayFromSlice<int32, 'a>(v0: slice<int32, borrowed, 'a, readonly>): destack.collections.array.Array<int32> {
entry(v0: slice<int32, borrowed, 'a, readonly>):
    v1: ref<destack.string.string.String, managed, mutable> = global.address string.3441301661858404811
    v2: ref<destack.string.string.String, managed, mutable, undefined> = cast.bit v1 -> ref<destack.string.string.String, managed, mutable, undefined>
    panic v2

b1:
    return
}

/// @layout.struct name=destack.collections.array.Array<int32> size=32 align=8
/// @layout.field owner=destack.collections.array.Array<int32> index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=destack.collections.array.Array<int32> index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=destack.collections.array.Array<int32> index=2 name=allocated offset=24 size=8 align=8
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=codeUnits offset=0 size=16 align=8
"#,
    );
}
