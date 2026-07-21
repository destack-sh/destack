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
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=NonNullable<string | null | undefined> reduced=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=NonNullable<string | null | undefined> reduced=string
/// @resolution.name source=NonNullable target=types.object.NonNullable

declare const present: Present;
/// @type.symbol symbol=present source=present type=Present reduced=string
/// @resolution.pattern source=present kind=binding target=present
/// @resolution.name source=Present target=Present

present satisfies string;
/// @resolution.name source=present target=present

/// @generic.instance id="NonNullable<string | null | undefined>" template=types.object.NonNullable arguments=(string | null | undefined)
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
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=NonNullable<string | null | undefined> reduced=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=NonNullable<string | null | undefined> reduced=string
/// @resolution.name source=NonNullable target=types.object.NonNullable

const bad: Present = null;
/// @type.symbol symbol=bad source=bad type=Present reduced=string
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Present target=Present

/// @generic.instance id="NonNullable<string | null | undefined>" template=types.object.NonNullable arguments=(string | null | undefined)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'null' is not assignable to type 'Present'"
/// @diagnostic.label line=4 column=22 span="null" line_source="const bad: Present = null;"
/// @diagnostic.related line=4 column=12 span="Present" line_source="const bad: Present = null;" message="expected due to this annotation"
/// @diagnostic.note message="'Present' reduces to 'string'"
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
/// @type.symbol symbol=Present source="type Present = NonNullable<never>" type=NonNullable<never> reduced=never
/// @definition.type symbol=Present source="type Present = NonNullable<never>" value=NonNullable<never> reduced=never
/// @resolution.name source=NonNullable target=types.object.NonNullable

let bad: Present = "no";
/// @type.symbol symbol=bad source=bad type=Present reduced=never
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Present target=Present

/// @generic.instance id=NonNullable<never> template=types.object.NonNullable arguments=(never)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'Present'"
/// @diagnostic.label line=4 column=20 span="\"no\"" line_source="let bad: Present = \"no\";"
/// @diagnostic.related line=4 column=10 span="Present" line_source="let bad: Present = \"no\";" message="expected due to this annotation"
/// @diagnostic.note message="'Present' reduces to 'never'"
"#,
    );
}
