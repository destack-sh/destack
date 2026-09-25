use crate::tests::{TestProgram, TestSession};

/// A write to a local invalidates an exclusive borrow of it.
#[test]
fn test_reject_local_set_while_exclusively_borrowed() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32, v1: int32): void {
    local l0: int32

entry(v0: int32, v1: int32):
    store l0, v0
    v2: ref<int32, borrowed, 'frame, exclusive> = address l0
    store l0, v1
    v3: int32 = load (*v2)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:8:5
   │
 5 │ entry(v0: int32, v1: int32):
 6 │     store l0, v0
 7 │     v2: ref<int32, borrowed, 'frame, exclusive> = address l0
   │     -------------------------------------------------------- borrow starts here
 8 │     store l0, v1
   │     ^^^^^^^^^^^^
 9 │     v3: int32 = load (*v2)
10 │     return
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

/// A write to a local invalidates an immutable borrow an aggregate carries.
#[test]
fn test_reject_local_set_while_immutable_borrow_is_stored_in_aggregate() {
    let mut program = TestProgram::mir(
        r#"
type Holder<'a> {
    value: ref<int32, borrowed, 'a, immutable>;
}

function test(v0: int32, v1: int32): void {
    local l0: int32

entry(v0: int32, v1: int32):
    store l0, v0
    v2: ref<int32, borrowed, 'frame, immutable> = address l0
    v3: Holder<'frame> = aggregate (v2)
    store l0, v1
    v4: ref<int32, borrowed, 'frame, immutable> = field.get v3, 0
    v5: int32 = load (*v4)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:13:5
   │
 9 │ entry(v0: int32, v1: int32):
10 │     store l0, v0
11 │     v2: ref<int32, borrowed, 'frame, immutable> = address l0
   │     -------------------------------------------------------- borrow starts here
12 │     v3: Holder<'frame> = aggregate (v2)
13 │     store l0, v1
   │     ^^^^^^^^^^^^
14 │     v4: ref<int32, borrowed, 'frame, immutable> = field.get v3, 0
15 │     v5: int32 = load (*v4)
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

/// Replacing the borrow an aggregate holds frees its source to change.
#[test]
fn test_allow_change_after_aggregate_borrow_is_replaced() {
    let mut program = TestProgram::mir(
        r#"
type Holder<'a> {
    value: ref<int32, borrowed, 'a, readonly>;
}

function test(v0: int32, v1: int32, v2: int32): void {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32, v2: int32):
    store l0, v0
    store l1, v1
    v3: ref<int32, borrowed, 'frame, readonly> = address l0
    v4: Holder<'frame> = aggregate (v3)
    v5: ref<int32, borrowed, 'frame, readonly> = address l1
    v6: Holder<'frame> = field.set v4, 0, v5
    store l0, v2
    v7: ref<int32, borrowed, 'frame, readonly> = field.get v6, 0
    v8: int32 = load (*v7)
    return
}
"#,
    );

    program.assert_verified();
}

/// A store through a readonly borrow is rejected.
#[test]
fn test_reject_store_through_readonly_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<int32, borrowed, 'a, readonly>, v1: int32): void {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: int32):
    store (*v0), v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[write-through-readonly-reference]: invalid MIR: cannot write through readonly reference
 ──▶ <test.dsm>:4:5
  │
2 │ function test<'a>(v0: ref<int32, borrowed, 'a, readonly>, v1: int32): void {
3 │ entry(v0: ref<int32, borrowed, 'a, readonly>, v1: int32):
4 │     store (*v0), v1
  │     ^^^^^^^^^^^^^^^
5 │     return
6 │ }
  │

for more information about an error, run `destack explain write-through-readonly-reference`
"#,
    );
}

/// Allow a Copy store over a field an aliasable loan through a parameter borrows.
#[test]
fn test_allow_a_copy_store_over_a_borrowed_field_through_a_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}
function test<'a>(v0: ref<Box, borrowed, 'a, mutable>, v1: int32): void {
entry(v0: ref<Box, borrowed, 'a, mutable>, v1: int32):
    v2: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v3: Box = aggregate (v1)
    store (*v0), v3
    v4: int32 = load (*v2)
    return
}
"#,
    );

    program.assert_verified();
}

/// A store through a mutable borrow of a field is allowed.
#[test]
fn test_allow_store_through_own_mutable_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>, v1: int32): void {
entry(v0: ref<Box, borrowed, 'a, mutable>, v1: int32):
    v2: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    store (*v2), v1
    v3: int32 = load (*v2)
    return
}
"#,
    );

    program.assert_verified();
}

