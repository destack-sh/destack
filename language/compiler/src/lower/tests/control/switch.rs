use crate::tests::TestSession;

#[test]
fn test_preserve_switch_fallthrough_and_break() {
    let session = TestSession::single(
        r#"
function classify(value: int32): int32 {
    let result: int32 = 0;
    switch (value) {
        case 1:
            debugger;
            result = 10;
        case 2:
            result += 2;
        case 2:
            result += 3;
            break;
        default:
            result = 99;
    }
    return result;
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.classify",
        r#"
function test.main.classify(v0: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32):
    store l0, v0
    v1: int32 = 0
    store l1, v1
    v2: int32 = load l0
    switch v2, b5, 1 => b2, 2 => b3

b1:
    v11: int32 = load l1
    return v11

b2:
    breakpoint
    v3: int32 = 10
    store l1, v3
    jump b3

b3:
    v4: int32 = load l1
    v5: int32 = 2
    v6: int32 = add v4, v5
    store l1, v6
    jump b4

b4:
    v7: int32 = load l1
    v8: int32 = 3
    v9: int32 = add v7, v8
    store l1, v9
    jump b1

b5:
    v10: int32 = 99
    store l1, v10
    jump b1
}
"#,
    );
}

#[test]
fn test_evaluate_switch_selectors_in_order() {
    let session = TestSession::single(
        r#"
function select(value: int32, first: int32, second: int32): int32 {
    switch (value) {
        case first:
            return 1;
        default:
            return 0;
        case second:
            return 2;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.select",
        r#"
function test.main.select(v0: int32, v1: int32, v2: int32): int32 {
    local l0: int32
    local l1: int32
    local l2: int32

entry(v0: int32, v1: int32, v2: int32):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: int32 = load l0
    v4: int32 = load l1
    v5: boolean = eq v3, v4
    branch v5 => b2 | b5

b1:
    return

b2:
    v8: int32 = 1
    return v8

b3:
    v9: int32 = 0
    return v9

b4:
    v10: int32 = 2
    return v10

b5:
    v6: int32 = load l2
    v7: boolean = eq v3, v6
    branch v7 => b4 | b6

b6:
    jump b3
}
"#,
    );
}

#[test]
fn test_compare_switch_cases_over_newtype_backings() {
    let session = TestSession::single(
        r#"
newtype Meters = int32;

function isTwo(value: Meters): boolean {
    switch (value) {
        case Meters(2):
            return true;
        default:
            return false;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.isTwo",
        r#"
type test.main.Meters = newtype<int32>;

function test.main.isTwo(v0: test.main.Meters): boolean {
    local l0: test.main.Meters

entry(v0: test.main.Meters):
    store l0, v0
    v1: test.main.Meters = load l0
    v2: int32 = field.get v1, 0
    v3: int32 = 2
    v4: test.main.Meters = aggregate (v3)
    v5: int32 = field.get v4, 0
    v6: boolean = eq v2, v5
    branch v6 => b2 | b4

b1:
    return

b2:
    v7: boolean = true
    return v7

b3:
    v8: boolean = false
    return v8

b4:
    jump b3
}
"#,
    );
}

#[test]
fn test_compare_switch_cases_over_the_scalar_representation_of_a_literal_union() {
    let session = TestSession::single(
        r#"
function isTwo(value: 1 | 2): boolean {
    switch (value) {
        case 2:
            return true;
        default:
            return false;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.isTwo",
        r#"
function test.main.isTwo(v0: int64): boolean {
    local l0: int64

entry(v0: int64):
    store l0, v0
    v1: int64 = load l0
    v2: int64 = 2
    v3: boolean = eq v1, v2
    branch v3 => b2 | b4

b1:
    return

b2:
    v4: boolean = true
    return v4

b3:
    v5: boolean = false
    return v5

b4:
    jump b3
}
"#,
    );
}

#[test]
fn test_compare_switch_cases_over_singleton_newtype_unions() {
    let session = TestSession::single(
        r#"
newtype Ready = true;
newtype Pending = false;

function isReady(state: Ready | Pending): boolean {
    switch (state) {
        case Ready(true):
            return true;
        default:
            return false;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.isReady",
        r#"
type test.main.Ready = newtype<literal.boolean.true>;

type literal.boolean.true { }

type test.main.Pending = newtype<literal.boolean.false>;

function test.main.isReady(v0: variant<uint1> { 0uint1 = test.main.Ready; 1uint1 = test.main.Pending; }): boolean {
    local l0: variant<uint1> { 0uint1 = test.main.Ready; 1uint1 = test.main.Pending; }

entry(v0: variant<uint1> { 0uint1 = test.main.Ready; 1uint1 = test.main.Pending; }):
    store l0, v0
    v1: variant<uint1> { 0uint1 = test.main.Ready; 1uint1 = test.main.Pending; } = load l0
    v2: literal.boolean.true = zeroed
    v3: test.main.Ready = aggregate (v2)
    v4: variant<uint1> { 0uint1 = test.main.Ready; 1uint1 = test.main.Pending; } = variant.new 0, v3
    v5: uint1 = variant.tag v1
    v6: uint1 = variant.tag v4
    v7: boolean = eq v5, v6
    branch v7 => b2 | b4

b1:
    return

b2:
    v8: boolean = true
    return v8

b3:
    v9: boolean = false
    return v9

b4:
    jump b3
}

/// @layout.struct name=literal.boolean.true size=0 align=1
/// @layout.variant name=type@8 size=1 align=1
/// @layout.discriminant owner=type@8 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=1
"#,
    );
}
