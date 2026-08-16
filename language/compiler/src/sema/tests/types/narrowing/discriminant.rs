use crate::tests::{DirRows, TestSession};

/// Narrow both branches of a generic newtype through its structural discriminant.
#[test]
fn test_narrow_generic_newtype_discriminant() {
    let session = TestSession::single(
        r#"
struct Pending<T> {
    kind: "pending" = "pending";
    waiting: T;
}

struct Ready<T> {
    kind: "ready" = "ready";
    value: T;
}

newtype State<T> = Pending<T> | Ready<T>;

function read<T>(state: State<T>): T {
    if (state.kind === "ready") {
        return state.value;
    }

    return state.waiting;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Pending<out T> {
    kind: "pending" = "pending";
    waiting: T;
}

struct Ready<out T> {
    kind: "ready" = "ready";
    value: T;
}

newtype State<out T> = Pending<T> | Ready<T>;

function read<T>(state: State<T>): T {
    if (state.kind === ("ready" as "pending" | "ready")) {
        return state.value;
    }

    return state.waiting;
}

=== dir ===
struct Pending<T> {
/// @generic.template symbol=Pending parameters=(out T#1)
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending template=(out T#1)
/// @definition.field symbol=Pending.kind source="kind: \"pending\" = \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.waiting source="waiting: T" key=waiting type=T#1
/// @type.symbol symbol=Pending.T source=T type=T#1

    kind: "pending" = "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\" = \"pending\"" type="pending"
    /// @type.node source="\"pending\"" type="pending"

    waiting: T;
    /// @type.symbol symbol=Pending.waiting source="waiting: T" type=T#1
    /// @resolution.name source=T target=Pending.T

}

struct Ready<T> {
/// @generic.template symbol=Ready parameters=(out T#2)
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready template=(out T#2)
/// @definition.field symbol=Ready.kind source="kind: \"ready\" = \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Ready.T source=T type=T#2

    kind: "ready" = "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\" = \"ready\"" type="ready"
    /// @type.node source="\"ready\"" type="ready"

    value: T;
    /// @type.symbol symbol=Ready.value source="value: T" type=T#2
    /// @resolution.name source=T target=Ready.T

}

newtype State<T> = Pending<T> | Ready<T>;
/// @generic.template symbol=State parameters=(out T#3)
/// @type.symbol symbol=State source="newtype State<T> = Pending<T> | Ready<T>" type=State
/// @definition.newtype symbol=State source="newtype State<T> = Pending<T> | Ready<T>" template=(out T#3) backing=Pending<T#3> | Ready<T#3> constructors=[<T#3>(Pending<T#3>) => State<T#3>, <T#3>(Ready<T#3>) => State<T#3>, <T#3>(Pending<T#3> | Ready<T#3>) => State<T#3>]
/// @type.symbol symbol=State.T source=T type=T#3
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=T target=State.T
/// @resolution.name source=Ready target=Ready
/// @resolution.name source=T target=State.T

function read<T>(state: State<T>): T {
/// @generic.template symbol=read parameters=(T#4)
/// @type.symbol symbol=read type=<T#4>(State<T#4>) => T#4
/// @type.symbol symbol=read.T source=T type=T#4
/// @type.symbol symbol=read.state source="state: State<T>" type=State<T#4>
/// @resolution.name source=State target=State
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    if (state.kind === "ready") {
    /// @type.node source="state.kind === \"ready\"" type=boolean
    /// @type.node source=state type=State<T#4>
    /// @type.node source=state.kind type="pending" | "ready"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind receiver=State<T#4> type="pending" | "ready" kind=projection target="discriminant(Pending<T#4> | Ready<T#4>, kind, cases=[Pending<T#4>: pending, Ready<T#4>: ready], \"pending\" | \"ready\")" adjustments=(newtype.payload(State, Pending<T#4> | Ready<T#4>))
    /// @resolution.operator source="state.kind === \"ready\"" type=boolean operator="===" kind=builtin operands=[state.kind as "pending" | "ready" families=(string), "ready" as "pending" | "ready" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @generic.instantiation id=State<T#4> template=State arguments=(T#4) owner=read
    /// @type.node source="\"ready\"" type="ready"

        return state.value;
        /// @type.node source=state type=Ready<T#4>
        /// @type.node source=state.value type=T#4
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.value receiver=Ready<T#4> type=T#4 kind=field target_receiver=Ready<T#4> key=value target=Ready.value target_type=T#4
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.value root=read.state keys=[value]

    }

    return state.waiting;
    /// @type.node source=state type=Pending<T#4>
    /// @type.node source=state.waiting type=T#4
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.waiting receiver=Pending<T#4> type=T#4 kind=field target_receiver=Pending<T#4> key=waiting target=Pending.waiting target_type=T#4
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.waiting placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.waiting root=read.state keys=[waiting]

}
"#);
}

/// Narrow an imported generic newtype through its structural discriminant.
#[test]
fn test_narrow_imported_generic_newtype_discriminant() {
    let session = TestSession::builder()
        .module(
            "state.ds",
            r#"
export struct Pending<T> {
    kind: "pending" = "pending";
    waiting: T;
}

export struct Ready<T> {
    kind: "ready" = "ready";
    value: T;
}

export newtype State<T> = Pending<T> | Ready<T>;
"#,
        )
        .module(
            "main.ds",
            r#"
import { State } from "./state.ds";

function read<T>(state: State<T>): T {
    if (state.kind === "ready") {
        return state.value;
    }

    return state.waiting;
}
"#,
        )
        .build();

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
import { State } from "./state.ds";

function read<T>(state: State<T>): T {
    if (state.kind === ("ready" as "pending" | "ready")) {
        return state.value;
    }

    return state.waiting;
}

=== dir ===
import { State } from "./state.ds";

function read<T>(state: State<T>): T {
/// @generic.template symbol=read parameters=(T)
/// @type.symbol symbol=read type=<T>(state.State<T>) => T
/// @type.symbol symbol=read.T source=T type=T
/// @type.symbol symbol=read.state source="state: State<T>" type=state.State<T>
/// @resolution.name source=State target=state.State
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    if (state.kind === "ready") {
    /// @type.node source="state.kind === \"ready\"" type=boolean
    /// @type.node source=state type=state.State<T>
    /// @type.node source=state.kind type="pending" | "ready"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind receiver=state.State<T> type="pending" | "ready" kind=projection target="discriminant(state.Pending<T> | state.Ready<T>, kind, cases=[state.Pending<T>: pending, state.Ready<T>: ready], \"pending\" | \"ready\")" adjustments=(newtype.payload(state.State, state.Pending<T> | state.Ready<T>))
    /// @resolution.operator source="state.kind === \"ready\"" type=boolean operator="===" kind=builtin operands=[state.kind as "pending" | "ready" families=(string), "ready" as "pending" | "ready" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @generic.instantiation id=state.State<T> template=state.State arguments=(T) owner=read
    /// @type.node source="\"ready\"" type="ready"

        return state.value;
        /// @type.node source=state type=state.Ready<T>
        /// @type.node source=state.value type=T
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.value receiver=state.Ready<T> type=T kind=field target_receiver=state.Ready<T> key=value target=state.Ready.value target_type=T
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.value root=read.state keys=[value]

    }

    return state.waiting;
    /// @type.node source=state type=state.Pending<T>
    /// @type.node source=state.waiting type=T
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.waiting receiver=state.Pending<T> type=T kind=field target_receiver=state.Pending<T> key=waiting target=state.Pending.waiting target_type=T
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.waiting placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.waiting root=read.state keys=[waiting]

}
"#);
}

#[test]
fn test_getter_equality_does_not_narrow_later_read() {
    let session = TestSession::single(
        r#"
interface Source {
    get kind(): "pending" | "ready";
}

function read(source: Source): "ready" {
    if (source.kind === "ready") {
        return source.kind;
    }

    return "ready";
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Source {
    get kind(): "pending" | "ready";
}

function read(source: Dynamic<Source>): "ready" {
    if (source.kind === ("ready" as "pending" | "ready")) {
        return source.kind;
    }

    return "ready";
}

=== dir ===
interface Source {
/// @type.symbol symbol=Source type=Source
/// @definition.interface symbol=Source
/// @definition.method symbol=Source.kind source="get kind(): \"pending\" | \"ready\"" slot=kind role=getter type=(this: this) => "pending" | "ready"

    get kind(): "pending" | "ready";
    /// @type.symbol symbol=Source.kind source="get kind(): \"pending\" | \"ready\"" type=(this: this) => "pending" | "ready"

}

function read(source: Source): "ready" {
/// @type.symbol symbol=read type=(Dynamic<Source>) => "ready"
/// @type.symbol symbol=read type=(Source) => "ready"
/// @type.symbol symbol=read.source source="source: Source" type=Dynamic<Source>
/// @resolution.name source=Source target=Source

    if (source.kind === "ready") {
    /// @resolution.name source=source target=read.source
    /// @resolution.member source=source.kind receiver=Dynamic<Source> type="pending" | "ready" kind=call target="dynamic(Dynamic<Source> as Source, Source.kind)(parameters=(), arguments=(), return=\"pending\" | \"ready\")"
    /// @resolution.operator source="source.kind === \"ready\"" type=boolean operator="===" kind=builtin operands=[source.kind as "pending" | "ready" families=(string), "ready" as "pending" | "ready" families=(string)]
    /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=source root=read.source

        return source.kind;
        /// @resolution.name source=source target=read.source
        /// @resolution.member source=source.kind receiver=Dynamic<Source> type="pending" | "ready" kind=call target="dynamic(Dynamic<Source> as Source, Source.kind)(parameters=(), arguments=(), return=\"pending\" | \"ready\")"
        /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=source root=read.source

    }

    return "ready";
}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type '\"pending\" | \"ready\"' is not assignable to the declared result type '\"ready\"'"
/// @diagnostic.label line=8 column=23 span="kind" line_source="return source.kind;"
/// @diagnostic.note message="expected '\"ready\"', found '\"pending\"'"
"#,
    );
}

#[test]
fn test_index_place_equality_narrows_later_read() {
    let session = TestSession::single(
        r#"
function read(values: ("pending" | "ready")[]): "ready" {
    if (values[0] === "ready") {
        return values[0];
    }

    return "ready";
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function read(values: ("pending" | "ready")[]): "ready" {
    if (values[0] === ("ready" as "pending" | "ready")) {
        return values[0];
    }

    return "ready";
}

=== dir ===
function read(values: ("pending" | "ready")[]): "ready" {
/// @type.symbol symbol=read type=(Array<"pending" | "ready">) => "ready"
/// @type.symbol symbol=read.values source="values: (\"pending\" | \"ready\")[]" type=Array<"pending" | "ready">

    if (values[0] === "ready") {
    /// @resolution.name source=values target=read.values
    /// @resolution.operator source="values[0] === \"ready\"" type=boolean operator="===" kind=builtin operands=[values[0] as "pending" | "ready" families=(string), "ready" as "pending" | "ready" families=(string)]
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=read.values
    /// @resolution.place source=values[0] placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values[0] root=read.values keys=[0]
    /// @resolution.subscript source=values[0] type="pending" | "ready" kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'frame \"pending\" | \"ready\", \"exclusive\">)"
    /// @generic.instantiation id="collections.array.index#1<\"pending\" | \"ready\", \"exclusive\">" template=collections.array.index#1 arguments=("pending" | "ready", "exclusive")

        return values[0];
        /// @resolution.name source=values target=read.values
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=read.values
        /// @resolution.place source=values[0] placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values[0] root=read.values keys=[0]
        /// @resolution.subscript source=values[0] type="pending" | "ready" kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'frame \"pending\" | \"ready\", \"exclusive\">)"
        /// @generic.instantiation id="collections.array.index#1<\"pending\" | \"ready\", \"exclusive\">" template=collections.array.index#1 arguments=("pending" | "ready", "exclusive")

    }

    return "ready";
}
"#,
        r#"

"#,
    );
}

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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: Dynamic<Pending> | Dynamic<Fulfilled>): int32 {
    if (state.kind == "pending") {
        return state.reactions;
    }

    return state.value;
}

=== dir ===
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.interface symbol=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32

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
/// @type.symbol symbol=read type=(Dynamic<Pending> | Dynamic<Fulfilled>) => int32
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State

    if (state.kind == "pending") {
    /// @type.node source="state.kind == \"pending\"" type=boolean
    /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind type="pending" | "fulfilled" kind=union arms=[receiver=Dynamic<Pending>, target=field(receiver=dynamic(Dynamic<Pending>, constraint=Pending), target=Pending.kind, type="pending"), type="pending", receiver=Dynamic<Fulfilled>, target=field(receiver=dynamic(Dynamic<Fulfilled>, constraint=Fulfilled), target=Fulfilled.kind, type="fulfilled"), type="fulfilled"]
    /// @resolution.operator source="state.kind == \"pending\"" type=boolean operator="==" kind=builtin operands=[state.kind as "pending" | "fulfilled" families=(string), "pending" as "pending" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"pending\"" type="pending"

        return state.reactions;
        /// @type.node source=state type=Dynamic<Pending>
        /// @type.node source=state.reactions type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.reactions receiver=Dynamic<Pending> type=int32 kind=field target_receiver=Dynamic<Pending> dispatch=dynamic constraint=Pending key=reactions target=Pending.reactions target_type=int32
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.reactions placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.reactions root=read.state keys=[reactions]

    }

    return state.value;
    /// @type.node source=state type=Dynamic<Fulfilled>
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Dynamic<Fulfilled> type=int32 kind=field target_receiver=Dynamic<Fulfilled> dispatch=dynamic constraint=Fulfilled key=value target=Fulfilled.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.value root=read.state keys=[value]

}
"#,
    );
}

#[test]
fn test_static_index_equality_narrows_parent_union() {
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
    if (state["kind"] == "pending") {
        return state.reactions;
    }

    return state.value;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: Dynamic<Pending> | Dynamic<Fulfilled>): int32 {
    if (state["kind"] == "pending") {
        return state.reactions;
    }

    return state.value;
}

=== dir ===
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.interface symbol=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32

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
/// @type.symbol symbol=read type=(Dynamic<Pending> | Dynamic<Fulfilled>) => int32
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State

    if (state["kind"] == "pending") {
    /// @type.node source="state[\"kind\"] == \"pending\"" type=boolean
    /// @type.node source="state[\"kind\"]" type="pending" | "fulfilled"
    /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @resolution.name source=state target=read.state
    /// @resolution.operator source="state[\"kind\"] == \"pending\"" type=boolean operator="==" kind=builtin operands=[state["kind"] as "pending" | "fulfilled" families=(string), "pending" as "pending" families=(string)]
    /// @resolution.place source="state[\"kind\"]" placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source="state[\"kind\"]" root=read.state keys=[kind]
    /// @resolution.subscript source="state[\"kind\"]" type="pending" | "fulfilled" kind=union arms=[member(receiver=Dynamic<Pending>, target=field(receiver=dynamic(Dynamic<Pending>, constraint=Pending), target=Pending.kind, type="pending"), type="pending"), member(receiver=Dynamic<Fulfilled>, target=field(receiver=dynamic(Dynamic<Fulfilled>, constraint=Fulfilled), target=Fulfilled.kind, type="fulfilled"), type="fulfilled")]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @type.node source="\"kind\"" type="kind"
    /// @type.node source="\"pending\"" type="pending"

        return state.reactions;
        /// @type.node source=state type=Dynamic<Pending>
        /// @type.node source=state.reactions type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.reactions receiver=Dynamic<Pending> type=int32 kind=field target_receiver=Dynamic<Pending> dispatch=dynamic constraint=Pending key=reactions target=Pending.reactions target_type=int32
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.reactions placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.reactions root=read.state keys=[reactions]

    }

    return state.value;
    /// @type.node source=state type=Dynamic<Fulfilled>
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Dynamic<Fulfilled> type=int32 kind=field target_receiver=Dynamic<Fulfilled> dispatch=dynamic constraint=Fulfilled key=value target=Fulfilled.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.value root=read.state keys=[value]

}
"#,
    );
}

