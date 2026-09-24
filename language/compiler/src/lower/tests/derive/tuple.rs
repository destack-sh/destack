use crate::tests::TestSession;

/// Lower a derived tuple equality element-wise.
#[test]
fn test_lower_a_derived_tuple_equality_element_wise() {
    let session = TestSession::single(
        r#"
function same(left: &immutable (int32, boolean), right: &immutable (int32, boolean)): boolean {
    return left.equal(right);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.same", r#"
@nocopy
@languageItem("ops.PartialEqual")
type PartialEqual<T>;

function test.main.same<'a, 'b>(v0: ref<(int32, boolean), borrowed, 'a, immutable>, v1: ref<(int32, boolean), borrowed, 'b, immutable>): boolean {
    local l0: ref<(int32, boolean), borrowed, 'a, immutable>
    local l1: ref<(int32, boolean), borrowed, 'b, immutable>

entry(v0: ref<(int32, boolean), borrowed, 'a, immutable>, v1: ref<(int32, boolean), borrowed, 'b, immutable>):
    store l0, v0
    store l1, v1
    v2: ref<(int32, boolean), borrowed, 'a, immutable> = load l0
    v3: ref<(int32, boolean), borrowed, 'b, immutable> = load l1
    v4: boolean = call.witness (int32, boolean), PartialEqual<(int32, boolean)>, PartialEqual.equal(v2, v3): (ref<(int32, boolean), borrowed, 'a, immutable>, ref<(int32, boolean), borrowed, 'b, immutable>) => boolean
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
function test.main.PartialEqual.equal<(int32, boolean), 'a, 'b>(v0: ref<(int32, boolean), borrowed, 'a, immutable>, v1: ref<(int32, boolean), borrowed, 'b, immutable>): boolean {
    local l0: ref<(int32, boolean), borrowed, 'b, immutable>
    local l1: ref<(int32, boolean), borrowed, 'a, immutable>
    local l2: boolean

entry(v0: ref<(int32, boolean), borrowed, 'a, immutable>, v1: ref<(int32, boolean), borrowed, 'b, immutable>):
    store l0, v1
    store l1, v0
    v2: ref<(int32, boolean), borrowed, 'a, immutable> = load l1
    v3: ref<(int32, boolean), borrowed, 'b, immutable> = load l0
    v4: ref<int32, borrowed, 'b, immutable> = address (*v3).0
    v5: ref<int32, borrowed, 'a, immutable> = address (*v2).0
    v6: boolean = call Integer.PartialEqual.equal<int32>(v5, v4): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    store l2, v6
    branch v6 => b1 | b2

b1:
    v7: ref<(int32, boolean), borrowed, 'a, immutable> = load l1
    v8: ref<(int32, boolean), borrowed, 'b, immutable> = load l0
    v9: ref<boolean, borrowed, 'b, immutable> = address (*v8).1
    v10: ref<boolean, borrowed, 'a, immutable> = address (*v7).1
    v11: boolean = call Boolean.PartialEqual.equal(v10, v9): (ref<boolean, borrowed, 'a, immutable>, ref<boolean, borrowed, 'b, immutable>) => boolean
    store l2, v11
    jump b2

b2:
    v12: boolean = load l2
    return v12
}

/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#,
    );
}
