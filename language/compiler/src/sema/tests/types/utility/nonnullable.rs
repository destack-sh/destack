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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<string | null | undefined>;

declare const present: Present;

present satisfies string;

=== dir ===
type Present = NonNullable<string | null | undefined>;
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=NonNullable<string | null | undefined>
/// @resolution.name source=NonNullable target=NonNullable

declare const present: Present;
/// @type.symbol symbol=present source=present type=Present
/// @resolution.pattern source=present kind=binding target=present
/// @resolution.name source=Present target=Present

present satisfies string;
/// @resolution.name source=present target=present
/// @resolution.place source=present placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<string | null | undefined>;

const bad: Present = null;

=== dir ===
type Present = NonNullable<string | null | undefined>;
/// @type.symbol symbol=Present source="type Present = NonNullable<string | null | undefined>" type=string
/// @definition.type symbol=Present source="type Present = NonNullable<string | null | undefined>" value=NonNullable<string | null | undefined>
/// @resolution.name source=NonNullable target=NonNullable

const bad: Present = null;
/// @type.symbol symbol=bad source=bad type=Present
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Present target=Present
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Present = NonNullable<never>;

let bad: Present = "no";

=== dir ===
type Present = NonNullable<never>;
/// @type.symbol symbol=Present source="type Present = NonNullable<never>" type=never
/// @definition.type symbol=Present source="type Present = NonNullable<never>" value=NonNullable<never>
/// @resolution.name source=NonNullable target=NonNullable

let bad: Present = "no";
/// @type.symbol symbol=bad source=bad type=Present
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Present target=Present
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'Present'"
/// @diagnostic.label line=4 column=20 span="\"no\"" line_source="let bad: Present = \"no\";"
/// @diagnostic.related line=4 column=10 span="Present" line_source="let bad: Present = \"no\";" message="expected due to this annotation"
/// @diagnostic.note message="'Present' reduces to 'never'"
"#,
    );
}

#[test]
fn test_reject_nullish_values_against_the_empty_shape() {
    let session = TestSession::single(
        r#"
function keep<T: {}>(value: T): T {
    return value;
}

const shaped: {} = undefined;
keep(null);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function keep<T: {}>(value: T): T {
    return value;
}

const shaped: {} = undefined;
keep<null>(null);

=== dir ===
function keep<T: {}>(value: T): T {
/// @generic.template symbol=keep parameters=(T: {})
/// @type.symbol symbol=keep type=<T: {}>(T) => T
/// @type.symbol symbol=keep.T source="T: {}" type=T
/// @type.symbol symbol=keep.value source="value: T" type=T
/// @resolution.name source=T target=keep.T
/// @resolution.name source=T target=keep.T

    return value;
    /// @resolution.name source=value target=keep.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=keep.value

}

const shaped: {} = undefined;
/// @type.symbol symbol=shaped source=shaped type={}
/// @resolution.pattern source=shaped kind=binding target=shaped

keep(null);
/// @resolution.name source=keep target=keep
/// @resolution.call source=keep(null) parameters=(null) arguments=(provided(null) as null) return=null kind=symbol target=keep instance=keep<null>
/// @generic.instantiation id=keep<null> template=keep arguments=(null)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'undefined' is not assignable to type '{}'"
/// @diagnostic.label line=6 column=20 span="undefined" line_source="const shaped: {} = undefined;"
/// @diagnostic.related line=6 column=15 span="{}" line_source="const shaped: {} = undefined;" message="expected due to this annotation"
/// @diagnostic.error id=constraint-not-satisfied message="type 'null' does not satisfy '{}'"
/// @diagnostic.label line=7 column=1 span="keep(null)" line_source="keep(null);"
/// @diagnostic.related line=2 column=15 span="T" line_source="function keep<T: {}>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}