#[test]
fn test_assignment_clears_member_discriminant_narrowing() {
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

function read(initial: State, next: State): int32 {
    let state = initial;
    if (state.kind == "pending") {
        state = next;

        return state.reactions;
    }

    return state.value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(
    initial: Dynamic<Pending> | Dynamic<Fulfilled>,
    next: Dynamic<Pending> | Dynamic<Fulfilled>,
): int32 {
    let state: Dynamic<Pending> | Dynamic<Fulfilled> = initial;
    if (state.kind == "pending") {
        state = next;

        return state.reactions;
    }

    return state.value;
}

=== dir ===
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.interface symbol=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32

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

function read(initial: State, next: State): int32 {
/// @type.symbol symbol=read type=(Dynamic<Pending> | Dynamic<Fulfilled>, Dynamic<Pending> | Dynamic<Fulfilled>) => int32
/// @type.symbol symbol=read type=(State, State) => int32
/// @type.symbol symbol=read.initial source="initial: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State
/// @type.symbol symbol=read.next source="next: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State

    let state = initial;
    /// @type.symbol symbol=read.state source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @resolution.pattern source=state kind=binding target=read.state
    /// @type.node source=initial type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @resolution.name source=initial target=read.initial
    /// @resolution.access source=initial root=read.initial

    if (state.kind == "pending") {
    /// @type.node source="state.kind == \"pending\"" type=boolean
    /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind type="pending" | "fulfilled" kind=union arms=[receiver=Dynamic<Pending>, target=field(receiver=dynamic(Dynamic<Pending>, constraint=Pending), target=Pending.kind, type="pending"), type="pending", receiver=Dynamic<Fulfilled>, target=field(receiver=dynamic(Dynamic<Fulfilled>, constraint=Fulfilled), target=Fulfilled.kind, type="fulfilled"), type="fulfilled"]
    /// @resolution.operator source="state.kind == \"pending\"" type=boolean operator="==" kind=builtin operands=[state.kind as "pending" | "fulfilled" families=(string), "pending" as "pending" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"pending\"" type="pending"

        state = next;
        /// @type.node source="state = next" type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @resolution.name source=state target=read.state
        /// @resolution.pattern.assign source=state kind=place
        /// @resolution.access source=state root=read.state
        /// @resolution.assignment source=state write=binding(read.state) type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @type.node source=next type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @resolution.name source=next target=read.next
        /// @resolution.place source=next placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=next root=read.next

        return state.reactions;
        /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @type.node source=state.reactions type=<error>
        /// @resolution.name source=state target=read.state
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.rejected source=state.reactions

    }

    return state.value;
    /// @type.node source=state type=Dynamic<Fulfilled>
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Dynamic<Fulfilled> type=int32 kind=field target_receiver=Dynamic<Fulfilled> dispatch=dynamic constraint=Fulfilled key=value target=Fulfilled.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.value root=read.state keys=[value]

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'reactions' does not exist on type 'Dynamic<Pending> | Dynamic<Fulfilled>'"
/// @diagnostic.label line=19 column=22 span="reactions" line_source="return state.reactions;"
"#,
    );
}

#[test]
fn test_early_return_narrows_a_member_discriminant() {
    let session = TestSession::single(
        r#"
class Waiter<T> {
    value: T;
    next: Waiter<T> | undefined;

    constructor(value: T) {
        this.value = value;
        this.next = undefined;
    }
}

struct Pending<T> {
    kind: "pending";
    head: Waiter<T> | undefined;
    tail: Waiter<T> | undefined;
}

struct Fulfilled<T> {
    kind: "fulfilled";
    value: T;
}

type State<T> = Pending<T> | Fulfilled<T>;

class Cell<T> {
    state: State<T>;

    constructor(pending: Pending<T>) {
        this.state = pending;
    }

    consume(value: T): void {
        value;
    }

    poke(waiter: Waiter<T>): void {
        if (this.state.kind == "fulfilled") {
            this.consume(this.state.value);
            return;
        }

        if (this.state.tail == undefined) {
            this.state.head = waiter;
            this.state.tail = waiter;
        } else {
            this.state.tail.next = waiter;
            this.state.tail = waiter;
        }
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Waiter<in out T> {
    value: T;
    next: Waiter<T> | undefined;

    constructor(value: T): this {
        this.value = value;
        this.next = undefined as Waiter<T> | undefined;
    }
}

struct Pending<in out T> {
    kind: "pending";
    head: Waiter<T> | undefined;
    tail: Waiter<T> | undefined;
}

struct Fulfilled<out T> {
    kind: "fulfilled";
    value: T;
}

type State<T> = Pending<T> | Fulfilled<T>;

class Cell<in out T> {
    state: State<T>;

    constructor(pending: Pending<T>): this {
        this.state = pending as Pending<T> | Fulfilled<T>;
    }

    consume(value: T): void {
        value;
    }

    poke(waiter: Waiter<T>): void {
        if (this.state.kind == "fulfilled") {
            this.consume<T>(this.state.value);
            return;
        }

        if (this.state.tail == undefined) {
            this.state.head = waiter as Waiter<T> | undefined;
            this.state.tail = waiter as Waiter<T> | undefined;
        } else {
            this.state.tail.next = waiter as Waiter<T> | undefined;
            this.state.tail = waiter as Waiter<T> | undefined;
        }
    }
}

=== dir ===
class Waiter<T> {
/// @generic.template symbol=Waiter parameters=(in out T#1)
/// @type.symbol symbol=Waiter type=Waiter
/// @definition.class symbol=Waiter template=(in out T#1)
/// @definition.field symbol=Waiter.next source="next: Waiter<T> | undefined" key=next type=Waiter<T#1> | undefined
/// @definition.field symbol=Waiter.value source="value: T" key=value type=T#1
/// @definition.method symbol=Waiter.constructor slot=constructor role=constructor type=(T#1) => this
/// @type.symbol symbol=Waiter.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Waiter.value source="value: T" type=T#1
    /// @resolution.name source=T target=Waiter.T

    next: Waiter<T> | undefined;
    /// @type.symbol symbol=Waiter.next source="next: Waiter<T> | undefined" type=Waiter<T#1> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Waiter.T

    constructor(value: T) {
    /// @type.symbol symbol=Waiter.constructor type=(T#1) => this
    /// @type.symbol symbol=Waiter.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Waiter.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Waiter type=Waiter<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Waiter<T#1>, target=field(receiver=Waiter<T#1>, target=Waiter.value, type=T#1), type=T#1" type=T#1
        /// @resolution.name source=value target=Waiter.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Waiter.constructor.value

        this.next = undefined;
        /// @resolution.receiver source=this kind=this declaration=Waiter type=Waiter<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.next kind=place
        /// @resolution.access source=this.next root=this keys=[next]
        /// @resolution.assignment source=this.next write="receiver=Waiter<T#1>, target=field(receiver=Waiter<T#1>, target=Waiter.next, type=Waiter<T#1> | undefined), type=Waiter<T#1> | undefined" type=Waiter<T#1> | undefined

    }
}

struct Pending<T> {
/// @generic.template symbol=Pending parameters=(in out T#2)
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending template=(in out T#2)
/// @definition.field symbol=Pending.head source="head: Waiter<T> | undefined" key=head type=Waiter<T#2> | undefined
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.tail source="tail: Waiter<T> | undefined" key=tail type=Waiter<T#2> | undefined
/// @type.symbol symbol=Pending.T source=T type=T#2

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    head: Waiter<T> | undefined;
    /// @type.symbol symbol=Pending.head source="head: Waiter<T> | undefined" type=Waiter<T#2> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Pending.T

    tail: Waiter<T> | undefined;
    /// @type.symbol symbol=Pending.tail source="tail: Waiter<T> | undefined" type=Waiter<T#2> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Pending.T

}

struct Fulfilled<T> {
/// @generic.template symbol=Fulfilled parameters=(out T#3)
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.struct symbol=Fulfilled template=(out T#3)
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: T" key=value type=T#3
/// @type.symbol symbol=Fulfilled.T source=T type=T#3

    kind: "fulfilled";
    /// @type.symbol symbol=Fulfilled.kind source="kind: \"fulfilled\"" type="fulfilled"

    value: T;
    /// @type.symbol symbol=Fulfilled.value source="value: T" type=T#3
    /// @resolution.name source=T target=Fulfilled.T

}

type State<T> = Pending<T> | Fulfilled<T>;
/// @generic.template symbol=State parameters=(T#4)
/// @type.symbol symbol=State source="type State<T> = Pending<T> | Fulfilled<T>" type=Pending<T#4> | Fulfilled<T#4>
/// @definition.type symbol=State source="type State<T> = Pending<T> | Fulfilled<T>" template=(T#4) value=Pending<T#4> | Fulfilled<T#4>
/// @type.symbol symbol=State.T source=T type=T#4
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=T target=State.T
/// @resolution.name source=Fulfilled target=Fulfilled
/// @resolution.name source=T target=State.T

class Cell<T> {
/// @generic.template symbol=Cell parameters=(in out T#5)
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell template=(in out T#5)
/// @definition.field symbol=Cell.state source="state: State<T>" key=state type=State<T#5>
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(Pending<T#5>) => this
/// @definition.method symbol=Cell.consume slot=consume type=(this: this, T#5) => void
/// @definition.method symbol=Cell.poke slot=poke type=(this: this, Waiter<T#5>) => void
/// @type.symbol symbol=Cell.T source=T type=T#5

    state: State<T>;
    /// @type.symbol symbol=Cell.state source="state: State<T>" type=State<T#5>
    /// @resolution.name source=State target=State
    /// @resolution.name source=T target=Cell.T

    constructor(pending: Pending<T>) {
    /// @type.symbol symbol=Cell.constructor type=(Pending<T#5>) => this
    /// @type.symbol symbol=Cell.constructor.pending source="pending: Pending<T>" type=Pending<T#5>
    /// @resolution.name source=Pending target=Pending
    /// @resolution.name source=T target=Cell.T

        this.state = pending;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.state kind=place
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.assignment source=this.state write="receiver=Cell<T#5>, target=field(receiver=Cell<T#5>, target=Cell.state, type=State<T#5>), type=State<T#5>" type=State<T#5>
        /// @resolution.name source=pending target=Cell.constructor.pending
        /// @resolution.place source=pending placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=pending root=Cell.constructor.pending

    }

    consume(value: T): void {
    /// @type.symbol symbol=Cell.consume type=(this: this, T#5) => void
    /// @type.symbol symbol=Cell.consume.value source="value: T" type=T#5
    /// @resolution.name source=T target=Cell.T

        value;
        /// @resolution.name source=value target=Cell.consume.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Cell.consume.value

    }

    poke(waiter: Waiter<T>): void {
    /// @type.symbol symbol=Cell.poke type=(this: this, Waiter<T#5>) => void
    /// @type.symbol symbol=Cell.poke.waiter source="waiter: Waiter<T>" type=Waiter<T#5>
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Cell.T

        if (this.state.kind == "fulfilled") {
        /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
        /// @resolution.member source=this.state.kind type="pending" | "fulfilled" kind=union arms=[receiver=State<T#5>, target=field(receiver=State<T#5>, target=Pending.kind, type="pending"), type="pending", receiver=State<T#5>, target=field(receiver=State<T#5>, target=Fulfilled.kind, type="fulfilled"), type="fulfilled"]
        /// @resolution.operator source="this.state.kind == \"fulfilled\"" type=boolean operator="==" kind=builtin operands=[this.state.kind as "pending" | "fulfilled" families=(string), "fulfilled" as "fulfilled" families=(string)]
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.place source=this.state.kind placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state.kind root=this keys=[state, kind]

            this.consume(this.state.value);
            /// @resolution.member source=this.consume receiver=Cell<T#5> type=(this: Cell<T#5>, T#5) => void kind=symbol target_receiver=Cell<T#5> target=Cell.consume
            /// @resolution.call source=this.consume(this.state.value) parameters=(T#5) arguments=(provided(this.state.value) as T#5) return=void kind=symbol target=Cell.consume receiver=Cell<T#5> instance=Cell<T#5>.consume
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @generic.instantiation id=Cell.consume<T#5> template=Cell.consume arguments=(T#5) owner=Cell
            /// @generic.instantiation id=Cell.consume<T#5> template=Cell.consume arguments=(T#5) owner=Cell
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.member source=this.state.value receiver=Fulfilled<T#5> type=T#5 kind=field target_receiver=Fulfilled<T#5> key=value target=Fulfilled.value target_type=T#5
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.place source=this.state.value placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state.value root=this keys=[state, value]

            return;
        }

        if (this.state.tail == undefined) {
        /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
        /// @resolution.member source=this.state.tail receiver=Pending<T#5> type=Waiter<T#5> | undefined kind=field target_receiver=Pending<T#5> key=tail target=Pending.tail target_type=Waiter<T#5> | undefined
        /// @resolution.operator source="this.state.tail == undefined" type=boolean operator="==" kind=builtin operands=[this.state.tail as Waiter<T#5> | undefined, undefined as undefined families=(undefined)]
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.place source=this.state.tail placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state.tail root=this keys=[state, tail]

            this.state.head = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.pattern.assign source=this.state.head kind=place
            /// @resolution.access source=this.state.head root=this keys=[state, head]
            /// @resolution.assignment source=this.state.head write="receiver=Pending<T#5>, target=field(receiver=Pending<T#5>, target=Pending.head, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

            this.state.tail = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.pattern.assign source=this.state.tail kind=place
            /// @resolution.access source=this.state.tail root=this keys=[state, tail]
            /// @resolution.assignment source=this.state.tail write="receiver=Pending<T#5>, target=field(receiver=Pending<T#5>, target=Pending.tail, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

        } else {
            this.state.tail.next = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.member source=this.state.tail receiver=Pending<T#5> type=Waiter<T#5> | undefined kind=field target_receiver=Pending<T#5> key=tail target=Pending.tail target_type=Waiter<T#5> | undefined
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.place source=this.state.tail placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state.tail root=this keys=[state, tail]
            /// @resolution.pattern.assign source=this.state.tail.next kind=place
            /// @resolution.access source=this.state.tail.next root=this keys=[state, tail, next]
            /// @resolution.assignment source=this.state.tail.next write="receiver=Waiter<T#5>, target=field(receiver=Waiter<T#5>, target=Waiter.next, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

            this.state.tail = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.pattern.assign source=this.state.tail kind=place
            /// @resolution.access source=this.state.tail root=this keys=[state, tail]
            /// @resolution.assignment source=this.state.tail write="receiver=Pending<T#5>, target=field(receiver=Pending<T#5>, target=Pending.tail, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

        }
    }
}
"#,
    );
}

#[test]
fn test_narrow_a_super_member_and_clear_it_on_assignment() {
    let session = TestSession::single(
        r#"
class Base {
    label: string | undefined = undefined;
}

class Child extends Base {
    read(next: string | undefined): string | undefined {
        if (super.label == undefined) {
            return undefined;
        }

        const kept = super.label;
        super.label = next;
        const cleared = super.label;

        return cleared;
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Base {
    label: string | undefined = undefined as string | undefined;
}

class Child extends Base {
    read(next: string | undefined): string | undefined {
        if (super.label == undefined) {
            return undefined as string | undefined;
        }

        const kept: string = super.label;
        super.label = next;
        const cleared: string | undefined = super.label;

        return cleared;
    }
}

=== dir ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.field symbol=Base.label source="label: string | undefined = undefined" key=label type=string | undefined

    label: string | undefined = undefined;
    /// @type.symbol symbol=Base.label source="label: string | undefined = undefined" type=string | undefined

}

class Child extends Base {
/// @type.symbol symbol=Child type=Child
/// @definition.class symbol=Child
/// @definition.extends symbol=Child source=Base target=Base
/// @definition.method symbol=Child.read slot=read type=(this: this, string | undefined) => string | undefined
/// @resolution.name source=Base target=Base

    read(next: string | undefined): string | undefined {
    /// @type.symbol symbol=Child.read type=(this: this, string | undefined) => string | undefined
    /// @type.symbol symbol=Child.read.next source="next: string | undefined" type=string | undefined

        if (super.label == undefined) {
        /// @resolution.member source=super.label receiver=Base type=string | undefined kind=field target_receiver=Base key=label target=Base.label target_type=string | undefined
        /// @resolution.operator source="super.label == undefined" type=boolean operator="==" kind=builtin operands=[super.label as string | undefined families=(string | undefined), undefined as undefined families=(undefined)]
        /// @resolution.receiver source=super kind=super declaration=Child type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.place source=super.label placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super.label root=this keys=[label]

            return undefined;
        }

        const kept = super.label;
        /// @type.symbol symbol=Child.read.kept source=kept type=string
        /// @resolution.pattern source=kept kind=binding target=Child.read.kept
        /// @resolution.member source=super.label receiver=Base type=string | undefined kind=field target_receiver=Base key=label target=Base.label target_type=string | undefined
        /// @resolution.receiver source=super kind=super declaration=Child type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.access source=super.label root=this keys=[label]

        super.label = next;
        /// @resolution.receiver source=super kind=super declaration=Child type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.pattern.assign source=super.label kind=place
        /// @resolution.access source=super.label root=this keys=[label]
        /// @resolution.assignment source=super.label write="receiver=Base, target=field(receiver=Base, target=Base.label, type=string | undefined), type=string | undefined" type=string | undefined
        /// @resolution.name source=next target=Child.read.next
        /// @resolution.place source=next placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=next root=Child.read.next

        const cleared = super.label;
        /// @type.symbol symbol=Child.read.cleared source=cleared type=string | undefined
        /// @resolution.pattern source=cleared kind=binding target=Child.read.cleared
        /// @resolution.member source=super.label receiver=Base type=string | undefined kind=field target_receiver=Base key=label target=Base.label target_type=string | undefined
        /// @resolution.receiver source=super kind=super declaration=Child type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.access source=super.label root=this keys=[label]

        return cleared;
        /// @resolution.name source=cleared target=Child.read.cleared
        /// @resolution.place source=cleared placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=cleared root=Child.read.cleared

    }
}
"#,
    );
}

#[test]
fn test_share_a_narrowed_member_across_this_and_super() {
    let session = TestSession::single(
        r#"
class Base {
    label: string | undefined = undefined;
}

class Child extends Base {
    read(next: string | undefined): string | undefined {
        if (this.label == undefined) {
            return undefined;
        }

        const shared = super.label;
        super.label = next;
        const cleared = this.label;

        return cleared;
    }
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
class Base {
    label: string | undefined = undefined as string | undefined;
}

class Child extends Base {
    read(next: string | undefined): string | undefined {
        if (this.label == undefined) {
            return undefined as string | undefined;
        }

        const shared: string = super.label;
        super.label = next;
        const cleared: string | undefined = this.label;

        return cleared;
    }
}

=== dir ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.field symbol=Base.label source="label: string | undefined = undefined" key=label type=string | undefined

    label: string | undefined = undefined;
    /// @type.symbol symbol=Base.label source="label: string | undefined = undefined" type=string | undefined

}

class Child extends Base {
/// @type.symbol symbol=Child type=Child
/// @definition.class symbol=Child
/// @definition.extends symbol=Child source=Base target=Base
/// @definition.method symbol=Child.read slot=read type=(this: this, string | undefined) => string | undefined
/// @resolution.name source=Base target=Base

    read(next: string | undefined): string | undefined {
    /// @type.symbol symbol=Child.read type=(this: this, string | undefined) => string | undefined
    /// @type.symbol symbol=Child.read.next source="next: string | undefined" type=string | undefined

        if (this.label == undefined) {
        /// @resolution.member source=this.label receiver=Child type=string | undefined kind=field target_receiver=Child key=label target=Base.label target_type=string | undefined
        /// @resolution.operator source="this.label == undefined" type=boolean operator="==" kind=builtin operands=[this.label as string | undefined families=(string | undefined), undefined as undefined families=(undefined)]
        /// @resolution.receiver source=this kind=this declaration=Child type=Child
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.label placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.label root=this keys=[label]

            return undefined;
        }

        const shared = super.label;
        /// @type.symbol symbol=Child.read.shared source=shared type=string
        /// @resolution.pattern source=shared kind=binding target=Child.read.shared
        /// @resolution.member source=super.label receiver=Base type=string | undefined kind=field target_receiver=Base key=label target=Base.label target_type=string | undefined
        /// @resolution.receiver source=super kind=super declaration=Child type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.access source=super.label root=this keys=[label]

        super.label = next;
        /// @resolution.receiver source=super kind=super declaration=Child type=Base
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.pattern.assign source=super.label kind=place
        /// @resolution.access source=super.label root=this keys=[label]
        /// @resolution.assignment source=super.label write="receiver=Base, target=field(receiver=Base, target=Base.label, type=string | undefined), type=string | undefined" type=string | undefined
        /// @resolution.name source=next target=Child.read.next
        /// @resolution.place source=next placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=next root=Child.read.next

        const cleared = this.label;
        /// @type.symbol symbol=Child.read.cleared source=cleared type=string | undefined
        /// @resolution.pattern source=cleared kind=binding target=Child.read.cleared
        /// @resolution.member source=this.label receiver=Child type=string | undefined kind=field target_receiver=Child key=label target=Base.label target_type=string | undefined
        /// @resolution.receiver source=this kind=this declaration=Child type=Child
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.access source=this.label root=this keys=[label]

        return cleared;
        /// @resolution.name source=cleared target=Child.read.cleared
        /// @resolution.place source=cleared placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=cleared root=Child.read.cleared

    }
}
"#);
}

#[test]
fn test_share_a_narrowed_member_across_this_and_super_in_a_closure() {
    let session = TestSession::single(
        r#"
class Base {
    label: string | undefined = undefined;
}

class Child extends Base {
    read(next: string | undefined): () => string | undefined {
        return () => {
            if (this.label == undefined) {
                return undefined;
            }

            const shared = super.label;
            super.label = next;
            const cleared = this.label;

            return cleared;
        };
    }
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
class Base {
    label: string | undefined = undefined as string | undefined;
}

class Child extends Base {
    read(next: string | undefined): () => string | undefined {
        return (): string | undefined => {
            if (this.label == undefined) {
                return undefined as string | undefined;
            }

            const shared: string = super.label;
            super.label = next;
            const cleared: string | undefined = this.label;

            return cleared;
        };
    }
}

=== dir ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.field symbol=Base.label source="label: string | undefined = undefined" key=label type=string | undefined

    label: string | undefined = undefined;
    /// @type.symbol symbol=Base.label source="label: string | undefined = undefined" type=string | undefined

}

class Child extends Base {
/// @type.symbol symbol=Child type=Child
/// @definition.class symbol=Child
/// @definition.extends symbol=Child source=Base target=Base
/// @definition.method symbol=Child.read slot=read type=(this: this, string | undefined) => Function<(), string | undefined>
/// @resolution.name source=Base target=Base

    read(next: string | undefined): () => string | undefined {
    /// @type.symbol symbol=Child.read type=(this: this, string | undefined) => Function<(), string | undefined>
    /// @type.symbol symbol=Child.read.next source="next: string | undefined" type=string | undefined

        return () => {
        /// @type.symbol symbol=Child.read.symbol8 type=Function<(), string | undefined>

            if (this.label == undefined) {
            /// @resolution.name source=this target=Child.read.this
            /// @resolution.member source=this.label receiver=Child type=string | undefined kind=field target_receiver=Child key=label target=Base.label target_type=string | undefined
            /// @resolution.operator source="this.label == undefined" type=boolean operator="==" kind=builtin operands=[this.label as string | undefined families=(string | undefined), undefined as undefined families=(undefined)]
            /// @resolution.receiver source=this kind=this declaration=Child type=Child
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.label placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.label root=this keys=[label]

                return undefined;
            }

            const shared = super.label;
            /// @type.symbol symbol=Child.read.symbol8.shared source=shared type=string
            /// @resolution.pattern source=shared kind=binding target=Child.read.symbol8.shared
            /// @resolution.member source=super.label receiver=Base type=string | undefined kind=field target_receiver=Base key=label target=Base.label target_type=string | undefined
            /// @resolution.receiver source=super kind=super declaration=Child type=Base
            /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=super root=this
            /// @resolution.access source=super.label root=this keys=[label]

            super.label = next;
            /// @resolution.receiver source=super kind=super declaration=Child type=Base
            /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=super root=this
            /// @resolution.pattern.assign source=super.label kind=place
            /// @resolution.access source=super.label root=this keys=[label]
            /// @resolution.assignment source=super.label write="receiver=Base, target=field(receiver=Base, target=Base.label, type=string | undefined), type=string | undefined" type=string | undefined
            /// @resolution.name source=next target=Child.read.next
            /// @resolution.place source=next placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=next root=Child.read.next

            const cleared = this.label;
            /// @type.symbol symbol=Child.read.symbol8.cleared source=cleared type=string | undefined
            /// @resolution.pattern source=cleared kind=binding target=Child.read.symbol8.cleared
            /// @resolution.name source=this target=Child.read.this
            /// @resolution.member source=this.label receiver=Child type=string | undefined kind=field target_receiver=Child key=label target=Base.label target_type=string | undefined
            /// @resolution.receiver source=this kind=this declaration=Child type=Child
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.access source=this.label root=this keys=[label]

            return cleared;
            /// @resolution.name source=cleared target=Child.read.symbol8.cleared
            /// @resolution.place source=cleared placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=cleared root=Child.read.symbol8.cleared

        };
    }
}
"#);
}

#[test]
fn test_narrow_a_member_through_an_explicit_this_parameter() {
    let session = TestSession::single(
        r#"
class Box {
    label: string | undefined = undefined;
}

function read(this: Box): string {
    if (this.label == undefined) {
        return "";
    }

    return this.label;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
class Box {
    label: string | undefined = undefined as string | undefined;
}

function read(this: Box): string {
    if (this.label == undefined) {
        return "";
    }

    return this.label;
}

=== dir ===
class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.label source="label: string | undefined = undefined" key=label type=string | undefined

    label: string | undefined = undefined;
    /// @type.symbol symbol=Box.label source="label: string | undefined = undefined" type=string | undefined

}

function read(this: Box): string {
/// @type.symbol symbol=read type=(this: Box) => string
/// @type.symbol symbol=read.this source="this: Box" type=Box
/// @resolution.name source=Box target=Box

    if (this.label == undefined) {
    /// @resolution.name source=this target=read.this
    /// @resolution.member source=this.label receiver=Box type=string | undefined kind=field target_receiver=Box key=label target=Box.label target_type=string | undefined
    /// @resolution.operator source="this.label == undefined" type=boolean operator="==" kind=builtin operands=[this.label as string | undefined families=(string | undefined), undefined as undefined families=(undefined)]
    /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=this root=this
    /// @resolution.place source=this.label placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=this.label root=this keys=[label]

        return "";
    }

    return this.label;
    /// @resolution.name source=this target=read.this
    /// @resolution.member source=this.label receiver=Box type=string | undefined kind=field target_receiver=Box key=label target=Box.label target_type=string | undefined
    /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=this root=this
    /// @resolution.place source=this.label placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=this.label root=this keys=[label]

}
"#);
}

/// Narrow a borrowed newtype discriminant to its selected arm.
#[test]
fn test_narrow_borrowed_newtype_discriminant() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending" = "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready" = "ready";
    value: int32;
}

newtype State = Pending | Ready;

function read(state: &readonly State): int32 {
    if (state.kind === "ready") {
        return state.value;
    }

    return state.waiting;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Pending {
    kind: "pending" = "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready" = "ready";
    value: int32;
}

newtype State = Pending | Ready;

function read<'a>(state: &'a readonly State): int32 {
    if (
        (state.kind as readonly "pending" | readonly "ready" | "ready") ===
        ("ready" as readonly "pending" | readonly "ready" | "ready")
    ) {
        return state.value;
    }

    return state.waiting;
}

=== dir ===
struct Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\" = \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.waiting source="waiting: int32" key=waiting type=int32

    kind: "pending" = "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\" = \"pending\"" type="pending"
    /// @type.node source="\"pending\"" type="pending"

    waiting: int32;
    /// @type.symbol symbol=Pending.waiting source="waiting: int32" type=int32

}

struct Ready {
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready
/// @definition.field symbol=Ready.kind source="kind: \"ready\" = \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: int32" key=value type=int32

    kind: "ready" = "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\" = \"ready\"" type="ready"
    /// @type.node source="\"ready\"" type="ready"

    value: int32;
    /// @type.symbol symbol=Ready.value source="value: int32" type=int32

}

newtype State = Pending | Ready;
/// @type.symbol symbol=State source="newtype State = Pending | Ready" type=State
/// @definition.newtype symbol=State source="newtype State = Pending | Ready" backing=Pending | Ready constructors=[(Pending) => State, (Ready) => State, (Pending | Ready) => State]
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Ready target=Ready

function read(state: &readonly State): int32 {
/// @generic.template symbol=read parameters=('a)
/// @type.symbol symbol=read type=<read.'a>(&read.'a readonly State) => int32
/// @type.symbol symbol=read.state source="state: &readonly State" type=&read.'a readonly State
/// @resolution.name source=State target=State

    if (state.kind === "ready") {
    /// @type.node source="state.kind === \"ready\"" type=boolean
    /// @type.node source=state type=&read.'a readonly State
    /// @type.node source=state.kind type=Readonly<"pending"> | Readonly<"ready">
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind receiver=&read.'a readonly State type=Readonly<"pending"> | Readonly<"ready"> kind=projection target="discriminant(Pending | Ready, kind, cases=[Pending: pending, Ready: ready], Readonly<\"pending\"> | Readonly<\"ready\">)" adjustments=(newtype.payload(State, &read.'a readonly Pending | Ready))
    /// @resolution.operator source="state.kind === \"ready\"" type=boolean operator="===" kind=builtin operands=[state.kind as Readonly<"pending"> | Readonly<"ready"> | "ready", "ready" as Readonly<"pending"> | Readonly<"ready"> | "ready"]
    /// @resolution.place source=state placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"ready\"" type="ready"

        return state.value;
        /// @type.node source=state type=&read.'a readonly Ready
        /// @type.node source=state.value type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.value receiver=&read.'a readonly Ready type=int32 kind=field target_receiver=&read.'a readonly Ready key=value target=Ready.value target_type=int32
        /// @resolution.place source=state placement="local" lifetime=read.'a access="readonly"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.value placement="local" lifetime=read.'a access="readonly"
        /// @resolution.access source=state.value root=read.state keys=[value]

    }

    return state.waiting;
    /// @type.node source=state type=&read.'a readonly Pending
    /// @type.node source=state.waiting type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.waiting receiver=&read.'a readonly Pending type=int32 kind=field target_receiver=&read.'a readonly Pending key=waiting target=Pending.waiting target_type=int32
    /// @resolution.place source=state placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.waiting placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state.waiting root=read.state keys=[waiting]

}
"#);
}

/// Narrow a struct union through an integer literal discriminant.
#[test]
fn test_narrow_struct_union_by_integer_discriminant() {
    let session = TestSession::single(
        r#"
struct Header {
    version: 1;
    length: int32;
}

struct Trailer {
    version: 2;
    checksum: int32;
}

type Frame = Header | Trailer;

function read(frame: Frame): int32 {
    if (frame.version == 1) {
        return frame.length;
    }

    return frame.checksum;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Header {
    version: 1;
    length: int32;
}

struct Trailer {
    version: 2;
    checksum: int32;
}

type Frame = Header | Trailer;

function read(frame: Header | Trailer): int32 {
    if (frame.version == (1 as 1 | 2)) {
        return frame.length;
    }

    return frame.checksum;
}

=== dir ===
struct Header {
/// @type.symbol symbol=Header type=Header
/// @definition.struct symbol=Header
/// @definition.field symbol=Header.length source="length: int32" key=length type=int32
/// @definition.field symbol=Header.version source="version: 1" key=version type=1

    version: 1;
    /// @type.symbol symbol=Header.version source="version: 1" type=1

    length: int32;
    /// @type.symbol symbol=Header.length source="length: int32" type=int32

}

struct Trailer {
/// @type.symbol symbol=Trailer type=Trailer
/// @definition.struct symbol=Trailer
/// @definition.field symbol=Trailer.checksum source="checksum: int32" key=checksum type=int32
/// @definition.field symbol=Trailer.version source="version: 2" key=version type=2

    version: 2;
    /// @type.symbol symbol=Trailer.version source="version: 2" type=2

    checksum: int32;
    /// @type.symbol symbol=Trailer.checksum source="checksum: int32" type=int32

}

type Frame = Header | Trailer;
/// @type.symbol symbol=Frame source="type Frame = Header | Trailer" type=Header | Trailer
/// @definition.type symbol=Frame source="type Frame = Header | Trailer" value=Header | Trailer
/// @resolution.name source=Header target=Header
/// @resolution.name source=Trailer target=Trailer

function read(frame: Frame): int32 {
/// @type.symbol symbol=read type=(Frame) => int32
/// @type.symbol symbol=read type=(Header | Trailer) => int32
/// @type.symbol symbol=read.frame source="frame: Frame" type=Header | Trailer
/// @resolution.name source=Frame target=Frame

    if (frame.version == 1) {
    /// @type.node source="frame.version == 1" type=boolean
    /// @type.node source=frame type=Header | Trailer
    /// @type.node source=frame.version type=1 | 2
    /// @resolution.name source=frame target=read.frame
    /// @resolution.member source=frame.version receiver=Header | Trailer type=1 | 2 kind=projection target="discriminant(Header | Trailer, version, cases=[Header: 1, Trailer: 2], 1 | 2)"
    /// @resolution.operator source="frame.version == 1" type=boolean operator="==" kind=builtin operands=[frame.version as 1 | 2 families=(integer), 1 as 1 | 2 families=(integer)]
    /// @resolution.place source=frame placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=frame root=read.frame
    /// @resolution.place source=frame.version placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=frame.version root=read.frame keys=[version]
    /// @type.node source=1 type=1

        return frame.length;
        /// @type.node source=frame type=Header
        /// @type.node source=frame.length type=int32
        /// @resolution.name source=frame target=read.frame
        /// @resolution.member source=frame.length receiver=Header type=int32 kind=field target_receiver=Header key=length target=Header.length target_type=int32
        /// @resolution.place source=frame placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=frame root=read.frame
        /// @resolution.place source=frame.length placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=frame.length root=read.frame keys=[length]

    }

    return frame.checksum;
    /// @type.node source=frame type=Trailer
    /// @type.node source=frame.checksum type=int32
    /// @resolution.name source=frame target=read.frame
    /// @resolution.member source=frame.checksum receiver=Trailer type=int32 kind=field target_receiver=Trailer key=checksum target=Trailer.checksum target_type=int32
    /// @resolution.place source=frame placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=frame root=read.frame
    /// @resolution.place source=frame.checksum placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=frame.checksum root=read.frame keys=[checksum]

}
"#);
}

/// Narrow a struct union through a boolean literal discriminant.
#[test]
fn test_narrow_struct_union_by_boolean_discriminant() {
    let session = TestSession::single(
        r#"
struct Success {
    ok: true;
    value: int32;
}

struct Failure {
    ok: false;
    code: int32;
}

type Outcome = Success | Failure;

function read(outcome: Outcome): int32 {
    if (outcome.ok === true) {
        return outcome.value;
    }

    return outcome.code;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Success {
    ok: true;
    value: int32;
}

struct Failure {
    ok: false;
    code: int32;
}

type Outcome = Success | Failure;

function read(outcome: Success | Failure): int32 {
    if (outcome.ok === (true as true | false)) {
        return outcome.value;
    }

    return outcome.code;
}

=== dir ===
struct Success {
/// @type.symbol symbol=Success type=Success
/// @definition.struct symbol=Success
/// @definition.field symbol=Success.ok source="ok: true" key=ok type=true
/// @definition.field symbol=Success.value source="value: int32" key=value type=int32

    ok: true;
    /// @type.symbol symbol=Success.ok source="ok: true" type=true

    value: int32;
    /// @type.symbol symbol=Success.value source="value: int32" type=int32

}

struct Failure {
/// @type.symbol symbol=Failure type=Failure
/// @definition.struct symbol=Failure
/// @definition.field symbol=Failure.code source="code: int32" key=code type=int32
/// @definition.field symbol=Failure.ok source="ok: false" key=ok type=false

    ok: false;
    /// @type.symbol symbol=Failure.ok source="ok: false" type=false

    code: int32;
    /// @type.symbol symbol=Failure.code source="code: int32" type=int32

}

type Outcome = Success | Failure;
/// @type.symbol symbol=Outcome source="type Outcome = Success | Failure" type=Success | Failure
/// @definition.type symbol=Outcome source="type Outcome = Success | Failure" value=Success | Failure
/// @resolution.name source=Success target=Success
/// @resolution.name source=Failure target=Failure

function read(outcome: Outcome): int32 {
/// @type.symbol symbol=read type=(Outcome) => int32
/// @type.symbol symbol=read type=(Success | Failure) => int32
/// @type.symbol symbol=read.outcome source="outcome: Outcome" type=Success | Failure
/// @resolution.name source=Outcome target=Outcome

    if (outcome.ok === true) {
    /// @type.node source="outcome.ok === true" type=boolean
    /// @type.node source=outcome type=Success | Failure
    /// @type.node source=outcome.ok type=true | false
    /// @resolution.name source=outcome target=read.outcome
    /// @resolution.member source=outcome.ok receiver=Success | Failure type=true | false kind=projection target="discriminant(Success | Failure, ok, cases=[Success: true, Failure: false], true | false)"
    /// @resolution.operator source="outcome.ok === true" type=boolean operator="===" kind=builtin operands=[outcome.ok as true | false families=(boolean), true as true | false families=(boolean)]
    /// @resolution.place source=outcome placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome root=read.outcome
    /// @resolution.place source=outcome.ok placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome.ok root=read.outcome keys=[ok]
    /// @type.node source=true type=true

        return outcome.value;
        /// @type.node source=outcome type=Success
        /// @type.node source=outcome.value type=int32
        /// @resolution.name source=outcome target=read.outcome
        /// @resolution.member source=outcome.value receiver=Success type=int32 kind=field target_receiver=Success key=value target=Success.value target_type=int32
        /// @resolution.place source=outcome placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=outcome root=read.outcome
        /// @resolution.place source=outcome.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=outcome.value root=read.outcome keys=[value]

    }

    return outcome.code;
    /// @type.node source=outcome type=Failure
    /// @type.node source=outcome.code type=int32
    /// @resolution.name source=outcome target=read.outcome
    /// @resolution.member source=outcome.code receiver=Failure type=int32 kind=field target_receiver=Failure key=code target=Failure.code target_type=int32
    /// @resolution.place source=outcome placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome root=read.outcome
    /// @resolution.place source=outcome.code placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome.code root=read.outcome keys=[code]

}
"#);
}

/// Narrow the else branch of a negated newtype discriminant test.
#[test]
fn test_negated_equality_narrows_the_else_branch() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending" = "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready" = "ready";
    value: int32;
}

newtype State = Pending | Ready;

function read(state: State): int32 {
    if (state.kind != "ready") {
        return state.waiting;
    }

    return state.value;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Pending {
    kind: "pending" = "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready" = "ready";
    value: int32;
}

newtype State = Pending | Ready;

function read(state: State): int32 {
    if (state.kind != "ready") {
        return state.waiting;
    }

    return state.value;
}

=== dir ===
struct Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\" = \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.waiting source="waiting: int32" key=waiting type=int32

    kind: "pending" = "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\" = \"pending\"" type="pending"
    /// @type.node source="\"pending\"" type="pending"

    waiting: int32;
    /// @type.symbol symbol=Pending.waiting source="waiting: int32" type=int32

}

struct Ready {
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready
/// @definition.field symbol=Ready.kind source="kind: \"ready\" = \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: int32" key=value type=int32

    kind: "ready" = "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\" = \"ready\"" type="ready"
    /// @type.node source="\"ready\"" type="ready"

    value: int32;
    /// @type.symbol symbol=Ready.value source="value: int32" type=int32

}

newtype State = Pending | Ready;
/// @type.symbol symbol=State source="newtype State = Pending | Ready" type=State
/// @definition.newtype symbol=State source="newtype State = Pending | Ready" backing=Pending | Ready constructors=[(Pending) => State, (Ready) => State, (Pending | Ready) => State]
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Ready target=Ready

function read(state: State): int32 {
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=State
/// @resolution.name source=State target=State

    if (state.kind != "ready") {
    /// @type.node source="state.kind != \"ready\"" type=boolean
    /// @type.node source=state type=State
    /// @type.node source=state.kind type="pending" | "ready"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind receiver=State type="pending" | "ready" kind=projection target="discriminant(Pending | Ready, kind, cases=[Pending: pending, Ready: ready], \"pending\" | \"ready\")" adjustments=(newtype.payload(State, Pending | Ready))
    /// @resolution.operator source="state.kind != \"ready\"" type=boolean operator="!=" kind=builtin operands=[state.kind as "pending" | "ready" families=(string), "ready" as "ready" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"ready\"" type="ready"

        return state.waiting;
        /// @type.node source=state type=Pending
        /// @type.node source=state.waiting type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.waiting receiver=Pending type=int32 kind=field target_receiver=Pending key=waiting target=Pending.waiting target_type=int32
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.waiting placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.waiting root=read.state keys=[waiting]

    }

    return state.value;
    /// @type.node source=state type=Ready
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Ready type=int32 kind=field target_receiver=Ready key=value target=Ready.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.value root=read.state keys=[value]

}
"#);
}

/// Narrow a newtype union through a static index discriminant test.
#[test]
fn test_static_index_equality_narrows_newtype_discriminant() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending" = "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready" = "ready";
    value: int32;
}

newtype State = Pending | Ready;

function read(state: State): int32 {
    if (state["kind"] === "ready") {
        return state.value;
    }

    return state.waiting;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Pending {
    kind: "pending" = "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready" = "ready";
    value: int32;
}

newtype State = Pending | Ready;

function read(state: State): int32 {
    if (state["kind"] === ("ready" as "pending" | "ready")) {
        return state.value;
    }

    return state.waiting;
}

=== dir ===
struct Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\" = \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.waiting source="waiting: int32" key=waiting type=int32

    kind: "pending" = "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\" = \"pending\"" type="pending"
    /// @type.node source="\"pending\"" type="pending"

    waiting: int32;
    /// @type.symbol symbol=Pending.waiting source="waiting: int32" type=int32

}

struct Ready {
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready
/// @definition.field symbol=Ready.kind source="kind: \"ready\" = \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: int32" key=value type=int32

    kind: "ready" = "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\" = \"ready\"" type="ready"
    /// @type.node source="\"ready\"" type="ready"

    value: int32;
    /// @type.symbol symbol=Ready.value source="value: int32" type=int32

}

newtype State = Pending | Ready;
/// @type.symbol symbol=State source="newtype State = Pending | Ready" type=State
/// @definition.newtype symbol=State source="newtype State = Pending | Ready" backing=Pending | Ready constructors=[(Pending) => State, (Ready) => State, (Pending | Ready) => State]
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Ready target=Ready

function read(state: State): int32 {
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=State
/// @resolution.name source=State target=State

    if (state["kind"] === "ready") {
    /// @type.node source="state[\"kind\"] === \"ready\"" type=boolean
    /// @type.node source="state[\"kind\"]" type="pending" | "ready"
    /// @type.node source=state type=State
    /// @resolution.name source=state target=read.state
    /// @resolution.operator source="state[\"kind\"] === \"ready\"" type=boolean operator="===" kind=builtin operands=[state["kind"] as "pending" | "ready" families=(string), "ready" as "pending" | "ready" families=(string)]
    /// @resolution.place source="state[\"kind\"]" placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source="state[\"kind\"]" root=read.state keys=[kind]
    /// @resolution.subscript source="state[\"kind\"]" type="pending" | "ready" kind=member target="receiver=State, target=discriminant(Pending | Ready, kind, cases=[Pending: pending, Ready: ready], \"pending\" | \"ready\") adjustments=(newtype.payload(State, Pending | Ready)), type=\"pending\" | \"ready\""
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @type.node source="\"kind\"" type="kind"
    /// @type.node source="\"ready\"" type="ready"

        return state.value;
        /// @type.node source=state type=Ready
        /// @type.node source=state.value type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.value receiver=Ready type=int32 kind=field target_receiver=Ready key=value target=Ready.value target_type=int32
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.value root=read.state keys=[value]

    }

    return state.waiting;
    /// @type.node source=state type=Pending
    /// @type.node source=state.waiting type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.waiting receiver=Pending type=int32 kind=field target_receiver=Pending key=waiting target=Pending.waiting target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.waiting placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.waiting root=read.state keys=[waiting]

}
"#);
}

/// Narrow a borrowed struct union through its discriminant.
#[test]
fn test_narrow_borrowed_struct_union_discriminant() {
    let session = TestSession::single(
        r#"
struct Pending {
    kind: "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready";
    value: int32;
}

type State = Pending | Ready;

function read(state: &readonly State): int32 {
    if (state.kind === "ready") {
        return state.value;
    }

    return state.waiting;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
struct Pending {
    kind: "pending";
    waiting: int32;
}

struct Ready {
    kind: "ready";
    value: int32;
}

type State = Pending | Ready;

function read<'a>(state: &'a readonly (Pending | Ready)): int32 {
    if (
        (state.kind as readonly "pending" | readonly "ready" | "ready") ===
        ("ready" as readonly "pending" | readonly "ready" | "ready")
    ) {
        return state.value;
    }

    return state.waiting;
}

=== dir ===
struct Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.waiting source="waiting: int32" key=waiting type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    waiting: int32;
    /// @type.symbol symbol=Pending.waiting source="waiting: int32" type=int32

}

struct Ready {
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready
/// @definition.field symbol=Ready.kind source="kind: \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: int32" key=value type=int32

    kind: "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\"" type="ready"

    value: int32;
    /// @type.symbol symbol=Ready.value source="value: int32" type=int32

}

type State = Pending | Ready;
/// @type.symbol symbol=State source="type State = Pending | Ready" type=Pending | Ready
/// @definition.type symbol=State source="type State = Pending | Ready" value=Pending | Ready
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Ready target=Ready

function read(state: &readonly State): int32 {
/// @generic.template symbol=read parameters=('a)
/// @type.symbol symbol=read type=<read.'a>(&read.'a readonly Pending | Ready) => int32
/// @type.symbol symbol=read type=<read.'a>(&read.'a readonly State) => int32
/// @type.symbol symbol=read.state source="state: &readonly State" type=&read.'a readonly Pending | Ready
/// @resolution.name source=State target=State

    if (state.kind === "ready") {
    /// @type.node source="state.kind === \"ready\"" type=boolean
    /// @type.node source=state type=&read.'a readonly Pending | Ready
    /// @type.node source=state.kind type=Readonly<"pending"> | Readonly<"ready">
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind receiver=&read.'a readonly Pending | Ready type=Readonly<"pending"> | Readonly<"ready"> kind=projection target="discriminant(Pending | Ready, kind, cases=[Pending: pending, Ready: ready], Readonly<\"pending\"> | Readonly<\"ready\">)"
    /// @resolution.operator source="state.kind === \"ready\"" type=boolean operator="===" kind=builtin operands=[state.kind as Readonly<"pending"> | Readonly<"ready"> | "ready", "ready" as Readonly<"pending"> | Readonly<"ready"> | "ready"]
    /// @resolution.place source=state placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"ready\"" type="ready"

        return state.value;
        /// @type.node source=state type=&read.'a readonly Ready
        /// @type.node source=state.value type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.value receiver=&read.'a readonly Ready type=int32 kind=field target_receiver=&read.'a readonly Ready key=value target=Ready.value target_type=int32
        /// @resolution.place source=state placement="local" lifetime=read.'a access="readonly"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.value placement="local" lifetime=read.'a access="readonly"
        /// @resolution.access source=state.value root=read.state keys=[value]

    }

    return state.waiting;
    /// @type.node source=state type=&read.'a readonly Pending
    /// @type.node source=state.waiting type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.waiting receiver=&read.'a readonly Pending type=int32 kind=field target_receiver=&read.'a readonly Pending key=waiting target=Pending.waiting target_type=int32
    /// @resolution.place source=state placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.waiting placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=state.waiting root=read.state keys=[waiting]

}
"#);
}

/// Narrow an interface union through a strict inequality discriminant test.
#[test]
fn test_strict_inequality_narrows_an_interface_union() {
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
    if (state.kind !== "pending") {
        return state.value;
    }

    return state.reactions;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: Dynamic<Pending> | Dynamic<Fulfilled>): int32 {
    if (state.kind !== ("pending" as "pending" | "fulfilled")) {
        return state.value;
    }

    return state.reactions;
}

=== dir ===
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.interface symbol=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32

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
/// @type.symbol symbol=read type=(Dynamic<Pending> | Dynamic<Fulfilled>) => int32
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State

    if (state.kind !== "pending") {
    /// @type.node source="state.kind !== \"pending\"" type=boolean
    /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind type="pending" | "fulfilled" kind=union arms=[receiver=Dynamic<Pending>, target=field(receiver=dynamic(Dynamic<Pending>, constraint=Pending), target=Pending.kind, type="pending"), type="pending", receiver=Dynamic<Fulfilled>, target=field(receiver=dynamic(Dynamic<Fulfilled>, constraint=Fulfilled), target=Fulfilled.kind, type="fulfilled"), type="fulfilled"]
    /// @resolution.operator source="state.kind !== \"pending\"" type=boolean operator="!==" kind=builtin operands=[state.kind as "pending" | "fulfilled" families=(string), "pending" as "pending" | "fulfilled" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"pending\"" type="pending"

        return state.value;
        /// @type.node source=state type=Dynamic<Fulfilled>
        /// @type.node source=state.value type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.value receiver=Dynamic<Fulfilled> type=int32 kind=field target_receiver=Dynamic<Fulfilled> dispatch=dynamic constraint=Fulfilled key=value target=Fulfilled.value target_type=int32
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.value root=read.state keys=[value]

    }

    return state.reactions;
    /// @type.node source=state type=Dynamic<Pending>
    /// @type.node source=state.reactions type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.reactions receiver=Dynamic<Pending> type=int32 kind=field target_receiver=Dynamic<Pending> dispatch=dynamic constraint=Pending key=reactions target=Pending.reactions target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.reactions placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.reactions root=read.state keys=[reactions]

}
"#);
}
