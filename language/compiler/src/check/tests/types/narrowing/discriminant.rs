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

function read(state: State): int32 {
    if (state.kind == "pending") {
        return state.reactions;
    }

    return state.value;
}

=== checked ===
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
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=State reduced=Pending | Fulfilled
/// @resolution.name source=State target=State

    if (state.kind == "pending") {
    /// @type.node source="state.kind == \"pending\"" type=boolean
    /// @type.node source=state type=State reduced=Pending | Fulfilled
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind receiver=Pending | Fulfilled kind=universal targets=[Pending.kind, Fulfilled.kind]
    /// @resolution.call source="state.kind == \"pending\"" parameters=() return=boolean kind=builtin builtin=binary.equal
    /// @type.node source="\"pending\"" type="pending"

        return state.reactions;
        /// @type.node source=state type=Pending
        /// @type.node source=state.reactions type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.reactions receiver=Pending kind=symbol target=Pending.reactions

    }

    return state.value;
    /// @type.node source=state type=Fulfilled
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Fulfilled kind=symbol target=Fulfilled.value

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

    session.assert_dir_checked(
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

function read(state: State): int32 {
    if (state["kind"] == "pending") {
        return state.reactions;
    }

    return state.value;
}

=== checked ===
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
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=State reduced=Pending | Fulfilled
/// @resolution.name source=State target=State

    if (state["kind"] == "pending") {
    /// @type.node source="state[\"kind\"] == \"pending\"" type=boolean
    /// @type.node source="state[\"kind\"]" type="pending" | "fulfilled"
    /// @type.node source=state type=State reduced=Pending | Fulfilled
    /// @resolution.name source=state target=read.state
    /// @resolution.member source="state[\"kind\"]" receiver=Pending | Fulfilled kind=universal targets=[Pending.kind, Fulfilled.kind]
    /// @resolution.call source="state[\"kind\"] == \"pending\"" parameters=() return=boolean kind=builtin builtin=binary.equal
    /// @type.node source="\"kind\"" type="kind"
    /// @type.node source="\"pending\"" type="pending"

        return state.reactions;
        /// @type.node source=state type=Pending
        /// @type.node source=state.reactions type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.reactions receiver=Pending kind=symbol target=Pending.reactions

    }

    return state.value;
    /// @type.node source=state type=Fulfilled
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Fulfilled kind=symbol target=Fulfilled.value

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

    session.assert_dir_checked_and_diagnostics(
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

function read(initial: State, next: State): int32 {
    let state: State = initial;
    if (state.kind == "pending") {
        state = next;

        return state.reactions;
    }

    return state.value;
}

=== checked ===
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
/// @type.symbol symbol=read type=(State, State) => int32
/// @type.symbol symbol=read.initial source="initial: State" type=State reduced=Pending | Fulfilled
/// @resolution.name source=State target=State
/// @type.symbol symbol=read.next source="next: State" type=State reduced=Pending | Fulfilled
/// @resolution.name source=State target=State

    let state = initial;
    /// @type.symbol symbol=read.state source=state type=State reduced=Pending | Fulfilled
    /// @type.node source=initial type=State reduced=Pending | Fulfilled
    /// @resolution.name source=initial target=read.initial

    if (state.kind == "pending") {
    /// @type.node source="state.kind == \"pending\"" type=boolean
    /// @type.node source=state type=State reduced=Pending | Fulfilled
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind receiver=Pending | Fulfilled kind=universal targets=[Pending.kind, Fulfilled.kind]
    /// @resolution.call source="state.kind == \"pending\"" parameters=() return=boolean kind=builtin builtin=binary.equal
    /// @type.node source="\"pending\"" type="pending"

        state = next;
        /// @type.node source="state = next" type=State reduced=Pending | Fulfilled
        /// @type.node source=state type=State reduced=Pending | Fulfilled
        /// @resolution.pattern.assign source=state kind=place place=binding(read.state) type=State
        /// @type.node source=next type=State reduced=Pending | Fulfilled
        /// @resolution.name source=next target=read.next

        return state.reactions;
        /// @type.node source=state type=State reduced=Pending | Fulfilled
        /// @type.node source=state.reactions type=<error>
        /// @resolution.name source=state target=read.state

    }

    return state.value;
    /// @type.node source=state type=Fulfilled
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Fulfilled kind=symbol target=Fulfilled.value

}
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'reactions' does not exist on type 'State'"
/// @diagnostic.label line=19 column=22 span="reactions" line_source="return state.reactions;"
"#,
    );
}

