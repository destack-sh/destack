use crate::tests::TestProgram;

#[test]
fn test_insert_drop_after_last_owned_use() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique>): int32 {
b0(v0: ref<int32, unique>):
    v1: int32 = load v0
    return v1
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(value0: ref<int32, unique>): int32 {
entry0(value0: ref<int32, unique>):
    value1: int32 = load value0
    free value0
    drop value0
    return value1
}
"#,
    );
}

#[test]
fn test_insert_drop_before_later_unrelated_work() {
    let mut program = TestProgram::mir(
        r#"
function later(): void {
b0:
    return
}

function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    v1: int32 = load v0
    call later(): () -> void
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function later(): void {
entry0:
    return
}

function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    value1: int32 = load value0
    free value0
    drop value0
    call later(): () -> void
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_for_unused_owned_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    free value0
    drop value0
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_for_managed_parameter() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, managed>): void {
b0(v0: ref<int32, managed>):
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(value0: ref<int32, managed>): void {
entry0(value0: ref<int32, managed>):
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_for_managed_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
b0:
    v0: ref<int32, managed> = new int32
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(): void {
entry0:
    value0: ref<int32, managed> = new int32
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_for_managed_slice_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
b0:
    v0: int64 = 4int64
    v1: slice<int32, managed> = new.slice int32, v0
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(): void {
entry0:
    value0: int64 = 4int64
    value1: slice<int32, managed> = new.slice int32, value0
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_for_borrow_into_managed_field() {
    let mut program = TestProgram::mir(
        r#"
type User {
    int32;
}

function test(v0: ref<User, managed>): int32 {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_dropped_mir(
        r#"type User {
    int32;
}

function test(value0: ref<User, managed>): int32 {
entry0(value0: ref<User, managed>):
    value1: ref<int32, borrowed, readonly> = field.address value0, 0
    value2: int32 = load value1
    return value2
}
"#,
    );
}

#[test]
fn test_skip_drop_for_borrow_into_managed_slice() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: slice<int32, managed>): int32 {
b0(v0: slice<int32, managed>):
    v1: int64 = 0int64
    v2: ref<int32, borrowed, readonly> = element.address v0, v1
    v3: int32 = load v2
    return v3
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(value0: slice<int32, managed>): int32 {
entry0(value0: slice<int32, managed>):
    value1: int64 = 0int64
    value2: ref<int32, borrowed, readonly> = element.address value0, value1
    value3: int32 = load value2
    return value3
}
"#,
    );
}

#[test]
fn test_insert_drop_for_unique_slice_allocation() {
    let mut program = TestProgram::mir(
        r#"
function test(): void {
b0:
    v0: int64 = 4int64
    v1: slice<int32, unique> = new.slice int32, v0
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(): void {
entry0:
    value0: int64 = 4int64
    value1: slice<int32, unique> = new.slice int32, value0
    free value1
    drop value1
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_on_unconsumed_branch() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}

function test(v0: ref<int32, unique>, v1: boolean): void {
b0(v0: ref<int32, unique>, v1: boolean):
    branch v1, b1(v0), b2(v0)
b1(v2: ref<int32, unique>):
    call consume(v2): (ref<int32, unique>) -> void
    return
b2(v3: ref<int32, unique>):
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function consume(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    free value0
    drop value0
    return
}

function test(value0: ref<int32, unique>, value1: boolean): void {
entry0(value0: ref<int32, unique>, value1: boolean):
    branch value1, block1(value0), block2(value0)

block1(value2: ref<int32, unique>):
    call consume(value2): (ref<int32, unique>) -> void
    return

block2(value3: ref<int32, unique>):
    free value3
    drop value3
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_on_branch_path_without_owned_use() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}

function test(v0: ref<int32, unique>, v1: boolean): void {
b0(v0: ref<int32, unique>, v1: boolean):
    branch v1, b1, b2
b1:
    call consume(v0): (ref<int32, unique>) -> void
    return
b2:
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function consume(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    free value0
    drop value0
    return
}

function test(value0: ref<int32, unique>, value1: boolean): void {
entry0(value0: ref<int32, unique>, value1: boolean):
    branch value1, block1, block2

block1:
    call consume(value0): (ref<int32, unique>) -> void
    return

block2:
    free value0
    drop value0
    return
}
"#,
    );
}

#[test]
fn test_skip_drop_after_owned_return() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique>): ref<int32, unique> {
b0(v0: ref<int32, unique>):
    return v0
}"#,
    );

    program.assert_dropped_mir(
        r#"function test(value0: ref<int32, unique>): ref<int32, unique> {
entry0(value0: ref<int32, unique>):
    return value0
}
"#,
    );
}

#[test]
fn test_skip_drop_after_owned_call() {
    let mut program = TestProgram::mir(
        r#"
function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}

function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    call consume(v0): (ref<int32, unique>) -> void
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"function consume(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    free value0
    drop value0
    return
}

function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    call consume(value0): (ref<int32, unique>) -> void
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_for_owned_aggregate() {
    let mut program = TestProgram::mir(
        r#"
@moveOnly
type Box {
    ref<int32, unique>;
}

function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    v1: Box = struct Box (v0)
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"@moveOnly
type Box {
    ref<int32, unique>;
}

function test(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    value1: Box = struct Box (value0)
    drop value1
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_for_remaining_aggregate_field() {
    let mut program = TestProgram::mir(
        r#"
@moveOnly
type Pair {
    ref<int32, unique>;
    ref<int32, unique>;
}

function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}

function test(v0: ref<int32, unique>, v1: ref<int32, unique>): void {
b0(v0: ref<int32, unique>, v1: ref<int32, unique>):
    v2: Pair = struct Pair (v0, v1)
    v3: ref<int32, unique> = field.get v2, 0
    call consume(v3): (ref<int32, unique>) -> void
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"@moveOnly
type Pair {
    ref<int32, unique>;
    ref<int32, unique>;
}

function consume(value0: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>):
    free value0
    drop value0
    return
}

function test(value0: ref<int32, unique>, value1: ref<int32, unique>): void {
entry0(value0: ref<int32, unique>, value1: ref<int32, unique>):
    value2: Pair = struct Pair (value0, value1)
    value3: ref<int32, unique> = field.get value2, 0
    drop place(value2, field(1))
    call consume(value3): (ref<int32, unique>) -> void
    return
}
"#,
    );
}

#[test]
fn test_union_payload_move_consumes_union() {
    let mut program = TestProgram::mir(
        r#"
@moveOnly
	type Value = variant<uint8, ref<int32, unique>> { 0uint8 = ref<int32, unique>; 1uint8 = int32; };

function test(v0: Value): void {
b0(v0: Value):
    v1: ref<int32, unique> = field.get v0, 0
    return
}"#,
    );

    program.assert_dropped_mir(
        r#"@moveOnly
type Value = variant<uint8, ref<int32, unique>> { 0uint8 = ref<int32, unique>; 1uint8 = int32; };

function test(value0: Value): void {
entry0(value0: Value):
    value1: ref<int32, unique> = field.get value0, 0
    free value1
    drop value1
    return
}
"#,
    );
}
