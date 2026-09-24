use crate::tests::{DirRows, TestSession};

#[test]
fn test_enum_backing_rejects_implicit_raw_assignment() {
    let session = TestSession::single(
        r#"
@repr("uint8")
enum Mode {
    Read = 1,
    Write = 2,
}

const raw: uint8 = Mode.Read;
const mode: Mode = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@repr("uint8")
enum Mode {
    Read = 1,
    Write = 2,
}

const raw: uint8 = Mode.Read;
const mode: Mode = 1;

=== dir ===
@repr("uint8")
/// @resolution.name source=repr target=repr

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

const raw: uint8 = Mode.Read;
/// @type.symbol symbol=raw source=raw type=uint8
/// @resolution.pattern source=raw kind=binding target=raw
/// @resolution.name source=Mode target=Mode
/// @resolution.member source=Mode.Read receiver=Mode type=Mode.Read kind=symbol target_receiver=Mode target=Mode.Read

const mode: Mode = 1;
/// @type.symbol symbol=mode source=mode type=Mode
/// @resolution.pattern source=mode kind=binding target=mode
/// @resolution.name source=Mode target=Mode
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Mode.Read' is not assignable to type 'uint8'"
/// @diagnostic.label line=8 column=20 span="Mode.Read" line_source="const raw: uint8 = Mode.Read;"
/// @diagnostic.related line=8 column=12 span="uint8" line_source="const raw: uint8 = Mode.Read;" message="expected due to this annotation"
/// @diagnostic.error id=not-assignable message="type '1' is not assignable to type 'Mode'"
/// @diagnostic.label line=9 column=20 span="1" line_source="const mode: Mode = 1;"
/// @diagnostic.related line=9 column=13 span="Mode" line_source="const mode: Mode = 1;" message="expected due to this annotation"
/// @diagnostic.error id=unsupported-representation message="representation 'uint8' is not supported by this declaration"
/// @diagnostic.label line=2 column=2 span="repr(\"uint8\")" line_source="@repr(\"uint8\")"
"#,
    );
}

