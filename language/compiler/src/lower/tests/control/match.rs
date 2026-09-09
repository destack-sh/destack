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

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@copy
type test.main.Off {
    kind: literal.string.off;
    code: int32;
}

@copy
type test.main.On {
    kind: literal.string.on;
    level: int32;
}

@copy
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.read(v0: test.main.State): int32 {
    local l0: test.main.State
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.State):
    local.set l0, v0
    v1: test.main.State = local.get l0
    v2: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v1, 0
    variant.switch v2, 0 => b1, 1 => b2, else b3

b1:
    v3: test.main.State = local.get l0
    v4: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v3, 0
    v5: test.main.Off = variant.payload v4, 0
    v6: int32 = field.get v5, 1
    local.set l2, v6
    v7: int32 = local.get l2
    local.set l1, v7
    jump b4

b2:
    v8: test.main.State = local.get l0
    v9: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v8, 0
    v10: test.main.On = variant.payload v9, 1
    v11: int32 = field.get v10, 1
    local.set l3, v11
    v12: int32 = local.get l3
    local.set l1, v12
    jump b4

b3:
    unreachable

b4:
    v13: int32 = local.get l1
    return v13
}

/// @layout.struct name=test.main.Off size=4 align=4
/// @layout.field owner=test.main.Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=test.main.On size=4 align=4
/// @layout.field owner=test.main.On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@15 size=8 align=4
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.classify",
        r#"
@copy
type test.main.Off {
    kind: literal.string.off;
    code: int32;
}

@copy
type test.main.On {
    kind: literal.string.on;
    level: int32;
}

@copy
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.classify(v0: test.main.State): int32 {
    local l0: test.main.State
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.State):
    local.set l0, v0
    v1: test.main.State = local.get l0
    v2: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v1, 0
    variant.switch v2, 0 => b1, 1 => b2, else b3

b1:
    v3: test.main.State = local.get l0
    v4: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v3, 0
    v5: test.main.Off = variant.payload v4, 0
    v6: int32 = field.get v5, 1
    local.set l1, v6
    v7: int32 = local.get l1
    return v7

b2:
    v8: test.main.State = local.get l0
    v9: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v8, 0
    v10: test.main.On = variant.payload v9, 1
    v11: int32 = field.get v10, 1
    local.set l2, v11
    v12: int32 = local.get l2
    v13: int32 = local.get l2
    v14: int32 = add v12, v13
    local.set l3, v14
    v15: int32 = local.get l3
    return v15

b3:
    jump b4

b4:
    return
}

