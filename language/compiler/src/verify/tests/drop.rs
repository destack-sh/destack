use crate::tests::TestProgram;

#[test]
fn test_reject_effectful_drop() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

@binding("test.park", { provider: "runtime", effect: "deterministic", park: true })
external function effectful(): void

function dropBox<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    call effectful(): () => void
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_verify_errors(
        r#"
error[drop-effect]: this drop may park
  ──▶ <test.dsm>:9:1
   │
 7 │ external function effectful(): void
 8 │
 9 │ function dropBox<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
   │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │ entry(v0: ref<Box, borrowed, 'a, mutable, local>):
   │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     call effectful(): () => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
12 │     return
   │     ^^^^^^
13 │ }
   │ ^
14 │
   │

 = help: move the parking work to an explicit dispose
for more information about an error, run `destack explain drop-effect`
"#,
    );
}

#[test]
fn test_allow_pure_drop() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

function dropBox<'a>(v0: ref<Box, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Box, borrowed, 'a, mutable, local>):
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_verify_errors(
        r#"
"#,
    );
}
