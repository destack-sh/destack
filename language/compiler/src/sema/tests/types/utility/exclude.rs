use crate::tests::{DirRows, TestSession};

/// Exclude removes the union members assignable to its second argument.
#[test]
fn test_exclude_removes_assignable_union_members() {
    let session = TestSession::single(
        r#"
type Letter = Exclude<"a" | "b" | "c", "b">;

declare const letter: Letter;

letter satisfies "a" | "c";
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Letter = Exclude<"a" | "b" | "c", "b">;

declare const letter: Letter;

letter satisfies "a" | "c";

=== dir ===
type Letter = Exclude<"a" | "b" | "c", "b">;
/// @type.symbol symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" type="a" | "c"
/// @definition.type symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" value=Exclude<"a" | "b" | "c", "b">
/// @resolution.name source=Exclude target=Exclude

declare const letter: Letter;
/// @type.symbol symbol=letter source=letter type=Letter
/// @resolution.pattern source=letter kind=binding target=letter
/// @resolution.name source=Letter target=Letter

letter satisfies "a" | "c";
/// @resolution.name source=letter target=letter
/// @resolution.place source=letter placement="local" lifetime="static" access="immutable"
/// @resolution.access source=letter root=letter
"#,
    );
}

/// A removed member reports a diagnostic against an Exclude type.
#[test]
fn test_exclude_rejects_removed_member() {
    let session = TestSession::single(
        r#"
type Letter = Exclude<"a" | "b" | "c", "b">;

const bad: Letter = "b";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Letter = Exclude<"a" | "b" | "c", "b">;

const bad: Letter = "b";

=== dir ===
type Letter = Exclude<"a" | "b" | "c", "b">;
/// @type.symbol symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" type="a" | "c"
/// @definition.type symbol=Letter source="type Letter = Exclude<\"a\" | \"b\" | \"c\", \"b\">" value=Exclude<"a" | "b" | "c", "b">
/// @resolution.name source=Exclude target=Exclude

const bad: Letter = "b";
/// @type.symbol symbol=bad source=bad type=Letter
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Letter target=Letter
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"b\"' is not assignable to type 'Letter'"
/// @diagnostic.label line=4 column=21 span="\"b\"" line_source="const bad: Letter = \"b\";"
/// @diagnostic.related line=4 column=12 span="Letter" line_source="const bad: Letter = \"b\";" message="expected due to this annotation"
/// @diagnostic.note message="'Letter' reduces to '\"a\" | \"c\"'"
"#,
    );
}

/// Exclude over never reduces to never.
#[test]
fn test_exclude_never_yields_never() {
    let session = TestSession::single(
        r#"
type Letter = Exclude<never, "b">;

let bad: Letter = "b";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Letter = Exclude<never, "b">;

let bad: Letter = "b";

=== dir ===
type Letter = Exclude<never, "b">;
/// @type.symbol symbol=Letter source="type Letter = Exclude<never, \"b\">" type=never
/// @definition.type symbol=Letter source="type Letter = Exclude<never, \"b\">" value=Exclude<never, "b">
/// @resolution.name source=Exclude target=Exclude

let bad: Letter = "b";
/// @type.symbol symbol=bad source=bad type=Letter
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Letter target=Letter
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"b\"' is not assignable to type 'Letter'"
/// @diagnostic.label line=4 column=19 span="\"b\"" line_source="let bad: Letter = \"b\";"
/// @diagnostic.related line=4 column=10 span="Letter" line_source="let bad: Letter = \"b\";" message="expected due to this annotation"
/// @diagnostic.note message="'Letter' reduces to 'never'"
"#,
    );
}
