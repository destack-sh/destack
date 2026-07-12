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
/// @resolution.name source=derive target=decorator.derive.derive

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=Status.Err
/// @type.symbol symbol=Status.Ok type=Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source="newtype Status = Ok<string> | Err<int32>" key=Err
/// @definition.variant symbol=Status.Ok source="newtype Status = Ok<string> | Err<int32>" key=Ok
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node type=string | int32
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Ok(value) => value satisfies string
    /// @resolution.name source=Status.Ok target=Status
    /// @resolution.pattern source=Status.Ok(value) kind=variant predicate="variant.tag(\"Ok\") is \"Ok\"" projection="variant.payload(Status.Ok, { value: string })" payload=tuple fields=(value)
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value

    Status.Err(code) => code satisfies int32
    /// @resolution.name source=Status.Err target=Status
    /// @resolution.pattern source=Status.Err(code) kind=variant predicate="variant.tag(\"Err\") is \"Err\"" projection="variant.payload(Status.Err, { error: int32 })" payload=tuple fields=(code)
    /// @type.symbol symbol=code source=code type=int32
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
/// @resolution.name source=derive target=decorator.derive.derive

newtype Event = { kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string };
/// @type.symbol symbol=Event type=Event
/// @type.symbol symbol=Event.Click type=Event.Click
/// @type.symbol symbol=Event.Key type=Event.Key
/// @definition.newtype symbol=Event value={ kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string }
/// @definition.variant symbol=Event.Click key=Click
/// @definition.variant symbol=Event.Key key=Key

declare const event: Event;
/// @type.symbol symbol=event source=event type=Event
/// @resolution.name source=Event target=Event

match (event) {
/// @type.node type=int32 | usize
/// @type.node source=event type=Event
/// @resolution.name source=event target=event

    Event.Click({ x, y }) => x + y
    /// @resolution.name source=Event.Click target=Event
    /// @resolution.pattern source="Event.Click({ x, y })" kind=variant predicate="variant.tag(\"click\") is \"click\"" projection="variant.payload(Event.Click, { x: int32; y: int32 })" payload=tuple fields=(pattern)
    /// @resolution.pattern source={ x, y } kind=object fields={ x, y }
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @type.node source="x + y" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @resolution.call source="x + y" parameters=() return=int32 kind=builtin builtin=binary.add
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y

    Event.Key({ key }) => key.length
    /// @resolution.name source=Event.Key target=Event
    /// @resolution.pattern source="Event.Key({ key })" kind=variant predicate="variant.tag(\"key\") is \"key\"" projection="variant.payload(Event.Key, { key: string })" payload=tuple fields=(pattern)
    /// @resolution.pattern source={ key } kind=object fields={ key }
    /// @type.symbol symbol=key source=key type=string
    /// @type.node source=key type=string
    /// @type.node source=key.length type=usize
    /// @resolution.name source=key target=key
    /// @resolution.member source=key.length receiver=string kind=symbol target=string.string.length

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
newtype Other = Ok<string>;

declare const status: Status;

match (status) {
    Other.Ok(value) => value
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
newtype Other = Ok<string>;

declare const status: Status;

match (status) {
    Other.Ok(value) => value
}

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=Status.Err
/// @type.symbol symbol=Status.Ok type=Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source="newtype Status = Ok<string> | Err<int32>" key=Err
/// @definition.variant symbol=Status.Ok source="newtype Status = Ok<string> | Err<int32>" key=Ok
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive

newtype Other = Ok<string>;
/// @type.symbol symbol=Other source="newtype Other = Ok<string>" type=Other
/// @type.symbol symbol=Other.Ok type=Other.Ok
/// @definition.newtype symbol=Other source="newtype Other = Ok<string>" value=Ok<string>
/// @definition.variant symbol=Other.Ok source="newtype Other = Ok<string>" key=Ok
/// @resolution.name source=Ok target=error.result.Ok

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node type=<error>
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Other.Ok(value) => value
    /// @resolution.name source=Other.Ok target=Other
    /// @type.symbol symbol=value source=value type=<error>
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value

}
"#,
        r#"
/// @diagnostic.error code=EC438 message="variant 'Other.Ok' is not a variant of type 'Status'"
/// @diagnostic.label line=11 column=5 span="Other.Ok(value)" line_source="Other.Ok(value) => value"
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
/// @resolution.name source=derive target=decorator.derive.derive

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=Status.Err
/// @type.symbol symbol=Status.Ok type=Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source="newtype Status = Ok<string> | Err<int32>" key=Err
/// @definition.variant symbol=Status.Ok source="newtype Status = Ok<string> | Err<int32>" key=Ok
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node type=<error>
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Done(value) => value
    /// @resolution.name source=Status.Done target=Status
    /// @type.symbol symbol=value source=value type=<error>
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value

}
"#,
        r#"
