use crate::tests::{DirRows, TestSession};

#[test]
fn test_variant_derivation_precedes_body_checking() {
    let session = TestSession::single(
        r#"
const circle = Shape.Round({ radius: 5 });
const rectangle = Shape.rectangle_shape({ width: 10, height: 20 });

@derive(Tagged({
    case: "snake_case",
    names: { circle: "Round" },
}))
newtype Shape =
    | { kind: "rectangleShape"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::none()
            .with_definitions()
            .with_decorators()
            .with_statics(),
        r#"
=== annotated ===
const circle = Shape.Round({ radius: 5 });
const rectangle = Shape.rectangle_shape({ width: 10, height: 20 });

@derive(
    Tagged({
        case: "snake_case",
        names: { circle: "Round" },
    }),
)
newtype Shape =
    | { kind: "rectangleShape"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };

=== checked ===
const circle = Shape.Round({ radius: 5 });
const rectangle = Shape.rectangle_shape({ width: 10, height: 20 });

@derive(Tagged({
/// @decorator.node expression=derive target=decorator.derive type=derive kind=derive providers=[decorator.derive.Tagged backing={ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames } type=Tagged] value="derive(Tagged({ case: \"snake_case\"; names: { circle: \"Round\" } }))"

    case: "snake_case",
    names: { circle: "Round" },
}))
newtype Shape =
/// @definition.newtype symbol=Shape discriminator=kind backing={ kind: "rectangleShape"; width: int32; height: int32 } | { kind: "circle"; radius: int32 }
/// @definition.variant symbol=Shape.Round source={ kind: "circle"; radius: int32 } key=Round discriminant=circle backing={ kind: "circle"; radius: int32 } argument={ radius: int32 }
/// @definition.variant symbol=Shape.rectangle_shape source={ kind: "rectangleShape"; width: int32; height: int32 } key=rectangle_shape discriminant=rectangleShape backing={ kind: "rectangleShape"; width: int32; height: int32 } argument={ width: int32; height: int32 }

    | { kind: "rectangleShape"; width: int32; height: int32 }
    | { kind: "circle"; radius: int32 };
"#,
    );
}

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
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=({ error: int32 }) => Status.Err
/// @type.symbol symbol=Status.Ok type=({ value: string }) => Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" discriminator=kind backing=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source=Err<int32> key=Err discriminant=Err backing=Err<int32> argument={ error: int32 }
/// @definition.variant symbol=Status.Ok source=Ok<string> key=Ok discriminant=Ok backing=Ok<string> argument={ value: string }
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.pattern source=status kind=binding target=status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node type=string | int32
/// @type.node source=status type=Status
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    Status.Ok(value) => value satisfies string
    /// @resolution.name source=Status.Ok target=Status
    /// @resolution.pattern source=Status.Ok(value) kind=variant predicate="variant.tag(Status, kind, \"Ok\") is \"Ok\"" projection="variant.payload(Status.Ok, backing=Ok<string>, discriminator=kind, value=String(#569faed0cbe4284e), type=Ok<string>)" payload=tuple fields=(value)
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source="value satisfies string" type=string
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value

    Status.Err(code) => code satisfies int32
    /// @resolution.name source=Status.Err target=Status
    /// @resolution.pattern source=Status.Err(code) kind=variant predicate="variant.tag(Status, kind, \"Err\") is \"Err\"" projection="variant.payload(Status.Err, backing=Err<int32>, discriminator=kind, value=String(#d43ec2a2697cb194), type=Err<int32>)" payload=tuple fields=(code)
    /// @type.symbol symbol=code source=code type=int32
    /// @resolution.pattern source=code kind=binding target=code
    /// @type.node source="code satisfies int32" type=int32
    /// @type.node source=code type=int32
    /// @resolution.name source=code target=code
    /// @resolution.place source=code placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=code root=code

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
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Event = { kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string };
/// @type.symbol symbol=Event type=Event
/// @type.symbol symbol=Event.Click type=({ x: int32; y: int32 }) => Event.Click
/// @type.symbol symbol=Event.Key type=({ key: string }) => Event.Key
/// @definition.newtype symbol=Event discriminator=kind backing={ kind: "click"; x: int32; y: int32 } | { kind: "key"; key: string }
/// @definition.variant symbol=Event.Click source={ kind: "click"; x: int32; y: int32 } key=Click discriminant=click backing={ kind: "click"; x: int32; y: int32 } argument={ x: int32; y: int32 }
/// @definition.variant symbol=Event.Key source={ kind: "key"; key: string } key=Key discriminant=key backing={ kind: "key"; key: string } argument={ key: string }

declare const event: Event;
/// @type.symbol symbol=event source=event type=Event
/// @resolution.pattern source=event kind=binding target=event
/// @resolution.name source=Event target=Event

match (event) {
/// @type.node type=int32 | usize
/// @type.node source=event type=Event
/// @resolution.name source=event target=event
/// @resolution.place source=event placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=event root=event

    Event.Click({ x, y }) => x + y
    /// @resolution.name source=Event.Click target=Event
    /// @resolution.pattern source="Event.Click({ x, y })" kind=variant predicate="variant.tag(Event, kind, \"click\") is \"click\"" projection="variant.payload(Event.Click, backing={ kind: \"click\"; x: int32; y: int32 }, discriminator=kind, value=String(#ed3e1eeb16139c23), type={ kind: \"click\"; x: int32; y: int32 })" payload=pattern
    /// @resolution.pattern source={ x, y } kind=object fields={ x, y }
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @type.node source="x + y" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @resolution.operator source="x + y" type=int32 operator="+" kind=builtin operands=[x as int32 families=(integer), y as int32 families=(integer)]
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=x
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y
    /// @resolution.place source=y placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=y root=y

    Event.Key({ key }) => key.length
    /// @resolution.name source=Event.Key target=Event
    /// @resolution.pattern source="Event.Key({ key })" kind=variant predicate="variant.tag(Event, kind, \"key\") is \"key\"" projection="variant.payload(Event.Key, backing={ kind: \"key\"; key: string }, discriminator=kind, value=String(#c1e0263bab2b8eff), type={ kind: \"key\"; key: string })" payload=pattern
    /// @resolution.pattern source={ key } kind=object fields={ key }
    /// @type.symbol symbol=key source=key type=string
    /// @type.node source=key type=string
    /// @type.node source=key.length type=usize
    /// @resolution.name source=key target=key
    /// @resolution.member source=key.length receiver=string type=usize kind=call target="string.string.length(parameters=(), arguments=(), return=usize)"
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

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
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=({ error: int32 }) => Status.Err
/// @type.symbol symbol=Status.Ok type=({ value: string }) => Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" discriminator=kind backing=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source=Err<int32> key=Err discriminant=Err backing=Err<int32> argument={ error: int32 }
/// @definition.variant symbol=Status.Ok source=Ok<string> key=Ok discriminant=Ok backing=Ok<string> argument={ value: string }
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Other = Ok<string>;
/// @type.symbol symbol=Other source="newtype Other = Ok<string>" type=Other
/// @type.symbol symbol=Other.Ok type=({ value: string }) => Other.Ok
/// @definition.newtype symbol=Other source="newtype Other = Ok<string>" discriminator=kind backing=Ok<string>
/// @definition.variant symbol=Other.Ok source=Ok<string> key=Ok discriminant=Ok backing=Ok<string> argument={ value: string }
/// @resolution.name source=Ok target=error.result.Ok

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.pattern source=status kind=binding target=status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node type=<error>
/// @type.node source=status type=Status
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    Other.Ok(value) => value
    /// @resolution.name source=Other.Ok target=Other
    /// @type.symbol symbol=value source=value type=<error>
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value

}
"#,
        r#"
/// @diagnostic.error id=pattern-variant-not-in-type message="variant 'Other.Ok' is not a variant of type 'Status'"
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
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=({ error: int32 }) => Status.Err
/// @type.symbol symbol=Status.Ok type=({ value: string }) => Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" discriminator=kind backing=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source=Err<int32> key=Err discriminant=Err backing=Err<int32> argument={ error: int32 }
/// @definition.variant symbol=Status.Ok source=Ok<string> key=Ok discriminant=Ok backing=Ok<string> argument={ value: string }
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.pattern source=status kind=binding target=status
/// @resolution.name source=Status target=Status

match (status) {
/// @type.node type=<error>
/// @type.node source=status type=Status
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    Status.Done(value) => value
    /// @resolution.name source=Status.Done target=Status
    /// @type.symbol symbol=value source=value type=<error>
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value

}
"#,
        r#"
/// @diagnostic.error id=pattern-variant-missing message="variant 'Done' does not exist on type 'Status'"
/// @diagnostic.label line=8 column=5 span="Status.Done(value)" line_source="Status.Done(value) => value"
"#,
    );
}

