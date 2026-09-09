use crate::tests::TestSession;

/// Lower a derived newtype clone through its backing value.
#[test]
fn test_lower_a_derived_newtype_clone_through_its_backing() {
    let session = TestSession::single(
        r#"
newtype Steps = ^Array<int32>;

function duplicate(steps: &readonly Steps): Steps {
    return steps.clone();
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.duplicate", r#"
@copy
type test.main.Steps = newtype<Array<int32>>;

function test.main.duplicate<'a>(v0: ref<test.main.Steps, borrowed, 'a, readonly, local>): test.main.Steps {
    local l0: ref<test.main.Steps, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Steps, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Steps, borrowed, 'a, readonly, local> = local.get l0
    v2: test.main.Steps = call test.main.Clone.clone<test.main.Steps>(v1): <'a>(ref<test.main.Steps, borrowed, 'a, readonly, local>) => test.main.Steps
    return v2
}
"#);

    session.assert_mir_function("main.ds", "test.main.Clone.clone<test.main.Steps>", r#"
@languageItem("collections.Array")
type Array<T>;

@copy
type test.main.Steps = newtype<Array<int32>>;

function test.main.Clone.clone<test.main.Steps, 'a>(v0: ref<test.main.Steps, borrowed, 'a, readonly, local>): test.main.Steps {
    local l0: ref<test.main.Steps, borrowed, 'a, readonly, local>

entry(v0: ref<test.main.Steps, borrowed, 'a, readonly, local>):
    local.set l0, v0
    v1: ref<test.main.Steps, borrowed, 'a, readonly, local> = local.get l0
    v2: ref<Array<int32>, borrowed, 'a, readonly, local> = cast.bit v1 -> ref<Array<int32>, borrowed, 'a, readonly, local>
    v3: Array<int32> = call Array.Clone.clone<int32>(v2): <'a>(ref<Array<int32>, borrowed, 'a, readonly, local>) => Array<int32>
    v4: test.main.Steps = aggregate (v3)
    return v4
}
"#);
}
