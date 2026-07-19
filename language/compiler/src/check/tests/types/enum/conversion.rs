use crate::tests::{DirRows, TestSession};

#[test]
fn test_enum_backing_does_not_allow_implicit_raw_assignment() {
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@repr("uint8" as Representation)
enum Mode {
    Read = 1,
    Write = 2,
}

const raw: uint8 = Mode.Read;
const mode: Mode = 1;

=== checked ===
@repr("uint8")
/// @resolution.name source=repr target=decorator.representation.repr

enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode backing=uint8 representation=uint8
/// @definition.variant symbol=Mode.Read source="Read = 1" key=Read value=1
/// @definition.variant symbol=Mode.Write source="Write = 2" key=Write value=2

    Read = 1,
    /// @type.symbol symbol=Mode.Read source="Read = 1" type=Mode.Read

    Write = 2,
    /// @type.symbol symbol=Mode.Write source="Write = 2" type=Mode.Write

}

const raw: uint8 = Mode.Read;
/// @type.symbol symbol=raw source=raw type=uint8
/// @resolution.name source=Mode target=Mode
/// @resolution.member source=Mode.Read receiver=Mode kind=symbol target=Mode.Read

const mode: Mode = 1;
/// @type.symbol symbol=mode source=mode type=Mode
/// @resolution.name source=Mode target=Mode
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Mode.Read' is not assignable to type 'uint8'"
/// @diagnostic.label line=8 column=20 span="Mode.Read" line_source="const raw: uint8 = Mode.Read;"
/// @diagnostic.error id=not-assignable message="type '1' is not assignable to type 'Mode'"
/// @diagnostic.label line=9 column=20 span="1" line_source="const mode: Mode = 1;"
"#,
    );
}
