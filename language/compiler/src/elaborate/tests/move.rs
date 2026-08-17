use crate::tests::TestProgram;

#[test]
fn test_drop_sibling_after_partial_move() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    return
}

function test(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    call consume(v1): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
    return
}

function test(v0: Pair): void {
entry(v0: Pair):
    v1: ref<int32, unique, mutable> = field.get v0, 0
    v2: ref<int32, unique, mutable> = field.get v0, 1
    free v2
    call consume(v1): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );
}

#[test]
fn test_drop_replaced_field_before_reconstruction() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair, v1: ref<int32, unique, mutable>): void {
entry(v0: Pair, v1: ref<int32, unique, mutable>):
    v2: Pair = field.set v0, 0, v1
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: Pair, v1: ref<int32, unique, mutable>): void {
entry(v0: Pair, v1: ref<int32, unique, mutable>):
    v3: ref<int32, unique, mutable> = field.get v0, 0
    free v3
    v2: Pair = field.set v0, 0, v1
    drop v2
    return
}

function drop.frame<Pair>(v0: ref<Pair, borrowed, exclusive, frame>): void {
entry(v0: ref<Pair, borrowed, exclusive, frame>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 1
    v2: ref<int32, unique, mutable> = load v1
    free v2
    v3: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v4: ref<int32, unique, mutable> = load v3
    free v4
    return
}
"#,
    );
}

#[test]
fn test_drop_path_dependent_local_before_join() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
    local l0: ref<int32, unique, mutable>

entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => initialize | skip

initialize:
    local.set l0, v0
    jump done

skip:
    jump done

done:
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
function test(v0: ref<int32, unique, mutable>, v1: boolean): void {
    local l0: ref<int32, unique, mutable>

entry(v0: ref<int32, unique, mutable>, v1: boolean):
    branch v1 => b1 | b2

b1:
    local.set l0, v0
    v2: ref<int32, unique, mutable> = local.get l0
    free v2
    jump b3

b2:
    free v0
    jump b3

b3:
    return
}
"#,
    );
}

#[test]
fn test_drop_edge_dependent_values_before_join() {
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
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
function consume(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    free v0
    return
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>, v2: boolean):
    branch v2 => b2(v0) | b1(v1)

b1(v5: ref<int32, unique, mutable>):
    free v0
    jump b3(v5)

b2(v4: ref<int32, unique, mutable>):
    free v1
    jump b3(v4)

b3(v3: ref<int32, unique, mutable>):
    call consume(v3): (ref<int32, unique, mutable>) => void
    return
}
"#,
    );
}

#[test]
fn test_retain_owner_after_borrow_escapes_through_alias() {
    let mut program = TestProgram::mir(
        r#"
type Owner {
    value: int32;
}

type View {
    value: ref<int32, borrowed, readonly, frame>;
}

function test(v0: ref<Owner, unique, mutable>, v1: ref<View, borrowed, mutable, frame>): void {
entry(v0: ref<Owner, unique, mutable>, v1: ref<View, borrowed, mutable, frame>):
    v2: ref<int32, borrowed, readonly, frame> = field.address v0, 0
    v3: ref<ref<int32, borrowed, readonly, frame>, borrowed, exclusive> = field.address v1, 0
    store v3, v2
    return
}
"#,
    );

    program.assert_elaborated(
        r#"
type Owner {
    value: int32;
}

type View {
    value: ref<int32, borrowed, readonly, frame>;
}

function test(v0: ref<Owner, unique, mutable>, v1: ref<View, borrowed, mutable, frame>): void {
entry(v0: ref<Owner, unique, mutable>, v1: ref<View, borrowed, mutable, frame>):
    v2: ref<int32, borrowed, readonly, frame> = field.address v0, 0
    v3: ref<ref<int32, borrowed, readonly, frame>, borrowed, exclusive> = field.address v1, 0
    store v3, v2
    free v0
    return
}
"#,
    );
}