/// A parameter may be read while a field reborrowed through it is live, the storage aliasing.
#[test]
fn test_allow_a_load_through_a_parameter_during_its_reborrow() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, mutable>): int32 {
entry(v0: ref<Box, borrowed, 'a, mutable>):
    v1: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    v2: Box = load (*v0)
    v3: int32 = load (*v1)
    return v3
}
"#,
    );

    program.assert_verified();
}

/// A store through a mutable borrow leaves that borrow live.
#[test]
fn test_allow_store_through_live_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable>, v1: int32): void {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: int32):
    store (*v0), v1
    v2: int32 = load (*v0)
    return
}
"#,
    );

    program.assert_verified();
}

/// A mutable borrow through a readonly reference is rejected.
#[test]
fn test_reject_writable_borrow_through_readonly_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test<'a>(v0: ref<Box, borrowed, 'a, readonly>): void {
entry(v0: ref<Box, borrowed, 'a, readonly>):
    v1: ref<int32, borrowed, 'a, mutable> = address (*v0).0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:8:5
   │
 6 │ function test<'a>(v0: ref<Box, borrowed, 'a, readonly>): void {
 7 │ entry(v0: ref<Box, borrowed, 'a, readonly>):
 8 │     v1: ref<int32, borrowed, 'a, mutable> = address (*v0).0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}

/// A read of a local is rejected while an exclusive borrow of it lives.
#[test]
fn test_reject_local_read_during_exclusive_borrow() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local l0: int32
entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'frame, exclusive> = address l0
    v2: int32 = load l0
    v3: int32 = load (*v1)
    v4: int32 = add v2, v3
    return v4
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-of-exclusively-borrowed-place]: cannot use exclusively borrowed place
 ──▶ <test.dsm>:7:5
  │
4 │ entry(v0: int32):
5 │     store l0, v0
6 │     v1: ref<int32, borrowed, 'frame, exclusive> = address l0
  │     -------------------------------------------------------- borrow starts here
7 │     v2: int32 = load l0
  │     ^^^^^^^^^^^^^^^^^^^
8 │     v3: int32 = load (*v1)
9 │     v4: int32 = add v2, v3
  │

for more information about an error, run `destack explain use-of-exclusively-borrowed-place`
"#,
    );
}

/// An atomic store through a readonly reference is rejected.
#[test]
fn test_reject_atomic_store_through_readonly_reference() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<uint32, borrowed, 'a, readonly>, v1: uint32): void {
entry(v0: ref<uint32, borrowed, 'a, readonly>, v1: uint32):
    atomic.store (*v0), v1, release, scope(device)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[write-through-readonly-reference]: invalid MIR: cannot write through readonly reference
 ──▶ <test.dsm>:4:5
  │
2 │ function test<'a>(v0: ref<uint32, borrowed, 'a, readonly>, v1: uint32): void {
3 │ entry(v0: ref<uint32, borrowed, 'a, readonly>, v1: uint32):
4 │     atomic.store (*v0), v1, release, scope(device)
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │     return
6 │ }
  │

for more information about an error, run `destack explain write-through-readonly-reference`
"#,
    );
}

