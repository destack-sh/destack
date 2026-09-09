use crate::tests::{DirRows, TestSession};

#[test]
fn test_assign_a_tuple_to_a_target_with_a_trailing_optional_element() {
    let session = TestSession::single(
        r#"
declare const pair: (int32, int32);
const triple: (int32, int32, int32?) = pair;
const same: (int32, int32) = pair;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare const pair: (int32, int32);
const triple: (int32, int32, int32?) = pair as (int32, int32, int32?);
const same: (int32, int32) = pair;

=== dir ===
declare const pair: (int32, int32);
/// @type.symbol symbol=pair source=pair type=(int32, int32)
/// @resolution.pattern source=pair kind=binding target=pair

const triple: (int32, int32, int32?) = pair;
/// @type.symbol symbol=triple source=triple type=(int32, int32, int32?)
/// @resolution.pattern source=triple kind=binding target=triple
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=pair root=pair
/// @coercion.node source=pair from=(int32, int32) adjustments=[{ kind: tuple, target: (int32, int32, int32?) }] origin=implicit

const same: (int32, int32) = pair;
/// @type.symbol symbol=same source=same type=(int32, int32)
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=pair root=pair
"#,
    );
}

#[test]
fn test_tuple_subscript_selects_literal_element() {
    let session = TestSession::single(
        r#"
const tuple = ["id", 42] as const;
const name = tuple[0];
const count = tuple[1];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const tuple: readonly ("id" | 42)[] = ["id" as "id" | 42, 42 as "id" | 42] as const;
const name: "id" | 42 = tuple[0];
const count: "id" | 42 = tuple[1];

=== dir ===
const tuple = ["id", 42] as const;
/// @type.symbol symbol=tuple source=tuple type=readonly "id" | 42[]
/// @resolution.pattern source=tuple kind=binding target=tuple
/// @generic.instance id="Array<\"id\" | 42>" template=Array arguments=("id" | 42)
/// @generic.instance id="assumeInitDrop#1<\"id\" | 42>" template=assumeInitDrop#1 arguments=("id" | 42)
/// @generic.instance id="assumeInitDrop<\"id\" | 42>" template=assumeInitDrop arguments=("id" | 42)
/// @generic.instance id="clear<\"id\" | 42>" template=clear arguments=("id" | 42)
/// @generic.instance id="drop<\"id\" | 42>" template=drop arguments=("id" | 42)
/// @generic.instance id="dropInPlace<\"id\" | 42>" template=dropInPlace arguments=("id" | 42)
/// @generic.instance id="elementSlot<\"id\" | 42, \"mutable\">" template=elementSlot arguments=("id" | 42, "mutable")
/// @generic.instance id="initAsPointer<\"id\" | 42, \"mutable\">" template=initAsPointer arguments=("id" | 42, "mutable")
/// @generic.instance id="sliceAssumeInit<MaybeUninit<\"id\" | 42>>" template=sliceAssumeInit arguments=(MaybeUninit<"id" | 42>)
/// @generic.instance id="sliceIndex<MaybeUninit<\"id\" | 42>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<"id" | 42>, "mutable")
/// @generic.instance id="sliceUninit<MaybeUninit<\"id\" | 42>>" template=sliceUninit arguments=(MaybeUninit<"id" | 42>)
/// @generic.instance id="truncate<\"id\" | 42>" template=truncate arguments=("id" | 42)
/// @resolution.call source=["id", 42] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest("id", 42) as "id" | 42) return="id" | 42[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<\"id\" | 42>"
/// @generic.instantiation id="arrayFromOwnedSlice<\"id\" | 42>" template=arrayFromOwnedSlice arguments=("id" | 42)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="arrayFromOwnedSlice<\"id\" | 42>" template=arrayFromOwnedSlice arguments=("id" | 42)
/// @generic.instance id="fromOwnedSlice<\"id\" | 42>" template=fromOwnedSlice arguments=("id" | 42)
/// @generic.instance id="intoUninit<\"id\" | 42>" template=intoUninit arguments=("id" | 42)
/// @generic.instance id="size<\"id\" | 42>" template=size arguments=("id" | 42)
/// @generic.instance id="sliceIntoUninit<\"id\" | 42>" template=sliceIntoUninit arguments=("id" | 42)
/// @generic.instance id="sliceLength<\"id\" | 42>" template=sliceLength arguments=("id" | 42)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)

const name = tuple[0];
/// @type.symbol symbol=name source=name type="id" | 42
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=tuple root=tuple
/// @resolution.access source=tuple[0] root=tuple keys=[0]
/// @resolution.subscript source=tuple[0] type="id" | 42 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<Borrowed<\"id\" | 42, \"managed\" & \"local\", \"mutable\">, \"readonly\">, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#1<\"id\" | 42, \"readonly\">" template=index#1 arguments=("id" | 42, "readonly")
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="WithAccess<&'bound0 \"id\" | 42, \"readonly\">" template=WithAccess arguments=(&'bound0 "id" | 42, "readonly")
/// @generic.instance id="WithAccess<&'bound0 \"id\" | 42[], \"readonly\">" template=WithAccess arguments=(&'bound0 "id" | 42[], "readonly")
/// @generic.instance id="assumeInitReference<\"id\" | 42, \"readonly\">" template=assumeInitReference arguments=("id" | 42, "readonly")
/// @generic.instance id="elementPosition<\"id\" | 42>" template=elementPosition arguments=("id" | 42)
/// @generic.instance id="elementSlot<\"id\" | 42, \"readonly\">" template=elementSlot arguments=("id" | 42, "readonly")
/// @generic.instance id="index#1<\"id\" | 42, \"readonly\">" template=index#1 arguments=("id" | 42, "readonly")
/// @generic.instance id="sliceIndex<MaybeUninit<\"id\" | 42>, \"readonly\">" template=sliceIndex arguments=(MaybeUninit<"id" | 42>, "readonly")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)

const count = tuple[1];
/// @type.symbol symbol=count source=count type="id" | 42
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=tuple root=tuple
/// @resolution.access source=tuple[1] root=tuple keys=[1]
/// @resolution.subscript source=tuple[1] type="id" | 42 kind=call target="index#1(parameters=(isize), arguments=(provided(1) as isize), return=WithAccess<Borrowed<\"id\" | 42, \"managed\" & \"local\", \"mutable\">, \"readonly\">, regions=(\"managed\" & \"local\"))"
"#,
    );
}
