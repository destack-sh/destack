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
        DirRows::checked().with_layout(),
        r#"
=== annotated ===
@repr("uint8")
enum Mode {
    Read = 1,
    Write = 2,
}

const raw: uint8 = Mode.Read;
const mode: Mode = 1;

=== checked ===
@repr("uint8")
/// @type.symbol symbol=Mode type=Mode
/// @layout.type type=Mode shape=enum size=1 align=1 backing=uint8

enum Mode {
    Read = 1,
    /// @type.symbol symbol=Mode.Read type=Mode

    Write = 2,
    /// @type.symbol symbol=Mode.Write type=Mode
}

const raw: uint8 = Mode.Read;
/// @type.symbol symbol=raw type=uint8
/// @resolution.name source=Mode target=Mode
/// @resolution.member source=Mode.Read receiver=Mode kind=symbol target=Mode.Read

const mode: Mode = 1;
/// @type.symbol symbol=mode type=Mode
/// @resolution.name source=Mode target=Mode
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Mode' is not assignable to type 'uint8'"
/// @diagnostic.label line=8 column=7 source="const raw: uint8 = Mode.Read;"
/// @diagnostic.error code=EC200 message="type '1' is not assignable to type 'Mode'"
/// @diagnostic.label line=9 column=7 source="const mode: Mode = 1;"
"#,
    );
}
