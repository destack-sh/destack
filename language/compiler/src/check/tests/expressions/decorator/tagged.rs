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
        DirRows::checked()
            .with_reference_types()
            .with_decorators(),
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
fn test_reject_tagged_arm_without_discriminator() {
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
/// @definition.variant symbol=Shape.symbol4 source="newtype Shape = { value: int32 }" index=0
"#,
        r#"
/// @diagnostic.error id=missing-tagged-discriminator message="Tagged backing has no common required field with distinct string literal types"
/// @diagnostic.label line=3 column=9 span="Shape" line_source="newtype Shape = { value: int32 };"
"#,
    );
}

#[test]
fn test_reject_optional_tagged_discriminator() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Event =
    | { type?: "click"; x: int32 }
    | { type: "key"; key: string };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Event = { type?: "click"; x: int32 } | { type: "key"; key: string };

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Event =
/// @type.symbol symbol=Event type=Event
/// @definition.newtype symbol=Event backing={ type?: "click"; x: int32 } | { type: "key"; key: string }
/// @definition.variant symbol=Event.symbol10 index=0
/// @definition.variant symbol=Event.symbol11 index=1

    | { type?: "click"; x: int32 }
    | { type: "key"; key: string };
"#,
        r#"
/// @diagnostic.error id=missing-tagged-discriminator message="Tagged backing has no common required field with distinct string literal types"
/// @diagnostic.label line=3 column=9 span="Event" line_source="newtype Event ="
"#,
    );
}

#[test]
fn test_infer_tagged_discriminator() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Event =
    | { type: "click"; x: int32 }
    | { type: "key"; key: string };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Event = { type: "click"; x: int32 } | { type: "key"; key: string };

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Event =
/// @type.symbol symbol=Event type=Event
/// @type.symbol symbol=Event.Click type=({ x: int32 }) => Event.Click
/// @type.symbol symbol=Event.Key type=({ key: string }) => Event.Key
/// @definition.newtype symbol=Event discriminator=type backing={ type: "click"; x: int32 } | { type: "key"; key: string }
/// @definition.variant symbol=Event.Click key=Click discriminant=click backing={ type: "click"; x: int32 } argument={ x: int32 }
/// @definition.variant symbol=Event.Key key=Key discriminant=key backing={ type: "key"; key: string } argument={ key: string }

    | { type: "click"; x: int32 }
    | { type: "key"; key: string };
"#,
    );
}

#[test]
fn test_reject_ambiguous_tagged_discriminator() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Event =
    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; type: "keyboard"; key: string };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Event =
    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; type: "keyboard"; key: string };

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Event =
/// @type.symbol symbol=Event type=Event
/// @definition.newtype symbol=Event backing={ kind: "click"; type: "pointer"; x: int32 } | { kind: "key"; type: "keyboard"; key: string }
/// @definition.variant symbol=Event.symbol14 index=0
/// @definition.variant symbol=Event.symbol15 index=1

    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; type: "keyboard"; key: string };
"#,
        r#"
/// @diagnostic.error id=ambiguous-tagged-discriminator message="Tagged backing has multiple possible discriminators: 'kind', 'type'"
/// @diagnostic.label line=3 column=9 span="Event" line_source="newtype Event ="
"#,
    );
}

#[test]
fn test_select_explicit_tagged_discriminator() {
    let session = TestSession::single(
        r#"
@derive(Tagged({ discriminator: "type" }))
newtype Event =
    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; type: "keyboard"; key: string };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Tagged({ discriminator: "type" }))
newtype Event =
    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; type: "keyboard"; key: string };

=== checked ===
@derive(Tagged({ discriminator: "type" }))
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged
/// @resolution.construct source="Tagged({ discriminator: \"type\" })" parameters=({ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) arguments=(provided({ discriminator: "type" }) as { discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) return=Tagged kind=newtype target=decorator.derive.Tagged backing={ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }

