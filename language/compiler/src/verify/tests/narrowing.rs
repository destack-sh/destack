use crate::tests::{TestProgram, TestSession};

/// A call writing a narrowed place through a loan taken before the test breaks the narrowing.
#[test]
fn test_reject_a_write_through_an_earlier_loan_under_a_narrowing() {
    let mut program = TestProgram::mir(
        r#"
type Slot {
    value: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
}

external function clear<'a>(ref<Slot, borrowed, 'a, mutable>): void

function test(v0: Slot): int32 {
    local l0: Slot

entry(v0: Slot):
    store l0, v0
    v1: ref<Slot, borrowed, 'frame, mutable> = address l0
    v2: ref<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, borrowed, 'frame, readonly> = fake.borrow (l0).0
    v3: uint1 = variant.tag.load (l0).0
    call clear(v1): <'a>(ref<Slot, borrowed, 'a, mutable>) => void
    fake.read v2
    v4: int32 = load ((l0).0 as 0)
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[stale-narrowing]: cannot change a place while a narrowing of it is used
  ──▶ <test.tsppm>:16:5
   │
12 │     store l0, v0
13 │     v1: ref<Slot, borrowed, 'frame, mutable> = address l0
14 │ ··<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, borrowed, 'frame, readonly> = fake.borrow (l0).0
   │   ---------------------------------------------------------------------------------------------------- narrowed here
15 │     v3: uint1 = variant.tag.load (l0).0
16 │     call clear(v1): <'a>(ref<Slot, borrowed, 'a, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
17 │     fake.read v2
18 │     v4: int32 = load ((l0).0 as 0)
   │

for more information about an error, run `tspp explain stale-narrowing`
"#,
    );
}

/// A narrowing no operation changes holds at its reads.
#[test]
fn test_allow_a_narrowed_read_of_an_unchanged_place() {
    let mut program = TestProgram::mir(
        r#"
type Slot {
    value: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
}

function test(v0: Slot): int32 {
    local l0: Slot

entry(v0: Slot):
    store l0, v0
    v1: ref<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, borrowed, 'frame, readonly> = fake.borrow (l0).0
    v2: uint1 = variant.tag.load (l0).0
    fake.read v1
    v3: int32 = load ((l0).0 as 0)
    return v3
}
"#,
    );

    program.assert_verified();
}