#[test]
fn test_match_borrowed_variant_exhaustively() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Status = Ok<string> | Err<int32>;

declare const status: &readonly Status;

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

declare const status: &'static readonly Status;

const label: "ok" | "err" = match (status) {
    Status.Ok(value) => "ok"
    Status.Err(code) => "err"
};

=== checked ===
@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=({ error: int32 }) => Status.Err
/// @type.symbol symbol=Status.Ok type=({ value: string }) => Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" discriminator=kind backing=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source=Err<int32> key=Err discriminant=Err backing=Err<int32> argument={ error: int32 }
/// @definition.variant symbol=Status.Ok source=Ok<string> key=Ok discriminant=Ok backing=Ok<string> argument={ value: string }
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: &readonly Status;
/// @type.symbol symbol=status source=status type=&'static readonly Status
/// @resolution.pattern source=status kind=binding target=status
/// @resolution.name source=Status target=Status

const label = match (status) {
/// @type.symbol symbol=label source=label type="ok" | "err"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="ok" | "err"
/// @type.node source=status type=&'static readonly Status
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="readonly"
/// @resolution.access source=status root=status

    Status.Ok(value) => "ok"
    /// @resolution.name source=Status.Ok target=Status
    /// @resolution.pattern source=Status.Ok(value) kind=variant predicate="variant.tag(Status, kind, \"Ok\") is \"Ok\"" projection="variant.payload(Status.Ok, backing=Ok<string>, discriminator=kind, value=String(#569faed0cbe4284e), type=Ok<string>)" payload=tuple fields=(value)
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source="\"ok\"" type="ok"

    Status.Err(code) => "err"
    /// @resolution.name source=Status.Err target=Status
    /// @resolution.pattern source=Status.Err(code) kind=variant predicate="variant.tag(Status, kind, \"Err\") is \"Err\"" projection="variant.payload(Status.Err, backing=Err<int32>, discriminator=kind, value=String(#d43ec2a2697cb194), type=Err<int32>)" payload=tuple fields=(code)
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
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Status = Ok<string> | Err<int32>;
/// @type.symbol symbol=Status source="newtype Status = Ok<string> | Err<int32>" type=Status
/// @type.symbol symbol=Status.Err type=({ error: int32 }) => Status.Err
/// @type.symbol symbol=Status.Ok type=({ value: string }) => Status.Ok
/// @definition.newtype symbol=Status source="newtype Status = Ok<string> | Err<int32>" discriminator=kind backing=Ok<string> | Err<int32>
/// @definition.variant symbol=Status.Err source=Err<int32> key=Err discriminant=Err backing=Err<int32> argument={ error: int32 }
/// @definition.variant symbol=Status.Ok source=Ok<string> key=Ok discriminant=Ok backing=Ok<string> argument={ value: string }
/// @resolution.name source=Ok target=error.result.Ok
/// @resolution.name source=Err target=error.result.Err

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.pattern source=status kind=binding target=status
/// @resolution.name source=Status target=Status

const label = match (status) {
/// @type.symbol symbol=label source=label type="ok" | "err"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="ok" | "err"
/// @type.node source=status type=Status
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    Status.Ok(value) if (value.length > 0) => "ok"
    /// @resolution.name source=Status.Ok target=Status
    /// @resolution.pattern source=Status.Ok(value) kind=variant predicate="variant.tag(Status, kind, \"Ok\") is \"Ok\"" projection="variant.payload(Status.Ok, backing=Ok<string>, discriminator=kind, value=String(#569faed0cbe4284e), type=Ok<string>)" payload=tuple fields=(value)
    /// @type.symbol symbol=value source=value type=string
    /// @resolution.pattern source=value kind=binding target=value
    /// @type.node source="value.length > 0" type=boolean
    /// @type.node source=value type=string
    /// @type.node source=value.length type=usize
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.length receiver=string type=usize kind=call target="string.string.length(parameters=(), arguments=(), return=usize)"
    /// @resolution.operator source="value.length > 0" type=boolean operator=">" kind=builtin operands=[value.length as usize families=(integer), 0 as usize families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value
    /// @type.node source=0 type=0
    /// @type.node source="\"ok\"" type="ok"

    Status.Err(code) => "err"
    /// @resolution.name source=Status.Err target=Status
    /// @resolution.pattern source=Status.Err(code) kind=variant predicate="variant.tag(Status, kind, \"Err\") is \"Err\"" projection="variant.payload(Status.Err, backing=Err<int32>, discriminator=kind, value=String(#d43ec2a2697cb194), type=Err<int32>)" payload=tuple fields=(code)
    /// @type.symbol symbol=code source=code type=int32
    /// @resolution.pattern source=code kind=binding target=code
    /// @type.node source="\"err\"" type="err"

};
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: 'Status.Ok' is not covered"
/// @diagnostic.label line=7 column=15 span="match (status) {\n    Status.Ok(value) if (value.length > 0) => \"ok\"\n    Status.Err(code) => \"err\"\n}" line_source="const label = match (status) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}

#[test]
fn test_enum_member_patterns_match_exhaustively() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10
        Mode.Write => 20
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10
        Mode.Write => 20
    }
}

