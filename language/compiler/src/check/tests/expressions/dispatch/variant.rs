use crate::tests::{DirRows, TestSession};

#[test]
fn test_tagged_variant_constructs_from_its_payload_shape() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Edge =
    | { kind: "bounded"; limit: int32 }
    | { kind: "open" };

function bounded(limit: int32): Edge {
    Edge.Bounded({ limit })
}

function open(): Edge {
    Edge.Open
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Edge = { kind: "bounded"; limit: int32 } | { kind: "open" };

function bounded(limit: int32): Edge {
    Edge.Bounded({ limit })
}

function open(): Edge {
    Edge.Open
}

=== checked ===
@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Edge =
/// @type.symbol symbol=Edge type=Edge
/// @type.symbol symbol=Edge.Bounded type=({ limit: int32 }) => Edge.Bounded
/// @type.symbol symbol=Edge.Open type=Edge.Open
/// @definition.newtype symbol=Edge discriminator=kind backing={ kind: "bounded"; limit: int32 } | { kind: "open" }
/// @definition.variant symbol=Edge.Bounded key=Bounded discriminant=bounded backing={ kind: "bounded"; limit: int32 } argument={ limit: int32 }
/// @definition.variant symbol=Edge.Open key=Open discriminant=open backing={ kind: "open" }

    | { kind: "bounded"; limit: int32 }
    | { kind: "open" };

function bounded(limit: int32): Edge {
/// @type.symbol symbol=bounded type=(int32) => Edge
/// @type.symbol symbol=bounded.limit source="limit: int32" type=int32
/// @resolution.name source=Edge target=Edge

    Edge.Bounded({ limit })
    /// @type.node source="Edge.Bounded({ limit })" type=Edge.Bounded
    /// @type.node source=Edge type=Edge
    /// @type.node source=Edge.Bounded type=({ limit: int32 }) => Edge.Bounded
    /// @resolution.name source=Edge target=Edge
    /// @resolution.member source=Edge.Bounded receiver=Edge type=({ limit: int32 }) => Edge.Bounded kind=symbol target_receiver=Edge target=Edge.Bounded
    /// @resolution.construct source="Edge.Bounded({ limit })" parameters=({ limit: int32 }) arguments=(provided({ limit }) as { limit: int32 }) return=Edge.Bounded kind=variant owner=Edge variant=Bounded backing={ kind: "bounded"; limit: int32 } argument={ limit: int32 } discriminant=bounded
    /// @type.node source={ limit } type={ limit: int32 }
    /// @type.node source=limit type=int32
    /// @resolution.name source=limit target=bounded.limit
    /// @resolution.place source=limit placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=limit root=bounded.limit

}

function open(): Edge {
/// @type.symbol symbol=open type=() => Edge
/// @resolution.name source=Edge target=Edge

    Edge.Open
    /// @type.node source=Edge type=Edge
    /// @type.node source=Edge.Open type=Edge.Open
    /// @resolution.name source=Edge target=Edge
    /// @resolution.member source=Edge.Open receiver=Edge type=Edge.Open kind=symbol target_receiver=Edge target=Edge.Open

}
"#,
    );
}

