use crate::tests::{DirRows, TestSession};

#[test]
fn test_bigint_interval_accepts_contained_literals() {
    let session = TestSession::single(
        r#"
type Small = 1n..=10n;

const value: Small = 5n;
const bad: Small = 11n;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Small = 1n..=10n;

const value: 1n..=10n = 5n;
const bad: 1n..=10n = 11n;

=== dir ===
type Small = 1n..=10n;
/// @type.symbol symbol=Small source="type Small = 1n..=10n" type=1n..=10n
/// @definition.type symbol=Small source="type Small = 1n..=10n" value=1n..=10n

const value: Small = 5n;
/// @type.symbol symbol=value source=value type=1n..=10n
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Small target=Small

const bad: Small = 11n;
/// @type.symbol symbol=bad source=bad type=1n..=10n
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Small target=Small
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '11n' is not assignable to type '1n..=10n'"
/// @diagnostic.label line=5 column=20 span="11n" line_source="const bad: Small = 11n;"
/// @diagnostic.related line=5 column=12 span="Small" line_source="const bad: Small = 11n;" message="expected due to this annotation"
"#,
    );
}
