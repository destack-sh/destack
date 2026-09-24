use crate::tests::TestSession;

#[test]
fn test_short_circuit_a_logical_and() {
    let session = TestSession::single(
        r#"
function both(a: boolean, b: boolean): boolean {
    return a && b;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.both",
        r#"
function test.main.both(v0: boolean, v1: boolean): boolean {
    local l0: boolean
    local l1: boolean
    local l2: boolean

entry(v0: boolean, v1: boolean):
    store l0, v0
    store l1, v1
    v2: boolean = load l0
    store l2, v2
    branch v2 => b1 | b2

b1:
    v3: boolean = load l1
    store l2, v3
    jump b2

b2:
    v4: boolean = load l2
    return v4
}
"#,
    );
}

#[test]
fn test_short_circuit_a_logical_or() {
    let session = TestSession::single(
        r#"
function either(a: boolean, b: boolean): boolean {
    return a || b;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.either",
        r#"
function test.main.either(v0: boolean, v1: boolean): boolean {
    local l0: boolean
    local l1: boolean
    local l2: boolean

entry(v0: boolean, v1: boolean):
    store l0, v0
    store l1, v1
    v2: boolean = load l0
    store l2, v2
    branch v2 => b2 | b1

b1:
    v3: boolean = load l1
    store l2, v3
    jump b2

b2:
    v4: boolean = load l2
    return v4
}
"#,
    );
}

#[test]
fn test_lower_a_coalesce_joining_a_niched_reference_into_an_optional() {
    let session = TestSession::single(
        r#"
function label(name: string | undefined): string | undefined {
    return name ?? "anonymous";
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.label", r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.label(v0: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }): variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l1: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }, readonly

entry(v0: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load l0
    variant.switch v1, 1 => b2, else b1

b1:
    store l1, v1
    jump b3

b2:
    v2: ref<String, managed, mutable, local> = address @string.0
    store l1, v2
    jump b3

b3:
    v3: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = load l1
    return v3
}

/// @layout.variant name=type@7 size=8 align=8
/// @layout.discriminant owner=type@7 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=0
"#);
}
