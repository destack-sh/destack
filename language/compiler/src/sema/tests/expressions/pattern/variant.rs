use crate::tests::{DirRows, TestSession};

/// Match every member of an enum through member patterns.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
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
    /// @resolution.coverage exhaustive=true disjoint=true
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

/// Report the enum member that a match leaves uncovered.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
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
    /// @resolution.coverage exhaustive=false disjoint=true
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
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

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Failed source="Failed = 2" key=Failed value=2
/// @definition.variant symbol=Status.Ready source="Ready = 1" key=Ready value=1

    Ready = 1,
    /// @type.symbol symbol=Status.Ready source="Ready = 1" type=Status.Ready

    Failed = 2,
    /// @type.symbol symbol=Status.Failed source="Failed = 2" type=Status.Failed

}

enum Other {
/// @type.symbol symbol=Other type=Other
/// @definition.enum symbol=Other
/// @definition.variant symbol=Other.Ready source="Ready = 1" key=Ready value=1

    Ready = 1,
    /// @type.symbol symbol=Other.Ready source="Ready = 1" type=Other.Ready

}

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.pattern source=status kind=binding target=status
/// @resolution.name source=Status target=Status

match (status) {
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="immutable"
/// @resolution.access source=status root=status

    Other.Ready => 0
    /// @resolution.name source=Other target=Other
    /// @resolution.rejected source=Other.Ready

}
"#, r#"
/// @diagnostic.error id=pattern-variant-not-in-type message="variant 'Other.Ready' is not a variant of type 'Status'"
/// @diagnostic.label line=14 column=5 span="Other.Ready" line_source="Other.Ready => 0"
"#);
}

/// Reject an enum member pattern naming an unknown member.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Ready = 1,
    Failed = 2,
}

declare const status: Status;

match (status) {
    Status.Done => 0
}

=== dir ===
enum Status {
/// @type.symbol symbol=Status type=Status
/// @definition.enum symbol=Status
/// @definition.variant symbol=Status.Failed source="Failed = 2" key=Failed value=2
/// @definition.variant symbol=Status.Ready source="Ready = 1" key=Ready value=1

    Ready = 1,
    /// @type.symbol symbol=Status.Ready source="Ready = 1" type=Status.Ready

    Failed = 2,
    /// @type.symbol symbol=Status.Failed source="Failed = 2" type=Status.Failed

}

declare const status: Status;
/// @type.symbol symbol=status source=status type=Status
/// @resolution.pattern source=status kind=binding target=status
/// @resolution.name source=Status target=Status

match (status) {
/// @resolution.coverage exhaustive=true disjoint=true
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="immutable"
/// @resolution.access source=status root=status

    Status.Done => 0
    /// @resolution.name source=Status target=Status
    /// @resolution.rejected source=Status.Done
    /// @resolution.pattern source=Status.Done kind=wildcard

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'Done' does not exist on type 'Status'"
/// @diagnostic.label line=10 column=12 span="Done" line_source="Status.Done => 0"
"#,
    );
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
enum Mode {
    Read = 1,
    Write = 2,
}

declare const mode: &'static readonly Mode;

const value: 10 | 20 = match (mode) {
    Mode.Read => 10
    Mode.Write => 20
};

=== dir ===
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

declare const mode: &readonly Mode;
/// @type.symbol symbol=mode source=mode type=&'static readonly Mode
/// @resolution.pattern source=mode kind=binding target=mode
/// @resolution.name source=Mode target=Mode

const value = match (mode) {
/// @type.symbol symbol=value source=value type=10 | 20
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.coverage exhaustive=true disjoint=true
/// @resolution.name source=mode target=mode
/// @resolution.place source=mode placement="local" lifetime="static" access="immutable"
/// @resolution.access source=mode root=mode

    Mode.Read => 10
    /// @resolution.name source=Mode target=Mode
    /// @resolution.pattern source=Mode.Read kind=variant predicate="Mode is 1"

    Mode.Write => 20
    /// @resolution.name source=Mode target=Mode
    /// @resolution.pattern source=Mode.Write kind=variant predicate="Mode is 2"

};
"#,
    );
}
