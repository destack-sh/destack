use crate::tests::TestSession;

#[test]
fn test_lower_switch_preserves_fallthrough_and_break() {
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
function main.classify(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    v1: int32 = 0
    local.set l0, v1
    v2: int32 = 1
    v3: boolean = int.eq v0, v2
    branch v3, b2, b6

b1:
    v16: int32 = local.get l0
    return v16

b2:
    breakpoint
    v8: int32 = 10
    local.set l0, v8
    jump b3

b3:
    v9: int32 = local.get l0
    v10: int32 = 2
    v11: int32 = int.add v9, v10
    local.set l0, v11
    jump b4

b4:
    v12: int32 = local.get l0
    v13: int32 = 3
    v14: int32 = int.add v12, v13
    local.set l0, v14
    jump b1

b5:
    v15: int32 = 99
    local.set l0, v15
    jump b1

b6:
    v4: int32 = 2
    v5: boolean = int.eq v0, v4
    branch v5, b3, b7

b7:
    v6: int32 = 2
    v7: boolean = int.eq v0, v6
    branch v7, b4, b8

b8:
    jump b5
}
"#,
    );
}

#[test]
fn test_lower_switch_evaluates_selectors_in_order() {
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
function main.select(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: boolean = int.eq v0, v1
    branch v3, b2, b5

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
    v4: boolean = int.eq v0, v2
    branch v4, b4, b6

b6:
    jump b3
}
"#,
    );
}

#[test]
fn test_lower_switch_compares_newtype_backings() {
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

function main.isTwo(v0: Meters): boolean {
entry(v0: Meters):
    v1: int32 = field.get v0, 0
    v2: int32 = 2
    v3: Meters = aggregate (v2)
    v4: int32 = field.get v3, 0
    v5: boolean = int.eq v1, v4
    branch v5, b2, b4

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
fn test_lower_switch_compares_literal_unions_through_their_scalar_carrier() {
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
function main.isTwo(v0: float64): boolean {
entry(v0: float64):
    v1: float64 = 2
    v2: boolean = float.eq v0, v1
    branch v2, b2, b4

b1:
    return

b2:
    v3: boolean = true
    return v3

b3:
    v4: boolean = false
    return v4

b4:
    jump b3
}
"#,
    );
}

#[test]
fn test_lower_switch_compares_singleton_newtype_union_cases() {
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

function main.isReady(v0: variant<uint8, Ready> { 0uint8 = Ready; 1uint8 = Pending; }): boolean {
entry(v0: variant<uint8, Ready> { 0uint8 = Ready; 1uint8 = Pending; }):
    v1: boolean = true
    v2: Ready = aggregate ()
    v3: variant<uint8, Ready> { 0uint8 = Ready; 1uint8 = Pending; } = variant.new 0, v2
    v4: uint8 = variant.tag v0
    v5: uint8 = variant.tag v3
    v6: boolean = int.eq v4, v5
    branch v6, b2, b4

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
/// @layout.variant name=type@7 size=1 align=1 encoding=direct(tag@0+1) cases=(0@1, 1@1)
/// @layout.variant name=type@19 size=1 align=1 encoding=direct(tag@0+1) cases=(0@1, 1@1)
"#,
    );
}
