use crate::tests::TestProgram;

#[test]
fn test_reject_managed_borrow_across_park() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, managed, mutable>): int32 {
entry(v0: ref<Box, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    call park(): () => void
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_verify_errors(r#"
error[managed-borrow-across-park]: borrow of managed storage cannot remain live while this call parks
  ──▶ <test.dsm>:13:5
   │
10 │ function test(v0: ref<Box, managed, mutable>): int32 {
11 │ entry(v0: ref<Box, managed, mutable>):
12 │     v1: ref<int32, borrowed, readonly> = field.address v0, 0
   │     -------------------------------------------------------- borrow starts here
13 │     call park(): () => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^
14 │     v2: int32 = load v1
15 │     return v2
   │

for more information about an error, run `destack explain managed-borrow-across-park`
"#);
}

#[test]
fn test_reject_managed_borrow_passed_to_park() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(ref<int32, borrowed, readonly>): void

function test(v0: ref<Box, managed, mutable>): void {
entry(v0: ref<Box, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    call park(v1): (ref<int32, borrowed, readonly>) => void
    return
}
"#,
    );

    program.assert_verify_errors(r#"
error[managed-borrow-across-park]: borrow of managed storage cannot remain live while this call parks
  ──▶ <test.dsm>:13:5
   │
10 │ function test(v0: ref<Box, managed, mutable>): void {
11 │ entry(v0: ref<Box, managed, mutable>):
12 │     v1: ref<int32, borrowed, readonly> = field.address v0, 0
   │     -------------------------------------------------------- borrow starts here
13 │     call park(v1): (ref<int32, borrowed, readonly>) => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
14 │     return
15 │ }
   │

for more information about an error, run `destack explain managed-borrow-across-park`
"#);
}

#[test]
fn test_reject_managed_borrow_across_transitive_park() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function helper(): void {
entry():
    call park(): () => void
    return
}

function test(v0: ref<Box, managed, mutable>): int32 {
entry(v0: ref<Box, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    call helper(): () => void
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_verify_errors(r#"
error[managed-borrow-across-park]: borrow of managed storage cannot remain live while this call parks
  ──▶ <test.dsm>:19:5
   │
16 │ function test(v0: ref<Box, managed, mutable>): int32 {
17 │ entry(v0: ref<Box, managed, mutable>):
18 │     v1: ref<int32, borrowed, readonly> = field.address v0, 0
   │     -------------------------------------------------------- borrow starts here
19 │     call helper(): () => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^
20 │     v2: int32 = load v1
21 │     return v2
   │

for more information about an error, run `destack explain managed-borrow-across-park`
"#);
}

#[test]
fn test_reject_managed_borrow_across_parking_invoke() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): int32

function test(v0: ref<Box, managed, mutable>): int32 {
entry(v0: ref<Box, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    invoke park(): () => int32 => resume | cleanup

resume(v2: int32):
    v3: int32 = load v1
    return v3

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_verify_errors(r#"
error[managed-borrow-across-park]: borrow of managed storage cannot remain live while this call parks
  ──▶ <test.dsm>:13:5
   │
10 │ function test(v0: ref<Box, managed, mutable>): int32 {
11 │ entry(v0: ref<Box, managed, mutable>):
12 │     v1: ref<int32, borrowed, readonly> = field.address v0, 0
   │     -------------------------------------------------------- borrow starts here
13 │     invoke park(): () => int32 => resume | cleanup
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
14 │
15 │ resume(v2: int32):
   │

for more information about an error, run `destack explain managed-borrow-across-park`
"#);
}

#[test]
fn test_allow_unique_borrow_across_park() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function park(): void

function test(v0: ref<Box, unique, mutable>): int32 {
entry(v0: ref<Box, unique, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    call park(): () => void
    v2: int32 = load v1
    return v2
}
"#,
    );

    program.assert_verified();
}
