use crate::tests::{DirRows, TestSession};

#[test]
fn test_nonnullable_removes_nullish_members() {
    let session = TestSession::single(
        r#"
type Present = NonNullable<string | null | undefined>;

declare const present: Present;

present satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<string | null | undefined>;

declare const present: Present;

present satisfies string;

=== checked ===
type Present = NonNullable<string | null | undefined>;
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=string
/// @resolution.name source=NonNullable target=types.object.NonNullable

declare const present: Present;
/// @type.symbol symbol=present source=present type=string
/// @resolution.name source=Present target=Present

present satisfies string;
"#,
    );
}

#[test]
fn test_nonnullable_rejects_nullish_member() {
    let session = TestSession::single(
        r#"
type Present = NonNullable<string | null | undefined>;

const bad: Present = null;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<string | null | undefined>;

const bad: Present = null;

=== checked ===
type Present = NonNullable<string | null | undefined>;
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=string
/// @resolution.name source=NonNullable target=types.object.NonNullable

const bad: Present = null;
/// @type.symbol symbol=bad source=bad type=string
/// @resolution.name source=Present target=Present
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'null' is not assignable to type 'Present'"
/// @diagnostic.label line=4 column=7 source="const bad: Present = null;"
"#,
    );
}

#[test]
fn test_nonnullable_never_yields_never() {
    let session = TestSession::single(
        r#"
type Present = NonNullable<never>;

let bad: Present = "no";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<never>;

let bad: Present = "no";

=== checked ===
type Present = NonNullable<never>;
/// @type.symbol symbol=Present source="type Present = NonNullable<never>" type=never
/// @definition.type symbol=Present source="type Present = NonNullable<never>" value=never
/// @resolution.name source=NonNullable target=types.object.NonNullable

let bad: Present = "no";
/// @type.symbol symbol=bad source=bad type=never
/// @resolution.name source=Present target=Present
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"no\"' is not assignable to type 'Present'"
/// @diagnostic.label line=4 column=5 source="let bad: Present = \"no\";"
"#,
    );
}
