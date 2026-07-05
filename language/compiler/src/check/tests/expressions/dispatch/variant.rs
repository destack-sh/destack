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
/// @type.symbol symbol=Edge type=Edge
/// @type.symbol symbol=Edge.Bounded type=Edge.Bounded
/// @type.symbol symbol=Edge.Open type=Edge.Open
/// @definition.newtype symbol=Edge value={ kind: "bounded"; limit: int32 } | { kind: "open" }
/// @definition.variant symbol=Edge.Bounded key=Bounded
/// @definition.variant symbol=Edge.Open key=Open
/// @resolution.name source=derive target=decorator.derive.derive

newtype Edge =
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
