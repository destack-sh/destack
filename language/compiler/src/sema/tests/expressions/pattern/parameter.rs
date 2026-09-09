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
/// @type.symbol symbol=label.name#1 source="name: string" type=string
/// @type.symbol symbol=label.age#1 source="age: int32" type=int32

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=label.name#2
    /// @resolution.place source=name placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=name root=label.name#2

    age satisfies int32;
    /// @type.node source="age satisfies int32" type=int32
    /// @type.node source=age type=int32
    /// @resolution.name source=age target=label.age#2
    /// @resolution.place source=age placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=age root=label.age#2

    name
    /// @type.node source=name type=string
    /// @resolution.name source=name target=label.name#2
    /// @resolution.place source=name placement="local" lifetime="managed" access="mutable"
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
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @resolution.pattern source=[head] kind=sequence element=int32 arity=1 fields=(first.head)
/// @generic.instantiation id="index#1<int32, \"mutable\">" template=index#1 arguments=(int32, "mutable")
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="WithAccess<&'bound0 int32, \"mutable\">" template=WithAccess arguments=(&'bound0 int32, "mutable")
/// @generic.instance id="WithAccess<&'bound0 int32[], \"mutable\">" template=WithAccess arguments=(&'bound0 int32[], "mutable")
/// @generic.instance id="assumeInitReference<int32, \"mutable\">" template=assumeInitReference arguments=(int32, "mutable")
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="index#1<int32, \"mutable\">" template=index#1 arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=elementPosition<int32> template=elementPosition arguments=(int32)
/// @type.symbol symbol=first.head source=head type=int32
/// @resolution.pattern source=head kind=binding target=first.head

    head
    /// @type.node source=head type=int32
    /// @resolution.name source=head target=first.head
    /// @resolution.place source=head placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=head root=first.head

}
"#,
    );
}
