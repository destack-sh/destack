use crate::tests::TestSession;

#[test]
fn test_lower_match_destructures_the_narrowed_union_member() {
    let session = TestSession::single(
        r#"
struct Off {
    kind: "off" = "off";
    code: int32;
}

struct On {
    kind: "on" = "on";
    level: int32;
}

newtype State = Off | On;

function read(state: State): int32 {
    match (state) {
        Off { code } => code
        On { level } => level
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Off {
    kind: void;
    code: int32;
}

@copy
type On {
    kind: void;
    level: int32;
}

@copy
type State = newtype<variant<uint1> { 0uint1 = Off; 1uint1 = On; }>;

function test.main.read(v0: State): int32 {
    local l0: int32

entry(v0: State):
    v1: variant<uint1> { 0uint1 = Off; 1uint1 = On; } = field.get v0, 0
    variant.switch v1, 0 => b1, 1 => b2, else b3

b1:
    v2: Off = variant.payload v1, 0
    v3: int32 = field.get v2, 1
    local.set l0, v3
    jump b4

b2:
    v4: On = variant.payload v1, 1
    v5: int32 = field.get v4, 1
    local.set l0, v5
    jump b4

b3:
    unreachable

b4:
    v6: int32 = local.get l0
    return v6
}

/// @layout.struct name=Off size=4 align=4
/// @layout.field owner=Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=On size=4 align=4
/// @layout.field owner=On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=4
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_a_match_statement_with_block_arms() {
    let session = TestSession::single(
        r#"
struct Off {
    kind: "off" = "off";
    code: int32;
}

struct On {
    kind: "on" = "on";
    level: int32;
}

newtype State = Off | On;

function classify(state: State): int32 {
    match (state) {
        Off { code } => {
            return code;
        }
        On { level } => {
            let doubled = level + level;
            return doubled;
        }
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Off {
    kind: void;
    code: int32;
}

@copy
type On {
    kind: void;
    level: int32;
}

@copy
type State = newtype<variant<uint1> { 0uint1 = Off; 1uint1 = On; }>;

function test.main.classify(v0: State): int32 {
    local l0: int32

entry(v0: State):
    v1: variant<uint1> { 0uint1 = Off; 1uint1 = On; } = field.get v0, 0
    variant.switch v1, 0 => b1, 1 => b2, else b3

b1:
    v2: Off = variant.payload v1, 0
    v3: int32 = field.get v2, 1
    return v3

b2:
    v4: On = variant.payload v1, 1
    v5: int32 = field.get v4, 1
    v6: int32 = add v5, v5
    local.set l0, v6
    v7: int32 = local.get l0
    return v7

b3:
    jump b4

b4:
    return
}

/// @layout.struct name=Off size=4 align=4
/// @layout.field owner=Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=On size=4 align=4
/// @layout.field owner=On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=4
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_a_match_statement_with_expression_arms() {
    let session = TestSession::single(
        r#"
struct Off {
    kind: "off" = "off";
    code: int32;
}

struct On {
    kind: "on" = "on";
    level: int32;
}

newtype State = Off | On;

function observe(state: State, sink: (value: int32) => void): void {
    match (state) {
        Off { code } => sink(code)
        On { level } => sink(level)
    }
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
@copy
type Off {
    kind: void;
    code: int32;
}

@copy
type On {
    kind: void;
    level: int32;
}

@copy
type State = newtype<variant<uint1> { 0uint1 = Off; 1uint1 = On; }>;

function test.main.observe(v0: State, v1: function<(int32) => void, repeatable, managed, mutable, local>): void {
entry(v0: State, v1: function<(int32) => void, repeatable, managed, mutable, local>):
    v2: variant<uint1> { 0uint1 = Off; 1uint1 = On; } = field.get v0, 0
    variant.switch v2, 0 => b1, 1 => b2, else b3

b1:
    v3: Off = variant.payload v2, 0
    v4: int32 = field.get v3, 1
    call.indirect v1(v4): (int32) => void
    v5: void = undefined
    jump b4

b2:
    v6: On = variant.payload v2, 1
    v7: int32 = field.get v6, 1
    call.indirect v1(v7): (int32) => void
    v8: void = undefined
    jump b4

b3:
    jump b4

b4:
    return
}

/// @layout.struct name=Off size=4 align=4
/// @layout.field owner=Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=On size=4 align=4
/// @layout.field owner=On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=4
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=4
"#);
}

#[test]
fn test_lower_two_arms_on_one_union_case_through_field_tests() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending" = "pending";
    attempts: int32;
}

struct Done {
    kind: "done" = "done";
    code: int32;
}

newtype Phase = Pending | Done;

function classify(phase: Phase): int32 {
    match (phase) {
        Pending { attempts: 0 } => -1
        Pending { attempts } => attempts
        Done { code } => code
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Pending {
    kind: void;
    attempts: int32;
}

@copy
type Done {
    kind: void;
    code: int32;
}

@copy
type Phase = newtype<variant<uint1> { 0uint1 = Pending; 1uint1 = Done; }>;

function test.main.classify(v0: Phase): int32 {
    local l0: int32

entry(v0: Phase):
    v1: variant<uint1> { 0uint1 = Pending; 1uint1 = Done; } = field.get v0, 0
    variant.switch v1, 0 => b1, 1 => b3, else b4

b1:
    v2: Pending = variant.payload v1, 0
    v3: int32 = field.get v2, 1
    v4: int32 = 0
    v5: boolean = eq v3, v4
    branch v5 => b6 | b2

b2:
    v8: Pending = variant.payload v1, 0
    v9: int32 = field.get v8, 1
    local.set l0, v9
    jump b5

b3:
    v10: Done = variant.payload v1, 1
    v11: int32 = field.get v10, 1
    local.set l0, v11
    jump b5

b4:
    unreachable

b5:
    v12: int32 = local.get l0
    return v12

b6:
    v6: int32 = field.get v2, 1
    v7: int32 = -1
    local.set l0, v7
    jump b5
}

/// @layout.struct name=Pending size=4 align=4
/// @layout.field owner=Pending index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Pending index=1 name=attempts offset=0 size=4 align=4
/// @layout.struct name=Done size=4 align=4
/// @layout.field owner=Done index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Done index=1 name=code offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=4
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_a_guarded_match_arm_to_a_condition_branch() {
    let session = TestSession::single(
        r#"
struct Off {
    kind: "off" = "off";
    code: int32;
}

struct On {
    kind: "on" = "on";
    level: int32;
}

newtype State = Off | On;

function read(state: State): int32 {
    match (state) {
        On { level } if (level > 10) => level
        On {} => 0
        Off { code } => code
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Off {
    kind: void;
    code: int32;
}

@copy
type On {
    kind: void;
    level: int32;
}

@copy
type State = newtype<variant<uint1> { 0uint1 = Off; 1uint1 = On; }>;

function test.main.read(v0: State): int32 {
    local l0: int32

entry(v0: State):
    v1: variant<uint1> { 0uint1 = Off; 1uint1 = On; } = field.get v0, 0
    variant.switch v1, 1 => b1, 0 => b3, else b4

b1:
    v2: On = variant.payload v1, 1
    v3: int32 = field.get v2, 1
    v4: int32 = 10
    v5: boolean = gt v3, v4
    branch v5 => b6 | b2

b2:
    v6: On = variant.payload v1, 1
    v7: int32 = 0
    local.set l0, v7
    jump b5

b3:
    v8: Off = variant.payload v1, 0
    v9: int32 = field.get v8, 1
    local.set l0, v9
    jump b5

b4:
    unreachable

b5:
    v10: int32 = local.get l0
    return v10

b6:
    local.set l0, v3
    jump b5
}

/// @layout.struct name=Off size=4 align=4
/// @layout.field owner=Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=On size=4 align=4
/// @layout.field owner=On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=4
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=4
"#,
    );
}

#[test]
fn test_lower_scalar_literal_arms_to_a_test_chain() {
    let session = TestSession::single(
        r#"
function grade(score: int32): int32 {
    match (score) {
        0 => -1
        1 | 2 => 0
        3..=5 => 1
        other => other
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.grade(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    jump b1

b1:
    v1: int32 = 0
    v2: boolean = eq v0, v1
    branch v2 => b7 | b2

b2:
    v4: int32 = 1
    v5: boolean = eq v0, v4
    branch v5 => b10 | b9

b3:
    v9: int32 = 3
    v10: boolean = ge v0, v9
    branch v10 => b12 | b4

b4:
    local.set l0, v0
    jump b6

b5:
    unreachable

b6:
    v14: int32 = local.get l0
    return v14

b7:
    v3: int32 = -1
    local.set l0, v3
    jump b6

b8:
    v8: int32 = 0
    local.set l0, v8
    jump b6

b9:
    v6: int32 = 2
    v7: boolean = eq v0, v6
    branch v7 => b11 | b3

b10:
    jump b8

b11:
    jump b8

b12:
    v11: int32 = 5
    v12: boolean = le v0, v11
    branch v12 => b13 | b4

b13:
    v13: int32 = 1
    local.set l0, v13
    jump b6
}
"#,
    );
}

#[test]
fn test_lower_a_leading_guarded_wildcard_before_case_arms() {
    let session = TestSession::single(
        r#"
struct Off {
    kind: "off" = "off";
    code: int32;
}

struct On {
    kind: "on" = "on";
    level: int32;
}

newtype State = Off | On;

function read(state: State, all: boolean): int32 {
    match (state) {
        _ if (all) => 100
        On { level } => level
        Off { code } => code
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Off {
    kind: void;
    code: int32;
}

@copy
type On {
    kind: void;
    level: int32;
}

@copy
type State = newtype<variant<uint1> { 0uint1 = Off; 1uint1 = On; }>;

function test.main.read(v0: State, v1: boolean): int32 {
    local l0: int32

entry(v0: State, v1: boolean):
    v2: variant<uint1> { 0uint1 = Off; 1uint1 = On; } = field.get v0, 0
    variant.switch v2, 1 => b1, 0 => b1, else b1

b1:
    branch v1 => b7 | b6

b2:
    v4: On = variant.payload v2, 1
    v5: int32 = field.get v4, 1
    local.set l0, v5
    jump b5

b3:
    v6: Off = variant.payload v2, 0
    v7: int32 = field.get v6, 1
    local.set l0, v7
    jump b5

b4:
    unreachable

b5:
    v8: int32 = local.get l0
    return v8

b6:
    variant.switch v2, 1 => b2, 0 => b3, else b4

b7:
    v3: int32 = 100
    local.set l0, v3
    jump b5
}

/// @layout.struct name=Off size=4 align=4
/// @layout.field owner=Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=On size=4 align=4
/// @layout.field owner=On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@11 size=8 align=4
/// @layout.discriminant owner=type@11 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@11 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@11 index=1 discriminant=1 payload_offset=4
"#,
    );
}
