use crate::tests::TestSession;

#[test]
fn test_lower_an_optional_chain_to_a_short_circuit() {
    let session = TestSession::single(
        r#"
struct Options {
    retries: int32;
}

function read(options: Options | undefined): int32 | undefined {
    return options?.retries;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Options {
    retries: int32;
}

function test.main.read(v0: variant<uint1> { 0uint1 = Options; 1uint1 = void; }): variant<uint1> { 0uint1 = int32; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, readonly

entry(v0: variant<uint1> { 0uint1 = Options; 1uint1 = void; }):
    variant.switch v0, 1 => b3, else b2

b1:
    v5: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = local.get l0
    return v5

b2:
    v2: Options = variant.payload v0, 0
    v3: int32 = field.get v2, 0
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v3
    local.set l0, v4
    jump b1

b3:
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    local.set l0, v1
    jump b1
}

/// @layout.struct name=Options size=4 align=4
/// @layout.field owner=Options index=0 name=retries offset=0 size=4 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@7 size=8 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_a_chained_method_call_through_the_guard() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    total(this): int32 {
        this.value
    }
}

function read(counter: Counter | undefined): int32 | undefined {
    return counter?.total();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Counter {
    value: int32;
}

function test.main.Counter.total(v0: Counter): int32 {
    local l0: Counter

entry(v0: Counter):
    local.set l0, v0
    v1: Counter = local.get l0
    v2: int32 = field.get v1, 0
    return v2
}

function test.main.read(v0: variant<uint1> { 0uint1 = Counter; 1uint1 = void; }): variant<uint1> { 0uint1 = int32; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, readonly

entry(v0: variant<uint1> { 0uint1 = Counter; 1uint1 = void; }):
    variant.switch v0, 1 => b3, else b2

b1:
    v5: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = local.get l0
    return v5

b2:
    v2: Counter = variant.payload v0, 0
    v3: int32 = call test.main.Counter.total(v2): (Counter) => int32
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v3
    local.set l0, v4
    jump b1

b3:
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 1
    local.set l0, v1
    jump b1
}

/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=value offset=0 size=4 align=4
/// @layout.variant name=type@7 size=8 align=4
/// @layout.discriminant owner=type@7 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@8 size=8 align=4
/// @layout.discriminant owner=type@8 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=4
"#,
    );
}