/// Overwriting a narrowed place directly breaks the narrowing.
#[test]
fn test_reject_an_overwrite_under_a_narrowing() {
    let mut program = TestProgram::mir(
        r#"
type Slot {
    value: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
}

function test(v0: Slot, v1: Slot): int32 {
    local l0: Slot

entry(v0: Slot, v1: Slot):
    store l0, v0
    v2: ref<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, borrowed, 'frame, readonly> = fake.borrow (l0).0
    v3: uint1 = variant.tag.load (l0).0
    store l0, v1
    fake.read v2
    v4: int32 = load ((l0).0 as 0)
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[stale-narrowing]: cannot change a place while a narrowing of it is used
  ──▶ <test.tsppm>:13:5
   │
 9 │ entry(v0: Slot, v1: Slot):
10 │     store l0, v0
11 │ ··<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, borrowed, 'frame, readonly> = fake.borrow (l0).0
   │   ---------------------------------------------------------------------------------------------------- narrowed here
12 │     v3: uint1 = variant.tag.load (l0).0
13 │     store l0, v1
   │     ^^^^^^^^^^^^
14 │     fake.read v2
15 │     v4: int32 = load ((l0).0 as 0)
   │

for more information about an error, run `tspp explain stale-narrowing`
"#,
    );
}

/// Writing inside a narrowed payload leaves the narrowing standing.
#[test]
fn test_allow_a_write_inside_a_narrowed_payload() {
    let mut program = TestProgram::mir(
        r#"
type Point {
    x: int32;
}

type Slot {
    value: variant<uint1> { 0uint1 = Point; 1uint1 = void; };
}

function test(v0: Slot, v1: int32): int32 {
    local l0: Slot

entry(v0: Slot, v1: int32):
    store l0, v0
    v2: ref<variant<uint1> { 0uint1 = Point; 1uint1 = void; }, borrowed, 'frame, readonly> = fake.borrow (l0).0
    v3: uint1 = variant.tag.load (l0).0
    store ((l0).0 as 0).0, v1
    fake.read v2
    v4: int32 = load ((l0).0 as 0).0
    return v4
}
"#,
    );

    program.assert_verified();
}

/// Any call may change a narrowed place other holders reach.
#[test]
fn test_reject_a_call_under_a_narrowing_of_a_managed_place() {
    let mut program = TestProgram::mir(
        r#"
type Slot {
    value: variant<uint1> { 0uint1 = int32; 1uint1 = void; };
}

external function tick(): void

function test(v0: ref<Slot, managed, mutable, local>): int32 {
entry(v0: ref<Slot, managed, mutable, local>):
    v1: ref<variant<uint1> { 0uint1 = int32; 1uint1 = void; }, borrowed, 'managed, readonly> = fake.borrow (*v0).0
    v2: uint1 = variant.tag.load (*v0).0
    call tick(): () => void
    fake.read v1
    v3: int32 = load ((*v0).0 as 0)
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[stale-narrowing]: cannot change a place while a narrowing of it is used
  ──▶ <test.tsppm>:12:5
   │
 8 │ function test(v0: ref<Slot, managed, mutable, local>): int32 {
 9 │ entry(v0: ref<Slot, managed, mutable, local>):
10 │ ··riant<uint1> { 0uint1 = int32; 1uint1 = void; }, borrowed, 'managed, readonly> = fake.borrow (*v0).0
   │   ---------------------------------------------------------------------------------------------------- narrowed here
11 │     v2: uint1 = variant.tag.load (*v0).0
12 │     call tick(): () => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^
13 │     fake.read v1
14 │     v3: int32 = load ((*v0).0 as 0)
   │

for more information about an error, run `tspp explain stale-narrowing`
"#,
    );
}

/// A call may change a narrowed field, and a local copy keeps its narrowing.
#[test]
fn test_reject_a_call_under_a_field_narrowing() {
    let session = TestSession::single(
        r#"
class Holder {
    name: string | undefined = undefined;
}

declare function reset(holder: Holder): void;

function stale(holder: Holder): string {
    if (holder.name != undefined) {
        reset(holder);
        holder.name
    } else {
        ""
    }
}

function copied(holder: Holder): string {
    const name = holder.name;
    if (name != undefined) {
        reset(holder);
        name
    } else {
        ""
    }
}

function kept(holder: Holder): string {
    if (holder.name != undefined) {
        holder.name
    } else {
        ""
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=stale-narrowing message="cannot change a place while a narrowing of it is used"
/// @diagnostic.label line=10 column=9 span="reset(holder)" line_source="reset(holder);"
/// @diagnostic.related line=9 column=9 span="holder.name != undefined" line_source="if (holder.name != undefined) {" message="narrowed here"
"#,
    );
}

/// A store may change a narrowed field of its own declaration, never a distinct field or one it writes inside.
#[test]
fn test_reject_a_store_to_the_same_field_under_a_narrowing() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending";
    tail: string | undefined;
}

struct Done {
    kind: "done";
}

class Holder {
    state: Pending | Done = Pending { kind: "pending", tail: undefined };
    name: string | undefined = undefined;
    count: int32 = 0;
}

function through(holder: Holder): string | undefined {
    if (holder.state.kind == "pending") {
        holder.state.tail = "a";
        holder.state.tail
    } else {
        undefined
    }
}

function disjoint(holder: Holder, other: Holder): string {
    if (holder.name != undefined) {
        other.count = 1;
        holder.name
    } else {
        ""
    }
}

function aliased(holder: Holder, other: Holder): string {
    if (holder.name != undefined) {
        other.name = undefined;
        holder.name
    } else {
        ""
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=stale-narrowing message="cannot change a place while a narrowing of it is used"
/// @diagnostic.label line=37 column=9 span="other.name = undefined" line_source="other.name = undefined;"
/// @diagnostic.related line=36 column=9 span="holder.name != undefined" line_source="if (holder.name != undefined) {" message="narrowed here"
"#,
    );
}

/// A write on a loop's back edge changes the place a narrowing from before the loop reads.
#[test]
fn test_reject_a_write_on_the_back_edge_under_a_narrowing() {
    let session = TestSession::single(
        r#"
declare function more(): boolean;

function count(): isize {
    let name: string | undefined = "a";
    let total: isize = 0;
    if (name != undefined) {
        while (more()) {
            total += name.length;
            name = undefined;
        }
    }
    total
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=stale-narrowing message="cannot change a place while a narrowing of it is used"
/// @diagnostic.label line=10 column=13 span="name = undefined" line_source="name = undefined;"
/// @diagnostic.related line=7 column=9 span="name != undefined" line_source="if (name != undefined) {" message="narrowed here"
"#,
    );
}

/// A write through a borrow taken before the test changes the narrowed binding.
#[test]
fn test_reject_a_write_through_an_earlier_borrow_under_a_narrowing() {
    let session = TestSession::single(
        r#"
export function stale(): isize {
    let name: string | undefined = "a";
    const alias = &name;
    if (name != undefined) {
        *alias = undefined;
        name.length
    } else {
        0
    }
}

export function kept(): isize {
    let name: string | undefined = "a";
    const alias = &name;
    *alias = undefined;
    if (name != undefined) {
        name.length
    } else {
        0
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=stale-narrowing message="cannot change a place while a narrowing of it is used"
/// @diagnostic.label line=6 column=9 span="*alias = undefined" line_source="*alias = undefined;"
/// @diagnostic.related line=5 column=9 span="name != undefined" line_source="if (name != undefined) {" message="narrowed here"
"#,
    );
}
