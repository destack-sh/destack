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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<string | null | undefined>;

declare const present: string;

present satisfies string;

=== dir ===
type Present = NonNullable<string | null | undefined>;
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=string
/// @resolution.name source=NonNullable target=NonNullable

declare const present: Present;
/// @type.symbol symbol=present source=present type=string
/// @resolution.pattern source=present kind=binding target=present
/// @resolution.name source=Present target=Present

present satisfies string;
/// @resolution.name source=present target=present
/// @resolution.place source=present placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=present root=present
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<string | null | undefined>;

const bad: string = null;

=== dir ===
type Present = NonNullable<string | null | undefined>;
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=string
/// @resolution.name source=NonNullable target=NonNullable

const bad: Present = null;
/// @type.symbol symbol=bad source=bad type=string
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Present target=Present
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'null' is not assignable to type 'string'"
/// @diagnostic.label line=4 column=22 span="null" line_source="const bad: Present = null;"
/// @diagnostic.related line=4 column=12 span="Present" line_source="const bad: Present = null;" message="expected due to this annotation"
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<never>;

let bad: never = "no";

=== dir ===
type Present = NonNullable<never>;
/// @type.symbol symbol=Present source="type Present = NonNullable<never>" type=never
/// @definition.type symbol=Present source="type Present = NonNullable<never>" value=never
/// @resolution.name source=NonNullable target=NonNullable

let bad: Present = "no";
/// @type.symbol symbol=bad source=bad type=never
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Present target=Present
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'never'"
/// @diagnostic.label line=4 column=20 span="\"no\"" line_source="let bad: Present = \"no\";"
/// @diagnostic.related line=4 column=10 span="Present" line_source="let bad: Present = \"no\";" message="expected due to this annotation"
"#,
    );
}
