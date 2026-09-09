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
    local.set l0, v0
    local.set l1, v1
    v2: boolean = local.get l0
    local.set l2, v2
    branch v2 => b1 | b2

b1:
    v3: boolean = local.get l1
    local.set l2, v3
    jump b2

b2:
    v4: boolean = local.get l2
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
    local.set l0, v0
    local.set l1, v1
    v2: boolean = local.get l0
    local.set l2, v2
    branch v2 => b2 | b1

b1:
    v3: boolean = local.get l1
    local.set l2, v3
    jump b2

b2:
    v4: boolean = local.get l2
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
@languageItem("string.String")
type String;

function test.main.label(v0: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }): variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } {
    local l0: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }
    local l1: ref<String, managed, mutable, local>, readonly

entry(v0: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; }):
    local.set l0, v0
    v1: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = local.get l0
    variant.switch v1, 1 => b2, else b1

b1:
    v2: ref<String, managed, mutable, local> = variant.payload v1, 0
    local.set l1, v2
    jump b3

b2:
    v3: ref<String, managed, mutable, local> = global.address string.0
    local.set l1, v3
    jump b3

b3:
    v4: ref<String, managed, mutable, local> = local.get l1
    v5: variant<uint1> { 0uint1 = ref<String, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v4
    return v5
}

/// @layout.variant name=type@8 size=8 align=8
/// @layout.discriminant owner=type@8 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=0
"#);
}