/// Nested fields of an owned value take assignments.
#[test]
fn test_allow_assignments_to_nested_fields() {
    let session = TestSession::single(
        r#"
struct B {
    a: int32;
}

struct A {
    a: int32;
    w: B;
}

export function main(): void {
    let p = A { a: 1, w: B { a: 1 } };
    p.a = 2;
    p.w.a = 2;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// Allow a Copy write under a live aliasable loan inside a loop.
#[test]
fn test_allow_a_copy_write_under_a_live_aliasable_loan_inside_a_loop() {
    let session = TestSession::single(
        r#"
struct Record {
    field: int32;
}

function length(value: &readonly int32): int32 {
    return 0;
}

export function readAfterWrite(): void {
    let record = Record { field: 1 };
    const value = &record.field;
    loop {
        record.field += 1;
        length(value);
        return;
    }
}

export function writeOnly(): void {
    let record = Record { field: 1 };
    const value = &record.field;
    loop {
        record.field += 1;
        return;
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"

"#,
    );
}

/// Allow a Copy write under a readonly loan taken through a reassigned reference.
#[test]
fn test_allow_a_copy_write_under_a_readonly_loan_through_a_reassigned_reference() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&this): void {}
}

struct Pair {
    a: int32;
    b: Token;
}

function read(value: &readonly int32): void {}

export function write(flag: boolean): void {
    let left = Pair { a: 1, b: Token { id: 2 } };
    let right = Pair { a: 3, b: Token { id: 4 } };
    let target = &left;
    if (flag) {
        target = &right;
    }
    const borrowed = &readonly target.a;
    target.a = 5;
    read(borrowed);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// A borrow of a field after its whole value moved is a use after move.
#[test]
fn test_reject_a_field_borrow_after_a_whole_move() {
    let session = TestSession::single(
        r#"
struct Token implements Drop {
    id: int32;

    drop(&this): void {}
}

struct Pair {
    a: int32;
    b: Token;
}

function read(value: &readonly int32): void {}

function consume(pair: Pair): void {}

export function borrowFieldAfterWholeMove(): void {
    const pair = Pair { a: 1, b: Token { id: 2 } };
    consume(pair);
    read(&readonly pair.a);
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
/// @diagnostic.error id=use-after-move message="use of moved value"
/// @diagnostic.label line=20 column=10 span="&readonly pair.a" line_source="read(&readonly pair.a);"
/// @diagnostic.related line=19 column=13 span="pair" line_source="consume(pair);" message="value moved here"
"#,
    );
}

/// Exclusive borrows into the payloads of two different cases are disjoint.
#[test]
fn test_allow_exclusive_borrows_into_different_variant_cases() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

function test(v0: int32, v1: int64, v6: Either): void {
    local l0: Either

entry(v0: int32, v1: int64, v6: Either):
    store l0, v6
    v2: ref<Either, borrowed, 'frame, exclusive> = address l0
    v3: ref<int32, borrowed, 'frame, exclusive> = address ((*v2) as 0)
    v4: ref<int64, borrowed, 'frame, exclusive> = address ((*v2) as 1)
    store (*v3), v0
    store (*v4), v1
    v5: int32 = load (*v3)
    return
}
"#,
    );

    program.assert_verified();
}

/// Two exclusive borrows into one case's payload conflict.
#[test]
fn test_reject_exclusive_borrows_into_one_variant_case() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

function test(v0: int32, v1: int32, v6: Either): void {
    local l0: Either

entry(v0: int32, v1: int32, v6: Either):
    store l0, v6
    v2: ref<Either, borrowed, 'frame, exclusive> = address l0
    v3: ref<int32, borrowed, 'frame, exclusive> = address ((*v2) as 0)
    v4: ref<int32, borrowed, 'frame, exclusive> = address ((*v2) as 0)
    store (*v3), v0
    store (*v4), v1
    v5: int32 = load (*v3)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-conflict]: borrow conflicts with active borrow
  ──▶ <test.dsm>:11:5
   │
 8 │     store l0, v6
 9 │     v2: ref<Either, borrowed, 'frame, exclusive> = address l0
10 │     v3: ref<int32, borrowed, 'frame, exclusive> = address ((*v2) as 0)
   │     ------------------------------------------------------------------ borrow starts here
11 │     v4: ref<int32, borrowed, 'frame, exclusive> = address ((*v2) as 0)
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
12 │     store (*v3), v0
13 │     store (*v4), v1
   │

for more information about an error, run `destack explain borrow-conflict`
"#,
    );
}

/// Read a field of a managed object through a readonly view of its handle.
#[test]
fn test_read_a_field_of_a_handle_through_a_readonly_view() {
    let session = TestSession::single(
        r#"
newtype HostError = {
    code: int32;
    message?: string;
};

extension of HostError {
    display(this: &readonly this): ^string {
        if (this.message == undefined) {
            "host error"
        } else {
            this.message.clone()
        }
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// Grant immutable and exclusive borrows of a class object through its handle if all copy.
#[test]
fn test_borrow_a_class_object_through_its_handle_on_every_rung() {
    let session = TestSession::single(
        r#"
class User {
    name: int32 = 0;
}

function view(user: User): void {
    const frozen = &immutable user;
    const owned = &exclusive user;
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds", r#"
"#,
    );
}

/// A field read through an immutable borrow of a class union newtype keeps immutable access.
#[test]
fn test_read_a_field_through_an_immutable_borrow_of_a_class_union_newtype() {
    let session = TestSession::single(
        r#"
newtype Alpha = {
    name: "Alpha";
    message: string;
};

newtype Beta = {
    name: "Beta";
    message: string;
};

newtype Failure = Alpha | Beta;

extension of Failure {
    display(this: &immutable this): ^string {
        this.message.clone()
    }
}
"#,
    );

    session.assert_mir_lowered("main.ds", r#"
type test.main.Alpha = newtype<{ name: literal.string.Alpha, message: ref<String, managed, mutable, local> }>;

type literal.string.Alpha { }

@nocopy
@languageItem("string.String")
type String {
    codeUnits: slice<uint16, unique, mutable>;
}

type test.main.Beta = newtype<{ name: literal.string.Beta, message: ref<String, managed, mutable, local> }>;

type literal.string.Beta { }

type test.main.Failure = newtype<variant<uint1> { 0uint1 = ref<test.main.Alpha, managed, mutable, local>; 1uint1 = ref<test.main.Beta, managed, mutable, local>; }>;

function test.main.Failure.display<'a>(v0: ref<test.main.Failure, borrowed, 'a, immutable>): String {
    local l0: ref<test.main.Failure, borrowed, 'a, immutable>
    local l1: ref<ref<String, managed, mutable, local>, borrowed, 'a, immutable>, readonly

entry(v0: ref<test.main.Failure, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Failure, borrowed, 'a, immutable> = load l0
    v2: ref<variant<uint1> { 0uint1 = ref<test.main.Alpha, managed, mutable, local>; 1uint1 = ref<test.main.Beta, managed, mutable, local>; }, borrowed, 'a, immutable> = address (*v1).0
    v3: uint1 = variant.tag.load (*v2)
    switch v3, b3, 0 => b1, 1 => b2

b1:
    v4: ref<ref<test.main.Alpha, managed, mutable, local>, borrowed, 'a, immutable> = address ((*v2) as 0)
    v5: ref<test.main.Alpha, managed, mutable, local> = load (*v4)
    v6: ref<test.main.Alpha, borrowed, 'managed, immutable> = cast.bit v5 -> ref<test.main.Alpha, borrowed, 'managed, immutable>
    v7: ref<test.main.Alpha, borrowed, 'a, immutable> = address (*v6)
    v8: ref<{ name: literal.string.Alpha, message: ref<String, managed, mutable, local> }, borrowed, 'a, immutable> = address (*v7).0
    v9: ref<ref<String, managed, mutable, local>, borrowed, 'a, immutable> = address (*v8).1
    store l1, v9
    jump b4

b2:
    v10: ref<ref<test.main.Beta, managed, mutable, local>, borrowed, 'a, immutable> = address ((*v2) as 1)
    v11: ref<test.main.Beta, managed, mutable, local> = load (*v10)
    v12: ref<test.main.Beta, borrowed, 'managed, immutable> = cast.bit v11 -> ref<test.main.Beta, borrowed, 'managed, immutable>
    v13: ref<test.main.Beta, borrowed, 'a, immutable> = address (*v12)
    v14: ref<{ name: literal.string.Beta, message: ref<String, managed, mutable, local> }, borrowed, 'a, immutable> = address (*v13).0
    v15: ref<ref<String, managed, mutable, local>, borrowed, 'a, immutable> = address (*v14).1
    store l1, v15
    jump b4

b3:
    unreachable

b4:
    v16: ref<ref<String, managed, mutable, local>, borrowed, 'a, immutable> = load l1
    v17: ref<String, managed, mutable, local> = load (*v16)
    v18: ref<String, borrowed, 'a, immutable> = cast.bit v17 -> ref<String, borrowed, 'a, immutable>
    v19: String = call String.Clone.clone(v18): (ref<String, borrowed, 'a, immutable>) => String
    return v19
}

external function String.Clone.clone<'a>(ref<String, borrowed, 'a, immutable>): String

/// @layout.struct name=literal.string.Alpha size=0 align=1
/// @layout.struct name=String size=16 align=8
/// @layout.field owner=String index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=literal.string.Beta size=0 align=1
/// @layout.struct name=type@3 size=0 align=1
/// @layout.struct name=type@8 size=16 align=8
/// @layout.field owner=type@8 index=0 name=codeUnits offset=0 size=16 align=8
/// @layout.struct name=type@9 size=8 align=8
/// @layout.field owner=type@9 index=0 name=name offset=8 size=0 align=1
/// @layout.field owner=type@9 index=1 name=message offset=0 size=8 align=8
/// @layout.struct name=type@14 size=8 align=8
/// @layout.field owner=type@14 index=0 name=name offset=8 size=0 align=1
/// @layout.field owner=type@14 index=1 name=message offset=0 size=8 align=8
/// @layout.variant name=type@18 size=16 align=8
/// @layout.discriminant owner=type@18 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@18 index=0 discriminant=0 payload_offset=8
/// @layout.case owner=type@18 index=1 discriminant=1 payload_offset=8
"#);
    session.assert_mir_verified_diagnostics(
        "main.ds", r#"

"#,
    );
}

/// A readonly handle grants no exclusive borrow of its object.
#[test]
fn test_reject_an_exclusive_borrow_cast_from_a_readonly_handle() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, readonly, local>): void {
entry(v0: ref<Box, managed, readonly, local>):
    v1: ref<Box, borrowed, 'managed, exclusive> = cast.bit v0 -> ref<Box, borrowed, 'managed, exclusive>
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: ref<Box, managed, readonly, local>): void {
 7 │ entry(v0: ref<Box, managed, readonly, local>):
 8 │ ··v1: ref<Box, borrowed, 'managed, exclusive> = cast.bit v0 -> ref<Box, borrowed, 'managed, exclusive>
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}

/// A handle grants no immutable borrow of an object holding an inline variant of two cases.
#[test]
fn test_reject_an_immutable_borrow_cast_from_a_handle_over_an_inline_variant_of_several_cases() {
    let mut program = TestProgram::mir(
        r#"
type Slot = variant<uint1> { 0uint1 = int32; 1uint1 = ref<int32, unique, mutable>; };

type Box {
    slot: Slot;
}

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<Box, borrowed, 'managed, immutable> = cast.bit v0 -> ref<Box, borrowed, 'managed, immutable>
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:10:5
   │
 8 │ function test(v0: ref<Box, managed, mutable, local>): void {
 9 │ entry(v0: ref<Box, managed, mutable, local>):
10 │ ··v1: ref<Box, borrowed, 'managed, immutable> = cast.bit v0 -> ref<Box, borrowed, 'managed, immutable>
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}

/// A handle grants no exclusive borrow of an object holding a move-only component.
#[test]
fn test_reject_an_exclusive_borrow_cast_from_a_handle_over_a_move_only_component() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<Box, borrowed, 'managed, exclusive> = cast.bit v0 -> ref<Box, borrowed, 'managed, exclusive>
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: ref<Box, managed, mutable, local>): void {
 7 │ entry(v0: ref<Box, managed, mutable, local>):
 8 │ ··v1: ref<Box, borrowed, 'managed, exclusive> = cast.bit v0 -> ref<Box, borrowed, 'managed, exclusive>
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}

/// A mutable handle grants an exclusive borrow of an object whose every component copies.
#[test]
fn test_allow_an_exclusive_borrow_cast_from_a_handle_over_copy_components() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<Box, borrowed, 'managed, exclusive> = cast.bit v0 -> ref<Box, borrowed, 'managed, exclusive>
    return
}
"#,
    );

    program.assert_verified();
}

/// A handle grants an immutable borrow of a field of an object holding no inline variant.
#[test]
fn test_allow_an_immutable_field_borrow_through_a_handle_over_no_inline_variant() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, borrowed, 'managed, immutable> = address (*v0).0
    return
}
"#,
    );

    program.assert_verified();
}

/// A managed interior address through a readonly handle cannot grant mutable access.
#[test]
fn test_reject_a_mutable_managed_address_through_a_readonly_handle() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, managed, readonly, local>): void {
entry(v0: ref<Box, managed, readonly, local>):
    v1: ref<int32, managed, mutable, local> = address (*v0).0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: ref<Box, managed, readonly, local>): void {
 7 │ entry(v0: ref<Box, managed, readonly, local>):
 8 │     v1: ref<int32, managed, mutable, local> = address (*v0).0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}

