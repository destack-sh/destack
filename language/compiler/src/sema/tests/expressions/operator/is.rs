use crate::tests::{DirRows, TestSession};

#[test]
fn test_logical_and_narrows_after_is_guard() {
    let session = TestSession::single(
        r#"
function positive(value: unknown): boolean {
    value is int32 && value > 0
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function positive(value: unknown): boolean {
    value is int32 && value > 0
}

=== dir ===
function positive(value: unknown): boolean {
/// @type.symbol symbol=positive type=(unknown) => boolean
/// @type.symbol symbol=positive.value source="value: unknown" type=unknown

    value is int32 && value > 0
    /// @resolution.name source=value target=positive.value
    /// @resolution.operator source="value is int32 && value > 0" type=boolean operator="&&" kind=builtin operands=[value is int32 as boolean families=(boolean), value > 0 as boolean families=(boolean)]
    /// @resolution.guard source="value is int32" kind=is value=unknown target=int32 predicate="unknown is int32" narrowed=Narrow<unknown, int32> projection=dynamic.payload(int32)
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=positive.value
    /// @resolution.name source=value target=positive.value
    /// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=positive.value

}
"#,
    );
}

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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: unknown;

if (value is string) {
    value satisfies string;
}

=== dir ===
declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown
/// @resolution.pattern source=value kind=binding target=value

if (value is string) {
/// @type.node source="value is string" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.guard source="value is string" kind=is value=unknown target=string predicate="unknown is string" narrowed=Narrow<unknown, string> projection=dynamic.payload(string)
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

    value satisfies string;
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value

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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: Dynamic<unknown>;

if (value is { name: string }) {
}

=== dir ===
declare const value: Dynamic<unknown>;
/// @type.symbol symbol=value source=value type=Dynamic<unknown>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Dynamic target=Dynamic

if (value is { name: string }) {
/// @type.node source="value is { name: string }" type=boolean
/// @type.node source=value type=Dynamic<unknown>
/// @resolution.name source=value target=value
/// @resolution.guard source="value is { name: string }" kind=is value=Dynamic<unknown> target={ name: string } predicate="Dynamic<unknown> is never"
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @type.symbol symbol=name source="name: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=runtime-predicate-not-testable message="type '{ name: string }' cannot be tested at runtime"
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

    session.assert_dir(
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

=== dir ===
declare const value: string | int32;
/// @type.symbol symbol=value source=value type=string | int32
/// @resolution.pattern source=value kind=binding target=value

if (value is string) {
/// @type.node source="value is string" type=boolean
/// @type.node source=value type=string | int32
/// @resolution.name source=value target=value
/// @resolution.guard source="value is string" kind=is value=string | int32 target=string predicate="string | int32 is string" narrowed=Narrow<string | int32, string>
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

    value satisfies string;
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.narrowing source=value union=string | int32 arms=string

} else {
    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.narrowing source=value union=string | int32 arms=int32

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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: string;

if (value is int32) {
}

=== dir ===
declare const value: string;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value

if (value is int32) {
/// @type.node source="value is int32" type=boolean
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.guard source="value is int32" kind=is value=string target=int32 predicate="string is int32" narrowed=Narrow<string, int32>
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

}
"#,
        r#"
/// @diagnostic.error id=impossible-is message="type 'string' can never satisfy runtime check 'int32'"
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

    session.assert_dir(
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

=== dir ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown
/// @resolution.pattern source=value kind=binding target=value

if (value is &readonly Node) {
/// @type.node source="value is &readonly Node" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.guard source="value is &readonly Node" kind=is value=unknown target=&'frame readonly Node predicate="dynamic.type(Type<unknown>) is type(&'frame readonly Node)" narrowed=Narrow<unknown, &'frame readonly Node> projection="dynamic.payload(&'frame readonly Node)"
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @generic.instance id=Type<unknown> template=Type arguments=(unknown)
/// @resolution.name source=Node target=Node

    value.id satisfies int32;
    /// @type.node source="value.id satisfies int32" type=int32
    /// @type.node source=value type=&'frame readonly Node
    /// @type.node source=value.id type=int32
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.id receiver=Narrow<unknown, &'frame readonly Node> type=int32 kind=field target_receiver=Narrow<unknown, &'frame readonly Node> key=id target=Node.id target_type=int32
    /// @resolution.place source=value placement="frame" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.place source=value.id placement="frame" lifetime="frame" access="readonly"
    /// @resolution.access source=value.id root=value keys=[id]

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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function check<T>(value: unknown): void {
    if (value is T) {
    }
}

=== dir ===
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
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=check.value
    /// @resolution.name source=T target=check.T

    }
}
"#,
        r#"
/// @diagnostic.error id=runtime-predicate-not-testable message="type 'T' cannot be tested at runtime"
/// @diagnostic.label line=3 column=18 span="T" line_source="if (value is T) {"
/// @diagnostic.error id=dynamic-safety-not-satisfied message="type 'T' is not dynamic-safe"
/// @diagnostic.label line=3 column=18 span="T" line_source="if (value is T) {"
"#,
    );
}
