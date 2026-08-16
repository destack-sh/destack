use crate::tests::TestProgram;

#[test]
fn test_reject_move_while_borrowed_through_block_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function consume(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    return
}

function test(v0: ref<Box, unique, mutable>, v1: boolean): void {
entry(v0: ref<Box, unique, mutable>, v1: boolean):
    v2: ref<int32, borrowed, mutable> = field.address v0, 0
    branch v1 => b1(v2) | b2

b1(v3: ref<int32, borrowed, mutable>):
    call consume(v0): (ref<Box, unique, mutable>) => void
    v4: int32 = load v3
    return

b2:
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[invalidation-of-borrowed-place]: cannot invalidate borrowed place
  ──▶ <test.dsm>:17:5
   │
11 │ function test(v0: ref<Box, unique, mutable>, v1: boolean): void {
12 │ entry(v0: ref<Box, unique, mutable>, v1: boolean):
13 │     v2: ref<int32, borrowed, mutable> = field.address v0, 0
   │     ------------------------------------------------------- borrow starts here
14 │     branch v1 => b1(v2) | b2
15 │
16 │ b1(v3: ref<int32, borrowed, mutable>):
17 │     call consume(v0): (ref<Box, unique, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
18 │     v4: int32 = load v3
19 │     return
   │

for more information about an error, run `destack explain invalidation-of-borrowed-place`
"#,
    );
}

#[test]
fn test_reject_use_after_call_move() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    call consume(v0): (ref<int32, unique, mutable>) => void
    v1: int32 = load v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:10:5
   │
 7 │ function test(v0: ref<int32, unique, mutable>): void {
 8 │ entry(v0: ref<int32, unique, mutable>):
 9 │     call consume(v0): (ref<int32, unique, mutable>) => void
   │     ------------------------------------------------------- value moved here
10 │     v1: int32 = load v0
   │     ^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

#[test]
fn test_reject_use_after_struct_move() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    v2: int32 = load v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:9:5
   │
 6 │ function test(v0: ref<int32, unique, mutable>): void {
 7 │ entry(v0: ref<int32, unique, mutable>):
 8 │     v1: Box = aggregate (v0)
   │     ------------------------ value moved here
 9 │     v2: int32 = load v0
   │     ^^^^^^^^^^^^^^^^^^^
10 │     return
11 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

#[test]
fn test_reject_use_after_new_complete() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function test(): void {
entry:
    v0: uninit<ref<Box, managed, mutable>> = new.uninit Box
    v1: ref<Box, managed, mutable> = new.complete v0
    v2: ref<Box, managed, mutable> = new.complete v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[use-after-move]: use of moved value
  ──▶ <test.dsm>:10:5
   │
 7 │ entry:
 8 │     v0: uninit<ref<Box, managed, mutable>> = new.uninit Box
 9 │     v1: ref<Box, managed, mutable> = new.complete v0
   │     ------------------------------------------------ value moved here
10 │     v2: ref<Box, managed, mutable> = new.complete v0
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     return
12 │ }
   │

for more information about an error, run `destack explain use-after-move`
"#,
    );
}

#[test]
fn test_reject_maybe_moved_after_join() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1 | b2

b1:
    call consume(v0): (ref<int32, unique, mutable>) => void
    jump b3

b2:
    jump b3

b3:
    v2: int32 = load v0
    return
}
"#,
    );

    program.assert_verify_errors(
        r#"
error[maybe-use-after-move]: value may have been moved
  ──▶ <test.dsm>:19:5
   │
10 │
11 │ b1:
12 │     call consume(v0): (ref<int32, unique, mutable>) => void
   │     ------------------------------------------------------- value moved on this path
13 │     jump b3
14 │
15 │ b2:
16 │     jump b3
17 │
18 │ b3:
19 │     v2: int32 = load v0
   │     ^^^^^^^^^^^^^^^^^^^
20 │     return
21 │ }
   │

for more information about an error, run `destack explain maybe-use-after-move`
"#,
    );
}

#[test]
fn test_reject_edge_dependent_move_after_join() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
    branch v2 => done(v0) | done(v1)

done(v3: ref<int32, unique, mutable>):
    call consume(v3): (ref<int32, unique, mutable>) => void
    call consume(v0): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_verify_errors(r#"
error[maybe-use-after-move]: value may have been moved
  ──▶ <test.dsm>:13:5
   │
 7 │ function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
 8 │ entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
 9 │     branch v2 => done(v0) | done(v1)
   │     -------------------------------- value moved on this path
10 │
11 │ done(v3: ref<int32, unique, mutable>):
12 │     call consume(v3): (ref<int32, unique, mutable>) => void
13 │     call consume(v0): (ref<int32, unique, mutable>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
14 │     return
15 │ }
   │

for more information about an error, run `destack explain maybe-use-after-move`
"#);
}
