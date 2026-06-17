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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Small = 1n..=10n;

const value: Small = 5n;
const bad: Small = 11n;

=== checked ===
type Small = 1n..=10n;
/// @type.symbol symbol=Small source="type Small = 1n..=10n" type=1n..=10n
/// @definition.type symbol=Small source="type Small = 1n..=10n" value=1n..=10n

const value: Small = 5n;
/// @type.symbol symbol=value source=value type=1n..=10n
/// @resolution.name source=Small target=Small
/// @type.node source=5n type=5n

const bad: Small = 11n;
/// @type.symbol symbol=bad source=bad type=1n..=10n
/// @resolution.name source=Small target=Small
/// @type.node source=11n type=11n
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '11n' is not assignable to type 'Small'"
/// @diagnostic.label line=5 column=7 source="const bad: Small = 11n;"
"#,
    );
}
