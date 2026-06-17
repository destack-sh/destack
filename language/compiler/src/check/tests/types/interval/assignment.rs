use crate::tests::{DirRows, TestSession};

#[test]
fn test_interval_assignment_requires_interval_compatible_type() {
    let session = TestSession::single(
        r#"
type Digit = 0..=9;

declare const value: int32;
const digit: Digit = value;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Digit = 0..=9;

declare const value: int32;
const digit: Digit = value;

=== checked ===
type Digit = 0..=9;
/// @type.symbol symbol=Digit source="type Digit = 0..=9" type=0..=9
/// @definition.type symbol=Digit source="type Digit = 0..=9" value=0..=9

declare const value: int32;
/// @type.symbol symbol=value source=value type=int32

const digit: Digit = value;
/// @type.symbol symbol=digit source=digit type=0..=9
/// @resolution.name source=Digit target=Digit
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'int32' is not assignable to type 'Digit'"
/// @diagnostic.label line=5 column=7 source="const digit: Digit = value;"
"#,
    );
}

#[test]
fn test_interval_assignment_does_not_prove_arithmetic_bounds() {
    let session = TestSession::single(
        r#"
type Digit = 0..=9;

declare const digit: Digit;
const next: Digit = digit + 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Digit = 0..=9;

declare const digit: Digit;
const next: Digit = digit + 1;

=== checked ===
type Digit = 0..=9;
/// @type.symbol symbol=Digit source="type Digit = 0..=9" type=0..=9
/// @definition.type symbol=Digit source="type Digit = 0..=9" value=0..=9

declare const digit: Digit;
/// @type.symbol symbol=digit source=digit type=0..=9
/// @resolution.name source=Digit target=Digit

const next: Digit = digit + 1;
/// @type.symbol symbol=next source=next type=0..=9
/// @resolution.name source=Digit target=Digit
/// @type.node source="digit + 1" type=int32
/// @type.node source=digit type=0..=9
/// @resolution.name source=digit target=digit
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'int32' is not assignable to type 'Digit'"
/// @diagnostic.label line=5 column=7 source="const next: Digit = digit + 1;"
"#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Edge = 0..=3 | 252..=255;

const low: Edge = 2 as Edge;
const high: Edge = 254 as Edge;
const bad: Edge = 128;

=== checked ===
type Edge = 0..=3 | 252..=255;
/// @type.symbol symbol=Edge source="type Edge = 0..=3 | 252..=255" type=0..=3 | 252..=255
/// @definition.type symbol=Edge source="type Edge = 0..=3 | 252..=255" value=0..=3 | 252..=255

const low: Edge = 2;
/// @type.symbol symbol=low source=low type=0..=3 | 252..=255
/// @resolution.name source=Edge target=Edge
/// @type.node source=2 type=2

const high: Edge = 254;
/// @type.symbol symbol=high source=high type=0..=3 | 252..=255
/// @resolution.name source=Edge target=Edge
/// @type.node source=254 type=254

const bad: Edge = 128;
/// @type.symbol symbol=bad source=bad type=0..=3 | 252..=255
/// @resolution.name source=Edge target=Edge
/// @type.node source=128 type=128
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '128' is not assignable to type 'Edge'"
/// @diagnostic.label line=6 column=7 source="const bad: Edge = 128;"
"#,
    );
}

#[test]
fn test_newtype_interval_requires_nominal_construction() {
    let session = TestSession::single(
        r#"
newtype Port = 1..=65535;

const raw: Port = 443;
const port = Port(443);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Port = 1..=65535;

const raw: Port = 443;
const port: Port = Port(443);

=== checked ===
newtype Port = 1..=65535;
/// @type.symbol symbol=Port source="newtype Port = 1..=65535" type=Port
/// @definition.newtype symbol=Port source="newtype Port = 1..=65535" value=1..=65535

const raw: Port = 443;
/// @type.symbol symbol=raw source=raw type=Port
/// @resolution.name source=Port target=Port
/// @type.node source=443 type=443

const port = Port(443);
/// @type.symbol symbol=port source=port type=Port
/// @type.node source=Port type=Port
/// @type.node source=Port(443) type=Port
/// @resolution.name source=Port target=Port
/// @resolution.construct source=Port(443) parameters=(443) return=Port kind=newtype target=Port
/// @type.node source=443 type=443
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '443' is not assignable to type 'Port'"
/// @diagnostic.label line=4 column=7 source="const raw: Port = 443;"
"#,
    );
}
