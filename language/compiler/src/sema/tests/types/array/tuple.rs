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
        "main.tspp",
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
/// @resolution.place source=pair placement="local" lifetime="static" access="immutable"
/// @resolution.access source=pair root=pair
/// @coercion.node source=pair from=(int32, int32) adjustments=[{ kind: tuple, target: (int32, int32, int32?) }] origin=implicit

const same: (int32, int32) = pair;
/// @type.symbol symbol=same source=same type=(int32, int32)
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
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
/// @generic.instance id="sliceAssumeInit<MaybeUninit<\"id\" | 42>>" template=sliceAssumeInit arguments=(MaybeUninit<"id" | 42>)
/// @generic.instance id="sliceUninit<MaybeUninit<\"id\" | 42>>" template=sliceUninit arguments=(MaybeUninit<"id" | 42>)
/// @resolution.call source=["id", 42] parameters=(^Slice<"id" | 42>) arguments=(rest(provided("id") as "id" | 42, provided(42) as "id" | 42) as "id" | 42) return="id" | 42[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<\"id\" | 42>"
/// @generic.instantiation id="arrayFromOwnedSlice<\"id\" | 42>" template=arrayFromOwnedSlice arguments=("id" | 42)
/// @generic.instance id="arrayFromOwnedSlice<\"id\" | 42>" template=arrayFromOwnedSlice arguments=("id" | 42)

const name = tuple[0];
/// @type.symbol symbol=name source=name type="id" | 42
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="static" access="immutable"
/// @resolution.access source=tuple root=tuple
/// @resolution.subscript source=tuple[0] type="id" | 42 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=\"id\" | 42, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<\"id\" | 42, \"managed\" & \"local\">" template=index#2 arguments=("id" | 42, "managed" & "local")
/// @generic.instance id="index#2<\"id\" | 42, \"bound0\" & \"local\">" template=index#2 arguments=("id" | 42, "bound0" & "local")

const count = tuple[1];
/// @type.symbol symbol=count source=count type="id" | 42
/// @resolution.pattern source=count kind=binding target=count
/// @resolution.name source=tuple target=tuple
/// @resolution.place source=tuple placement="local" lifetime="static" access="immutable"
/// @resolution.access source=tuple root=tuple
/// @resolution.subscript source=tuple[1] type="id" | 42 kind=call target="index#2(parameters=(isize), arguments=(provided(1) as isize), return=\"id\" | 42, regions=(\"managed\" & \"local\"))"
"#,
    );
}
