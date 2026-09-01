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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.both(v0: boolean, v1: boolean): boolean {
    local l0: boolean

entry(v0: boolean, v1: boolean):
    local.set l0, v0
    branch v0 => b1 | b2

b1:
    local.set l0, v1
    jump b2

b2:
    v2: boolean = local.get l0
    return v2
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.either(v0: boolean, v1: boolean): boolean {
    local l0: boolean

entry(v0: boolean, v1: boolean):
    local.set l0, v0
    branch v0 => b2 | b1

b1:
    local.set l0, v1
    jump b2

b2:
    v2: boolean = local.get l0
    return v2
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

    session.assert_mir_lowered("main.ds", r#"
type String {
    codeUnits: slice<uint16, unique, exclusive, local>;
}

function test.main.label(v0: ref<String, managed, mutable, undefined, local>): ref<String, managed, mutable, undefined, local> {
    local l0: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, undefined, local>):
    v1: ref<String, managed, mutable, undefined, local> = undefined
    v2: boolean = eq v0, v1
    branch v2 => b2 | b1

b1:
    v3: ref<String, managed, mutable, local> = cast.bit v0 -> ref<String, managed, mutable, local>
    local.set l0, v3
    jump b3

b2:
    v4: void = undefined
    v5: ref<String, managed, mutable, local> = cast.bit v4 -> ref<String, managed, mutable, local>
    local.set l0, v5
    jump b3

b3:
    v6: ref<String, managed, mutable, local> = local.get l0
    v7: ref<String, managed, mutable, undefined, local> = cast.bit v6 -> ref<String, managed, mutable, undefined, local>
    return v7
}

/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
"#);
}
