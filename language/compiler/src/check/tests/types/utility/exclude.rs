use crate::tests::{DirRows, TestSession};

#[test]
fn test_exclude_removes_assignable_union_members() {
    let session = TestSession::single(
        r#"
type Letter = Exclude<"a" | "b" | "c", "b">;

declare const letter: Letter;

letter satisfies "a" | "c";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Letter = Exclude<"a" | "b" | "c", "b">;

declare const letter: Letter;

letter satisfies "a" | "c";

=== checked ===
type Letter = Exclude<"a" | "b" | "c", "b">;
/// @type.symbol symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" type=Exclude<"a" | "b" | "c", "b"> reduced="a" | "c"
/// @definition.type symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" value=Exclude<"a" | "b" | "c", "b"> reduced="a" | "c"
/// @resolution.name source=Exclude target=types.object.Exclude

declare const letter: Letter;
/// @type.symbol symbol=letter source=letter type=Letter reduced="a" | "c"
/// @resolution.name source=Letter target=Letter

letter satisfies "a" | "c";
/// @resolution.name source=letter target=letter

/// @generic.instance id="Exclude<\"a\" | \"b\" | \"c\", \"b\">" template=types.object.Exclude arguments=("a" | "b" | "c", "b")
"#,
    );
}

#[test]
fn test_exclude_rejects_removed_member() {
    let session = TestSession::single(
        r#"
type Letter = Exclude<"a" | "b" | "c", "b">;

const bad: Letter = "b";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Letter = Exclude<"a" | "b" | "c", "b">;

const bad: Letter = "b";

=== checked ===
type Letter = Exclude<"a" | "b" | "c", "b">;
/// @type.symbol symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" type=Exclude<"a" | "b" | "c", "b"> reduced="a" | "c"
/// @definition.type symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" value=Exclude<"a" | "b" | "c", "b"> reduced="a" | "c"
/// @resolution.name source=Exclude target=types.object.Exclude

const bad: Letter = "b";
/// @type.symbol symbol=bad source=bad type=Letter reduced="a" | "c"
/// @resolution.name source=Letter target=Letter

/// @generic.instance id="Exclude<\"a\" | \"b\" | \"c\", \"b\">" template=types.object.Exclude arguments=("a" | "b" | "c", "b")
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"b\"' is not assignable to type 'Letter'"
/// @diagnostic.label line=4 column=21 span="\"b\"" line_source="const bad: Letter = \"b\";"
"#,
    );
}

#[test]
fn test_exclude_never_yields_never() {
    let session = TestSession::single(
        r#"
type Letter = Exclude<never, "b">;

let bad: Letter = "b";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Letter = Exclude<never, "b">;

let bad: Letter = "b";

=== checked ===
type Letter = Exclude<never, "b">;
/// @type.symbol symbol=Letter source="type Letter = Exclude<never, \"b\">" type=Exclude<never, "b"> reduced=never
/// @definition.type symbol=Letter source="type Letter = Exclude<never, \"b\">" value=Exclude<never, "b"> reduced=never
/// @resolution.name source=Exclude target=types.object.Exclude

let bad: Letter = "b";
/// @type.symbol symbol=bad source=bad type=Letter reduced=never
/// @resolution.name source=Letter target=Letter

/// @generic.instance id="Exclude<never, \"b\">" template=types.object.Exclude arguments=(never, "b")
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"b\"' is not assignable to type 'Letter'"
/// @diagnostic.label line=4 column=19 span="\"b\"" line_source="let bad: Letter = \"b\";"
"#,
    );
}
