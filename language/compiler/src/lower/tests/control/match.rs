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
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.read(v0: test.main.State): int32 {
    local l0: test.main.State
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.State):
    store l0, v0
    v1: uint1 = variant.tag.load (l0).0
    switch v1, b3, 0 => b1, 1 => b2

b1:
    v2: int32 = load ((l0).0 as 0).1
    store l2, v2
    v3: int32 = load l2
    store l1, v3
    jump b4

b2:
    v4: int32 = load ((l0).0 as 1).1
    store l3, v4
    v5: int32 = load l3
    store l1, v5
    jump b4

b3:
    unreachable

b4:
    v6: int32 = load l1
    return v6
}
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
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.classify(v0: test.main.State): int32 {
    local l0: test.main.State
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.State):
    store l0, v0
    v1: uint1 = variant.tag.load (l0).0
    switch v1, b3, 0 => b1, 1 => b2

b1:
    v2: int32 = load ((l0).0 as 0).1
    store l1, v2
    v3: int32 = load l1
    return v3

b2:
    v4: int32 = load ((l0).0 as 1).1
    store l2, v4
    v5: int32 = load l2
    v6: int32 = load l2
    v7: int32 = add v5, v6
    store l3, v7
    v8: int32 = load l3
    return v8

b3:
    jump b4

b4:
    return
}
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
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.observe(v0: test.main.State, v1: function<(int32) => void, repeatable, managed, mutable, local>): void {
    local l0: test.main.State
    local l1: function<(int32) => void, repeatable, managed, mutable, local>
    local l2: int32
    local l3: int32

entry(v0: test.main.State, v1: function<(int32) => void, repeatable, managed, mutable, local>):
    store l0, v0
    store l1, v1
    v2: uint1 = variant.tag.load (l0).0
    switch v2, b3, 0 => b1, 1 => b2

b1:
    v3: int32 = load ((l0).0 as 0).1
    store l2, v3
    v4: function<(int32) => void, repeatable, managed, mutable, local> = load l1
    v5: int32 = load l2
    v6: function<(int32) => void, repeatable, borrowed, 'managed, mutable> = cast.bit v4 -> function<(int32) => void, repeatable, borrowed, 'managed, mutable>
    call.indirect v6(v5): (int32) => void
    jump b4

b2:
    v7: int32 = load ((l0).0 as 1).1
    store l3, v7
    v8: function<(int32) => void, repeatable, managed, mutable, local> = load l1
    v9: int32 = load l3
    v10: function<(int32) => void, repeatable, borrowed, 'managed, mutable> = cast.bit v8 -> function<(int32) => void, repeatable, borrowed, 'managed, mutable>
    call.indirect v10(v9): (int32) => void
    jump b4

b3:
    jump b4

b4:
    return
}
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
type test.main.Phase = newtype<variant<uint1> { 0uint1 = test.main.Pending; 1uint1 = test.main.Done; }>;

function test.main.classify(v0: test.main.Phase): int32 {
    local l0: test.main.Phase
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.Phase):
    store l0, v0
    v1: uint1 = variant.tag.load (l0).0
    switch v1, b4, 0 => b1, 1 => b3

b1:
    v2: int32 = load ((l0).0 as 0).1
    v3: int32 = 0
    v4: boolean = eq v2, v3
    branch v4 => b6 | b2

b2:
    v6: int32 = load ((l0).0 as 0).1
    store l2, v6
    v7: int32 = load l2
    store l1, v7
    jump b5

b3:
    v8: int32 = load ((l0).0 as 1).1
    store l3, v8
    v9: int32 = load l3
    store l1, v9
    jump b5

b4:
    unreachable

b5:
    v10: int32 = load l1
    return v10

b6:
    v5: int32 = -1
    store l1, v5
    jump b5
}
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
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.read(v0: test.main.State): int32 {
    local l0: test.main.State
    local l1: int32
    local l2: int32
    local l3: int32

entry(v0: test.main.State):
    store l0, v0
    v1: uint1 = variant.tag.load (l0).0
    switch v1, b4, 1 => b1, 0 => b3

b1:
    v2: int32 = load ((l0).0 as 1).1
    store l2, v2
    v3: int32 = load l2
    v4: int32 = 10
    v5: boolean = gt v3, v4
    branch v5 => b6 | b2

b2:
    v7: int32 = 0
    store l1, v7
    jump b5

b3:
    v8: int32 = load ((l0).0 as 0).1
    store l3, v8
    v9: int32 = load l3
    store l1, v9
    jump b5

b4:
    unreachable

b5:
    v10: int32 = load l1
    return v10

b6:
    v6: int32 = load l2
    store l1, v6
    jump b5
}
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
    store l0, v0
    jump b1

b1:
    v1: int32 = load l0
    v2: int32 = 0
    v3: boolean = eq v1, v2
    branch v3 => b7 | b2

b2:
    v5: int32 = load l0
    v6: int32 = 1
    v7: boolean = eq v5, v6
    branch v7 => b10 | b9

