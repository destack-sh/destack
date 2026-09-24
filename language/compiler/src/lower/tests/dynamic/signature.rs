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
@nocopy
@languageItem("string.String")
type String;

function test.main.pick(v0: dynamic<{  }, managed, mutable, local>, v1: ref<String, managed, mutable, local>): variant<uint1> { 0uint1 = int32; 1uint1 = void; } {
    local l0: dynamic<{  }, managed, mutable, local>
    local l1: ref<String, managed, mutable, local>

entry(v0: dynamic<{  }, managed, mutable, local>, v1: ref<String, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: dynamic<{  }, managed, mutable, local> = load l0
    v3: ref<String, managed, mutable, local> = load l1
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = dynamic.find v2, v3
    return v4
}

/// @layout.struct name=type@0 size=0 align=1
/// @layout.variant name=type@10 size=8 align=4
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=4
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
@nocopy
@languageItem("string.String")
type String;

function test.main.pick(v0: dynamic<{  }, managed, mutable, local>, v1: ref<String, managed, mutable, local>): variant<uint1> { 0uint1 = int32; 1uint1 = void; } {
    local l0: dynamic<{  }, managed, mutable, local>
    local l1: ref<String, managed, mutable, local>

entry(v0: dynamic<{  }, managed, mutable, local>, v1: ref<String, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: dynamic<{  }, managed, mutable, local> = load l0
    v3: ref<String, managed, mutable, local> = load l1
    v4: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = dynamic.find v2, v3
    return v4
}

/// @layout.struct name=type@0 size=0 align=1
/// @layout.variant name=type@10 size=8 align=4
/// @layout.discriminant owner=type@10 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=4
"#);
}