#[test]
fn test_tagged_variant_constructs_from_nominal_arms() {
    let session = TestSession::single(
        r#"
struct Borrowed {
    kind: "borrowed" = "borrowed";
    value: string;
}

struct Owned {
    kind: "owned" = "owned";
    value: string;
}

@derive(Tagged)
newtype Cow = Borrowed | Owned;

const borrowed: Cow = Cow.Borrowed({ value: "value" });
const owned: Cow = Cow.Owned({ value: "value" });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
struct Borrowed {
    kind: "borrowed" = "borrowed";
    value: string;
}

struct Owned {
    kind: "owned" = "owned";
    value: string;
}

@derive(Tagged)
newtype Cow = Borrowed | Owned;

const borrowed: Cow = Cow.Borrowed({ value: "value" });
const owned: Cow = Cow.Owned({ value: "value" });

=== checked ===
struct Borrowed {
/// @type.symbol symbol=Borrowed type=Borrowed
/// @definition.struct symbol=Borrowed
/// @definition.field symbol=Borrowed.kind source="kind: \"borrowed\" = \"borrowed\"" key=kind type="borrowed"
/// @definition.field symbol=Borrowed.value source="value: string" key=value type=string

    kind: "borrowed" = "borrowed";
    /// @type.symbol symbol=Borrowed.kind source="kind: \"borrowed\" = \"borrowed\"" type="borrowed"
    /// @type.node source="\"borrowed\"" type="borrowed"

    value: string;
    /// @type.symbol symbol=Borrowed.value source="value: string" type=string

}

struct Owned {
/// @type.symbol symbol=Owned type=Owned
/// @definition.struct symbol=Owned
/// @definition.field symbol=Owned.kind source="kind: \"owned\" = \"owned\"" key=kind type="owned"
/// @definition.field symbol=Owned.value source="value: string" key=value type=string

    kind: "owned" = "owned";
    /// @type.symbol symbol=Owned.kind source="kind: \"owned\" = \"owned\"" type="owned"
    /// @type.node source="\"owned\"" type="owned"

    value: string;
    /// @type.symbol symbol=Owned.value source="value: string" type=string

}

@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="newtype Cow = Borrowed | Owned" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Cow = Borrowed | Owned;
/// @type.symbol symbol=Cow source="newtype Cow = Borrowed | Owned" type=Cow
/// @type.symbol symbol=Cow.Borrowed type=({ value: string }) => Cow.Borrowed
/// @type.symbol symbol=Cow.Owned type=({ value: string }) => Cow.Owned
/// @definition.newtype symbol=Cow source="newtype Cow = Borrowed | Owned" discriminator=kind backing=Borrowed | Owned
/// @definition.variant symbol=Cow.Borrowed source="newtype Cow = Borrowed | Owned" key=Borrowed discriminant=borrowed backing=Borrowed argument={ value: string }
/// @definition.variant symbol=Cow.Owned source="newtype Cow = Borrowed | Owned" key=Owned discriminant=owned backing=Owned argument={ value: string }
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Owned target=Owned

const borrowed: Cow = Cow.Borrowed({ value: "value" });
/// @type.symbol symbol=borrowed source=borrowed type=Cow
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=Cow target=Cow
/// @type.node source="Cow.Borrowed({ value: \"value\" })" type=Cow.Borrowed
/// @type.node source=Cow type=Cow
/// @type.node source=Cow.Borrowed type=({ value: string }) => Cow.Borrowed
/// @resolution.name source=Cow target=Cow
/// @resolution.member source=Cow.Borrowed receiver=Cow type=({ value: string }) => Cow.Borrowed kind=symbol target_receiver=Cow target=Cow.Borrowed
/// @resolution.construct source="Cow.Borrowed({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Cow.Borrowed kind=variant owner=Cow variant=Borrowed backing=Borrowed argument={ value: string } discriminant=borrowed
/// @type.node source={ value: "value" } type={ value: string }
/// @type.node source="\"value\"" type="value"

const owned: Cow = Cow.Owned({ value: "value" });
/// @type.symbol symbol=owned source=owned type=Cow
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Cow target=Cow
/// @type.node source="Cow.Owned({ value: \"value\" })" type=Cow.Owned
/// @type.node source=Cow type=Cow
/// @type.node source=Cow.Owned type=({ value: string }) => Cow.Owned
/// @resolution.name source=Cow target=Cow
/// @resolution.member source=Cow.Owned receiver=Cow type=({ value: string }) => Cow.Owned kind=symbol target_receiver=Cow target=Cow.Owned
/// @resolution.construct source="Cow.Owned({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Cow.Owned kind=variant owner=Cow variant=Owned backing=Owned argument={ value: string } discriminant=owned
/// @type.node source={ value: "value" } type={ value: string }
/// @type.node source="\"value\"" type="value"
"#,
    );
}

#[test]
fn test_tagged_variant_constructs_from_nested_newtypes() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Read = { kind: "read"; value: string };

@derive(Tagged)
newtype Write = { kind: "write"; value: string };

@derive(Tagged)
newtype Action = Read | Write;

