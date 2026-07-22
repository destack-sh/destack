use crate::tests::{DirRows, TestSession};

#[test]
fn test_for_of_binds_array_elements() {
    let session = TestSession::single(
        r#"
const values: int32[] = [1, 2, 3];

for (const value of values) {
    value satisfies int32;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: int32[] = [1, 2, 3];

for (const value of values) {
    value satisfies int32;
}

=== checked ===
const values: int32[] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] type=Array<int32>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

for (const value of values) {
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value

}
"#,
    );
}

#[test]
fn test_for_of_rejects_non_iterable_receiver() {
    let session = TestSession::single(
        r#"
for (const value of 1) {
    value;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const value of 1) {
    value;
}

=== checked ===
for (const value of 1) {
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

    value;
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value

}

"#,
        r#"
/// @diagnostic.error id=for-of-source-not-iterable message="for-of source must be iterable"
/// @diagnostic.label line=2 column=1 span="for (const value of 1) {\n    value;\n}" line_source="for (const value of 1) {"
"#,
    );
}

#[test]
fn test_for_in_binds_property_names_as_string() {
    let session = TestSession::single(
        r#"
const target = { a: 1, b: 2 };

for (const key in target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const target: { a: float64; b: float64 } = { a: 1, b: 2 };

for (const key in target) {
    key satisfies string;
}

=== checked ===
const target = { a: 1, b: 2 };
/// @type.symbol symbol=target source=target type={ a: float64; b: float64 }
/// @resolution.pattern source=target kind=binding target=target
/// @type.node source={ a: 1, b: 2 } type={ a: 1; b: 2 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ a: float64; b: float64 }
/// @resolution.name source=target target=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
    );
}

#[test]
fn test_for_in_rejects_literal_key_union_binding() {
    let session = TestSession::single(
        r#"
const target = { a: 1, b: 2 };

for (const key in target) {
    key satisfies "a" | "b";
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const target: { a: float64; b: float64 } = { a: 1, b: 2 };

for (const key in target) {
    key satisfies "a" | "b";
}

=== checked ===
const target = { a: 1, b: 2 };
/// @type.symbol symbol=target source=target type={ a: float64; b: float64 }
/// @resolution.pattern source=target kind=binding target=target
/// @type.node source={ a: 1, b: 2 } type={ a: 1; b: 2 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ a: float64; b: float64 }
/// @resolution.name source=target target=target

    key satisfies "a" | "b";
    /// @type.node source="key satisfies \"a\" | \"b\"" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'string' does not satisfy '\"a\" | \"b\"'"
/// @diagnostic.label line=5 column=9 span="satisfies" line_source="key satisfies \"a\" | \"b\";"
"#,
    );
}

#[test]
fn test_for_in_binds_union_property_names_as_string() {
    let session = TestSession::single(
        r#"
declare const target: { a: int32 } | { b: int32 };

for (const key in target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const target: { a: int32 } | { b: int32 };

for (const key in target) {
    key satisfies string;
}

=== checked ===
declare const target: { a: int32 } | { b: int32 };
/// @type.symbol symbol=target source=target type={ a: int32 } | { b: int32 }
/// @resolution.pattern source=target kind=binding target=target

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ a: int32 } | { b: int32 }
/// @resolution.name source=target target=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
    );
}

#[test]
fn test_for_in_accepts_readonly_object_receiver() {
    let session = TestSession::single(
        r#"
const target = { a: 1, b: 2 };

for (const key in &readonly target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const target: { a: float64; b: float64 } = { a: 1, b: 2 };

for (const key in &readonly target) {
    key satisfies string;
}

=== checked ===
const target = { a: 1, b: 2 };
/// @type.symbol symbol=target source=target type={ a: float64; b: float64 }
/// @resolution.pattern source=target kind=binding target=target
/// @type.node source={ a: 1, b: 2 } type={ a: 1; b: 2 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

for (const key in &readonly target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source="&readonly target" type=&'static readonly { a: float64; b: float64 }
/// @type.node source=target type={ a: float64; b: float64 }
/// @resolution.name source=target target=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
    );
}

#[test]
fn test_for_in_rejects_primitive_receiver() {
    let session = TestSession::single(
        r#"
for (const key in 1) {
    key;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const key in 1) {
    key;
}

=== checked ===
for (const key in 1) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=1 type=1

    key;
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}

"#,
        r#"
/// @diagnostic.error id=for-in-source-not-object-shaped message="for-in source must be object-shaped"
/// @diagnostic.label line=2 column=1 span="for (const key in 1) {\n    key;\n}" line_source="for (const key in 1) {"
"#,
    );
}

#[test]
fn test_for_in_rejects_unknown_receiver() {
    let session = TestSession::single(
        r#"
declare const target: unknown;

for (const key in target) {
    key;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const target: unknown;

for (const key in target) {
    key;
}

=== checked ===
declare const target: unknown;
/// @type.symbol symbol=target source=target type=unknown
/// @resolution.pattern source=target kind=binding target=target

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type=unknown
/// @resolution.name source=target target=target

    key;
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
        r#"
/// @diagnostic.error id=for-in-source-not-object-shaped message="for-in source must be object-shaped"
/// @diagnostic.label line=4 column=1 span="for (const key in target) {\n    key;\n}" line_source="for (const key in target) {"
"#,
    );
}

#[test]
fn test_for_in_rejects_array_receiver() {
    let session = TestSession::single(
        r#"
for (const key in [1, 2, 3]) {
    key;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const key in [1, 2, 3]) {
    key;
}

=== checked ===
for (const key in [1, 2, 3]) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

    key;
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
        r#"
/// @diagnostic.error id=for-in-source-not-object-shaped message="for-in source must be object-shaped"
/// @diagnostic.label line=2 column=1 span="for (const key in [1, 2, 3]) {\n    key;\n}" line_source="for (const key in [1, 2, 3]) {"
"#,
    );
}

#[test]
fn test_for_in_accepts_struct_receiver() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

for (const key in point) {
    key satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

for (const key in point) {
    key satisfies string;
}

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

for (const key in point) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=point type=Point
/// @resolution.name source=point target=point

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
    );
}

#[test]
fn test_for_in_accepts_class_receiver() {
    let session = TestSession::single(
        r#"
class User {
    name: string = "";
}

declare const user: User;

for (const key in user) {
    key satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string = "";
}

declare const user: User;

for (const key in user) {
    key satisfies string;
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string
    /// @type.node source="\"\"" type=""

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

for (const key in user) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=user type=User
/// @resolution.name source=user target=user

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
    );
}

#[test]
fn test_for_in_excludes_symbol_keys() {
    let session = TestSession::single(
        r#"
declare const token: unique symbol;
declare const target: { name: string; readonly [token]: int32 };

for (const key in target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const token: unique symbol;
declare const target: { name: string; readonly [token]: int32 };

for (const key in target) {
    key satisfies string;
}

=== checked ===
declare const token: unique symbol;
/// @type.symbol symbol=token source=token type=unique symbol
/// @resolution.pattern source=token kind=binding target=token

declare const target: { name: string; readonly [token]: int32 };
/// @type.symbol symbol=target source=target type={ name: string; readonly [token]: int32 }
/// @resolution.pattern source=target kind=binding target=target

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ name: string; readonly [token]: int32 }
/// @resolution.name source=target target=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
    );
}

#[test]
fn test_for_in_accepts_optional_fields() {
    let session = TestSession::single(
        r#"
declare const target: { name?: string; active: boolean };

for (const key in target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const target: { name?: string; active: boolean };

for (const key in target) {
    key satisfies string;
}

=== checked ===
declare const target: { name?: string; active: boolean };
/// @type.symbol symbol=target source=target type={ name?: string; active: boolean }
/// @resolution.pattern source=target kind=binding target=target

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ name?: string; active: boolean }
/// @resolution.name source=target target=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key

}
"#,
    );
}
