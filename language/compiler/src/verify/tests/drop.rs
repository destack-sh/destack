use crate::tests::TestProgram;

#[test]
fn test_reject_effectful_drop() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: int32;
}

external function effectful(): void

function dropBox(v0: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>):
    call effectful(): () => void
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_verify_errors(
        r#"
error[drop-effect]: this drop may allocate, park, and panic
  ──▶ <test.dsm>:8:1
   │
 6 │ external function effectful(): void
 7 │
 8 │ function dropBox(v0: ref<Box, borrowed, exclusive>): void {
   │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │ entry(v0: ref<Box, borrowed, exclusive>):
   │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     call effectful(): () => void
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │     return
   │     ^^^^^^
12 │ }
   │ ^
13 │
   │

 = help: move the effectful work to an explicit dispose
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

function dropBox(v0: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>):
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
