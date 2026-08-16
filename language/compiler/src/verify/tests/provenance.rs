use crate::tests::TestProgram;

#[test]
fn test_reject_selected_borrow_with_wrong_lifetime() {
    let mut program = TestProgram::mir(
        r#"
function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean):
    v3: ref<int32, borrowed, 'a, readonly> = select v2, v0, v1
    return v3
}
"#,
    );

    program.assert_verify_errors(r#"
error[borrow-outlives-origin]: borrow does not live long enough
 ──▶ <test.dsm>:5:5
  │
3 │ entry(v0: ref<int32, borrowed, 'a, readonly>, v1: ref<int32, borrowed, 'b, readonly>, v2: boolean):
4 │     v3: ref<int32, borrowed, 'a, readonly> = select v2, v0, v1
5 │     return v3
  │     ^^^^^^^^^
6 │ }
7 │
  │

for more information about an error, run `destack explain borrow-outlives-origin`
"#);
}

#[test]
fn test_allow_managed_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test<'a>(v0: ref<User, managed, 'a, mutable>, v1: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: ref<User, managed, 'a, mutable>, v1: boolean):
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    branch v1 => b1(v2) | b2(v2)

b1(v3: ref<int32, borrowed, 'a, readonly>):
    return v3

b2(v4: ref<int32, borrowed, 'a, readonly>):
    return v4
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_managed_slice_borrow_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: slice<int32, managed, 'a, mutable>, v1: boolean): ref<int32, borrowed, 'a, readonly> {
entry(v0: slice<int32, managed, 'a, mutable>, v1: boolean):
    v2: int64 = 0
    v3: ref<int32, borrowed, readonly> = element.address v0, v2
    branch v1 => b1(v3) | b2(v3)

b1(v4: ref<int32, borrowed, 'a, readonly>):
    return v4

b2(v5: ref<int32, borrowed, 'a, readonly>):
    return v5
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_carry_lifetime_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable>, v1: boolean): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: ref<int32, borrowed, 'a, mutable>):
    return v2

b2(v3: ref<int32, borrowed, 'a, mutable>):
    return v3
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_carry_aggregate_path_lifetimes_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>, v1: boolean): ref<int32, borrowed, 'b, readonly> {
entry(v0: Pair<'a, 'b>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: Pair<'a, 'b>):
    jump b3(v2)

b2(v3: Pair<'a, 'b>):
    jump b3(v3)

b3(v4: Pair<'a, 'b>):
    v5: ref<int32, borrowed, 'b, readonly> = field.get v4, 1
    return v5
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_allow_field_write_satisfying_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>): void where 'a: 'b {
entry(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>):
    v2: Pair<'a, 'b> = field.set v0, 1, v1
    return
}
"#,
    );

    program.assert_verified();
}

#[test]
fn test_reject_field_write_violating_declared_lifetime() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function test<'a, 'b>(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>): void {
entry(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>):
    v2: Pair<'a, 'b> = field.set v0, 1, v1
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:9:5
   │
 7 │ function test<'a, 'b>(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>): void {
 8 │ entry(v0: Pair<'a, 'b>, v1: ref<int32, borrowed, 'a, readonly>):
 9 │     v2: Pair<'a, 'b> = field.set v0, 1, v1
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}

#[test]
fn test_merge_lifetimes_at_join() {
    let mut program = TestProgram::mir(
        r#"
function test<'a, 'b>(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>, v2: boolean): ref<int32, borrowed, 'a, mutable> {
entry(v0: ref<int32, borrowed, 'a, mutable>, v1: ref<int32, borrowed, 'b, mutable>, v2: boolean):
    branch v2 => b1 | b2

b1:
    jump b3(v0)

b2:
    jump b3(v1)

b3(v3: ref<int32, borrowed, mutable>):
    return v3
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[borrow-outlives-origin]: borrow does not live long enough
  ──▶ <test.dsm>:13:5
   │
11 │
12 │ b3(v3: ref<int32, borrowed, mutable>):
13 │     return v3
   │     ^^^^^^^^^
14 │ }
15 │
   │

for more information about an error, run `destack explain borrow-outlives-origin`
"#,
    );
}