const read: Action = Action.Read({ value: "value" });
const write: Action = Action.Write({ value: "value" });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Read = { kind: "read"; value: string };

@derive(Tagged)
newtype Write = { kind: "write"; value: string };

@derive(Tagged)
newtype Action = Read | Write;

const read: Action = Action.Read({ value: "value" });
const write: Action = Action.Write({ value: "value" });

=== checked ===
@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="newtype Read = { kind: \"read\"; value: string }" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Read = { kind: "read"; value: string };
/// @type.symbol symbol=Read source="newtype Read = { kind: \"read\"; value: string }" type=Read
/// @type.symbol symbol=Read.Read type=({ value: string }) => Read.Read
/// @definition.newtype symbol=Read source="newtype Read = { kind: \"read\"; value: string }" discriminator=kind backing={ kind: "read"; value: string }
/// @definition.variant symbol=Read.Read source="newtype Read = { kind: \"read\"; value: string }" key=Read discriminant=read backing={ kind: "read"; value: string } argument={ value: string }

@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="newtype Write = { kind: \"write\"; value: string }" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Write = { kind: "write"; value: string };
/// @type.symbol symbol=Write source="newtype Write = { kind: \"write\"; value: string }" type=Write
/// @type.symbol symbol=Write.Write type=({ value: string }) => Write.Write
/// @definition.newtype symbol=Write source="newtype Write = { kind: \"write\"; value: string }" discriminator=kind backing={ kind: "write"; value: string }
/// @definition.variant symbol=Write.Write source="newtype Write = { kind: \"write\"; value: string }" key=Write discriminant=write backing={ kind: "write"; value: string } argument={ value: string }

@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="newtype Action = Read | Write" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Action = Read | Write;
/// @type.symbol symbol=Action source="newtype Action = Read | Write" type=Action
/// @type.symbol symbol=Action.Read type=({ value: string }) => Action.Read
/// @type.symbol symbol=Action.Write type=({ value: string }) => Action.Write
/// @definition.newtype symbol=Action source="newtype Action = Read | Write" discriminator=kind backing=Read | Write
/// @definition.variant symbol=Action.Read source="newtype Action = Read | Write" key=Read discriminant=read backing={ kind: "read"; value: string } argument={ value: string }
/// @definition.variant symbol=Action.Write source="newtype Action = Read | Write" key=Write discriminant=write backing={ kind: "write"; value: string } argument={ value: string }
/// @resolution.name source=Read target=Read
/// @resolution.name source=Write target=Write

const read: Action = Action.Read({ value: "value" });
/// @type.symbol symbol=read source=read type=Action
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Action target=Action
/// @type.node source="Action.Read({ value: \"value\" })" type=Action.Read
/// @type.node source=Action type=Action
/// @type.node source=Action.Read type=({ value: string }) => Action.Read
/// @resolution.name source=Action target=Action
/// @resolution.member source=Action.Read receiver=Action type=({ value: string }) => Action.Read kind=symbol target_receiver=Action target=Action.Read
/// @resolution.construct source="Action.Read({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Action.Read kind=variant owner=Action variant=Read backing={ kind: "read"; value: string } argument={ value: string } discriminant=read
/// @type.node source={ value: "value" } type={ value: string }
/// @type.node source="\"value\"" type="value"

const write: Action = Action.Write({ value: "value" });
/// @type.symbol symbol=write source=write type=Action
/// @resolution.pattern source=write kind=binding target=write
/// @resolution.name source=Action target=Action
/// @type.node source="Action.Write({ value: \"value\" })" type=Action.Write
/// @type.node source=Action type=Action
/// @type.node source=Action.Write type=({ value: string }) => Action.Write
/// @resolution.name source=Action target=Action
/// @resolution.member source=Action.Write receiver=Action type=({ value: string }) => Action.Write kind=symbol target_receiver=Action target=Action.Write
/// @resolution.construct source="Action.Write({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Action.Write kind=variant owner=Action variant=Write backing={ kind: "write"; value: string } argument={ value: string } discriminant=write
/// @type.node source={ value: "value" } type={ value: string }
/// @type.node source="\"value\"" type="value"
"#,
    );
}

