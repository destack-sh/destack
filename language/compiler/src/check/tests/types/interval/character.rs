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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type LowerAscii = 'a'..='z';

const letter: LowerAscii = 'm';
const bad: LowerAscii = 'A';

=== checked ===
type LowerAscii = 'a'..='z';
/// @type.symbol symbol=LowerAscii source="type LowerAscii = 'a'..='z'" type='a'..='z'
/// @definition.type symbol=LowerAscii source="type LowerAscii = 'a'..='z'" value='a'..='z'

const letter: LowerAscii = 'm';
/// @type.symbol symbol=letter source=letter type='a'..='z'
/// @resolution.name source=LowerAscii target=LowerAscii
/// @type.node source="'m'" type='m'

const bad: LowerAscii = 'A';
/// @type.symbol symbol=bad source=bad type='a'..='z'
/// @resolution.name source=LowerAscii target=LowerAscii
/// @type.node source="'A'" type='A'
"#,
        r#"
/// @diagnostic.error code=EC200 message="type ''A'' is not assignable to type 'LowerAscii'"
/// @diagnostic.label line=5 column=7 source="const bad: LowerAscii = 'A';"
"#,
    );
}
