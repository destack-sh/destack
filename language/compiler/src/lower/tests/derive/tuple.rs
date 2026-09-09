use crate::tests::TestSession;

/// Lower a derived tuple equality element-wise.
#[test]
fn test_lower_a_derived_tuple_equality_element_wise() {
    let session = TestSession::single(
        r#"
function same(left: &readonly (int32, boolean), right: &readonly (int32, boolean)): boolean {
    return left.equal(right);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.same", r#"
function test.main.same<'a, 'b>(v0: ref<(int32, boolean), borrowed, 'a, readonly, local>, v1: ref<(int32, boolean), borrowed, 'b, readonly, local>): boolean {
    local l0: ref<(int32, boolean), borrowed, 'a, readonly, local>
    local l1: ref<(int32, boolean), borrowed, 'b, readonly, local>

entry(v0: ref<(int32, boolean), borrowed, 'a, readonly, local>, v1: ref<(int32, boolean), borrowed, 'b, readonly, local>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<(int32, boolean), borrowed, 'a, readonly, local> = local.get l0
    v3: ref<(int32, boolean), borrowed, 'b, readonly, local> = local.get l1
    v4: boolean = call test.main.PartialEqual.equal<(int32, boolean)>(v2, v3): <'a, 'b>(ref<(int32, boolean), borrowed, 'a, readonly, local>, ref<(int32, boolean), borrowed, 'b, readonly, local>) => boolean
    return v4
}

/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.PartialEqual.equal<(int32, boolean)>",
        r#"
function test.main.PartialEqual.equal<(int32, boolean), 'a, 'b>(v0: ref<(int32, boolean), borrowed, 'a, readonly, local>, v1: ref<(int32, boolean), borrowed, 'b, readonly, local>): boolean {
    local l0: ref<(int32, boolean), borrowed, 'b, readonly, local>
    local l1: ref<(int32, boolean), borrowed, 'a, readonly, local>
    local l2: boolean

entry(v0: ref<(int32, boolean), borrowed, 'a, readonly, local>, v1: ref<(int32, boolean), borrowed, 'b, readonly, local>):
    local.set l0, v1
    local.set l1, v0
    v2: ref<(int32, boolean), borrowed, 'a, readonly, local> = local.get l1
    v3: ref<(int32, boolean), borrowed, 'b, readonly, local> = local.get l0
    v4: ref<int32, borrowed, 'b, readonly, local> = field.address v3, 0
    v5: ref<int32, borrowed, 'a, readonly, local> = field.address v2, 0
    v6: boolean = call Integer.PartialEqual.equal<int32>(v5, v4): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, ref<int32, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v6
    branch v6 => b1 | b2

b1:
    v7: ref<(int32, boolean), borrowed, 'a, readonly, local> = local.get l1
    v8: ref<(int32, boolean), borrowed, 'b, readonly, local> = local.get l0
    v9: ref<boolean, borrowed, 'b, readonly, local> = field.address v8, 1
    v10: ref<boolean, borrowed, 'a, readonly, local> = field.address v7, 1
    v11: boolean = call Boolean.PartialEqual.equal(v10, v9): <'a, 'b>(ref<boolean, borrowed, 'a, readonly, local>, ref<boolean, borrowed, 'b, readonly, local>) => boolean
    local.set l2, v11
    jump b2

b2:
    v12: boolean = local.get l2
    return v12
}

/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#,
    );
}
