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

    session.assert_mir_function("main.tspp", "test.main.same", r#"
@nocopy
@languageItem("ops.PartialEqual")
type PartialEqual<T>;

export function test.main.same<'a, 'b>(v0: ref<(int32, boolean), borrowed, 'a, immutable>, v1: ref<(int32, boolean), borrowed, 'b, immutable>): boolean {
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
        "main.tspp",
        "test.main.PartialEqual.equal<(int32, boolean)>",
        r#"
export function test.main.PartialEqual.equal<(int32, boolean), 'a, 'b>(v0: ref<(int32, boolean), borrowed, 'a, immutable>, v1: ref<(int32, boolean), borrowed, 'b, immutable>): boolean {
    local l0: ref<(int32, boolean), borrowed, 'b, immutable>
    local l1: ref<(int32, boolean), borrowed, 'a, immutable>
    local l2: boolean

entry(v0: ref<(int32, boolean), borrowed, 'a, immutable>, v1: ref<(int32, boolean), borrowed, 'b, immutable>):
    store l0, v1
    store l1, v0
    v2: ref<int32, borrowed, 'b, immutable> = address (*l0).0
    v3: ref<int32, borrowed, 'a, immutable> = address (*l1).0
    v4: boolean = call Integer.PartialEqual.equal<int32>(v3, v2): (ref<int32, borrowed, 'a, immutable>, ref<int32, borrowed, 'b, immutable>) => boolean
    store l2, v4
    branch v4 => b1 | b2

b1:
    v5: ref<boolean, borrowed, 'b, immutable> = address (*l0).1
    v6: ref<boolean, borrowed, 'a, immutable> = address (*l1).1
    v7: boolean = call Boolean.PartialEqual.equal(v6, v5): (ref<boolean, borrowed, 'a, immutable>, ref<boolean, borrowed, 'b, immutable>) => boolean
    store l2, v7
    jump b2

b2:
    v8: boolean = load l2
    return v8
}

/// @layout.tuple name=type@2 size=8 align=4
/// @layout.element owner=type@2 index=0 offset=0 size=4 align=4
/// @layout.element owner=type@2 index=1 offset=4 size=1 align=1
"#,
    );
}
