use crate::tests::{DirRows, TestSession};

#[test]
fn test_range_pattern_narrows_interval() {
    let session = TestSession::single(
        r#"
function isByte(value: int32): boolean {
    return match (value) {
        0..=255 => {
            value satisfies 0..=255;
            true
        }
        _ => false
    };
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function isByte(value: int32): boolean {
    return match (value) {
        0..=255 => {
            value satisfies 0..=255;
            true
        }
        _ => false
    };
}

=== dir ===
function isByte(value: int32): boolean {
/// @type.symbol symbol=isByte type=(int32) => boolean
/// @type.symbol symbol=isByte.value source="value: int32" type=int32

    return match (value) {
    /// @type.node type=boolean
    /// @resolution.coverage exhaustive=true disjoint=false
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=isByte.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=isByte.value

        0..=255 => {
        /// @type.node source=0 type=0
        /// @resolution.pattern source=0..=255 kind=range domain=int32 start=0 end=255 bound=inclusive
        /// @type.node source=255 type=255

            value satisfies 0..=255;
            /// @type.node source="value satisfies 0..=255" type=0..=255
            /// @type.node source=value type=0..=255
            /// @resolution.name source=value target=isByte.value
            /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=value root=isByte.value

            true
            /// @type.node source=true type=true

        }
        _ => false
        /// @resolution.pattern source=_ kind=wildcard
        /// @type.node source=false type=false

    };
}
"#,
    );
}

#[test]
fn test_range_patterns_cover_integer_interval() {
    let session = TestSession::single(
        r#"
type Tiny = 0..=2;

declare const value: Tiny;

const label = match (value) {
    0..=1 => "low"
    2 => "two"
};
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Tiny = 0..=2;

declare const value: Tiny;

const label: "low" | "two" = match (value) {
    0..=1 => "low"
    2 => "two"
};

=== dir ===
type Tiny = 0..=2;
/// @type.symbol symbol=Tiny source="type Tiny = 0..=2" type=0..=2
/// @definition.type symbol=Tiny source="type Tiny = 0..=2" value=0..=2

declare const value: Tiny;
/// @type.symbol symbol=value source=value type=Tiny
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Tiny target=Tiny

const label = match (value) {
/// @type.symbol symbol=label source=label type="low" | "two"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="low" | "two"
/// @resolution.coverage exhaustive=true disjoint=true
/// @type.node source=value type=Tiny
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

    0..=1 => "low"
    /// @type.node source=0 type=0
    /// @resolution.pattern source=0..=1 kind=range domain=0..=2 start=0 end=1 bound=inclusive
    /// @type.node source=1 type=1
    /// @type.node source="\"low\"" type="low"

    2 => "two"
    /// @type.node source=2 type=2
    /// @resolution.pattern source=2 kind=literal value=2
    /// @type.node source="\"two\"" type="two"

};
"#,
    );
}

#[test]
fn test_range_patterns_cover_character_interval() {
    let session = TestSession::single(
        r#"
type LowerAscii = 'a'..='z';

declare const value: LowerAscii;

const isEarly = match (value) {
    'a'..='m' => true
    'n'..='z' => false
};
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type LowerAscii = 'a'..='z';

declare const value: LowerAscii;

const isEarly: boolean = match (value) {
    'a'..='m' => true
    'n'..='z' => false
};

=== dir ===
type LowerAscii = 'a'..='z';
/// @type.symbol symbol=LowerAscii source="type LowerAscii = 'a'..='z'" type='a'..='z'
/// @definition.type symbol=LowerAscii source="type LowerAscii = 'a'..='z'" value='a'..='z'

declare const value: LowerAscii;
/// @type.symbol symbol=value source=value type=LowerAscii
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=LowerAscii target=LowerAscii

const isEarly = match (value) {
/// @type.symbol symbol=isEarly source=isEarly type=boolean
/// @resolution.pattern source=isEarly kind=binding target=isEarly
/// @type.node type=boolean
/// @resolution.coverage exhaustive=true disjoint=true
/// @type.node source=value type=LowerAscii
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

    'a'..='m' => true
    /// @type.node source='a' type='a'
    /// @resolution.pattern source='a'..='m' kind=range domain='a'..='z' start='a' end='m' bound=inclusive
    /// @type.node source='m' type='m'
    /// @type.node source=true type=true

    'n'..='z' => false
    /// @type.node source='n' type='n'
    /// @resolution.pattern source='n'..='z' kind=range domain='a'..='z' start='n' end='z' bound=inclusive
    /// @type.node source='z' type='z'
    /// @type.node source=false type=false

};
"#,
    );
}

#[test]
fn test_range_patterns_report_uncovered_interval_hole() {
    let session = TestSession::single(
        r#"
type Tiny = 0..=3;

declare const value: Tiny;

const label = match (value) {
    0..=1 => "low"
    3 => "high"
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Tiny = 0..=3;

declare const value: Tiny;

const label: "low" | "high" = match (value) {
    0..=1 => "low"
    3 => "high"
};

=== dir ===
type Tiny = 0..=3;
/// @type.symbol symbol=Tiny source="type Tiny = 0..=3" type=0..=3
/// @definition.type symbol=Tiny source="type Tiny = 0..=3" value=0..=3

declare const value: Tiny;
/// @type.symbol symbol=value source=value type=Tiny
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Tiny target=Tiny

const label = match (value) {
/// @type.symbol symbol=label source=label type="low" | "high"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="low" | "high"
/// @resolution.coverage exhaustive=false disjoint=true
/// @type.node source=value type=Tiny
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

    0..=1 => "low"
    /// @type.node source=0 type=0
    /// @resolution.pattern source=0..=1 kind=range domain=0..=3 start=0 end=1 bound=inclusive
    /// @type.node source=1 type=1
    /// @type.node source="\"low\"" type="low"

    3 => "high"
    /// @type.node source=3 type=3
    /// @resolution.pattern source=3 kind=literal value=3
    /// @type.node source="\"high\"" type="high"

};
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: '2' is not covered"
/// @diagnostic.label line=6 column=15 span="match" line_source="const label = match (value) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}