=== checked ===
enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Read source="Read = 1" key=Read value=1
/// @definition.variant symbol=Mode.Write source="Write = 2" key=Write value=2

    Read = 1,
    /// @type.symbol symbol=Mode.Read source="Read = 1" type=Mode.Read

    Write = 2,
    /// @type.symbol symbol=Mode.Write source="Write = 2" type=Mode.Write

}

function describe(mode: Mode): int32 {
/// @type.symbol symbol=describe type=(Mode) => int32
/// @type.symbol symbol=describe.mode source="mode: Mode" type=Mode
/// @resolution.name source=Mode target=Mode

    match (mode) {
    /// @resolution.name source=mode target=describe.mode
    /// @resolution.place source=mode placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=mode root=describe.mode

        Mode.Read => 10
        /// @resolution.name source=Mode target=Mode
        /// @resolution.pattern source=Mode.Read kind=variant predicate="Mode is 1"

        Mode.Write => 20
        /// @resolution.name source=Mode target=Mode
        /// @resolution.pattern source=Mode.Write kind=variant predicate="Mode is 2"

    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_enum_member_match_reports_the_uncovered_member() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Mode {
    Read = 1,
    Write = 2,
}

function describe(mode: Mode): int32 {
    match (mode) {
        Mode.Read => 10
    }
}

=== checked ===
enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Read source="Read = 1" key=Read value=1
/// @definition.variant symbol=Mode.Write source="Write = 2" key=Write value=2

    Read = 1,
    /// @type.symbol symbol=Mode.Read source="Read = 1" type=Mode.Read

    Write = 2,
    /// @type.symbol symbol=Mode.Write source="Write = 2" type=Mode.Write

}

function describe(mode: Mode): int32 {
/// @type.symbol symbol=describe type=(Mode) => int32
/// @type.symbol symbol=describe.mode source="mode: Mode" type=Mode
/// @resolution.name source=Mode target=Mode

    match (mode) {
    /// @resolution.name source=mode target=describe.mode
    /// @resolution.place source=mode placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=mode root=describe.mode

        Mode.Read => 10
        /// @resolution.name source=Mode target=Mode
        /// @resolution.pattern source=Mode.Read kind=variant predicate="Mode is 1"

    }
}
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: 'Mode.Write' is not covered"
/// @diagnostic.label line=8 column=5 span="match (mode) {\n        Mode.Read => 10\n    }" line_source="match (mode) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}
