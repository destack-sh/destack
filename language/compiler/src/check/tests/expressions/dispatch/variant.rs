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
/// @resolution.name source=derive target=decorator.derive.derive

newtype Edge =
/// @type.symbol symbol=Edge type=Edge
/// @type.symbol symbol=Edge.Bounded type=Edge.Bounded
/// @type.symbol symbol=Edge.Open type=Edge.Open
/// @definition.newtype symbol=Edge value={ kind: "bounded"; limit: int32 } | { kind: "open" }
/// @definition.variant symbol=Edge.Bounded key=Bounded
/// @definition.variant symbol=Edge.Open key=Open

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
/// @definition.newtype symbol=Wrapper source="newtype Wrapper<T> = Promise<T>" template=(in out T#1) value=Promise<T#1>
/// @type.symbol symbol=Wrapper.T source=T type=T#1
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.name source=T target=Wrapper.T

extension<T> of Wrapper<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Wrapper<T#2>
/// @definition.method symbol=take slot=take type=async (this: Wrapper<T#2>) => Promise<T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Wrapper target=Wrapper
/// @resolution.name source=T target=T

    async take(): Promise<T> {
    /// @type.symbol symbol=take type=async (this: Wrapper<T#2>) => Promise<T#2>
    /// @resolution.name source=Promise target=async.promise.Promise
    /// @resolution.name source=T target=T

        const value = await this;
        /// @type.symbol symbol=take.value source=value type=T#2
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
