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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = 0..5;

const low: Count = 0;
const high: Count = 4;
const bad: Count = 5;

=== dir ===
type Count = 0..5;
/// @type.symbol symbol=Count source="type Count = 0..5" type=0..5
/// @definition.type symbol=Count source="type Count = 0..5" value=0..5

const low: Count = 0;
/// @type.symbol symbol=low source=low type=Count
/// @resolution.pattern source=low kind=binding target=low
/// @resolution.name source=Count target=Count

const high: Count = 4;
/// @type.symbol symbol=high source=high type=Count
/// @resolution.pattern source=high kind=binding target=high
/// @resolution.name source=Count target=Count

const bad: Count = 5;
/// @type.symbol symbol=bad source=bad type=Count
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Count target=Count
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '5' is not assignable to type 'Count'"
/// @diagnostic.label line=6 column=20 span="5" line_source="const bad: Count = 5;"
/// @diagnostic.related line=6 column=12 span="Count" line_source="const bad: Count = 5;" message="expected due to this annotation"
/// @diagnostic.note message="'Count' reduces to '0..5'"
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Digit = 0..=9;

const zero: Digit = 0;
const nine: Digit = 9;
const bad: Digit = 10;

=== dir ===
type Digit = 0..=9;
/// @type.symbol symbol=Digit source="type Digit = 0..=9" type=0..=9
/// @definition.type symbol=Digit source="type Digit = 0..=9" value=0..=9

const zero: Digit = 0;
/// @type.symbol symbol=zero source=zero type=Digit
/// @resolution.pattern source=zero kind=binding target=zero
/// @resolution.name source=Digit target=Digit

const nine: Digit = 9;
/// @type.symbol symbol=nine source=nine type=Digit
/// @resolution.pattern source=nine kind=binding target=nine
/// @resolution.name source=Digit target=Digit

const bad: Digit = 10;
/// @type.symbol symbol=bad source=bad type=Digit
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Digit target=Digit
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '10' is not assignable to type 'Digit'"
/// @diagnostic.label line=6 column=20 span="10" line_source="const bad: Digit = 10;"
/// @diagnostic.related line=6 column=12 span="Digit" line_source="const bad: Digit = 10;" message="expected due to this annotation"
/// @diagnostic.note message="'Digit' reduces to '0..=9'"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Offset = -4..=4;

const left: Offset = -4;
const right: Offset = 4;

=== dir ===
type Offset = -4..=4;
/// @type.symbol symbol=Offset source="type Offset = -4..=4" type=-4..=4
/// @definition.type symbol=Offset source="type Offset = -4..=4" value=-4..=4

const left: Offset = -4;
/// @type.symbol symbol=left source=left type=Offset
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=Offset target=Offset
/// @resolution.operator source=-4 type=-4 operator="-" kind=builtin operands=[4 as 4 families=(integer)]

const right: Offset = 4;
/// @type.symbol symbol=right source=right type=Offset
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=Offset target=Offset
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = 0..;

=== dir ===
type Count = 0..;
/// @type.symbol symbol=Count source="type Count = 0.." type=<error>
/// @definition.type symbol=Count source="type Count = 0.." value=<error>
"#,
        r#"
/// @diagnostic.error id=unbounded-interval-type message="interval type must be bounded"
/// @diagnostic.label line=2 column=15 span=".." line_source="type Count = 0..;"
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = ..;

=== dir ===
type Count = ..;
/// @type.symbol symbol=Count source="type Count = .." type=<error>
/// @definition.type symbol=Count source="type Count = .." value=<error>
"#,
        r#"
/// @diagnostic.error id=unbounded-interval-type message="interval type must be bounded"
/// @diagnostic.label line=2 column=14 span=".." line_source="type Count = ..;"
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Unit = 0.0..=1.0;

=== dir ===
type Unit = 0.0..=1.0;
/// @type.symbol symbol=Unit source="type Unit = 0.0..=1.0" type=<error>
/// @definition.type symbol=Unit source="type Unit = 0.0..=1.0" value=<error>
"#,
        r#"
/// @diagnostic.error id=invalid-interval-domain message="interval type bounds must be integer, bigint, or char literals"
/// @diagnostic.label line=2 column=16 span="..=" line_source="type Unit = 0.0..=1.0;"
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Mixed = 0..='z';

=== dir ===
type Mixed = 0..='z';
/// @type.symbol symbol=Mixed source="type Mixed = 0..='z'" type=<error>
/// @definition.type symbol=Mixed source="type Mixed = 0..='z'" value=<error>
"#,
        r#"
/// @diagnostic.error id=invalid-interval-domain message="interval type bounds must be integer, bigint, or char literals"
/// @diagnostic.label line=2 column=15 span="..=" line_source="type Mixed = 0..='z';"
"#,
    );
}
