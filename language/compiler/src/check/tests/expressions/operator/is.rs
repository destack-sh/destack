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
/// @type.node source="value is string" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.guard source="value is string" kind=is value=unknown target=string predicate="unknown is string" narrowed=string

    value satisfies string;
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value

}
"#,
    );
}

#[test]
fn test_is_guard_rejects_dynamic_structural_target() {
    let session = TestSession::single(
        r#"
declare const value: Dynamic<unknown>;

if (value is { name: string }) {
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: Dynamic<unknown>;

if (value is { name: string }) {
}

=== checked ===
declare const value: Dynamic<unknown>;
/// @type.symbol symbol=value source=value type=Dynamic<unknown>
/// @resolution.name source=Dynamic target=memory.dynamic.Dynamic

if (value is { name: string }) {
/// @type.node source="value is { name: string }" type=boolean
/// @type.node source=value type=Dynamic<unknown>
/// @resolution.name source=value target=value
/// @resolution.guard source="value is { name: string }" kind=is value=Dynamic<unknown> target={ name: string } predicate="Dynamic<unknown> is never"
/// @generic.instance source=value id=Dynamic<unknown>

}

/// @generic.instance id=Dynamic<unknown> template=memory.dynamic.Dynamic arguments=(unknown)
"#,
        r#"
/// @diagnostic.error code=EC320 message="type '{ name: string }' cannot be tested at runtime"
/// @diagnostic.label line=4 column=14 span="{ name: string }" line_source="if (value is { name: string }) {"
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
/// @type.node source="value is string" type=boolean
/// @type.node source=value type=string | int32
/// @resolution.name source=value target=value
/// @resolution.guard source="value is string" kind=is value=string | int32 target=string predicate="string | int32 is string" narrowed=string

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
fn test_is_guard_rejects_impossible_scalar_check() {
    let session = TestSession::single(
        r#"
declare const value: string;

if (value is int32) {
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: string;

if (value is int32) {
}

=== checked ===
declare const value: string;
/// @type.symbol symbol=value source=value type=string

if (value is int32) {
/// @type.node source="value is int32" type=boolean
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.guard source="value is int32" kind=is value=string target=int32 predicate="string is int32" narrowed=int32

}
"#,
        r#"
/// @diagnostic.error code=EC319 message="type 'string' can never satisfy runtime check 'int32'"
/// @diagnostic.label line=4 column=5 span="value" line_source="if (value is int32) {"
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

if (value is &readonly Node) {
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
/// @type.node source="value is &readonly Node" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.guard source="value is &readonly Node" kind=is value=unknown target=Borrowed<Node, "frame", "readonly"> predicate="unknown is type(Borrowed<Node, \"frame\", \"readonly\">)" narrowed=Borrowed<Node, "frame", "readonly">
/// @resolution.name source=Node target=Node

    value.id satisfies int32;
    /// @type.node source="value.id satisfies int32" type=int32
    /// @type.node source=value type=Borrowed<Node, "frame", "readonly">
    /// @type.node source=value.id type=int32
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.id receiver=Borrowed<Node, "frame", "readonly"> kind=symbol target=Node.id

}
"#,
    );
}

#[test]
fn test_is_guard_rejects_non_dynamic_safe_target() {
    let session = TestSession::single(
        r#"
function check<T>(value: unknown): void {
    if (value is T) {
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function check<T>(value: unknown): void {
    if (value is T) {
    }
}

=== checked ===
function check<T>(value: unknown): void {
/// @generic.template symbol=check parameters=(T)
/// @type.symbol symbol=check type=<T>(unknown) => void
/// @type.symbol symbol=check.T source=T type=T
/// @type.symbol symbol=check.value source="value: unknown" type=unknown

    if (value is T) {
    /// @type.node source="value is T" type=boolean
    /// @type.node source=value type=unknown
    /// @resolution.name source=value target=check.value
    /// @resolution.guard source="value is T" kind=is value=unknown target=T predicate="unknown is never"
    /// @resolution.name source=T target=check.T

    }
}
"#,
        r#"
/// @diagnostic.error code=EC504 message="type 'T' is not dynamic-safe"
/// @diagnostic.label line=3 column=18 span="T" line_source="if (value is T) {"
"#,
    );
}