#[test]
fn test_tagged_variant_omits_optional_payload() {
    let session = TestSession::single(
        r#"
struct Ready {
    kind: "ready" = "ready";
    value: int32 = 1;
}

@derive(Tagged)
newtype State = Ready;

const state: State = State.Ready();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Ready {
    kind: "ready" = "ready";
    value: int32 = 1;
}

@derive(Tagged)
newtype State = Ready;

const state: State = State.Ready();

=== checked ===
struct Ready {
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready
/// @definition.field symbol=Ready.kind source="kind: \"ready\" = \"ready\"" key=kind type="ready"
/// @definition.field symbol=Ready.value source="value: int32 = 1" key=value type=int32

    kind: "ready" = "ready";
    /// @type.symbol symbol=Ready.kind source="kind: \"ready\" = \"ready\"" type="ready"

    value: int32 = 1;
    /// @type.symbol symbol=Ready.value source="value: int32 = 1" type=int32

}

@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype State = Ready;
/// @type.symbol symbol=State source="newtype State = Ready" type=State
/// @type.symbol symbol=State.Ready type=({ value?: int32 }?) => State.Ready
/// @definition.newtype symbol=State source="newtype State = Ready" discriminator=kind backing=Ready
/// @definition.variant symbol=State.Ready source="newtype State = Ready" key=Ready discriminant=ready backing=Ready argument={ value?: int32 }
/// @resolution.name source=Ready target=Ready

const state: State = State.Ready();
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State
/// @resolution.name source=State target=State
/// @resolution.member source=State.Ready receiver=State type=({ value?: int32 }?) => State.Ready kind=symbol target_receiver=State target=State.Ready
/// @resolution.construct source=State.Ready() parameters=({ value?: int32 }) arguments=(omitted as { value?: int32 }) return=State.Ready kind=variant owner=State variant=Ready backing=Ready argument={ value?: int32 } discriminant=ready
"#,
    );
}

#[test]
fn test_calls_stored_tagged_variant_constructor() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Edge = { kind: "bounded"; limit: int32 };

const bounded = Edge.Bounded;
const edge: Edge = bounded({ limit: 1 });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Edge = { kind: "bounded"; limit: int32 };

const bounded = Edge.Bounded;
const edge: Edge = bounded({ limit: 1 });

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Edge = { kind: "bounded"; limit: int32 };
/// @type.symbol symbol=Edge source="newtype Edge = { kind: \"bounded\"; limit: int32 }" type=Edge
/// @type.symbol symbol=Edge.Bounded type=({ limit: int32 }) => Edge.Bounded
/// @definition.newtype symbol=Edge source="newtype Edge = { kind: \"bounded\"; limit: int32 }" discriminator=kind backing={ kind: "bounded"; limit: int32 }
/// @definition.variant symbol=Edge.Bounded source="newtype Edge = { kind: \"bounded\"; limit: int32 }" key=Bounded discriminant=bounded backing={ kind: "bounded"; limit: int32 } argument={ limit: int32 }

const bounded = Edge.Bounded;
/// @type.symbol symbol=bounded source=bounded type=({ limit: int32 }) => Edge.Bounded
/// @resolution.pattern source=bounded kind=binding target=bounded
/// @resolution.name source=Edge target=Edge
/// @resolution.member source=Edge.Bounded receiver=Edge type=({ limit: int32 }) => Edge.Bounded kind=symbol target_receiver=Edge target=Edge.Bounded

const edge: Edge = bounded({ limit: 1 });
/// @type.symbol symbol=edge source=edge type=Edge
/// @resolution.pattern source=edge kind=binding target=edge
/// @resolution.name source=Edge target=Edge
/// @resolution.name source=bounded target=bounded
/// @resolution.call source="bounded({ limit: 1 })" parameters=({ limit: int32 }) arguments=(provided({ limit: 1 }) as { limit: int32 }) return=Edge.Bounded kind=expression target=expression
/// @resolution.place source=bounded placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=bounded root=bounded
"#,
    );
}

