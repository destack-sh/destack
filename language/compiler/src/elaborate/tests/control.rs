use crate::tests::TestProgram;

#[test]
fn test_insert_drop_on_unconsumed_branch() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: ref<int32, unique, mutable>):
    call consume(v2): (ref<int32, unique, mutable>) => void
    return

b2(v3: ref<int32, unique, mutable>):
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1(v0) | b2(v0)

b1(v2: ref<int32, unique, mutable>):
    call consume(v2): (ref<int32, unique, mutable>) => void
    return

b2(v3: ref<int32, unique, mutable>):
    release v3
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_on_branch_path_without_owned_use() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

external function observe(): void

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1 | b2

b1:
    call consume(v0): (ref<int32, unique, mutable>) => void
    return

b2:
    call observe(): () => void
    return
}
"#,
    );

    program.assert_optimized(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    release v0
    return
}

external function observe(): void

function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1 | b2

b1:
    call consume(v0): (ref<int32, unique, mutable>) => void
    return

b2:
    release v0
    call observe(): () => void
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_on_call_unwind_path() {
    let mut program = TestProgram::mir(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    invoke callee(): () => int32 => b1 | cleanup

b1(v1: int32):
    return v1

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_optimized(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    release v0
    invoke callee(): () => int32 => b1 | b2

b1(v1: int32):
    return v1

b2:
    unwind.resume
}
"#,
    );
}

#[test]
fn test_insert_drop_on_call_cleanup_when_value_remains_live() {
    let mut program = TestProgram::mir(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    invoke callee(): () => int32 => b1 | cleanup

b1(v1: int32):
    v2: int32 = load (*v0)
    return v2

cleanup:
    unwind.resume
}
"#,
    );

    program.assert_optimized(
        r#"
external function callee(): int32

function test(v0: ref<int32, unique, mutable>): int32 {
entry(v0: ref<int32, unique, mutable>):
    invoke callee(): () => int32 => b1 | b2

b1(v1: int32):
    v2: int32 = load (*v0)
    release v0
    return v2

b2:
    release v0
    unwind.resume
}
"#,
    );
}
