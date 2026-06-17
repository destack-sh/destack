use crate::tests::{DirRows, TestSession};

#[test]
fn test_variant_pattern_binds_tuple_payload() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

match (status) {
    Status.Ok(value) => value satisfies string
    Status.Err(code) => code satisfies int32
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

match (status) {
    Status.Ok(value) => value satisfies string
    Status.Err(code) => code satisfies int32
}

=== checked ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @resolution.name source=Tagged target=decorator.derive.Tagged
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=Err target=Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Ok(value) => value satisfies string
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source="Status.Ok(value)" kind=variant owner=Status variant=Ok payload=tuple fields=[0: value]
    /// @resolution.name source=Status target=Status
    /// @resolution.variant source=Status.Ok owner=Status variant=Ok
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value

    Status.Err(code) => code satisfies int32
    /// @type.symbol symbol=code source=code type=int32
    /// @resolution.pattern source="Status.Err(code)" kind=variant owner=Status variant=Err payload=tuple fields=[0: code]
    /// @resolution.name source=Status target=Status
    /// @resolution.variant source=Status.Err owner=Status variant=Err
    /// @resolution.pattern source=code kind=binding target=code
    /// @type.node source="code satisfies int32" type=int32
    /// @type.node source=code type=int32
    /// @resolution.name source=code target=code

}
"#,
    );
}

#[test]
fn test_variant_pattern_binds_object_payload() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Event = { kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string };

declare const event: Event;

match (event) {
    Event.Click({ x, y }) => x + y
    Event.Key({ key }) => key.length
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Event = { kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string };

declare const event: Event;

match (event) {
    Event.Click({ x, y }) => x + y
    Event.Key({ key }) => key.length
}

=== checked ===
@derive(Tagged)
newtype Event = { kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string };
/// @type.symbol symbol=Event source="newtype Event = { kind: \"click\"; x: int32; y: int32 } | { kind: \"key\"; key: string }" type=Event
/// @definition.newtype symbol=Event source="newtype Event = { kind: \"click\"; x: int32; y: int32 } | { kind: \"key\"; key: string }" value={ kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string }
/// @resolution.name source=Tagged target=decorator.derive.Tagged

declare const event: Event;
/// @type.symbol symbol=event source=event type=Event
/// @resolution.name source=Event target=Event

match (event) {
/// @type.node source=event type=Event
/// @resolution.name source=event target=event

    Event.Click({ x, y }) => x + y
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @resolution.pattern source="Event.Click({ x, y })" kind=variant owner=Event variant=Click payload=object fields=[x, y]
    /// @resolution.name source=Event target=Event
    /// @resolution.variant source=Event.Click owner=Event variant=Click
    /// @resolution.pattern source="{ x, y }" kind=object fields=[x, y]
    /// @type.node source="x + y" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y

    Event.Key({ key }) => key.length
    /// @type.symbol symbol=key source=key type=string
    /// @resolution.pattern source="Event.Key({ key })" kind=variant owner=Event variant=Key payload=object fields=[key]
    /// @resolution.name source=Event target=Event
    /// @resolution.variant source=Event.Key owner=Event variant=Key
    /// @resolution.pattern source="{ key }" kind=object fields=[key]
    /// @type.node source=key.length type=usize
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.member source=key.length receiver=string kind=field key=length

}
"#,
    );
}

#[test]
fn test_variant_pattern_rejects_wrong_owner() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

@derive(Tagged)
newtype Other = Done<string>;

declare const status: Status;