#[test]
fn test_await_unwraps_the_newtype_backing() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

newtype Wrapper<T: Copy> = Promise<T>;

extension<T: Copy> of Wrapper<T> {
    async take(): Promise<T> {
        const value = await this;
        value
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

newtype Wrapper<in out T: Copy> = Promise<T>;

extension<T: Copy> of Wrapper<T> {
    async take(): Promise<T> {
        const value: T = await this;
        value
    }
}

=== checked ===
import { Promise } from "destack:async";

newtype Wrapper<T: Copy> = Promise<T>;
/// @generic.template symbol=Wrapper parameters=(in out T#1: Copy)
/// @type.symbol symbol=Wrapper source="newtype Wrapper<T: Copy> = Promise<T>" type=Wrapper
/// @definition.newtype symbol=Wrapper source="newtype Wrapper<T: Copy> = Promise<T>" template=(in out T#1: Copy) backing=Promise<T#1> constructors=[<T#1>(Promise<T#1>) => Wrapper<T#1>]
/// @type.symbol symbol=Wrapper.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=memory.capability.Copy
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.name source=T target=Wrapper.T

extension<T: Copy> of Wrapper<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2: Copy)
/// @definition.extension symbol=<module>#2 form=local target=Wrapper<T#2>
/// @definition.method symbol=take slot=take type=async (this: this) => Promise<T#2>
/// @type.symbol symbol=T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=memory.capability.Copy
/// @resolution.name source=Wrapper target=Wrapper
/// @resolution.name source=T target=T

    async take(): Promise<T> {
    /// @type.symbol symbol=take type=async (this: this) => Promise<T#2>
    /// @resolution.name source=Promise target=async.promise.Promise
    /// @resolution.name source=T target=T

        const value = await this;
        /// @type.symbol symbol=take.value source=value type=T#2
        /// @resolution.pattern source=value kind=binding target=take.value
        /// @type.node source="await this" type=T#2
        /// @type.node source=this type=Wrapper<T#2>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Wrapper<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instance source=this id=Wrapper<T#2>

        value
        /// @type.node source=value type=T#2
        /// @resolution.name source=value target=take.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=take.value

    }
}

/// @generic.instance id=Promise<T#2> template=async.promise.Promise arguments=(T#2)
/// @generic.instance id=Wrapper<T#2> template=Wrapper arguments=(T#2)
"#,
    );
}

