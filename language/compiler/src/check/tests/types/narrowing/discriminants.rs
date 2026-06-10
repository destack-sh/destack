use crate::tests::{DirRows, TestSession};

#[test]
fn test_member_equality_narrows_parent_union() {
    let session = TestSession::single(
        r#"
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: State): int32 {
    if (state.kind == "pending") {
        return state.reactions;
    }

    return state.value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32
/// @definition.interface symbol=Pending

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32
/// @definition.interface symbol=Fulfilled

    kind: "fulfilled";
    /// @type.symbol symbol=Fulfilled.kind source="kind: \"fulfilled\"" type="fulfilled"

    value: int32;
    /// @type.symbol symbol=Fulfilled.value source="value: int32" type=int32

}

type State = Pending | Fulfilled;
/// @type.symbol symbol=State source="type State = Pending | Fulfilled" type=Pending | Fulfilled
/// @definition.type symbol=State source="type State = Pending | Fulfilled" value=Pending | Fulfilled
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Fulfilled target=Fulfilled

function read(state: State): int32 {
/// @type.symbol symbol=read type=(Pending | Fulfilled) => int32
/// @type.symbol symbol=state source="state: State" type=Pending | Fulfilled
/// @resolution.name source=State target=State

    if (state.kind == "pending") {
    /// @type.node type=void | void
    /// @type.node source="state.kind == \"pending\"" type=boolean
    /// @type.node source=state type=Pending | Fulfilled
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=state
    /// @resolution.member source=state.kind receiver=Pending | Fulfilled kind=union targets=[Pending.kind, Fulfilled.kind]
    /// @resolution.call source="state.kind == \"pending\"" parameters=("pending" | "fulfilled", "pending") return=boolean kind=builtin builtin=binary.equal
    /// @type.node source="\"pending\"" type="pending"

        return state.reactions;
        /// @type.node source=state type=Pending
        /// @type.node source=state.reactions type=int32
        /// @resolution.name source=state target=state
        /// @resolution.member source=state.reactions receiver=Pending kind=symbol target=Pending.reactions

    }

    return state.value;
    /// @type.node source=state type=Fulfilled
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=state
    /// @resolution.member source=state.value receiver=Fulfilled kind=symbol target=Fulfilled.value

}
"#,
    );
}