/// An enum casts explicitly to its backing type.
#[test]
fn test_enum_backing_casts_explicitly_to_the_backing_type() {
    let session = TestSession::single(
        r#"
@repr("uint8")
enum Mode {
    Read = 1,
    Write = 2,
}

enum Direction {
    Up = "UP",
    Down = "DOWN",
}

declare const mode: Mode;

const raw = Mode.Read as uint8;
const name = Direction.Up as string;
const one = Mode.Read as 1;
const up = Direction.Up as "UP";
const backing = mode as uint8;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
@repr("uint8")
enum Mode {
    Read = 1,
    Write = 2,
}

enum Direction {
    Up = "UP",
    Down = "DOWN",
}

declare const mode: Mode;

const raw: uint8 = Mode.Read as uint8;
const name: string = Direction.Up as string;
const one: 1 = Mode.Read as 1;
const up: "UP" = Direction.Up as "UP";
const backing: uint8 = mode as uint8;

=== dir ===
@repr("uint8")
/// @resolution.name source=repr target=repr

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

enum Direction {
/// @type.symbol symbol=Direction type=Direction
/// @definition.enum symbol=Direction backing=string
/// @definition.variant symbol=Direction.Down source="Down = \"DOWN\"" key=Down value="\"DOWN\""
/// @definition.variant symbol=Direction.Up source="Up = \"UP\"" key=Up value="\"UP\""

    Up = "UP",
    /// @type.symbol symbol=Direction.Up source="Up = \"UP\"" type=Direction.Up

    Down = "DOWN",
    /// @type.symbol symbol=Direction.Down source="Down = \"DOWN\"" type=Direction.Down

}

declare const mode: Mode;
/// @type.symbol symbol=mode source=mode type=Mode
/// @resolution.pattern source=mode kind=binding target=mode
/// @resolution.name source=Mode target=Mode

const raw = Mode.Read as uint8;
/// @type.symbol symbol=raw source=raw type=uint8
/// @resolution.pattern source=raw kind=binding target=raw
/// @resolution.name source=Mode target=Mode
/// @resolution.member source=Mode.Read receiver=Mode type=Mode.Read kind=symbol target_receiver=Mode target=Mode.Read

const name = Direction.Up as string;
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Direction target=Direction
/// @resolution.member source=Direction.Up receiver=Direction type=Direction.Up kind=symbol target_receiver=Direction target=Direction.Up

const one = Mode.Read as 1;
/// @type.symbol symbol=one source=one type=1
/// @resolution.pattern source=one kind=binding target=one
/// @resolution.name source=Mode target=Mode
/// @resolution.member source=Mode.Read receiver=Mode type=Mode.Read kind=symbol target_receiver=Mode target=Mode.Read

const up = Direction.Up as "UP";
/// @type.symbol symbol=up source=up type="UP"
/// @resolution.pattern source=up kind=binding target=up
/// @resolution.name source=Direction target=Direction
/// @resolution.member source=Direction.Up receiver=Direction type=Direction.Up kind=symbol target_receiver=Direction target=Direction.Up

const backing = mode as uint8;
/// @type.symbol symbol=backing source=backing type=uint8
/// @resolution.pattern source=backing kind=binding target=backing
/// @resolution.name source=mode target=mode
/// @resolution.place source=mode placement="local" lifetime="static" access="immutable"
/// @resolution.access source=mode root=mode
"#, r#"
/// @diagnostic.error id=invalid-cast message="type 'Mode' cannot be cast to 'uint8'"
/// @diagnostic.label line=19 column=17 span="mode" line_source="const backing = mode as uint8;"
/// @diagnostic.error id=unsupported-representation message="representation 'uint8' is not supported by this declaration"
/// @diagnostic.label line=2 column=2 span="repr(\"uint8\")" line_source="@repr(\"uint8\")"
"#);
}

/// An enum rejects casts to a mismatched literal or from an undecided value.
#[test]
fn test_enum_rejects_a_cast_to_a_mismatched_literal() {
    let session = TestSession::single(
        r#"
@repr("uint8")
enum Mode {
    Read = 1,
    Write = 2,
}

enum Direction {
    Up = "UP",
    Down = "DOWN",
}

declare const mode: Mode;
declare const direction: Direction;

const wrong = Mode.Read as 2;
const guessed = direction as "UP";
const undecided = mode as 1;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
@repr("uint8")
enum Mode {
    Read = 1,
    Write = 2,
}

enum Direction {
    Up = "UP",
    Down = "DOWN",
}

declare const mode: Mode;
declare const direction: Direction;

const wrong: 2 = Mode.Read as 2;
const guessed: "UP" = direction as "UP";
const undecided: 1 = mode as 1;

=== dir ===
@repr("uint8")
/// @resolution.name source=repr target=repr

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

enum Direction {
/// @type.symbol symbol=Direction type=Direction
/// @definition.enum symbol=Direction backing=string
/// @definition.variant symbol=Direction.Down source="Down = \"DOWN\"" key=Down value="\"DOWN\""
/// @definition.variant symbol=Direction.Up source="Up = \"UP\"" key=Up value="\"UP\""

    Up = "UP",
    /// @type.symbol symbol=Direction.Up source="Up = \"UP\"" type=Direction.Up

    Down = "DOWN",
    /// @type.symbol symbol=Direction.Down source="Down = \"DOWN\"" type=Direction.Down

}

declare const mode: Mode;
/// @type.symbol symbol=mode source=mode type=Mode
/// @resolution.pattern source=mode kind=binding target=mode
/// @resolution.name source=Mode target=Mode

declare const direction: Direction;
/// @type.symbol symbol=direction source=direction type=Direction
/// @resolution.pattern source=direction kind=binding target=direction
/// @resolution.name source=Direction target=Direction

const wrong = Mode.Read as 2;
/// @type.symbol symbol=wrong source=wrong type=2
/// @resolution.pattern source=wrong kind=binding target=wrong
/// @resolution.name source=Mode target=Mode
/// @resolution.member source=Mode.Read receiver=Mode type=Mode.Read kind=symbol target_receiver=Mode target=Mode.Read

const guessed = direction as "UP";
/// @type.symbol symbol=guessed source=guessed type="UP"
/// @resolution.pattern source=guessed kind=binding target=guessed
/// @resolution.name source=direction target=direction
/// @resolution.place source=direction placement="local" lifetime="static" access="immutable"
/// @resolution.access source=direction root=direction

const undecided = mode as 1;
/// @type.symbol symbol=undecided source=undecided type=1
/// @resolution.pattern source=undecided kind=binding target=undecided
/// @resolution.name source=mode target=mode
/// @resolution.place source=mode placement="local" lifetime="static" access="immutable"
/// @resolution.access source=mode root=mode
"#, r#"
/// @diagnostic.error id=invalid-cast message="type 'Mode.Read' cannot be cast to '2'"
/// @diagnostic.label line=16 column=15 span="Mode.Read" line_source="const wrong = Mode.Read as 2;"
/// @diagnostic.error id=invalid-cast message="type 'Direction' cannot be cast to '\"UP\"'"
/// @diagnostic.label line=17 column=17 span="direction" line_source="const guessed = direction as \"UP\";"
/// @diagnostic.error id=invalid-cast message="type 'Mode' cannot be cast to '1'"
/// @diagnostic.label line=18 column=19 span="mode" line_source="const undecided = mode as 1;"
/// @diagnostic.error id=unsupported-representation message="representation 'uint8' is not supported by this declaration"
/// @diagnostic.label line=2 column=2 span="repr(\"uint8\")" line_source="@repr(\"uint8\")"
"#);
}

/// An enum absorbs its own variant in a union, so a nullish fallback to a variant keeps the enum type.
#[test]
fn test_absorb_an_enum_variant_into_its_enum_in_a_union() {
    let session = TestSession::single(
        r#"
enum Kind {
    Internal,
    Server,
}

function pick(kind?: Kind): Kind {
    return kind ?? Kind.Internal;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Kind {
    Internal,
    Server,
}

function pick(kind?: Kind): Kind {
    return kind ?? Kind.Internal;
}

=== dir ===
enum Kind {
/// @type.symbol symbol=Kind type=Kind
/// @definition.enum symbol=Kind
/// @definition.variant symbol=Kind.Internal source=Internal key=Internal value=0
/// @definition.variant symbol=Kind.Server source=Server key=Server value=1

    Internal,
    /// @type.symbol symbol=Kind.Internal source=Internal type=Kind.Internal

    Server,
    /// @type.symbol symbol=Kind.Server source=Server type=Kind.Server

}

function pick(kind?: Kind): Kind {
/// @type.symbol symbol=pick type=(Kind | undefined?) => Kind
/// @type.symbol symbol=pick.kind source="kind?: Kind" type=Kind | undefined
/// @resolution.name source=Kind target=Kind
/// @resolution.name source=Kind target=Kind

    return kind ?? Kind.Internal;
    /// @resolution.name source=kind target=pick.kind
    /// @resolution.operator source="kind ?? Kind.Internal" type=Kind operator="??" kind=builtin operands=[kind as Kind | undefined families=(Kind | undefined), Kind.Internal as Kind.Internal families=(Kind)]
    /// @resolution.place source=kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=kind root=pick.kind
    /// @resolution.name source=Kind target=Kind
    /// @resolution.member source=Kind.Internal receiver=Kind type=Kind.Internal kind=symbol target_receiver=Kind target=Kind.Internal

}
"#,
    );
}