b3:
    v12: int32 = load l0
    v13: int32 = 3
    v14: boolean = ge v12, v13
    branch v14 => b12 | b4

b4:
    v18: int32 = load l0
    store l2, v18
    v19: int32 = load l2
    store l1, v19
    jump b6

b5:
    unreachable

b6:
    v20: int32 = load l1
    return v20

b7:
    v4: int32 = -1
    store l1, v4
    jump b6

b8:
    v11: int32 = 0
    store l1, v11
    jump b6

b9:
    v8: int32 = load l0
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
    store l1, v17
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
type test.main.State = newtype<variant<uint1> { 0uint1 = test.main.Off; 1uint1 = test.main.On; }>;

function test.main.read(v0: test.main.State, v1: boolean): int32 {
    local l0: test.main.State
    local l1: boolean
    local l2: int32
    local l3: int32
    local l4: int32

entry(v0: test.main.State, v1: boolean):
    store l0, v0
    store l1, v1
    v2: uint1 = variant.tag.load (l0).0
    switch v2, b1, 1 => b1, 0 => b1

b1:
    v4: boolean = load l1
    branch v4 => b7 | b6

b2:
    v6: int32 = load ((l0).0 as 1).1
    store l3, v6
    v7: int32 = load l3
    store l2, v7
    jump b5

b3:
    v8: int32 = load ((l0).0 as 0).1
    store l4, v8
    v9: int32 = load l4
    store l2, v9
    jump b5

b4:
    unreachable

b5:
    v10: int32 = load l2
    return v10

b6:
    v3: uint1 = variant.tag.load (l0).0
    switch v3, b4, 1 => b2, 0 => b3

b7:
    v5: int32 = 100
    store l2, v5
    jump b5
}
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

function same<T: Equal<T>, E: Equal<E>>(left: &immutable Outcome<T, E>, right: &immutable Outcome<T, E>): boolean {
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
type test.main.Outcome<T, E> = newtype<variant<uint1> { 0uint1 = test.main.Ok<T>; 1uint1 = test.main.Err<E>; }>;

@nocopy
@languageItem("ops.PartialEqual")
type PartialEqual<T>;

function test.main.same<T: Equal<T>, E: Equal<E>, 'a, 'b>(v0: ref<test.main.Outcome<T, E>, borrowed, 'a, immutable>, v1: ref<test.main.Outcome<T, E>, borrowed, 'b, immutable>): boolean {
    local l0: ref<test.main.Outcome<T, E>, borrowed, 'a, immutable>
    local l1: ref<test.main.Outcome<T, E>, borrowed, 'b, immutable>
    local l2: boolean
    local l3: ref<?T, borrowed, 'a, immutable>
    local l4: ref<?T, borrowed, 'b, immutable>
    local l5: ref<?E, borrowed, 'a, immutable>
    local l6: ref<?E, borrowed, 'b, immutable>

entry(v0: ref<test.main.Outcome<T, E>, borrowed, 'a, immutable>, v1: ref<test.main.Outcome<T, E>, borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<test.main.Outcome<T, E>, borrowed, 'a, immutable> = load l0
    v3: uint1 = variant.tag.load (*v2).0
    switch v3, b3, 0 => b1, 1 => b2

b1:
    v4: T = load ((*v2).0 as 0).0
    store l3, v4
    v5: ref<test.main.Outcome<T, E>, borrowed, 'b, immutable> = load l1
    v6: uint1 = variant.tag.load (*v5).0
    switch v6, b7, 0 => b5, 1 => b6

b2:
    v12: E = load ((*v2).0 as 1).0
    store l5, v12
    v13: ref<test.main.Outcome<T, E>, borrowed, 'b, immutable> = load l1
    v14: uint1 = variant.tag.load (*v13).0
    switch v14, b11, 1 => b9, 0 => b10

b3:
    unreachable

b4:
    v20: boolean = load l2
    return v20

b5:
    v7: T = load ((*v5).0 as 0).0
    store l4, v7
    v8: ref<?T, borrowed, 'a, immutable> = load l3
    v9: ref<?T, borrowed, 'b, immutable> = load l4
    v10: boolean = call.witness T, PartialEqual<T>, PartialEqual.equal(v8, v9): (ref<?T, borrowed, 'a, immutable>, ref<?T, borrowed, 'b, immutable>) => boolean
    store l2, v10
    jump b8

b6:
    v11: boolean = false
    store l2, v11
    jump b8

b7:
    unreachable

b8:
    jump b4

b9:
    v15: E = load ((*v13).0 as 1).0
    store l6, v15
    v16: ref<?E, borrowed, 'a, immutable> = load l5
    v17: ref<?E, borrowed, 'b, immutable> = load l6
    v18: boolean = call.witness E, PartialEqual<E>, PartialEqual.equal(v16, v17): (ref<?E, borrowed, 'a, immutable>, ref<?E, borrowed, 'b, immutable>) => boolean
    store l2, v18
    jump b12

b10:
    v19: boolean = false
    store l2, v19
    jump b12

b11:
    unreachable

b12:
    jump b4
}
"#);
}
