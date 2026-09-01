use crate::tests::{DirRows, TestSession};

#[test]
fn test_for_of_destructures_object_elements() {
    let session = TestSession::single(
        r#"
declare const items: { name: string; value: int32 }[];

for (const { name, value } of items) {
    name satisfies string;
    value satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const items: { name: string; value: int32 }[];

for (const { name, value } of items) {
    name satisfies string;
    value satisfies int32;
}

=== dir ===
declare const items: { name: string; value: int32 }[];
/// @type.symbol symbol=items source=items type={ name: string; value: int32 }[]
/// @resolution.pattern source=items kind=binding target=items
/// @generic.instance id="Array<{ name: string; value: int32 }>" template=Array arguments=({ name: string; value: int32 })
/// @generic.instance id="MaybeUninit<{ name: string; value: int32 }>" template=MaybeUninit arguments=({ name: string; value: int32 })
/// @generic.instance id="new<MaybeUninit<{ name: string; value: int32 }>>" template=new arguments=(MaybeUninit<{ name: string; value: int32 }>)
/// @type.symbol symbol=name#1 source="name: string" type=string
/// @type.symbol symbol=value#1 source="value: int32" type=int32

for (const { name, value } of items) {
/// @resolution.pattern source={ name, value } kind=object fields={ name, value }
/// @type.symbol symbol=name#2 source=name type=string
/// @type.symbol symbol=value#2 source=value type=int32
/// @type.node source=items type={ name: string; value: int32 }[]
/// @resolution.name source=items target=items
/// @resolution.access source=items root=items

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name#2

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=value root=value#2

}
"#,
    );
}
