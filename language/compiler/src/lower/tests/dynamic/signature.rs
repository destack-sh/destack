use crate::tests::TestSession;

/// Read one computed key through a structural signature's dynamic table.
#[test]
fn test_read_a_computed_key_through_an_index_signature() {
    let session = TestSession::single(
        r#"
type Counts = { [key: string]: int32 };

export function pick(counts: Counts, key: string): int32 | undefined {
    return counts[key];
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.pick", r#"
type test.main.Counts = dynamic<{  }, managed, mutable, local>;

@languageItem("string.String")
type String;

function test.main.pick(v0: test.main.Counts, v1: ref<String, managed, mutable, local>): variant<uint1> { 0uint1 = void; 1uint1 = int32; } {
    local l0: test.main.Counts
    local l1: ref<String, managed, mutable, local>

entry(v0: test.main.Counts, v1: ref<String, managed, mutable, local>):
    local.set l0, v0
    local.set l1, v1
    v2: test.main.Counts = local.get l0
    v3: ref<String, managed, mutable, local> = local.get l1
    v4: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = dynamic.find v2, v3
    return v4
}

/// @layout.variant name=type@13 size=8 align=4
/// @layout.discriminant owner=type@13 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@13 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@13 index=1 discriminant=1 payload_offset=4
"#);
}

/// Read one keyed view over a concrete object through its Record type.
#[test]
fn test_read_a_keyed_view_through_a_record_type() {
    let session = TestSession::single(
        r#"
type Counts = Record<string, int32>;

export function pick(counts: Counts, key: string): int32 | undefined {
    return counts[key];
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.pick", r#"
type test.main.Counts = dynamic<{  }, managed, mutable, local>;

@languageItem("string.String")
type String;

function test.main.pick(v0: test.main.Counts, v1: ref<String, managed, mutable, local>): variant<uint1> { 0uint1 = void; 1uint1 = int32; } {
    local l0: test.main.Counts
    local l1: ref<String, managed, mutable, local>

entry(v0: test.main.Counts, v1: ref<String, managed, mutable, local>):
    local.set l0, v0
    local.set l1, v1
    v2: test.main.Counts = local.get l0
    v3: ref<String, managed, mutable, local> = local.get l1
    v4: variant<uint1> { 0uint1 = void; 1uint1 = int32; } = dynamic.find v2, v3
    return v4
}

/// @layout.variant name=type@13 size=8 align=4
/// @layout.discriminant owner=type@13 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@13 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@13 index=1 discriminant=1 payload_offset=4
"#);
}
