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

    session.assert_mir_function(
        "main.ds",
        "test.main.greet",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }): int32 {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    local l1: int32, readonly
    local l2: int32

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l0
    variant.switch v1, 0 => b2, else b1

b1:
    v2: int32 = variant.payload v1, 1
    local.set l1, v2
    jump b3

b2:
    v3: int32 = 3
    local.set l1, v3
    jump b3

b3:
    v4: int32 = local.get l1
    local.set l2, v4
    v5: int32 = local.get l2
    return v5
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.main", r#"
function test.main.main(): int32 {
entry:
    v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 0
    v1: int32 = call test.main.greet(v0): (variant<uint1> { 0uint1 = void; 1uint1 = int32; }) => int32
    return v1
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
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

    session.assert_mir_function(
        "main.ds",
        "test.main.greet",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }): int32 {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    local l1: int32, readonly
    local l2: int32

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l0
    variant.switch v1, 0 => b2, else b1

b1:
    v2: int32 = variant.payload v1, 1
    local.set l1, v2
    jump b3

b2:
    v3: int32 = 3
    local.set l1, v3
    jump b3

b3:
    v4: int32 = local.get l1
    local.set l2, v4
    v5: int32 = local.get l2
    return v5
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.main", r#"
function test.main.main(): int32 {
entry:
    v0: int32 = 7
    v1: int32 = call test.main.greet(v0): (variant<uint1> { 0uint1 = void; 1uint1 = int32; }) => int32
    return v1
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
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

    session.assert_mir_function(
        "main.ds",
        "test.main.greet",
        r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }): int32 {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }
    local l1: int32, readonly
    local l2: int32

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = int32; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = local.get l0
    variant.switch v1, 0 => b2, else b1

b1:
    v2: int32 = variant.payload v1, 1
    local.set l1, v2
    jump b3

b2:
    v3: int32 = 3
    local.set l1, v3
    jump b3

b3:
    v4: int32 = local.get l1
    local.set l2, v4
    v5: int32 = local.get l2
    return v5
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.main", r#"
function test.main.main(): int32 {
entry:
    v0: void = zeroed
    v1: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = variant.new 0
    v2: int32 = call test.main.greet(v1): (variant<uint1> { 0uint1 = void; 1uint1 = int32; }) => int32
    return v2
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_rest_arguments_pack_an_owned_slice() {
    let session = TestSession::single(
        r#"
function total(...values: ^[int32]): isize {
    return values.length;
}

function main(): isize {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.main",
        r#"
function test.main.main(): isize {
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
    v12: isize = call test.main.total(v11): (slice<int32, unique, mutable, local>) => isize
    return v12
}
"#,
    );
}

#[test]
fn test_lower_rest_arguments_pack_a_frame_slice() {
    let session = TestSession::single(
        r#"
function total(...values: &readonly [int32]): isize {
    return values.length;
}

function main(): isize {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.main",
        r#"
function test.main.main(): isize {
    local l0: [int32; 3], readonly

entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    local.set l0, v3
    v4: ref<[int32; 3], borrowed, 'frame, readonly, frame> = local.address l0
    v5: uint64 = 0
    v6: usize = 3
    v7: slice<int32, borrowed, 'frame, readonly, frame> = slice.view v4, v5, v6
    v8: isize = call test.main.total(v7): <'a>(slice<int32, borrowed, 'a, readonly, local>) => isize
    return v8
}
"#,
    );
}

#[test]
fn test_lower_rest_arguments_pack_an_array() {
    let session = TestSession::single(
        r#"
function total(...values: int32[]): isize {
    return values.length;
}

function main(): isize {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.total", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.total(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = local.get l0
    v2: ref<Array<int32>, borrowed, 'managed, readonly, local> = cast.bit v1 -> ref<Array<int32>, borrowed, 'managed, readonly, local>
    v3: isize = call Array.length.get<int32>(v2): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => isize
    return v3
}
"#);

    session.assert_mir_function("main.ds", "test.main.main", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.main(): isize {
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
    v13: isize = call test.main.total(v12): (ref<Array<int32>, managed, mutable, local>) => isize
    return v13
}
"#);
}

#[test]
fn test_lower_a_sole_spread_forwarding_into_a_rest_parameter() {
    let session = TestSession::single(
        r#"
function sum(...values: int32[]): isize {
    return values.length;
}

function forward(values: int32[]): isize {
    return sum(...values);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.sum", r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.sum(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = local.get l0
    v2: ref<Array<int32>, borrowed, 'managed, readonly, local> = cast.bit v1 -> ref<Array<int32>, borrowed, 'managed, readonly, local>
    v3: isize = call Array.length.get<int32>(v2): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => isize
    return v3
}
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.forward",
        r#"
@languageItem("collections.Array")
type Array<T>;

function test.main.forward(v0: ref<Array<int32>, managed, mutable, local>): isize {
    local l0: ref<Array<int32>, managed, mutable, local>

entry(v0: ref<Array<int32>, managed, mutable, local>):
    local.set l0, v0
    v1: ref<Array<int32>, managed, mutable, local> = local.get l0
    v2: isize = call test.main.sum(v1): (ref<Array<int32>, managed, mutable, local>) => isize
    return v2
}
"#,
    );
}
