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
/// @type.symbol symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" type="a" | "c"
/// @definition.type symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" value="a" | "c"
/// @resolution.name source=Exclude target=types.object.Exclude

declare const letter: Letter;
/// @type.symbol symbol=letter source=letter type="a" | "c"
/// @resolution.name source=Letter target=Letter

letter satisfies "a" | "c";
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
/// @type.symbol symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" type="a" | "c"
/// @definition.type symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" value="a" | "c"
/// @resolution.name source=Exclude target=types.object.Exclude

const bad: Letter = "b";
/// @type.symbol symbol=bad source=bad type="a" | "c"
/// @resolution.name source=Letter target=Letter
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"b\"' is not assignable to type 'Letter'"
/// @diagnostic.label line=4 column=7 source="const bad: Letter = \"b\";"
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
/// @type.symbol symbol=Letter source="type Letter = Exclude<never, \"b\">" type=never
/// @definition.type symbol=Letter source="type Letter = Exclude<never, \"b\">" value=never
/// @resolution.name source=Exclude target=types.object.Exclude

let bad: Letter = "b";
/// @type.symbol symbol=bad source=bad type=never
/// @resolution.name source=Letter target=Letter
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"b\"' is not assignable to type 'Letter'"
/// @diagnostic.label line=4 column=5 source="let bad: Letter = \"b\";"
"#,
    );
}