/// @layout.struct name=test.main.Off size=4 align=4
/// @layout.field owner=test.main.Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=test.main.On size=4 align=4
/// @layout.field owner=test.main.On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@15 size=8 align=4
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function("main.ds", "test.main.observe", r#"
@copy
type test.main.Off {
    kind: literal.string.off;
    code: int32;
}

@copy
type test.main.On {
    kind: literal.string.on;
    level: int32;
}

@copy
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.observe(v0: test.main.State, v1: function<(int32) => void, repeatable, managed, mutable, local>): void {
    local l0: test.main.State
    local l1: function<(int32) => void, repeatable, managed, mutable, local>
    local l2: int32
    local l3: int32

entry(v0: test.main.State, v1: function<(int32) => void, repeatable, managed, mutable, local>):
    local.set l0, v0
    local.set l1, v1
    v2: test.main.State = local.get l0
    v3: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v2, 0
    variant.switch v3, 0 => b1, 1 => b2, else b3

b1:
    v4: test.main.State = local.get l0
    v5: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v4, 0
    v6: test.main.Off = variant.payload v5, 0
    v7: int32 = field.get v6, 1
    local.set l2, v7
    v8: function<(int32) => void, repeatable, managed, mutable, local> = local.get l1
    v9: int32 = local.get l2
    v10: function<(int32) => void, repeatable, borrowed, 'managed, mutable, local> = cast.bit v8 -> function<(int32) => void, repeatable, borrowed, 'managed, mutable, local>
    call.indirect v10(v9): (int32) => void
    jump b4

b2:
    v11: test.main.State = local.get l0
    v12: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v11, 0
    v13: test.main.On = variant.payload v12, 1
    v14: int32 = field.get v13, 1
    local.set l3, v14
    v15: function<(int32) => void, repeatable, managed, mutable, local> = local.get l1
    v16: int32 = local.get l3
    v17: function<(int32) => void, repeatable, borrowed, 'managed, mutable, local> = cast.bit v15 -> function<(int32) => void, repeatable, borrowed, 'managed, mutable, local>
    call.indirect v17(v16): (int32) => void
    jump b4

b3:
    jump b4

b4:
    return
}

/// @layout.struct name=test.main.Off size=4 align=4
/// @layout.field owner=test.main.Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=test.main.On size=4 align=4
/// @layout.field owner=test.main.On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@15 size=8 align=4
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.classify",
        r#"
@copy
type test.main.Pending {
    kind: literal.string.pending;
    attempts: int32;
}

@copy
type test.main.Done {
    kind: literal.string.done;
    code: int32;
}

@copy
type test.main.Phase = newtype<variant<uint1> { 0uint1 = test.main.Pending; 1uint1 = test.main.Done; }>;

function test.main.classify(v0: test.main.Phase): int32 {
    local l0: test.main.Phase
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.Phase):
    local.set l0, v0
    v1: test.main.Phase = local.get l0
    v2: variant<uint1> { 0uint1 = test.main.Pending; 1uint1 = test.main.Done; } = field.get v1, 0
    variant.switch v2, 0 => b1, 1 => b3, else b4

b1:
    v3: test.main.Phase = local.get l0
    v4: variant<uint1> { 0uint1 = test.main.Pending; 1uint1 = test.main.Done; } = field.get v3, 0
    v5: test.main.Pending = variant.payload v4, 0
    v6: int32 = field.get v5, 1
    v7: int32 = 0
    v8: boolean = eq v6, v7
    branch v8 => b6 | b2

b2:
    v10: test.main.Phase = local.get l0
    v11: variant<uint1> { 0uint1 = test.main.Pending; 1uint1 = test.main.Done; } = field.get v10, 0
    v12: test.main.Pending = variant.payload v11, 0
    v13: int32 = field.get v12, 1
    local.set l2, v13
    v14: int32 = local.get l2
    local.set l1, v14
    jump b5

b3:
    v15: test.main.Phase = local.get l0
    v16: variant<uint1> { 0uint1 = test.main.Pending; 1uint1 = test.main.Done; } = field.get v15, 0
    v17: test.main.Done = variant.payload v16, 1
    v18: int32 = field.get v17, 1
    local.set l3, v18
    v19: int32 = local.get l3
    local.set l1, v19
    jump b5

b4:
    unreachable

b5:
    v20: int32 = local.get l1
    return v20

b6:
    v9: int32 = -1
    local.set l1, v9
    jump b5
}

/// @layout.struct name=test.main.Pending size=4 align=4
/// @layout.field owner=test.main.Pending index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Pending index=1 name=attempts offset=0 size=4 align=4
/// @layout.struct name=test.main.Done size=4 align=4
/// @layout.field owner=test.main.Done index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Done index=1 name=code offset=0 size=4 align=4
/// @layout.variant name=type@15 size=8 align=4
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@copy
type test.main.Off {
    kind: literal.string.off;
    code: int32;
}

@copy
type test.main.On {
    kind: literal.string.on;
    level: int32;
}

@copy
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.read(v0: test.main.State): int32 {
    local l0: test.main.State
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.State):
    local.set l0, v0
    v1: test.main.State = local.get l0
    v2: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v1, 0
    variant.switch v2, 1 => b1, 0 => b3, else b4

b1:
    v3: test.main.State = local.get l0
    v4: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v3, 0
    v5: test.main.On = variant.payload v4, 1
    v6: int32 = field.get v5, 1
    local.set l2, v6
    v7: int32 = local.get l2
    v8: int32 = 10
    v9: boolean = gt v7, v8
    branch v9 => b6 | b2

b2:
    v11: int32 = 0
    local.set l1, v11
    jump b5

b3:
    v12: test.main.State = local.get l0
    v13: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v12, 0
    v14: test.main.Off = variant.payload v13, 0
    v15: int32 = field.get v14, 1
    local.set l3, v15
    v16: int32 = local.get l3
    local.set l1, v16
    jump b5

b4:
    unreachable

b5:
    v17: int32 = local.get l1
    return v17

b6:
    v10: int32 = local.get l2
    local.set l1, v10
    jump b5
}

/// @layout.struct name=test.main.Off size=4 align=4
/// @layout.field owner=test.main.Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=test.main.On size=4 align=4
/// @layout.field owner=test.main.On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@15 size=8 align=4
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.grade",
        r#"
