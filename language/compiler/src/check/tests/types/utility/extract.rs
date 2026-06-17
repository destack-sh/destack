use crate::tests::{DirRows, TestSession};

#[test]
fn test_extract_keeps_assignable_union_members() {
    let session = TestSession::single(
        r#"
type Match = Extract<"a" | "b" | "c", "a" | "c">;

declare const matched: Match;

matched satisfies "a" | "c";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Match = Extract<"a" | "b" | "c", "a" | "c">;

declare const matched: Match;

matched satisfies "a" | "c";

=== checked ===
type Match = Extract<"a" | "b" | "c", "a" | "c">;
/// @type.symbol symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" type="a" | "c"
/// @definition.type symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" value="a" | "c"
/// @resolution.name source=Extract target=types.object.Extract

declare const matched: Match;
/// @type.symbol symbol=matched source=matched type="a" | "c"
/// @resolution.name source=Match target=Match

matched satisfies "a" | "c";
"#,
    );
}

#[test]
fn test_extract_rejects_unmatched_member() {
    let session = TestSession::single(
        r#"
type Match = Extract<"a" | "b" | "c", "a" | "c">;

const bad: Match = "b";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Match = Extract<"a" | "b" | "c", "a" | "c">;

const bad: Match = "b";

=== checked ===
type Match = Extract<"a" | "b" | "c", "a" | "c">;
/// @type.symbol symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" type="a" | "c"
/// @definition.type symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" value="a" | "c"
/// @resolution.name source=Extract target=types.object.Extract

const bad: Match = "b";
/// @type.symbol symbol=bad source=bad type="a" | "c"
/// @resolution.name source=Match target=Match
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"b\"' is not assignable to type 'Match'"
/// @diagnostic.label line=4 column=7 source="const bad: Match = \"b\";"
"#,
    );
}

#[test]
fn test_extract_never_yields_never() {
    let session = TestSession::single(
        r#"
type Match = Extract<never, "a">;

let bad: Match = "a";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Match = Extract<never, "a">;

let bad: Match = "a";

=== checked ===
type Match = Extract<never, "a">;
/// @type.symbol symbol=Match source="type Match = Extract<never, \"a\">" type=never
/// @definition.type symbol=Match source="type Match = Extract<never, \"a\">" value=never
/// @resolution.name source=Extract target=types.object.Extract

let bad: Match = "a";
/// @type.symbol symbol=bad source=bad type=never
/// @resolution.name source=Match target=Match
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"a\"' is not assignable to type 'Match'"
/// @diagnostic.label line=4 column=5 source="let bad: Match = \"a\";"
"#,
    );
}