#[test]
fn test_tagged_variant_constructs_from_applied_owner() {
    let session = TestSession::single(
        r#"
struct Included<T> {
    kind: "included" = "included";
    value: T;
}

struct Excluded<E> {
    kind: "excluded" = "excluded";
    error: E;
}

@derive(Tagged)
newtype Bound<T, E> = Included<T> | Excluded<E>;

function included(value: string): Bound<string, Error> {
    Bound<string, Error>.Included({ value })
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Included<out T> {
    kind: "included" = "included";
    value: T;
}

struct Excluded<out E> {
    kind: "excluded" = "excluded";
    error: E;
}

@derive(Tagged)
newtype Bound<out T, out E> = Included<T> | Excluded<E>;

function included(value: string): Bound<string, Error> {
    Bound<string, Error>.Included({ value })
}

=== checked ===
struct Included<T> {
/// @generic.template symbol=Included parameters=(out T#1)
/// @type.symbol symbol=Included type=Included
/// @definition.struct symbol=Included template=(out T#1)
/// @definition.field symbol=Included.kind source="kind: \"included\" = \"included\"" key=kind type="included"
/// @definition.field symbol=Included.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Included.T source=T type=T#1

    kind: "included" = "included";
    /// @type.symbol symbol=Included.kind source="kind: \"included\" = \"included\"" type="included"
    /// @type.node source="\"included\"" type="included"

    value: T;
    /// @type.symbol symbol=Included.value source="value: T" type=T#1
    /// @resolution.name source=T target=Included.T

}

struct Excluded<E> {
/// @generic.template symbol=Excluded parameters=(out E#1)
/// @type.symbol symbol=Excluded type=Excluded
/// @definition.struct symbol=Excluded template=(out E#1)
/// @definition.field symbol=Excluded.error source="error: E" key=error type=E#1
/// @definition.field symbol=Excluded.kind source="kind: \"excluded\" = \"excluded\"" key=kind type="excluded"
/// @type.symbol symbol=Excluded.E source=E type=E#1

    kind: "excluded" = "excluded";
    /// @type.symbol symbol=Excluded.kind source="kind: \"excluded\" = \"excluded\"" type="excluded"
    /// @type.node source="\"excluded\"" type="excluded"

    error: E;
    /// @type.symbol symbol=Excluded.error source="error: E" type=E#1
    /// @resolution.name source=E target=Excluded.E

}

@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Bound<T, E> = Included<T> | Excluded<E>;
/// @generic.template symbol=Bound parameters=(out T#2, out E#2)
/// @type.symbol symbol=Bound source="newtype Bound<T, E> = Included<T> | Excluded<E>" type=Bound
/// @type.symbol symbol=Bound.Excluded type=<T#2, E#2>({ error: E#2 }) => Bound.Excluded<T#2, E#2>
/// @type.symbol symbol=Bound.Included type=<T#2, E#2>({ value: T#2 }) => Bound.Included<T#2, E#2>
/// @definition.newtype symbol=Bound source="newtype Bound<T, E> = Included<T> | Excluded<E>" template=(out T#2, out E#2) discriminator=kind backing=Included<T#2> | Excluded<E#2>
/// @definition.variant symbol=Bound.Excluded source="newtype Bound<T, E> = Included<T> | Excluded<E>" key=Excluded discriminant=excluded backing=Excluded<E#2> argument={ error: E#2 }
/// @definition.variant symbol=Bound.Included source="newtype Bound<T, E> = Included<T> | Excluded<E>" key=Included discriminant=included backing=Included<T#2> argument={ value: T#2 }
/// @type.symbol symbol=Bound.T source=T type=T#2
/// @type.symbol symbol=Bound.E source=E type=E#2
/// @resolution.name source=Included target=Included
/// @resolution.name source=T target=Bound.T
/// @resolution.name source=Excluded target=Excluded
/// @resolution.name source=E target=Bound.E

function included(value: string): Bound<string, Error> {
/// @type.symbol symbol=included type=(string) => Bound<string, Error>
/// @type.symbol symbol=included.value source="value: string" type=string
/// @resolution.name source=Bound target=Bound
/// @resolution.name source=Error target=error.error.Error

    Bound<string, Error>.Included({ value })
    /// @type.node source="Bound<string, Error>" type=Bound<string, Error>
    /// @type.node source="Bound<string, Error>.Included" type=<T#2, E#2>({ value: string }) => Bound.Included<string, Error>
    /// @type.node source="Bound<string, Error>.Included({ value })" type=Bound.Included<string, Error>
    /// @resolution.name source=Bound target=Bound
    /// @resolution.member source="Bound<string, Error>.Included" receiver=Bound<string, Error> type=<T#2, E#2>({ value: string }) => Bound.Included<string, Error> kind=symbol target_receiver=Bound<string, Error> target=Bound.Included
    /// @resolution.instantiation source="Bound<string, Error>" target=Bound instance="Bound<string, Error>"
    /// @resolution.construct source="Bound<string, Error>.Included({ value })" parameters=({ value: string }) arguments=(provided({ value }) as { value: string }) return=Bound.Included<string, Error> kind=variant owner=Bound variant=Included instance="Bound<string, Error>" backing=Included<string> argument={ value: string } discriminant=included
    /// @generic.instance source="Bound<string, Error>" id="Bound<string, Error>"
    /// @generic.instance source="Bound<string, Error>.Included" id="Bound<string, Error>"
    /// @generic.instance source="Bound<string, Error>.Included({ value })" id="Bound<string, Error>"
    /// @resolution.name source=Error target=error.error.Error
    /// @type.node source={ value } type={ value: string }
    /// @type.node source=value type=string
    /// @resolution.name source=value target=included.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=included.value

}

/// @generic.instance id="Bound<T#2, E#2>" template=Bound arguments=(T#2, E#2)
/// @generic.instance id="Bound<string, Error>" template=Bound arguments=(string, Error)
"#,
    );
}

#[test]
fn test_tagged_unit_variant_accepts_zero_arguments() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Option<T> =
    | { kind: "some"; value: T }
    | { kind: "none" };

const none: Option<string> = Option.None();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Option<in out T> = { kind: "some"; value: T } | { kind: "none" };

const none: Option<string> = Option.None();

=== checked ===
@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Option<T> =
/// @generic.template symbol=Option parameters=(in out T)
/// @type.symbol symbol=Option type=Option
/// @type.symbol symbol=Option.None type=Option.None<T>
/// @type.symbol symbol=Option.Some type=<T>({ value: T }) => Option.Some<T>
/// @definition.newtype symbol=Option template=(in out T) discriminator=kind backing={ kind: "some"; value: T } | { kind: "none" }
/// @definition.variant symbol=Option.None key=None discriminant=none backing={ kind: "none" }
/// @definition.variant symbol=Option.Some key=Some discriminant=some backing={ kind: "some"; value: T } argument={ value: T }
/// @type.symbol symbol=Option.T source=T type=T

    | { kind: "some"; value: T }
    /// @resolution.name source=T target=Option.T

    | { kind: "none" };

const none: Option<string> = Option.None();
/// @type.symbol symbol=none source=none type=Option<string>
/// @resolution.pattern source=none kind=binding target=none
/// @resolution.name source=Option target=Option
/// @type.node source=Option type=Option
/// @type.node source=Option.None type=Option.None<string>
/// @type.node source=Option.None() type=Option.None<string>
/// @resolution.name source=Option target=Option
/// @resolution.member source=Option.None receiver=Option type=Option.None<string> kind=symbol target_receiver=Option target=Option.None
/// @resolution.construct source=Option.None() parameters=() return=Option.None<string> kind=variant owner=Option variant=None instance=Option<string> backing={ kind: "none" } discriminant=none
/// @generic.instance source=Option.None id=Option<string>
/// @generic.instance source=Option.None() id=Option<string>

/// @generic.instance id=Option<T> template=Option arguments=(T)
/// @generic.instance id=Option<string> template=Option arguments=(string)
"#,
    );
}

