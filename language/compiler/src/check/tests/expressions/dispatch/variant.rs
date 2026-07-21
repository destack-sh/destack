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

    session.assert_dir_checked_and_diagnostics(
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
/// @type.symbol symbol=Edge.Bounded type=Edge.Bounded
/// @type.symbol symbol=Edge.Open type=Edge.Open
/// @definition.newtype symbol=Edge backing={ kind: "bounded"; limit: int32 } | { kind: "open" }
/// @definition.variant symbol=Edge.Bounded key=Bounded discriminant=bounded backing={ kind: "bounded"; limit: int32 }
/// @definition.variant symbol=Edge.Open key=Open discriminant=open backing={ kind: "open" }

    | { kind: "bounded"; limit: int32 }
    | { kind: "open" };

function bounded(limit: int32): Edge {
/// @type.symbol symbol=bounded type=(int32) => Edge
/// @type.symbol symbol=bounded.limit source="limit: int32" type=int32
/// @resolution.name source=Edge target=Edge

    Edge.Bounded({ limit })
    /// @type.node source="Edge.Bounded({ limit })" type=Edge
    /// @type.node source=Edge type=Edge
    /// @type.node source=Edge.Bounded type=Edge.Bounded
    /// @resolution.name source=Edge target=Edge
    /// @resolution.member source=Edge.Bounded receiver=Edge kind=symbol target=Edge.Bounded
    /// @resolution.construct source="Edge.Bounded({ limit })" parameters=({ limit: int32 }) arguments=(provided({ limit }) as { limit: int32 }) return=Edge kind=variant owner=Edge variant=Bounded discriminant=bounded
    /// @type.node source={ limit } type={ limit: int32 }
    /// @type.node source=limit type=int32
    /// @resolution.name source=limit target=bounded.limit

}

function open(): Edge {
/// @type.symbol symbol=open type=() => Edge
/// @resolution.name source=Edge target=Edge

    Edge.Open
    /// @type.node source=Edge type=Edge
    /// @type.node source=Edge.Open type=Edge.Open
    /// @resolution.name source=Edge target=Edge
    /// @resolution.member source=Edge.Open receiver=Edge kind=symbol target=Edge.Open

}
"#,
        r#""#,
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

    session.assert_dir_checked_and_diagnostics(
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
/// @type.symbol symbol=Cow.Borrowed type=Cow.Borrowed
/// @type.symbol symbol=Cow.Owned type=Cow.Owned
/// @definition.newtype symbol=Cow source="newtype Cow = Borrowed | Owned" backing=Borrowed | Owned
/// @definition.variant symbol=Cow.Borrowed source="newtype Cow = Borrowed | Owned" key=Borrowed discriminant=borrowed backing=Borrowed
/// @definition.variant symbol=Cow.Owned source="newtype Cow = Borrowed | Owned" key=Owned discriminant=owned backing=Owned
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Owned target=Owned

const borrowed: Cow = Cow.Borrowed({ value: "value" });
/// @type.symbol symbol=borrowed source=borrowed type=Cow
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=Cow target=Cow
/// @type.node source="Cow.Borrowed({ value: \"value\" })" type=Cow
/// @type.node source=Cow type=Cow
/// @type.node source=Cow.Borrowed type=Cow.Borrowed
/// @resolution.name source=Cow target=Cow
/// @resolution.member source=Cow.Borrowed receiver=Cow kind=symbol target=Cow.Borrowed
/// @resolution.construct source="Cow.Borrowed({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Cow kind=variant owner=Cow variant=Borrowed discriminant=borrowed
/// @type.node source={ value: "value" } type={ value: "value" }
/// @type.node source="\"value\"" type="value"

const owned: Cow = Cow.Owned({ value: "value" });
/// @type.symbol symbol=owned source=owned type=Cow
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=Cow target=Cow
/// @type.node source="Cow.Owned({ value: \"value\" })" type=Cow
/// @type.node source=Cow type=Cow
/// @type.node source=Cow.Owned type=Cow.Owned
/// @resolution.name source=Cow target=Cow
/// @resolution.member source=Cow.Owned receiver=Cow kind=symbol target=Cow.Owned
/// @resolution.construct source="Cow.Owned({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Cow kind=variant owner=Cow variant=Owned discriminant=owned
/// @type.node source={ value: "value" } type={ value: "value" }
/// @type.node source="\"value\"" type="value"
"#,
        r#""#,
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

    session.assert_dir_checked_and_diagnostics(
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
/// @type.symbol symbol=Read.Read type=Read.Read
/// @definition.newtype symbol=Read source="newtype Read = { kind: \"read\"; value: string }" backing={ kind: "read"; value: string }
/// @definition.variant symbol=Read.Read source="newtype Read = { kind: \"read\"; value: string }" key=Read discriminant=read backing={ kind: "read"; value: string }

@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="newtype Write = { kind: \"write\"; value: string }" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Write = { kind: "write"; value: string };
/// @type.symbol symbol=Write source="newtype Write = { kind: \"write\"; value: string }" type=Write
/// @type.symbol symbol=Write.Write type=Write.Write
/// @definition.newtype symbol=Write source="newtype Write = { kind: \"write\"; value: string }" backing={ kind: "write"; value: string }
/// @definition.variant symbol=Write.Write source="newtype Write = { kind: \"write\"; value: string }" key=Write discriminant=write backing={ kind: "write"; value: string }

@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="newtype Action = Read | Write" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Action = Read | Write;
/// @type.symbol symbol=Action source="newtype Action = Read | Write" type=Action
/// @type.symbol symbol=Action.Read type=Action.Read
/// @type.symbol symbol=Action.Write type=Action.Write
/// @definition.newtype symbol=Action source="newtype Action = Read | Write" backing=Read | Write
/// @definition.variant symbol=Action.Read source="newtype Action = Read | Write" key=Read discriminant=read backing={ kind: "read"; value: string }
/// @definition.variant symbol=Action.Write source="newtype Action = Read | Write" key=Write discriminant=write backing={ kind: "write"; value: string }
/// @resolution.name source=Read target=Read
/// @resolution.name source=Write target=Write

const read: Action = Action.Read({ value: "value" });
/// @type.symbol symbol=read source=read type=Action
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Action target=Action
/// @type.node source="Action.Read({ value: \"value\" })" type=Action
/// @type.node source=Action type=Action
/// @type.node source=Action.Read type=Action.Read
/// @resolution.name source=Action target=Action
/// @resolution.member source=Action.Read receiver=Action kind=symbol target=Action.Read
/// @resolution.construct source="Action.Read({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Action kind=variant owner=Action variant=Read discriminant=read
/// @type.node source={ value: "value" } type={ value: "value" }
/// @type.node source="\"value\"" type="value"

const write: Action = Action.Write({ value: "value" });
/// @type.symbol symbol=write source=write type=Action
/// @resolution.pattern source=write kind=binding target=write
/// @resolution.name source=Action target=Action
/// @type.node source="Action.Write({ value: \"value\" })" type=Action
/// @type.node source=Action type=Action
/// @type.node source=Action.Write type=Action.Write
/// @resolution.name source=Action target=Action
/// @resolution.member source=Action.Write receiver=Action kind=symbol target=Action.Write
/// @resolution.construct source="Action.Write({ value: \"value\" })" parameters=({ value: string }) arguments=(provided({ value: "value" }) as { value: string }) return=Action kind=variant owner=Action variant=Write discriminant=write
/// @type.node source={ value: "value" } type={ value: "value" }
/// @type.node source="\"value\"" type="value"
"#,
        r#""#,
    );
}

#[test]
fn test_await_unwraps_the_newtype_backing() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

newtype Wrapper<T> = Promise<T>;

extension<T> of Wrapper<T> {
    async take(): Promise<T> {
        const value = await this;
        value
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

newtype Wrapper<in out T> = Promise<T>;

extension<T> of Wrapper<T> {
    async take(): Promise<T> {
        const value: T = await this;
        value
    }
}

=== checked ===
import { Promise } from "destack:async";

newtype Wrapper<T> = Promise<T>;
/// @generic.template symbol=Wrapper parameters=(in out T#1)
/// @type.symbol symbol=Wrapper source="newtype Wrapper<T> = Promise<T>" type=Wrapper
/// @definition.newtype symbol=Wrapper source="newtype Wrapper<T> = Promise<T>" template=(in out T#1) backing=Promise<T#1>
/// @type.symbol symbol=Wrapper.T source=T type=T#1
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.name source=T target=Wrapper.T

extension<T> of Wrapper<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Wrapper<T#2>
/// @definition.method symbol=take slot=take type=async (this: this) => Promise<T#2>
/// @type.symbol symbol=T source=T type=T#2
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
        /// @generic.instance source=this id=Wrapper<T#2>

        value
        /// @type.node source=value type=T#2
        /// @resolution.name source=value target=take.value

    }
}

/// @generic.instance id=Promise<T#2> template=async.promise.Promise arguments=(T#2)
/// @generic.instance id=Wrapper<T#2> template=Wrapper arguments=(T#2)
"#,
        r#""#,
    );
}