#[test]
fn test_early_return_narrows_a_member_discriminant() {
    let session = TestSession::single(
        r#"
struct Waiter<T> {
    value: T;
    next: Waiter<T> | undefined;
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Waiter<out T> {
    value: T;
    next: Waiter<T> | undefined;
}

struct Pending<out T> {
    kind: "pending";
    head: Waiter<T> | undefined;
    tail: Waiter<T> | undefined;
}

struct Fulfilled<out T> {
    kind: "fulfilled";
    value: T;
}

type State<T> = Pending<T> | Fulfilled<T>;

class Cell<in T> {
    state: State<T>;

    constructor(pending: Pending<T>): this {
        this.state = pending as State<T>;
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
            this.state.head = waiter;
            this.state.tail = waiter;
        } else {
            this.state.tail.next = waiter as Waiter<T> | undefined;
            this.state.tail = waiter as Waiter<T> | undefined;
        }
    }
}

=== checked ===
struct Waiter<T> {
/// @generic.template symbol=Waiter parameters=(out T#1)
/// @type.symbol symbol=Waiter type=Waiter
/// @definition.struct symbol=Waiter template=(out T#1)
/// @definition.field symbol=Waiter.next source="next: Waiter<T> | undefined" key=next type=Waiter<T#1> | undefined
/// @definition.field symbol=Waiter.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Waiter.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Waiter.value source="value: T" type=T#1
    /// @resolution.name source=T target=Waiter.T

    next: Waiter<T> | undefined;
    /// @type.symbol symbol=Waiter.next source="next: Waiter<T> | undefined" type=Waiter<T#1> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Waiter.T

}

struct Pending<T> {
/// @generic.template symbol=Pending parameters=(out T#2)
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending template=(out T#2)
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
/// @generic.template symbol=Cell parameters=(in T#5)
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell template=(in T#5)
/// @definition.field symbol=Cell.state source="state: State<T>" key=state type=State<T#5>
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(Pending<T#5>) => this
/// @definition.method symbol=Cell.consume slot=consume type=(this: this, T#5) => void
/// @definition.method symbol=Cell.poke slot=poke type=(this: this, Waiter<T#5>) => void
/// @type.symbol symbol=Cell.T source=T type=T#5

    state: State<T>;
    /// @type.symbol symbol=Cell.state source="state: State<T>" type=State<T#5> reduced=Pending<T#5> | Fulfilled<T#5>
    /// @resolution.name source=State target=State
    /// @resolution.name source=T target=Cell.T

    constructor(pending: Pending<T>) {
    /// @type.symbol symbol=Cell.constructor type=(Pending<T#5>) => this
    /// @type.symbol symbol=Cell.constructor.pending source="pending: Pending<T>" type=Pending<T#5>
    /// @resolution.name source=Pending target=Pending
    /// @resolution.name source=T target=Cell.T

        this.state = pending;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
        /// @resolution.pattern.assign source=this.state kind=place place=field(Cell.state) type=State<T#5>
        /// @resolution.name source=pending target=Cell.constructor.pending

    }

    consume(value: T): void {
    /// @type.symbol symbol=Cell.consume type=(this: this, T#5) => void
    /// @type.symbol symbol=Cell.consume.value source="value: T" type=T#5
    /// @resolution.name source=T target=Cell.T

        value;
        /// @resolution.name source=value target=Cell.consume.value

    }

    poke(waiter: Waiter<T>): void {
    /// @type.symbol symbol=Cell.poke type=(this: this, Waiter<T#5>) => void
    /// @type.symbol symbol=Cell.poke.waiter source="waiter: Waiter<T>" type=Waiter<T#5>
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Cell.T

        if (this.state.kind == "fulfilled") {
        /// @resolution.member source=this.state receiver=Cell<T#5> kind=symbol target=Cell.state
        /// @resolution.member source=this.state.kind receiver=Pending<T#5> | Fulfilled<T#5> kind=universal targets=[Pending.kind, Fulfilled.kind]
        /// @resolution.call source="this.state.kind == \"fulfilled\"" parameters=() return=boolean kind=builtin builtin=binary.equal
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>

            this.consume(this.state.value);
            /// @resolution.member source=this.consume receiver=Cell<T#5> kind=symbol target=Cell.consume
            /// @resolution.call source=this.consume(this.state.value) parameters=(T#5) arguments=(provided(this.state.value) as T#5) return=void kind=symbol target=Cell.consume receiver=Cell<T#5> instance=Cell<T#5>.consume
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @generic.instance source=this.consume(this.state.value) id=Cell<T#5>.consume
            /// @resolution.member source=this.state receiver=Cell<T#5> kind=symbol target=Cell.state
            /// @resolution.member source=this.state.value receiver=Fulfilled<T#5> kind=symbol target=Fulfilled.value
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>

            return;
        }

        if (this.state.tail == undefined) {
        /// @resolution.member source=this.state receiver=Cell<T#5> kind=symbol target=Cell.state
        /// @resolution.member source=this.state.tail receiver=Pending<T#5> kind=symbol target=Pending.tail
        /// @resolution.call source="this.state.tail == undefined" parameters=() return=boolean kind=builtin builtin=binary.equal
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>

            this.state.head = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> kind=symbol target=Cell.state
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.name source=waiter target=Cell.poke.waiter

            this.state.tail = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> kind=symbol target=Cell.state
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.name source=waiter target=Cell.poke.waiter

        } else {
            this.state.tail.next = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> kind=symbol target=Cell.state
            /// @resolution.member source=this.state.tail receiver=Pending<T#5> kind=symbol target=Pending.tail
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.pattern.assign source=this.state.tail.next kind=place place=field(Waiter.next) type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter

            this.state.tail = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> kind=symbol target=Cell.state
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.pattern.assign source=this.state.tail kind=place place=field(Pending.tail) type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter

        }
    }
}

/// @generic.instance id=Cell<T#5>.consume template=Cell.consume arguments=(T#5)
/// @generic.instance id=Fulfilled<T#4> template=Fulfilled arguments=(T#4)
/// @generic.instance id=Pending<T#4> template=Pending arguments=(T#4)
/// @generic.instance id=Pending<T#5> template=Pending arguments=(T#5)
/// @generic.instance id=State<T#5> template=State arguments=(T#5)
/// @generic.instance id=Waiter<T#1> template=Waiter arguments=(T#1)
/// @generic.instance id=Waiter<T#2> template=Waiter arguments=(T#2)
/// @generic.instance id=Waiter<T#5> template=Waiter arguments=(T#5)
"#,
    );
}
