use crate::tests::{DirRows, TestSession};

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
/// @diagnostic.label line=8 column=5 span="match" line_source="match (mode) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}

/// Reject an enum member pattern from an unrelated enum.
#[test]
fn test_reject_enum_member_pattern_from_another_enum() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready = 1,
    Failed = 2,
}

enum Other {
    Ready = 1,
}

declare const status: Status;

match (status) {
    Other.Ready => 0
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#""#, r#""#);
}

/// Reject an enum member pattern absent from its written enum.
#[test]
fn test_reject_missing_enum_member_pattern() {
    let session = TestSession::single(
        r#"
enum Status {
    Ready = 1,
    Failed = 2,
}

declare const status: Status;

match (status) {
    Status.Done => 0
}
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#""#, r#""#);
}

/// Match every case of an enum through a readonly borrow.
#[test]
fn test_match_borrowed_enum_exhaustively() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
}

declare const mode: &readonly Mode;

const value = match (mode) {
    Mode.Read => 10
    Mode.Write => 20
};
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#""#);
}
