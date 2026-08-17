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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.classify(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    v1: int32 = 0
    local.set l0, v1
    switch v0, b5, 1 => b2, 2 => b3

b1:
    v10: int32 = local.get l0
    return v10

b2:
    breakpoint
    v2: int32 = 10
    local.set l0, v2
    jump b3

b3:
    v3: int32 = local.get l0
    v4: int32 = 2
    v5: int32 = add v3, v4
    local.set l0, v5
    jump b4

b4:
    v6: int32 = local.get l0
    v7: int32 = 3
    v8: int32 = add v6, v7
    local.set l0, v8
    jump b1

b5:
    v9: int32 = 99
    local.set l0, v9
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.select(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: boolean = eq v0, v1
    branch v3 => b2 | b5

b1:
    return

b2:
    v5: int32 = 1
    return v5

b3:
    v6: int32 = 0
    return v6

b4:
    v7: int32 = 2
    return v7

b5:
    v4: boolean = eq v0, v2
    branch v4 => b4 | b6

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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Meters = newtype<int32>;

function test.main.isTwo(v0: Meters): boolean {
entry(v0: Meters):
    v1: int32 = field.get v0, 0
    v2: int32 = 2
    v3: Meters = aggregate (v2)
    v4: int32 = field.get v3, 0
    v5: boolean = eq v1, v4
    branch v5 => b2 | b4

b1:
    return

b2:
    v6: boolean = true
    return v6

b3:
    v7: boolean = false
    return v7

b4:
    jump b3
}
"#,
    );
}

#[test]
fn test_compare_switch_cases_over_the_scalar_carrier_of_a_literal_union() {
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.isTwo(v0: variant<uint1> { 0uint1 = void; 1uint1 = void; }): boolean {
entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = void; }):
    v1: variant<uint1> { 0uint1 = void; 1uint1 = void; } = variant.new 1
    v2: uint1 = variant.tag v0
    v3: uint1 = variant.tag v1
    v4: boolean = eq v2, v3
    branch v4 => b2 | b4

b1:
    return

b2:
    v5: boolean = true
    return v5

b3:
    v6: boolean = false
    return v6

b4:
    jump b3
}

/// @layout.variant name=type@2 size=1 align=1
/// @layout.discriminant owner=type@2 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@2 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@2 index=1 discriminant=1 payload_offset=1
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Ready = newtype<void>;

@copy
type Pending = newtype<void>;

function test.main.isReady(v0: variant<uint1> { 0uint1 = Ready; 1uint1 = Pending; }): boolean {
entry(v0: variant<uint1> { 0uint1 = Ready; 1uint1 = Pending; }):
    v1: boolean = true
    v2: Ready = aggregate ()
    v3: variant<uint1> { 0uint1 = Ready; 1uint1 = Pending; } = variant.new 0, v2
    v4: uint1 = variant.tag v0
    v5: uint1 = variant.tag v3
    v6: boolean = eq v4, v5
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

/// @layout.variant name=type@6 size=1 align=1
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=1
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=1
"#,
    );
}
