use crate::tests::TestSession;

/// Lower a derived newtype clone through its backing value.
#[test]
fn test_lower_a_derived_newtype_clone_through_its_backing() {
    let session = TestSession::single(
        r#"
newtype Steps = ^Array<int32>;

function duplicate(steps: &immutable Steps): Steps {
    return steps.clone();
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.duplicate", r#"
type test.main.Steps = newtype<Array<int32>>;

@nocopy
@languageItem("memory.Clone")
type Clone;

function test.main.duplicate<'a>(v0: ref<test.main.Steps, borrowed, 'a, immutable>): test.main.Steps {
    local l0: ref<test.main.Steps, borrowed, 'a, immutable>

entry(v0: ref<test.main.Steps, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Steps, borrowed, 'a, immutable> = load l0
    v2: test.main.Steps = call.witness test.main.Steps, Clone, Clone.clone(v1): (ref<test.main.Steps, borrowed, 'a, immutable>) => test.main.Steps
    return v2
}
"#);

    session.assert_mir_function("main.ds", "test.main.Clone.clone<test.main.Steps>", r#"
type test.main.Steps = newtype<Array<int32>>;

@nocopy
@languageItem("collections.Array")
type Array<T>;

function test.main.Clone.clone<test.main.Steps, 'a>(v0: ref<test.main.Steps, borrowed, 'a, immutable>): test.main.Steps {
    local l0: ref<test.main.Steps, borrowed, 'a, immutable>

entry(v0: ref<test.main.Steps, borrowed, 'a, immutable>):
    store l0, v0
    v1: ref<test.main.Steps, borrowed, 'a, immutable> = load l0
    v2: ref<Array<int32>, borrowed, 'a, immutable> = address (*v1)
    v3: Array<int32> = call Array.Clone.clone<int32>(v2): (ref<Array<int32>, borrowed, 'a, immutable>) => Array<int32>
    v4: test.main.Steps = aggregate (v3)
    return v4
}
"#);
}
