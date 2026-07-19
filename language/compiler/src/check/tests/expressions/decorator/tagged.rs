use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_tagged_on_struct() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
struct Shape {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@derive(Tagged)
struct Shape {}

=== checked ===
@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="struct Shape {}" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

struct Shape {}
/// @type.symbol symbol=Shape source="struct Shape {}" type=Shape
/// @definition.struct symbol=Shape source="struct Shape {}"
"#,
        r#"
/// @diagnostic.error id=invalid-derive-target message="'Tagged' cannot be derived for this declaration"
/// @diagnostic.label line=2 column=9 span="Tagged" line_source="@derive(Tagged)"
"#,
    );
}

#[test]
fn test_reject_tagged_arm_without_discriminant() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Shape = { value: int32 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Shape = { value: int32 };

=== checked ===
@derive(Tagged)
/// @decorator.node source=@derive(Tagged) owner="newtype Shape = { value: int32 }" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing=() type=Tagged] value=derive(Tagged())
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Shape = { value: int32 };
/// @type.symbol symbol=Shape source="newtype Shape = { value: int32 }" type=Shape
/// @definition.newtype symbol=Shape source="newtype Shape = { value: int32 }" backing={ value: int32 }
"#,
        r#"
/// @diagnostic.error id=invalid-tagged-variant message="Tagged backing arm must declare a string literal 'kind' field"
/// @diagnostic.label line=2 column=9 span="Tagged" line_source="@derive(Tagged)"
"#,
    );
}

#[test]
fn test_reject_duplicate_tagged_case() {
    let session = TestSession::single(
        r#"
@derive(Tagged({ case: "snake_case" }))
newtype Shape = { kind: "fooBar" } | { kind: "foo_bar" };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
        r#"
=== annotated ===
@derive(Tagged({ case: "snake_case" }))
newtype Shape = { kind: "fooBar" } | { kind: "foo_bar" };

=== checked ===
@derive(Tagged({ case: "snake_case" }))
/// @decorator.node source="@derive(Tagged({ case: \"snake_case\" }))" owner="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing={ discriminant?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames } type=Tagged] value="derive(Tagged({ case: \"snake_case\" }))"
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source="Tagged({ case: \"snake_case\" })" type=Tagged
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged
/// @resolution.construct source="Tagged({ case: \"snake_case\" })" parameters=({ discriminant?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) arguments=(provided({ case: "snake_case" }) as { discriminant?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) return=Tagged kind=newtype target=decorator.derive.Tagged backing={ discriminant?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }
/// @type.node source={ case: "snake_case" } type={ case: "snake_case" }
/// @type.node source="\"snake_case\"" type="snake_case"

newtype Shape = { kind: "fooBar" } | { kind: "foo_bar" };
/// @type.symbol symbol=Shape source="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" type=Shape
/// @definition.newtype symbol=Shape source="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" backing={ kind: "fooBar" } | { kind: "foo_bar" }
"#,
        r#"
/// @diagnostic.error id=duplicate-tagged-case message="duplicate Tagged case 'foo_bar'"
/// @diagnostic.label line=2 column=9 span="Tagged({ case: \"snake_case\" })" line_source="@derive(Tagged({ case: \"snake_case\" }))"
"#,
    );
}

#[test]
fn test_reject_duplicate_derive_provider() {
    let session = TestSession::single(
        r#"
@derive(Tagged, Tagged)
newtype Shape = { kind: "shape" };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
@derive(Tagged, Tagged)
newtype Shape = { kind: "shape" };

=== checked ===
@derive(Tagged, Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Shape = { kind: "shape" };
/// @type.symbol symbol=Shape source="newtype Shape = { kind: \"shape\" }" type=Shape
/// @definition.newtype symbol=Shape source="newtype Shape = { kind: \"shape\" }" backing={ kind: "shape" }
"#,
        r#"
/// @diagnostic.error id=duplicate-derive-provider message="duplicate derive provider 'Tagged'"
/// @diagnostic.label line=2 column=17 span="Tagged" line_source="@derive(Tagged, Tagged)"
"#,
    );
}