/// A managed interior address cannot select a payload an alias may retag.
#[test]
fn test_reject_a_managed_address_of_a_variant_payload() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

type Box {
    value: Either;
}

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<int32, managed, mutable, local> = address (((*v0).0) as 0)
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-of-aliasable-variant]: cannot borrow an inline variant payload through aliasable access
  ──▶ <test.dsm>:10:5
   │
 8 │ function test(v0: ref<Box, managed, mutable, local>): void {
 9 │ entry(v0: ref<Box, managed, mutable, local>):
10 │     v1: ref<int32, managed, mutable, local> = address (((*v0).0) as 0)
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `destack explain borrow-of-aliasable-variant`
"#,
    );
}

/// A fresh handle has no alias, so it grants a borrow of a variant payload in its object.
#[test]
fn test_allow_a_payload_borrow_through_a_fresh_handle() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

type Box {
    value: Either;
}

function test(): int32 {
entry:
    v0: ref<Box, managed, mutable, local> = new.zeroed Box, local
    v1: ref<int32, borrowed, 'frame, readonly> = address (((*v0).0) as 0)
    v2: int32 = load (*v1)
    return v2
}
"#,
    );

    program.assert_verified();
}

/// A case change through a fresh handle invalidates a live borrow of the selected payload.
#[test]
fn test_reject_a_case_change_through_a_fresh_handle_under_a_payload_borrow() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

