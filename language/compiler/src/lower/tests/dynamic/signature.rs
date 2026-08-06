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

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counts = dynamic<{  }, managed, mutable>;

type destack.memory.unique.Unique<slice<uint8, managed, mutable>> = slice<uint8, unique, exclusive>;

type destack.string.string.String {
    bytes: destack.memory.unique.Unique<slice<uint8, managed, mutable>>;
}

function test.main.pick(v0: Counts, v1: ref<destack.string.string.String, managed, mutable>): variant<uint8> { 0uint8 = int32; 1uint8 = void; } {
entry(v0: Counts, v1: ref<destack.string.string.String, managed, mutable>):
    v2: variant<uint8> { 0uint8 = int32; 1uint8 = void; } = dynamic.find v0, v1
    return v2
}
/// @layout.struct name=destack.string.string.String size=16 align=8
/// @layout.field owner=destack.string.string.String index=0 name=bytes offset=0 size=16 align=8
/// @layout.struct name=type@1 size=0 align=1
/// @layout.variant name=type@14 size=8 align=4
/// @layout.discriminant owner=type@14 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@14 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@14 index=1 discriminant=1 payload_offset=4

/// @dispatch.shape constraint=type@1
"#,
    );
}
