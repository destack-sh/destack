use crate::assert_format_program;
use destack_source::FileType;

/// Format range expressions with tight operator spacing.
#[test]
fn test_format_range_expression() {
    assert_format_program!(
        r#"const a = 1 .. 10
const b = 1 ..= 10
const c = 1 ..
const d = .. 10
const e = ..= 10
const f = ..
"#,
        r#"const a = 1..10;
const b = 1..=10;
const c = 1..;
const d = ..10;
const e = ..=10;
const f = ..;
"#,
        FileType::Destack,
    );
}

/// Keep arithmetic precedence inside range endpoints.
#[test]
fn test_format_range_expression_precedence() {
    assert_format_program!(
        r#"const window = (start + 1) .. (end * 2)
const nested = (1..4) + count
const negative = -3 .. 3
"#,
        r#"const window = start + 1..end * 2;
const nested = (1..4) + count;
const negative = -3..3;
"#,
        FileType::Destack,
    );
}

/// Format range index expressions with subscript spacing.
#[test]
fn test_format_range_index_expression() {
    assert_format_program!(
        r#"const middle = items[ 1 .. count ]
const through = items[ 1 ..= count ]
const tail = items[ start .. ]
const head = items[ .. end ]
const prefix = items[ ..= end ]
const all = items[ .. ]
"#,
        r#"const middle = items[1..count];
const through = items[1..=count];
const tail = items[start..];
const head = items[..end];
const prefix = items[..=end];
const all = items[..];
"#,
        FileType::Destack,
    );
}

/// Format range patterns with tight operator spacing.
#[test]
fn test_format_range_pattern() {
    assert_format_program!(
        r#"const label = match (value) {
    0 .. 10 => "small"
    0 ..= 10 => "inclusive"
    0 .. => "from"
    .. 10 => "to"
    ..= 10 => "through"
    90 ..= 99 | 100 .. 110 => "edge"
    _ => "other"
}
"#,
        r#"const label = match (value) {
    0..10 => "small"
    0..=10 => "inclusive"
    0.. => "from"
    ..10 => "to"
    ..=10 => "through"
    90..=99 | 100..110 => "edge"
    _ => "other"
};
"#,
        FileType::Destack,
    );
}

/// Use value-range spelling for range type expressions.
#[test]
fn test_format_range_type_expression() {
    assert_format_program!(
        r#"type Window = Start .. End
type Inclusive = Start ..= End
type From = Start ..
type To = .. End
type Full = ..
"#,
        r#"type Window = Start..End;
type Inclusive = Start..=End;
type From = Start..;
type To = ..End;
type Full = ..;
"#,
        FileType::Destack,
    );
}