type Box {
    value: Either;
}

function test(v0: Either): int32 {
entry(v0: Either):
    v1: ref<Box, managed, mutable, local> = new.zeroed Box, local
    v2: ref<int32, borrowed, 'frame, readonly> = address (((*v1).0) as 0)
    store (*v1).0, v0
    v3: int32 = load (*v2)
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:12:5
   │
 9 │ entry(v0: Either):
10 │     v1: ref<Box, managed, mutable, local> = new.zeroed Box, local
11 │     v2: ref<int32, borrowed, 'frame, readonly> = address (((*v1).0) as 0)
   │     --------------------------------------------------------------------- borrow starts here
12 │     store (*v1).0, v0
   │     ^^^^^^^^^^^^^^^^^
13 │     v3: int32 = load (*v2)
14 │     return v3
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

/// A handle grants no immutable borrow of an object holding an all-Copy variant of two cases.
#[test]
fn test_reject_an_immutable_borrow_cast_from_a_handle_over_a_copy_variant() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

type Box {
    value: Either;
}

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<Box, borrowed, 'managed, immutable> = cast.bit v0 -> ref<Box, borrowed, 'managed, immutable>
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:10:5
   │
 8 │ function test(v0: ref<Box, managed, mutable, local>): void {
 9 │ entry(v0: ref<Box, managed, mutable, local>):
10 │ ··v1: ref<Box, borrowed, 'managed, immutable> = cast.bit v0 -> ref<Box, borrowed, 'managed, immutable>
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}

/// A handle grants no immutable borrow of a single-case variant wrapping a variant of two cases.
#[test]
fn test_reject_an_immutable_borrow_cast_from_a_handle_over_a_nested_variant() {
    let mut program = TestProgram::mir(
        r#"
type Either = variant<uint1> { 0uint1 = int32; 1uint1 = int64; };

type Single = variant<uint1> { 0uint1 = Either; };

type Box {
    value: Single;
}

function test(v0: ref<Box, managed, mutable, local>): void {
entry(v0: ref<Box, managed, mutable, local>):
    v1: ref<Box, borrowed, 'managed, immutable> = cast.bit v0 -> ref<Box, borrowed, 'managed, immutable>
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:12:5
   │
10 │ function test(v0: ref<Box, managed, mutable, local>): void {
11 │ entry(v0: ref<Box, managed, mutable, local>):
12 │ ··v1: ref<Box, borrowed, 'managed, immutable> = cast.bit v0 -> ref<Box, borrowed, 'managed, immutable>
   │   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
13 │     return
14 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}

/// A readonly unique reference cannot be cast to a mutable borrow.
#[test]
fn test_reject_a_mutable_borrow_cast_from_a_readonly_unique_reference() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(v0: ref<Box, unique, readonly>): void {
entry(v0: ref<Box, unique, readonly>):
    v1: ref<Box, borrowed, 'frame, mutable> = cast.bit v0 -> ref<Box, borrowed, 'frame, mutable>
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-access-strengthening]: cannot strengthen borrowed access
  ──▶ <test.dsm>:8:5
   │
 6 │ function test(v0: ref<Box, unique, readonly>): void {
 7 │ entry(v0: ref<Box, unique, readonly>):
 8 │     v1: ref<Box, borrowed, 'frame, mutable> = cast.bit v0 -> ref<Box, borrowed, 'frame, mutable>
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return
10 │ }
   │

for more information about an error, run `destack explain borrow-access-strengthening`
"#,
    );
}
