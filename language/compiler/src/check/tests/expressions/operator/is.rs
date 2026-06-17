use crate::tests::{DirRows, TestSession};

#[test]
fn test_is_guard_narrows_unknown_to_string() {
    let session = TestSession::single(
        r#"
declare const value: unknown;

if (value is string) {
    value satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: unknown;

if (value is string) {
    value satisfies string;
}

=== checked ===
declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

if (value is string) {
/// @type.node type=void | void
/// @type.node source="value is string" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value

    value satisfies string;
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value

}
"#,
    );
}

#[test]
fn test_is_guard_narrows_dynamic_unknown_to_target() {
    let session = TestSession::single(
        r#"
declare const value: Dynamic<unknown>;

if (value is { name: string }) {
    value.name satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: Dynamic<unknown>;

if (value is { name: string }) {
    value.name satisfies string;
}

=== checked ===
declare const value: Dynamic<unknown>;
/// @type.symbol symbol=value source=value type=Dynamic<unknown>
/// @resolution.name source=Dynamic target=memory.Dynamic

if (value is { name: string }) {
/// @type.node type=void | void
/// @type.node source="value is { name: string }" type=boolean
/// @type.node source=value type=Dynamic<unknown>
/// @resolution.name source=value target=value

    value.name satisfies string;
    /// @type.node source="value.name satisfies string" type=string
    /// @type.node source=value.name type=string
    /// @type.node source=value type={ name: string }
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.name receiver={ name: string } kind=field key=name

}
"#,
    );
}

#[test]
fn test_is_guard_narrows_union() {
    let session = TestSession::single(
        r#"
declare const value: string | int32;

if (value is string) {
    value satisfies string;
} else {
    value satisfies int32;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: string | int32;

if (value is string) {
    value satisfies string;
} else {
    value satisfies int32;
}

=== checked ===
declare const value: string | int32;
/// @type.symbol symbol=value source=value type=string | int32

if (value is string) {
/// @type.node type=void | void
/// @type.node source="value is string" type=boolean
/// @type.node source=value type=string | int32
/// @resolution.name source=value target=value

    value satisfies string;
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value

} else {
    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value

}
"#,
    );
}

#[test]
fn test_is_guard_narrows_borrow_form() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

declare const value: unknown;

if (value is &readonly Node) {
    value.id satisfies int32;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

declare const value: unknown;

if (value is Borrowed<Node, L0, "readonly">) {
    value.id satisfies int32;
}

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

if (value is &readonly Node) {
/// @type.node type=void | void
/// @type.node source="value is &readonly Node" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.name source=Node target=Node

    value.id satisfies int32;
    /// @type.node source="value.id satisfies int32" type=int32
    /// @type.node source=value.id type=int32
    /// @type.node source=value type=Borrowed<Node, L0, "readonly">
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.id receiver=Borrowed<Node, L0, "readonly"> kind=symbol target=Node.id

}
"#,
    );
}

#[test]
fn test_is_guard_rejects_non_dynamic_safe_target() {
    let session = TestSession::single(
        r#"
declare const value: unknown;

if (value is <T>(T) => T) {
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: unknown;

if (value is <T>(T) => T) {
}

=== checked ===
declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

if (value is <T>(T) => T) {
/// @type.node type=void | void
/// @type.node source="value is <T>(T) => T" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value

}
"#,
        r#"
/// @diagnostic.error code=EC200 message="type test target '<T>(T) => T' is not DynamicSafe"
/// @diagnostic.label line=4 column=14 source="<T>(T) => T"
"#,
    );
}