/// @diagnostic.error code=EC439 message="variant 'Done' does not exist on type 'Status'"
/// @diagnostic.label line=8 column=5 span="Status.Done(value)" line_source="Status.Done(value) => value"
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
/// @resolution.name source=derive target=decorator.derive.derive

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=Status.Err
/// @type.symbol symbol=Status.Ok type=Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source="newtype Status = Ok<string> | Err<int32>" key=Err
/// @definition.variant symbol=Status.Ok source="newtype Status = Ok<string> | Err<int32>" key=Ok
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

const label = match (status) {
/// @type.symbol symbol=label source=label type="ok" | "err"
/// @type.node type="ok" | "err"
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Ok(value) => "ok"
    /// @resolution.name source=Status.Ok target=Status
    /// @resolution.pattern source=Status.Ok(value) kind=variant predicate="variant.tag(\"Ok\") is \"Ok\"" projection="variant.payload(Status.Ok, { value: string })" payload=tuple fields=(value)
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source="\"ok\"" type="ok"

    Status.Err(code) => "err"
    /// @resolution.name source=Status.Err target=Status
    /// @resolution.pattern source=Status.Err(code) kind=variant predicate="variant.tag(\"Err\") is \"Err\"" projection="variant.payload(Status.Err, { error: int32 })" payload=tuple fields=(code)
    /// @type.symbol symbol=code source=code type=int32
    /// @resolution.pattern source=code kind=binding target=code
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
/// @resolution.name source=derive target=decorator.derive.derive

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=Status.Err
/// @type.symbol symbol=Status.Ok type=Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" value=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source="newtype Status = Ok<string> | Err<int32>" key=Err
/// @definition.variant symbol=Status.Ok source="newtype Status = Ok<string> | Err<int32>" key=Ok
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.name source=Status target=Status

const label = match (status) {
/// @type.symbol symbol=label source=label type="ok" | "err"
/// @type.node type="ok" | "err"
/// @type.node source=status type=Status
/// @resolution.name source=status target=status

    Status.Ok(value) if (value.length > 0) => "ok"
    /// @resolution.name source=Status.Ok target=Status
    /// @resolution.pattern source=Status.Ok(value) kind=variant predicate="variant.tag(\"Ok\") is \"Ok\"" projection="variant.payload(Status.Ok, { value: string })" payload=tuple fields=(value)
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source="value.length > 0" type=boolean
    /// @type.node source=value type=string
    /// @type.node source=value.length type=usize
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.length receiver=string kind=symbol target=string.string.length
    /// @resolution.call source="value.length > 0" parameters=() return=boolean kind=builtin builtin=binary.greater_than
    /// @type.node source=0 type=0
    /// @type.node source="\"ok\"" type="ok"

    Status.Err(code) => "err"
    /// @resolution.name source=Status.Err target=Status
    /// @resolution.pattern source=Status.Err(code) kind=variant predicate="variant.tag(\"Err\") is \"Err\"" projection="variant.payload(Status.Err, { error: int32 })" payload=tuple fields=(code)
    /// @type.symbol symbol=code source=code type=int32
    /// @resolution.pattern source=code kind=binding target=code
    /// @type.node source="\"err\"" type="err"

};
"#,
        r#"
/// @diagnostic.error code=EC403 message="match is not exhaustive: 'Status.Ok' is not covered"
/// @diagnostic.label line=7 column=21 span="(status) {\n    Status.Ok(value) if (value.length > 0) => \"ok\"\n    Status.Err(code) => \"err\"\n}" line_source="const label = match (status) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}