newtype Event =
/// @type.symbol symbol=Event type=Event
/// @type.symbol symbol=Event.Keyboard type=({ kind: "key"; key: string }) => Event.Keyboard
/// @type.symbol symbol=Event.Pointer type=({ kind: "click"; x: int32 }) => Event.Pointer
/// @definition.newtype symbol=Event discriminator=type backing={ kind: "click"; type: "pointer"; x: int32 } | { kind: "key"; type: "keyboard"; key: string }
/// @definition.variant symbol=Event.Keyboard key=Keyboard discriminant=keyboard backing={ kind: "key"; type: "keyboard"; key: string } argument={ kind: "key"; key: string }
/// @definition.variant symbol=Event.Pointer key=Pointer discriminant=pointer backing={ kind: "click"; type: "pointer"; x: int32 } argument={ kind: "click"; x: int32 }

    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; type: "keyboard"; key: string };
"#,
    );
}

#[test]
fn test_reject_invalid_tagged_discriminator() {
    let session = TestSession::single(
        r#"
@derive(Tagged({ discriminator: "type" }))
newtype Event =
    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; key: string };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Tagged({ discriminator: "type" }))
newtype Event = { kind: "click"; type: "pointer"; x: int32 } | { kind: "key"; key: string };

=== checked ===
@derive(Tagged({ discriminator: "type" }))
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged
/// @resolution.construct source="Tagged({ discriminator: \"type\" })" parameters=({ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) arguments=(provided({ discriminator: "type" }) as { discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) return=Tagged kind=newtype target=decorator.derive.Tagged backing={ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }

newtype Event =
/// @type.symbol symbol=Event type=Event
/// @definition.newtype symbol=Event backing={ kind: "click"; type: "pointer"; x: int32 } | { kind: "key"; key: string }
/// @definition.variant symbol=Event.symbol12 index=0
/// @definition.variant symbol=Event.symbol13 index=1

    | { kind: "click"; type: "pointer"; x: int32 }
    | { kind: "key"; key: string };
"#,
        r#"
/// @diagnostic.error id=invalid-tagged-discriminator message="Tagged discriminator 'type' must be a required string literal field in every backing arm"
/// @diagnostic.label line=3 column=9 span="Event" line_source="newtype Event ="
"#,
    );
}

#[test]
fn test_reject_duplicate_tagged_discriminant() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Shape =
    | { kind: "shape"; width: int32 }
    | { kind: "shape"; radius: float64 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Shape = { kind: "shape"; width: int32 } | { kind: "shape"; radius: float64 };

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Shape =
/// @type.symbol symbol=Shape type=Shape
/// @definition.newtype symbol=Shape backing={ kind: "shape"; width: int32 } | { kind: "shape"; radius: float64 }
/// @definition.variant symbol=Shape.symbol10 index=0
/// @definition.variant symbol=Shape.symbol11 index=1

    | { kind: "shape"; width: int32 }
    | { kind: "shape"; radius: float64 };
"#,
        r#"
/// @diagnostic.error id=duplicate-tagged-discriminant message="duplicate Tagged discriminant 'shape'"
/// @diagnostic.label line=3 column=9 span="Shape" line_source="newtype Shape ="
"#,
    );
}

#[test]
fn test_reject_tagged_class_arm() {
    let session = TestSession::single(
        r#"
class Active {
    kind: "active" = "active";
}

@derive(Tagged)
newtype State = Active;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Active {
    kind: "active" = "active";
}

@derive(Tagged)
newtype State = Active;

=== checked ===
class Active {
/// @type.symbol symbol=Active type=Active
/// @definition.class symbol=Active
/// @definition.field symbol=Active.kind source="kind: \"active\" = \"active\"" key=kind type="active"

    kind: "active" = "active";
    /// @type.symbol symbol=Active.kind source="kind: \"active\" = \"active\"" type="active"

}

@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype State = Active;
/// @type.symbol symbol=State source="newtype State = Active" type=State
/// @definition.newtype symbol=State source="newtype State = Active" backing=Active
/// @definition.variant symbol=State.symbol5 source="newtype State = Active" index=0
/// @resolution.name source=Active target=Active
"#,
        r#"
/// @diagnostic.error id=invalid-tagged-variant message="Tagged backing arm must be a constructible shape or struct"
/// @diagnostic.label line=7 column=9 span="State" line_source="newtype State = Active;"
"#,
    );
}

