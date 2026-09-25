use crate::tests::{DirRows, TestSession};

/// Assigning a plain scalar to an interval alias reports a diagnostic.
#[test]
fn test_interval_assignment_requires_interval_compatible_type() {
    let session = TestSession::single(
        r#"
type Digit = 0..=9;

declare const value: int32;
const digit: Digit = value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Digit = 0..=9;

declare const value: int32;
const digit: Digit = value;

=== dir ===
type Digit = 0..=9;
/// @type.symbol symbol=Digit source="type Digit = 0..=9" type=0..=9
/// @definition.type symbol=Digit source="type Digit = 0..=9" value=0..=9

declare const value: int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

const digit: Digit = value;
/// @type.symbol symbol=digit source=digit type=Digit
/// @resolution.pattern source=digit kind=binding target=digit
/// @resolution.name source=Digit target=Digit
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'int32' is not assignable to type 'Digit'"
/// @diagnostic.label line=5 column=22 span="value" line_source="const digit: Digit = value;"
/// @diagnostic.related line=5 column=14 span="Digit" line_source="const digit: Digit = value;" message="expected due to this annotation"
/// @diagnostic.note message="'Digit' reduces to '0..=9'"
"#,
    );
}

/// Arithmetic on an interval widens past its bounds and reports a diagnostic.
#[test]
fn test_interval_assignment_does_not_prove_arithmetic_bounds() {
    let session = TestSession::single(
        r#"
type Digit = 0..=9;

declare const digit: Digit;
const next: Digit = digit + 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Digit = 0..=9;

declare const digit: Digit;
const next: Digit = digit + 1;

=== dir ===
type Digit = 0..=9;
/// @type.symbol symbol=Digit source="type Digit = 0..=9" type=0..=9
/// @definition.type symbol=Digit source="type Digit = 0..=9" value=0..=9

declare const digit: Digit;
/// @type.symbol symbol=digit source=digit type=Digit
/// @resolution.pattern source=digit kind=binding target=digit
/// @resolution.name source=Digit target=Digit

const next: Digit = digit + 1;
/// @type.symbol symbol=next source=next type=Digit
/// @resolution.pattern source=next kind=binding target=next
/// @resolution.name source=Digit target=Digit
/// @resolution.name source=digit target=digit
/// @resolution.operator source="digit + 1" type=int64 operator="+" kind=builtin operands=[digit as int64 families=(integer), 1 as int64 families=(integer)]
/// @resolution.place source=digit placement="local" lifetime="static" access="immutable"
/// @resolution.access source=digit root=digit
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'int64' is not assignable to type 'Digit'"
/// @diagnostic.label line=5 column=21 span="digit + 1" line_source="const next: Digit = digit + 1;"
/// @diagnostic.related line=5 column=13 span="Digit" line_source="const next: Digit = digit + 1;" message="expected due to this annotation"
/// @diagnostic.note message="'Digit' reduces to '0..=9'"
"#,
    );
}

/// A union of intervals accepts values inside its arms and rejects values between them.
#[test]
fn test_interval_union_accepts_member_bounds() {
    let session = TestSession::single(
        r#"
type Edge = 0..=3 | 252..=255;

const low: Edge = 2;
const high: Edge = 254;
const bad: Edge = 128;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Edge = 0..=3 | 252..=255;

const low: Edge = 2 as Edge;
const high: Edge = 254 as Edge;
const bad: Edge = 128;

=== dir ===
type Edge = 0..=3 | 252..=255;
/// @type.symbol symbol=Edge source="type Edge = 0..=3 | 252..=255" type=0..=3 | 252..=255
/// @definition.type symbol=Edge source="type Edge = 0..=3 | 252..=255" value=0..=3 | 252..=255

const low: Edge = 2;
/// @type.symbol symbol=low source=low type=Edge
/// @resolution.pattern source=low kind=binding target=low
/// @resolution.name source=Edge target=Edge

const high: Edge = 254;
/// @type.symbol symbol=high source=high type=Edge
/// @resolution.pattern source=high kind=binding target=high
/// @resolution.name source=Edge target=Edge

const bad: Edge = 128;
/// @type.symbol symbol=bad source=bad type=Edge
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Edge target=Edge
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '128' is not assignable to type 'Edge'"
/// @diagnostic.label line=6 column=19 span="128" line_source="const bad: Edge = 128;"
/// @diagnostic.related line=6 column=12 span="Edge" line_source="const bad: Edge = 128;" message="expected due to this annotation"
/// @diagnostic.note message="'Edge' reduces to '0..=3 | 252..=255'"
"#,
    );
}

/// A newtype over an interval requires a nominal construction.
#[test]
fn test_newtype_interval_requires_nominal_construction() {
    let session = TestSession::single(
        r#"
newtype Port = 1..=65535;

const raw: Port = 443;
const port = Port(443);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Port = 1..=65535;

const raw: Port = 443;
const port: Port = Port(443);

=== dir ===
newtype Port = 1..=65535;
/// @type.symbol symbol=Port source="newtype Port = 1..=65535" type=Port
/// @definition.newtype symbol=Port source="newtype Port = 1..=65535" backing=1..=65535 constructors=[(1..=65535) => Port]

const raw: Port = 443;
/// @type.symbol symbol=raw source=raw type=Port
/// @resolution.pattern source=raw kind=binding target=raw
/// @resolution.name source=Port target=Port

const port = Port(443);
/// @type.symbol symbol=port source=port type=Port
/// @resolution.pattern source=port kind=binding target=port
/// @resolution.name source=Port target=Port
/// @resolution.construct source=Port(443) parameters=(1..=65535) arguments=(provided(443) as 1..=65535) return=Port kind=newtype target=Port backing=1..=65535
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '443' is not assignable to type 'Port'"
/// @diagnostic.label line=4 column=19 span="443" line_source="const raw: Port = 443;"
/// @diagnostic.related line=4 column=12 span="Port" line_source="const raw: Port = 443;" message="expected due to this annotation"
"#,
    );
}
