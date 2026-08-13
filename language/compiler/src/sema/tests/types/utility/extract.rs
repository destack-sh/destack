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

declare const matched: "a" | "c";

matched satisfies "a" | "c";

=== checked ===
type Match = Extract<"a" | "b" | "c", "a" | "c">;
/// @type.symbol symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" type="a" | "c"
/// @definition.type symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" value="a" | "c"
/// @resolution.name source=Extract target=types.object.Extract

declare const matched: Match;
/// @type.symbol symbol=matched source=matched type="a" | "c"
/// @resolution.pattern source=matched kind=binding target=matched
/// @resolution.name source=Match target=Match

matched satisfies "a" | "c";
/// @resolution.name source=matched target=matched
/// @resolution.place source=matched placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=matched root=matched
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

const bad: "a" | "c" = "b";

=== checked ===
type Match = Extract<"a" | "b" | "c", "a" | "c">;
/// @type.symbol symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" type="a" | "c"
/// @definition.type symbol=Match source="type Match = Extract<\"a\" | \"b\" | \"c\", \"a\" | \"c\">" value="a" | "c"
/// @resolution.name source=Extract target=types.object.Extract

const bad: Match = "b";
/// @type.symbol symbol=bad source=bad type="a" | "c"
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Match target=Match
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"b\"' is not assignable to type '\"a\" | \"c\"'"
/// @diagnostic.label line=4 column=20 span="\"b\"" line_source="const bad: Match = \"b\";"
/// @diagnostic.related line=4 column=12 span="Match" line_source="const bad: Match = \"b\";" message="expected due to this annotation"
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

let bad: never = "a";

=== checked ===
type Match = Extract<never, "a">;
/// @type.symbol symbol=Match source="type Match = Extract<never, \"a\">" type=never
/// @definition.type symbol=Match source="type Match = Extract<never, \"a\">" value=never
/// @resolution.name source=Extract target=types.object.Extract

let bad: Match = "a";
/// @type.symbol symbol=bad source=bad type=never
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Match target=Match
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"a\"' is not assignable to type 'never'"
/// @diagnostic.label line=4 column=18 span="\"a\"" line_source="let bad: Match = \"a\";"
/// @diagnostic.related line=4 column=10 span="Match" line_source="let bad: Match = \"a\";" message="expected due to this annotation"
"#,
    );
}
