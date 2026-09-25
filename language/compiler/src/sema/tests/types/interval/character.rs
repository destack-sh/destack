use crate::tests::{DirRows, TestSession};

#[test]
fn test_char_interval_accepts_contained_literals() {
    let session = TestSession::single(
        r#"
type LowerAscii = 'a'..='z';

const letter: LowerAscii = 'm';
const bad: LowerAscii = 'A';
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type LowerAscii = 'a'..='z';

const letter: LowerAscii = 'm';
const bad: LowerAscii = 'A';

=== dir ===
type LowerAscii = 'a'..='z';
/// @type.symbol symbol=LowerAscii source="type LowerAscii = 'a'..='z'" type='a'..='z'
/// @definition.type symbol=LowerAscii source="type LowerAscii = 'a'..='z'" value='a'..='z'

const letter: LowerAscii = 'm';
/// @type.symbol symbol=letter source=letter type=LowerAscii
/// @resolution.pattern source=letter kind=binding target=letter
/// @resolution.name source=LowerAscii target=LowerAscii

const bad: LowerAscii = 'A';
/// @type.symbol symbol=bad source=bad type=LowerAscii
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=LowerAscii target=LowerAscii
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type ''A'' is not assignable to type 'LowerAscii'"
/// @diagnostic.label line=5 column=25 span="'A'" line_source="const bad: LowerAscii = 'A';"
/// @diagnostic.related line=5 column=12 span="LowerAscii" line_source="const bad: LowerAscii = 'A';" message="expected due to this annotation"
/// @diagnostic.note message="'LowerAscii' reduces to ''a'..='z''"
"#,
    );
}
