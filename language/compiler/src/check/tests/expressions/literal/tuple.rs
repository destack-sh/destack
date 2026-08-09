use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_tuple_widens_binding_elements() {
    let session = TestSession::single(
        r#"
let value = (1, "two", true);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_coercion()
            ,
        r#"
=== annotated ===
let value: (float64, string, boolean) = (1, "two", true);

=== checked ===
let value = (1, "two", true);
/// @type.symbol symbol=value source=value type=(float64, string, boolean)
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=(1, "two", true) type=(float64, string, boolean)
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: widen, target: float64 }] origin=implicit
/// @type.node source="\"two\"" type="two"
/// @coercion.node source="\"two\"" from="two" adjustments=[{ kind: widen, target: string }] origin=implicit
/// @type.node source=true type=true
/// @coercion.node source=true from=true adjustments=[{ kind: widen, target: boolean }] origin=implicit
"#,
    );
}

#[test]
fn test_const_asserted_tuple_preserves_literal_elements() {
    let session = TestSession::single(
        r#"
const value = (1, "two", true) as const;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: readonly (1, "two", true) = (1, "two", true) as const;

=== checked ===
const value = (1, "two", true) as const;
/// @type.symbol symbol=value source=value type=readonly (1, "two", true) reduced=(1, "two", true)
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="(1, \"two\", true) as const" type=readonly (1, "two", true) reduced=(1, "two", true)
/// @type.node source=(1, "two", true) type=readonly (1, "two", true) reduced=(1, "two", true)
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_const_tuple_widens_binding_without_const_assertion() {
    let session = TestSession::single(
        r#"
const value = (1, "two", true);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: (float64, string, boolean) = (1, "two", true);

=== checked ===
const value = (1, "two", true);
/// @type.symbol symbol=value source=value type=(float64, string, boolean)
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=(1, "two", true) type=(float64, string, boolean)
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_nested_tuple_preserves_nested_shape() {
    let session = TestSession::single(
        r#"
const value = (1, (2, 3));
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: (float64, (float64, float64)) = (1, (2, 3));

=== checked ===
const value = (1, (2, 3));
/// @type.symbol symbol=value source=value type=(float64, (float64, float64))
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=(1, (2, 3)) type=(float64, (float64, float64))
/// @type.node source=1 type=1
/// @type.node source=(2, 3) type=(float64, float64)
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
    );
}

#[test]
fn test_contextual_tuple_literal_preserves_union_elements() {
    let session = TestSession::single(
        r#"
const value: (1 | 2, "a" | "b") = (1, "a");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: (1 | 2, "a" | "b") = (1 as 1 | 2, "a" as "a" | "b");

=== checked ===
const value: (1 | 2, "a" | "b") = (1, "a");
/// @type.symbol symbol=value source=value type=(1 | 2, "a" | "b")
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=(1, "a") type=(1 | 2, "a" | "b")
/// @type.node source=1 type=1
/// @type.node source="\"a\"" type="a"
"#,
    );
}

#[test]
fn test_contextual_tuple_literal_rejects_element_mismatch() {
    let session = TestSession::single(
        r#"
const value: (number, string) = (1, 2);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: (float64, string) = (1, 2);

=== checked ===
const value: (number, string) = (1, 2);
/// @type.symbol symbol=value source=value type=(float64, string)
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=(1, 2) type=(float64, string)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '2' is not assignable to type 'string'"
/// @diagnostic.label line=2 column=37 span="2" line_source="const value: (number, string) = (1, 2);"
/// @diagnostic.related line=2 column=14 span="(number, string)" line_source="const value: (number, string) = (1, 2);" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}