#[test]
fn test_reject_tagged_interface_arm() {
    let session = TestSession::single(
        r#"
interface Active {
    kind: "active";
}

@derive(Tagged)
newtype State = Active;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Active {
    kind: "active";
}

@derive(Tagged)
newtype State = Active;

=== checked ===
interface Active {
/// @type.symbol symbol=Active type=Active
/// @definition.interface symbol=Active
/// @definition.field symbol=Active.kind source="kind: \"active\"" key=kind type="active"

    kind: "active";
    /// @type.symbol symbol=Active.kind source="kind: \"active\"" type="active"

}

@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype State = Active;
/// @type.symbol symbol=State source="newtype State = Active" type=State
/// @definition.newtype symbol=State source="newtype State = Active" backing=Active
/// @definition.variant symbol=State.symbol5 source="newtype State = Active" index=0
/// @resolution.name source=Active target=Active
"#,
        r#"
/// @diagnostic.error id=invalid-tagged-variant message="Tagged backing arm must be a constructible shape or struct"
/// @diagnostic.label line=7 column=9 span="State" line_source="newtype State = Active;"
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
/// @decorator.node source="@derive(Tagged({ case: \"snake_case\" }))" owner="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing={ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames } type=Tagged] value="derive(Tagged({ case: \"snake_case\" }))"
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source="Tagged({ case: \"snake_case\" })" type=Tagged
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged
/// @resolution.construct source="Tagged({ case: \"snake_case\" })" parameters=({ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) arguments=(provided({ case: "snake_case" }) as { discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) return=Tagged kind=newtype target=decorator.derive.Tagged backing={ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }
/// @type.node source={ case: "snake_case" } type={ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }
/// @type.node source="\"snake_case\"" type="snake_case"

newtype Shape = { kind: "fooBar" } | { kind: "foo_bar" };
/// @type.symbol symbol=Shape source="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" type=Shape
/// @definition.newtype symbol=Shape source="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" backing={ kind: "fooBar" } | { kind: "foo_bar" }
/// @definition.variant symbol=Shape.symbol6 source="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" index=0
/// @definition.variant symbol=Shape.symbol7 source="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" }" index=1
"#,
        r#"
/// @diagnostic.error id=duplicate-tagged-case message="duplicate Tagged case 'foo_bar'"
/// @diagnostic.label line=3 column=9 span="Shape" line_source="newtype Shape = { kind: \"fooBar\" } | { kind: \"foo_bar\" };"
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
/// @type.symbol symbol=Shape.Shape type=Shape.Shape
/// @definition.newtype symbol=Shape source="newtype Shape = { kind: \"shape\" }" discriminator=kind backing={ kind: "shape" }
/// @definition.variant symbol=Shape.Shape source="newtype Shape = { kind: \"shape\" }" key=Shape discriminant=shape backing={ kind: "shape" }
"#,
        r#"
/// @diagnostic.error id=duplicate-derive-provider message="duplicate derive provider 'Tagged'"
/// @diagnostic.label line=2 column=17 span="Tagged" line_source="@derive(Tagged, Tagged)"
"#,
    );
}

#[test]
fn test_reject_non_newtype_derive_provider() {
    let session = TestSession::single(
        r#"
const provider = 1;

@derive(provider)
newtype Shape = { kind: "shape" };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_decorators(),
        r#"
=== annotated ===
const provider: 1 = 1;

@derive(provider)
newtype Shape = { kind: "shape" };

=== checked ===
const provider = 1;
/// @type.symbol symbol=provider source=provider type=1
/// @resolution.pattern source=provider kind=binding target=provider
/// @type.node source=1 type=1

@derive(provider)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=provider target=provider

newtype Shape = { kind: "shape" };
/// @type.symbol symbol=Shape source="newtype Shape = { kind: \"shape\" }" type=Shape
/// @definition.newtype symbol=Shape source="newtype Shape = { kind: \"shape\" }" backing={ kind: "shape" } constructors=[({ kind: "shape" }) => Shape]
"#,
        r#"
/// @diagnostic.error id=invalid-derive-provider message="derive argument must name a registered provider newtype"
/// @diagnostic.label line=4 column=9 span="provider" line_source="@derive(provider)"
"#,
    );
}
