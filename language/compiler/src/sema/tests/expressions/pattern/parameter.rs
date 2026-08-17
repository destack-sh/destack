use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_parameter_binds_fields() {
    let session = TestSession::single(
        r#"
function label({ name, age }: { name: string; age: int32 }): string {
    name satisfies string;
    age satisfies int32;

    name
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function label({ name, age }: { name: string; age: int32 }): string {
    name satisfies string;
    age satisfies int32;

    name
}

=== dir ===
function label({ name, age }: { name: string; age: int32 }): string {
/// @type.symbol symbol=label type=({ name: string; age: int32 }) => string
/// @resolution.pattern source={ name, age } kind=object fields={ name, age }
/// @type.symbol symbol=label.name#2 source=name type=string
/// @type.symbol symbol=label.age#2 source=age type=int32

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=label.name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=label.name#2

    age satisfies int32;
    /// @type.node source="age satisfies int32" type=int32
    /// @type.node source=age type=int32
    /// @resolution.name source=age target=label.age#2
    /// @resolution.place source=age placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=age root=label.age#2

    name
    /// @type.node source=name type=string
    /// @resolution.name source=name target=label.name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=label.name#2

}
"#,
    );
}

#[test]
fn test_sequence_pattern_parameter_binds_elements() {
    let session = TestSession::single(
        r#"
function first([head]: int32[]): int32 {
    head
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function first([head]: int32[]): int32 {
    head
}

=== dir ===
function first([head]: int32[]): int32 {
/// @type.symbol symbol=first type=(int32[]) => int32
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<int32>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<int32>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<int32>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<int32>)
/// @resolution.pattern source=[head] kind=sequence element=int32 arity=1 fields=(first.head)
/// @generic.instantiation id="collections.array.index#1<int32, \"exclusive\">" template=collections.array.index#1 arguments=(int32, "exclusive")
/// @generic.instance id="collections.array.index#1<int32, \"exclusive\">" template=collections.array.index#1 arguments=(int32, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame int32, \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame int32, "exclusive")
/// @generic.instance id="memory.type.WithAccess<&'frame int32[], \"exclusive\">" template=memory.type.WithAccess arguments=(&'frame int32[], "exclusive")
/// @type.symbol symbol=first.head source=head type=int32
/// @resolution.pattern source=head kind=binding target=first.head

    head
    /// @type.node source=head type=int32
    /// @resolution.name source=head target=first.head
    /// @resolution.place source=head placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=head root=first.head

}
"#,
    );
}