function test.main.grade(v0: int32): int32 {
    local l0: int32
    local l1: int32
    local l2: int32

entry(v0: int32):
    local.set l0, v0
    jump b1

b1:
    v1: int32 = local.get l0
    v2: int32 = 0
    v3: boolean = eq v1, v2
    branch v3 => b7 | b2

b2:
    v5: int32 = local.get l0
    v6: int32 = 1
    v7: boolean = eq v5, v6
    branch v7 => b10 | b9

b3:
    v12: int32 = local.get l0
    v13: int32 = 3
    v14: boolean = ge v12, v13
    branch v14 => b12 | b4

b4:
    v18: int32 = local.get l0
    local.set l2, v18
    v19: int32 = local.get l2
    local.set l1, v19
    jump b6

b5:
    unreachable

b6:
    v20: int32 = local.get l1
    return v20

b7:
    v4: int32 = -1
    local.set l1, v4
    jump b6

b8:
    v11: int32 = 0
    local.set l1, v11
    jump b6

b9:
    v8: int32 = local.get l0
    v9: int32 = 2
    v10: boolean = eq v8, v9
    branch v10 => b11 | b3

b10:
    jump b8

b11:
    jump b8

b12:
    v15: int32 = 5
    v16: boolean = le v12, v15
    branch v16 => b13 | b4

b13:
    v17: int32 = 1
    local.set l1, v17
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

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@copy
type test.main.Off {
    kind: literal.string.off;
    code: int32;
}

@copy
type test.main.On {
    kind: literal.string.on;
    level: int32;
}

@copy
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.read(v0: test.main.State, v1: boolean): int32 {
    local l0: test.main.State
    local l1: boolean
    local l2: int32
    local l3: int32
    local l4: int32

entry(v0: test.main.State, v1: boolean):
    local.set l0, v0
    local.set l1, v1
    v2: test.main.State = local.get l0
    v3: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v2, 0
    variant.switch v3, 1 => b1, 0 => b1, else b1

b1:
    v6: boolean = local.get l1
    branch v6 => b7 | b6

b2:
    v8: test.main.State = local.get l0
    v9: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v8, 0
    v10: test.main.On = variant.payload v9, 1
    v11: int32 = field.get v10, 1
    local.set l3, v11
    v12: int32 = local.get l3
    local.set l2, v12
    jump b5

b3:
    v13: test.main.State = local.get l0
    v14: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v13, 0
    v15: test.main.Off = variant.payload v14, 0
    v16: int32 = field.get v15, 1
    local.set l4, v16
    v17: int32 = local.get l4
    local.set l2, v17
    jump b5

b4:
    unreachable

b5:
    v18: int32 = local.get l2
    return v18

b6:
    v4: test.main.State = local.get l0
    v5: variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; } = field.get v4, 0
    variant.switch v5, 1 => b2, 0 => b3, else b4

b7:
    v7: int32 = 100
    local.set l2, v7
    jump b5
}

/// @layout.struct name=test.main.Off size=4 align=4
/// @layout.field owner=test.main.Off index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.Off index=1 name=code offset=0 size=4 align=4
/// @layout.struct name=test.main.On size=4 align=4
/// @layout.field owner=test.main.On index=0 name=kind offset=4 size=0 align=1
/// @layout.field owner=test.main.On index=1 name=level offset=0 size=4 align=4
/// @layout.variant name=type@15 size=8 align=4
/// @layout.discriminant owner=type@15 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=4
"#,
    );
}