#[test]
fn test_tagged_unit_variant_instantiates_from_expected_type() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Option<T> =
    | { kind: "some"; value: T }
    | { kind: "none" };

const none: Option<string> = Option.None;
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
@derive(Tagged)
newtype Option<in out T> = { kind: "some"; value: T } | { kind: "none" };

const none: Option<string> = Option.None;

=== checked ===
@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Option<T> =
/// @generic.template symbol=Option parameters=(in out T)
/// @type.symbol symbol=Option type=Option
/// @type.symbol symbol=Option.None type=Option.None<T>
/// @type.symbol symbol=Option.Some type=<T>({ value: T }) => Option.Some<T>
/// @definition.newtype symbol=Option template=(in out T) discriminator=kind backing={ kind: "some"; value: T } | { kind: "none" }
/// @definition.variant symbol=Option.None key=None discriminant=none backing={ kind: "none" }
/// @definition.variant symbol=Option.Some key=Some discriminant=some backing={ kind: "some"; value: T } argument={ value: T }
/// @type.symbol symbol=Option.T source=T type=T

    | { kind: "some"; value: T }
    /// @resolution.name source=T target=Option.T

    | { kind: "none" };

const none: Option<string> = Option.None;
/// @type.symbol symbol=none source=none type=Option<string>
/// @resolution.pattern source=none kind=binding target=none
/// @resolution.name source=Option target=Option
/// @type.node source=Option type=Option
/// @type.node source=Option.None type=Option.None<string>
/// @resolution.name source=Option target=Option
/// @resolution.member source=Option.None receiver=Option type=Option.None<string> kind=symbol target_receiver=Option target=Option.None
/// @generic.instance source=Option.None id=Option<string>

/// @generic.instance id=Option<T> template=Option arguments=(T)
/// @generic.instance id=Option<string> template=Option arguments=(string)
"#);
}
