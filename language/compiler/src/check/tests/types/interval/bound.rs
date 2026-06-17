use crate::tests::{DirRows, TestSession};

#[test]
fn test_half_open_interval_excludes_end_bound() {
    let session = TestSession::single(
        r#"
type Count = 0..5;

const low: Count = 0;
const high: Count = 4;
const bad: Count = 5;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = 0..5;

const low: Count = 0;
const high: Count = 4;
const bad: Count = 5;

=== checked ===
type Count = 0..5;
/// @type.symbol symbol=Count source="type Count = 0..5" type=0..5
/// @definition.type symbol=Count source="type Count = 0..5" value=0..5

const low: Count = 0;
/// @type.symbol symbol=low source=low type=0..5
/// @resolution.name source=Count target=Count
/// @type.node source=0 type=0

const high: Count = 4;
/// @type.symbol symbol=high source=high type=0..5
/// @resolution.name source=Count target=Count
/// @type.node source=4 type=4

const bad: Count = 5;
/// @type.symbol symbol=bad source=bad type=0..5
/// @resolution.name source=Count target=Count
/// @type.node source=5 type=5
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '5' is not assignable to type 'Count'"
/// @diagnostic.label line=6 column=7 source="const bad: Count = 5;"
"#,
    );
}

#[test]
fn test_inclusive_interval_includes_end_bound() {
    let session = TestSession::single(
        r#"
type Digit = 0..=9;

const zero: Digit = 0;
const nine: Digit = 9;
const bad: Digit = 10;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Digit = 0..=9;

const zero: Digit = 0;
const nine: Digit = 9;
const bad: Digit = 10;

=== checked ===
type Digit = 0..=9;
/// @type.symbol symbol=Digit source="type Digit = 0..=9" type=0..=9
/// @definition.type symbol=Digit source="type Digit = 0..=9" value=0..=9

const zero: Digit = 0;
/// @type.symbol symbol=zero source=zero type=0..=9
/// @resolution.name source=Digit target=Digit
/// @type.node source=0 type=0

const nine: Digit = 9;
/// @type.symbol symbol=nine source=nine type=0..=9
/// @resolution.name source=Digit target=Digit
/// @type.node source=9 type=9

const bad: Digit = 10;
/// @type.symbol symbol=bad source=bad type=0..=9
/// @resolution.name source=Digit target=Digit
/// @type.node source=10 type=10
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '10' is not assignable to type 'Digit'"
/// @diagnostic.label line=6 column=7 source="const bad: Digit = 10;"
"#,
    );
}

#[test]
fn test_negative_interval_preserves_signed_bounds() {
    let session = TestSession::single(
        r#"
type Offset = -4..=4;

const left: Offset = -4;
const right: Offset = 4;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Offset = -4..=4;

const left: Offset = -4;
const right: Offset = 4;

=== checked ===
type Offset = -4..=4;
/// @type.symbol symbol=Offset source="type Offset = -4..=4" type=-4..=4
/// @definition.type symbol=Offset source="type Offset = -4..=4" value=-4..=4

const left: Offset = -4;
/// @type.symbol symbol=left source=left type=-4..=4
/// @resolution.name source=Offset target=Offset
/// @type.node source=-4 type=-4

const right: Offset = 4;
/// @type.symbol symbol=right source=right type=-4..=4
/// @resolution.name source=Offset target=Offset
/// @type.node source=4 type=4
"#,
    );
}

#[test]
fn test_interval_type_rejects_unbounded_range() {
    let session = TestSession::single(
        r#"
type Count = 0..;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = 0..;

=== checked ===
type Count = 0..;
/// @type.symbol symbol=Count source="type Count = 0.." type=<error>
/// @definition.type symbol=Count source="type Count = 0.." value=<error>
"#,
        r#"
/// @diagnostic.error code=EC502 message="interval type must be bounded"
/// @diagnostic.label line=2 column=14 source="0.."
"#,
    );
}

#[test]
fn test_interval_type_rejects_full_range() {
    let session = TestSession::single(
        r#"
type Count = ..;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = ..;

=== checked ===
type Count = ..;
/// @type.symbol symbol=Count source="type Count = .." type=<error>
/// @definition.type symbol=Count source="type Count = .." value=<error>
"#,
        r#"
/// @diagnostic.error code=EC502 message="interval type must be bounded"
/// @diagnostic.label line=2 column=14 source=".."
"#,
    );
}

#[test]
fn test_interval_type_rejects_float_bounds() {
    let session = TestSession::single(
        r#"
type Unit = 0.0..=1.0;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Unit = 0.0..=1.0;

=== checked ===
type Unit = 0.0..=1.0;
/// @type.symbol symbol=Unit source="type Unit = 0.0..=1.0" type=<error>
/// @definition.type symbol=Unit source="type Unit = 0.0..=1.0" value=<error>
"#,
        r#"
/// @diagnostic.error code=EC503 message="interval type bounds must be integer, bigint, or char literals"
/// @diagnostic.label line=2 column=13 source="0.0..=1.0"
"#,
    );
}

#[test]
fn test_interval_type_rejects_mixed_bound_domains() {
    let session = TestSession::single(
        r#"
type Mixed = 0..='z';
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Mixed = 0..='z';

=== checked ===
type Mixed = 0..='z';
/// @type.symbol symbol=Mixed source="type Mixed = 0..='z'" type=<error>
/// @definition.type symbol=Mixed source="type Mixed = 0..='z'" value=<error>
"#,
        r#"
/// @diagnostic.error code=EC503 message="interval type bounds must be integer, bigint, or char literals"
/// @diagnostic.label line=2 column=14 source="0..='z'"
"#,
    );
}