/// A match through a borrowed scrutinee borrows the destructured fields it binds.
#[test]
fn test_lower_a_match_through_a_borrowed_scrutinee_to_field_borrows() {
    let session = TestSession::single(
        r#"
import { Equal } from "destack:ops";

struct Ok<T> {
    value: T;
}

struct Err<E> {
    error: E;
}

newtype Outcome<T, E> = Ok<T> | Err<E>;

function same<T: Equal<T>, E: Equal<E>>(left: &readonly Outcome<T, E>, right: &readonly Outcome<T, E>): boolean {
    match (left) {
        Ok { value: a } => {
            match (right) {
                Ok { value: b } => a.equal(b)
                Err { error: _ } => false
            }
        }
        Err { error: a } => {
            match (right) {
                Err { error: b } => a.equal(b)
                Ok { value: _ } => false
            }
        }
    }
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.same", r#"
@copy
type test.main.Ok<T> {
    value: T;
}

@copy
type test.main.Err<E> {
    error: E;
}

@copy
type test.main.Outcome<T, E> = newtype<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }>;

@languageItem("ops.PartialEqual")
type PartialEqual<T>;

function test.main.same<T: Equal<T>, E: Equal<E>, 'a, 'b>(v0: ref<test.main.Outcome<T, E>, borrowed, 'a, readonly, local>, v1: ref<test.main.Outcome<T, E>, borrowed, 'b, readonly, local>): boolean {
    local l0: ref<test.main.Outcome<T, E>, borrowed, 'a, readonly, local>
    local l1: ref<test.main.Outcome<T, E>, borrowed, 'b, readonly, local>
    local l2: boolean
    local l3: ref<T, borrowed, 'a, readonly, local>
    local l4: ref<T, borrowed, 'b, readonly, local>
    local l5: ref<E, borrowed, 'a, readonly, local>
    local l6: ref<E, borrowed, 'b, readonly, local>

entry(v0: ref<test.main.Outcome<T, E>, borrowed, 'a, readonly, local>, v1: ref<test.main.Outcome<T, E>, borrowed, 'b, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<test.main.Outcome<T, E>, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }, borrowed, 'a, readonly, local> = field.project v2, 0
    v4: uint1 = variant.tag.load v3
    switch v4, b3, 1 => b1, 0 => b2

b1:
    v5: ref<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }, borrowed, 'a, readonly, local> = field.project v2, 0
    v6: uint1 = variant.tag.load v5
    switch v6, b6, 1 => b5

b2:
    v20: ref<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }, borrowed, 'a, readonly, local> = field.project v2, 0
    v21: uint1 = variant.tag.load v20
    switch v21, b14, 0 => b13

b3:
    unreachable

b4:
    v35: boolean = local.get l2
    return v35

b5:
    v7: ref<test.main.Ok<T>, borrowed, 'a, readonly, local> = variant.payload.project v5, 1
    v8: ref<T, borrowed, 'a, readonly, local> = field.address v7, 0
    local.set l3, v8
    v9: ref<test.main.Outcome<T, E>, borrowed, 'b, readonly, local> = local.get l1
    v10: ref<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }, borrowed, 'b, readonly, local> = field.project v9, 0
    v11: uint1 = variant.tag.load v10
    switch v11, b9, 1 => b7, 0 => b8

b6:
    panic

b7:
    v12: ref<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }, borrowed, 'b, readonly, local> = field.project v9, 0
    v13: uint1 = variant.tag.load v12
    switch v13, b12, 1 => b11

b8:
    v19: boolean = false
    local.set l2, v19
    jump b10

b9:
    unreachable

b10:
    jump b4

b11:
    v14: ref<test.main.Ok<T>, borrowed, 'b, readonly, local> = variant.payload.project v12, 1
    v15: ref<T, borrowed, 'b, readonly, local> = field.address v14, 0
    local.set l4, v15
    v16: ref<T, borrowed, 'a, readonly, local> = local.get l3
    v17: ref<T, borrowed, 'b, readonly, local> = local.get l4
    v18: boolean = call.witness T, PartialEqual<T>, PartialEqual.equal(v16, v17): <'a, 'b>(ref<T, borrowed, 'a, readonly, local>, ref<T, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v18
    jump b10

b12:
    panic

b13:
    v22: ref<test.main.Err<E>, borrowed, 'a, readonly, local> = variant.payload.project v20, 0
    v23: ref<E, borrowed, 'a, readonly, local> = field.address v22, 0
    local.set l5, v23
    v24: ref<test.main.Outcome<T, E>, borrowed, 'b, readonly, local> = local.get l1
    v25: ref<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }, borrowed, 'b, readonly, local> = field.project v24, 0
    v26: uint1 = variant.tag.load v25
    switch v26, b17, 0 => b15, 1 => b16

b14:
    panic

b15:
    v27: ref<variant<uint1> { 0uint1 = test.main.Err<E>; 1uint1 = test.main.Ok<T>; }, borrowed, 'b, readonly, local> = field.project v24, 0
    v28: uint1 = variant.tag.load v27
    switch v28, b20, 0 => b19

b16:
    v34: boolean = false
    local.set l2, v34
    jump b18

b17:
    unreachable

b18:
    jump b4

b19:
    v29: ref<test.main.Err<E>, borrowed, 'b, readonly, local> = variant.payload.project v27, 0
    v30: ref<E, borrowed, 'b, readonly, local> = field.address v29, 0
    local.set l6, v30
    v31: ref<E, borrowed, 'a, readonly, local> = local.get l5
    v32: ref<E, borrowed, 'b, readonly, local> = local.get l6
    v33: boolean = call.witness E, PartialEqual<E>, PartialEqual.equal(v31, v32): <'a, 'b>(ref<E, borrowed, 'a, readonly, local>, ref<E, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v33
    jump b18

b20:
    panic
}
"#);
}