match (status) {
    Other.Done(value) => value
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

@derive(Tagged)
newtype Other = Done<string>;

declare const status: Status;

match (status) {
    Other.Done(value) => value
}

=== checked ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @resolution.name source=Tagged target=decorator.derive.Tagged

@derive(Tagged)
newtype Other = Done<string>;
/// @type.symbol symbol=Other source="newtype Other = Done<string>" type=Other
/// @definition.newtype symbol=Other source="newtype Other = Done<string>" value=Done<string>
/// @resolution.name source=Tagged target=decorator.derive.Tagged

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Other.Done(value) => value
    /// @type.symbol symbol=value source=value type=<error>
    /// @resolution.pattern source="Other.Done(value)" kind=variant owner=Other variant=Done payload=tuple fields=[0: value]
    /// @resolution.name source=Other target=Other
    /// @resolution.variant source=Other.Done owner=Other variant=Done
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value

}
"#,
        r#"
/// @diagnostic.error code=EC438 message="variant 'Other.Done' is not a variant of type 'Status'"
/// @diagnostic.label line=11 column=5 source="Other.Done(value)"
"#,
    );
}

#[test]
fn test_variant_pattern_rejects_missing_variant() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

match (status) {
    Status.Done(value) => value
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

match (status) {
    Status.Done(value) => value
}

=== checked ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @resolution.name source=Tagged target=decorator.derive.Tagged

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Done(value) => value
    /// @type.symbol symbol=value source=value type=<error>
    /// @resolution.pattern source="Status.Done(value)" kind=variant owner=Status variant=Done payload=tuple fields=[0: value]
    /// @resolution.name source=Status target=Status
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value

}
"#,
        r#"
/// @diagnostic.error code=EC439 message="variant 'Done' does not exist on type 'Status'"
/// @diagnostic.label line=8 column=12 source=Done
"#,
    );
}

#[test]
fn test_variant_pattern_participates_in_exhaustiveness() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

const label = match (status) {
    Status.Ok(value) => "ok"
    Status.Err(code) => "err"
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

const label: "ok" | "err" = match (status) {
    Status.Ok(value) => "ok"
    Status.Err(code) => "err"
};

=== checked ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @resolution.name source=Tagged target=decorator.derive.Tagged

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

const label = match (status) {
/// @type.symbol symbol=label source=label type="ok" | "err"
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Ok(value) => "ok"
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source="Status.Ok(value)" kind=variant owner=Status variant=Ok payload=tuple fields=[0: value]
    /// @type.node source="\"ok\"" type="ok"

    Status.Err(code) => "err"
    /// @type.symbol symbol=code source=code type=int32
    /// @resolution.pattern source="Status.Err(code)" kind=variant owner=Status variant=Err payload=tuple fields=[0: code]
    /// @type.node source="\"err\"" type="err"

};
"#,
    );
}

#[test]
fn test_guarded_variant_pattern_does_not_prove_exhaustiveness() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

const label = match (status) {
    Status.Ok(value) if (value.length > 0) => "ok"
    Status.Err(code) => "err"
};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: Status;

const label: "ok" | "err" = match (status) {
    Status.Ok(value) if (value.length > 0) => "ok"
    Status.Err(code) => "err"
};

=== checked ===
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @resolution.name source=Tagged target=decorator.derive.Tagged

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

const label = match (status) {
/// @type.symbol symbol=label source=label type="ok" | "err"
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Ok(value) if (value.length > 0) => "ok"
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source="Status.Ok(value)" kind=variant owner=Status variant=Ok payload=tuple fields=[0: value]
    /// @type.node source="value.length > 0" type=boolean
    /// @type.node source="\"ok\"" type="ok"

    Status.Err(code) => "err"
    /// @type.symbol symbol=code source=code type=int32
    /// @resolution.pattern source="Status.Err(code)" kind=variant owner=Status variant=Err payload=tuple fields=[0: code]
    /// @type.node source="\"err\"" type="err"

};
"#,
        r#"
/// @diagnostic.error code=EC403 message="match is not exhaustive: 'Status.Ok' is not covered"
/// @diagnostic.label line=7 column=15 source="match (status) {\n    Status.Ok(value) if (value.length > 0) => \"ok\"\n    Status.Err(code) => \"err\"\n}"
"#,
    );
}
