use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_infer_extracts_segment() {
    let session = TestSession::single(
        r#"
type Segment<T> = T extends `/${infer Name}` ? Name : never;
type Name = Segment<"/api">;

declare const name: Name;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Segment<T> = T extends `/${infer Name}` ? Name : never;
type Name = Segment<"/api">;

declare const name: Name;

=== checked ===
type Segment<T> = T extends `/${infer Name}` ? Name : never;
/// @generic.template symbol=Segment parameters=(T)
/// @type.symbol symbol=Segment type=T extends `/${infer Name}` ? Name : never

type Name = Segment<"/api">;
/// @resolution.name source=Segment target=Segment
/// @generic.instance source="Segment<\"/api\">" id="Segment<\"/api\">"
/// @type.symbol symbol=Name type="api"

declare const name: Name;
/// @resolution.name source=Name target=Name
/// @type.symbol symbol=name type="api"

/// @generic.instance id="Segment<\"/api\">" template=Segment arguments=("/api")
"#,
    );
}

#[test]
fn test_template_literal_infer_can_ignore_unnamed_span() {
    let session = TestSession::single(
        r#"
type HasId<T> = T extends `id:${infer _}` ? true : false;
type Yes = HasId<"id:users">;
type No = HasId<"users">;

const yes: Yes = true;
const no: No = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type HasId<T> = T extends `id:${infer _}` ? true : false;
type Yes = HasId<"id:users">;
type No = HasId<"users">;

const yes: Yes = true;
const no: No = false;

=== checked ===
type HasId<T> = T extends `id:${infer _}` ? true : false;
/// @generic.template symbol=HasId parameters=(T)
/// @type.symbol symbol=HasId type=T extends `id:${infer _}` ? true : false
/// @generic.infer symbol=_ constraint=unknown

type Yes = HasId<"id:users">;
/// @resolution.name source=HasId target=HasId
/// @generic.instance source="HasId<\"id:users\">" id="HasId<\"id:users\">"
/// @type.symbol symbol=Yes type=true

type No = HasId<"users">;
/// @resolution.name source=HasId target=HasId
/// @generic.instance source="HasId<\"users\">" id="HasId<\"users\">"
/// @type.symbol symbol=No type=false

const yes: Yes = true;
/// @type.symbol symbol=yes source=yes type=true
/// @resolution.name source=Yes target=Yes

const no: No = false;
/// @type.symbol symbol=no source=no type=false
/// @resolution.name source=No target=No
/// @generic.instance id="HasId<\"id:users\">" template=HasId arguments=("id:users")
/// @generic.instance id="HasId<\"users\">" template=HasId arguments=("users")
"#,
    );
}

#[test]
fn test_template_literal_infer_distributes_over_union_templates() {
    let session = TestSession::single(
        r#"
type Extract<T> = T extends `foo-${infer A}` ? A : never;
type Result = Extract<`foo-a` | `foo-b`>;

const a: Result = "a";
const b: Result = "b";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Extract<T> = T extends `foo-${infer A}` ? A : never;
type Result = Extract<`foo-a` | `foo-b`>;

const a: Result = "a" as Result;
const b: Result = "b" as Result;

=== checked ===
type Extract<T> = T extends `foo-${infer A}` ? A : never;
/// @generic.template symbol=Extract parameters=(T)
/// @type.symbol symbol=Extract type=T extends `foo-${infer A}` ? A : never

type Result = Extract<`foo-a` | `foo-b`>;
/// @resolution.name source=Extract target=Extract
/// @generic.instance source="Extract<`foo-a` | `foo-b`>" id="Extract<`foo-a` | `foo-b`>"
/// @type.symbol symbol=Result type="a" | "b"

const a: Result = "a";
/// @type.symbol symbol=a source=a type="a" | "b"
/// @resolution.name source=Result target=Result

const b: Result = "b";
/// @type.symbol symbol=b source=b type="a" | "b"
/// @resolution.name source=Result target=Result

/// @generic.instance id="Extract<`foo-a` | `foo-b`>" template=Extract arguments=(`foo-a` | `foo-b`)
"#,
    );
}

#[test]
fn test_template_literal_infer_merges_repeated_spans() {
    let session = TestSession::single(
        r#"
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-foo">;

const matched: Match = "foo";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-foo">;

const matched: Match = "foo";

=== checked ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
/// @generic.template symbol=Repeat parameters=(T)
/// @type.symbol symbol=Repeat type=T extends `${infer A}-${infer A}` ? A : "no"

type Match = Repeat<"foo-foo">;
/// @resolution.name source=Repeat target=Repeat
/// @generic.instance source="Repeat<\"foo-foo\">" id="Repeat<\"foo-foo\">"
/// @type.symbol symbol=Match type="foo"

const matched: Match = "foo";
/// @type.symbol symbol=matched source=matched type="foo"
/// @resolution.name source=Match target=Match

/// @generic.instance id="Repeat<\"foo-foo\">" template=Repeat arguments=("foo-foo")
"#,
    );
}

#[test]
fn test_template_literal_infer_falls_back_for_mismatched_repeated_spans() {
    let session = TestSession::single(
        r#"
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-bar">;

const matched: Match = "no";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-bar">;

const matched: Match = "no";

=== checked ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
/// @generic.template symbol=Repeat parameters=(T)
/// @type.symbol symbol=Repeat type=T extends `${infer A}-${infer A}` ? A : "no"

type Match = Repeat<"foo-bar">;
/// @resolution.name source=Repeat target=Repeat
/// @generic.instance source="Repeat<\"foo-bar\">" id="Repeat<\"foo-bar\">"
/// @type.symbol symbol=Match type="no"

const matched: Match = "no";
/// @type.symbol symbol=matched source=matched type="no"
/// @resolution.name source=Match target=Match

/// @generic.instance id="Repeat<\"foo-bar\">" template=Repeat arguments=("foo-bar")
"#,
    );
}

#[test]
fn test_template_literal_infer_splits_on_first_literal_boundary() {
    let session = TestSession::single(
        r#"
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const result: Result = ("foo", "bar-baz");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const result: Result = ("foo", "bar-baz");

=== checked ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
/// @generic.template symbol=Pair parameters=(T)
/// @type.symbol symbol=Pair type=T extends `${infer A}-${infer B}` ? (A, B) : never

type Result = Pair<"foo-bar-baz">;
/// @resolution.name source=Pair target=Pair
/// @generic.instance source="Pair<\"foo-bar-baz\">" id="Pair<\"foo-bar-baz\">"
/// @type.symbol symbol=Result type=("foo", "bar-baz")

const result: Result = ("foo", "bar-baz");
/// @type.symbol symbol=result source=result type=("foo", "bar-baz")
/// @resolution.name source=Result target=Result

/// @generic.instance id="Pair<\"foo-bar-baz\">" template=Pair arguments=("foo-bar-baz")
"#,
    );
}

#[test]
fn test_template_literal_infer_uses_earliest_literal_boundary() {
    let session = TestSession::single(
        r#"
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const bad: Result = ("foo-bar", "baz");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const bad: Result = ("foo-bar", "baz");

=== checked ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
/// @generic.template symbol=Pair parameters=(T)
/// @type.symbol symbol=Pair type=T extends `${infer A}-${infer B}` ? (A, B) : never

type Result = Pair<"foo-bar-baz">;
/// @resolution.name source=Pair target=Pair
/// @generic.instance source="Pair<\"foo-bar-baz\">" id="Pair<\"foo-bar-baz\">"
/// @type.symbol symbol=Result type=("foo", "bar-baz")

const bad: Result = ("foo-bar", "baz");
/// @type.symbol symbol=bad source=bad type=("foo", "bar-baz")
/// @resolution.name source=Result target=Result
/// @generic.instance id="Pair<\"foo-bar-baz\">" template=Pair arguments=("foo-bar-baz")
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '(\"foo-bar\", \"baz\")' is not assignable to type 'Result'"
/// @diagnostic.label line=5 column=7 source="const bad: Result = (\"foo-bar\", \"baz\");"
"#,
    );
}

#[test]
fn test_template_literal_infer_adjacent_spans_keep_first_span_nonempty() {
    let session = TestSession::single(
        r#"
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("a", "");
const bad: Result = ("", "a");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("a", "");
const bad: Result = ("", "a");

=== checked ===
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;
/// @generic.template symbol=Split parameters=(T)
/// @type.symbol symbol=Split type=T extends `${infer A}${infer B}` ? (A, B) : never

type Result = Split<"a">;
/// @resolution.name source=Split target=Split
/// @generic.instance source="Split<\"a\">" id="Split<\"a\">"
/// @type.symbol symbol=Result type=("a", "")

const ok: Result = ("a", "");
/// @type.symbol symbol=ok source=ok type=("a", "")
/// @resolution.name source=Result target=Result

const bad: Result = ("", "a");
/// @type.symbol symbol=bad source=bad type=("a", "")
/// @resolution.name source=Result target=Result
/// @generic.instance id="Split<\"a\">" template=Split arguments=("a")
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '(\"\", \"a\")' is not assignable to type 'Result'"
/// @diagnostic.label line=6 column=7 source="const bad: Result = (\"\", \"a\");"
"#,
    );
}

#[test]
fn test_template_literal_infer_literal_boundary_allows_empty_adjacent_spans() {
    let session = TestSession::single(
        r#"
type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("", "");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("", "");

=== checked ===
type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never;
/// @generic.template symbol=Split parameters=(T)
/// @type.symbol symbol=Split type=T extends `a${infer A}${infer B}` ? (A, B) : never

type Result = Split<"a">;
/// @resolution.name source=Split target=Split
/// @generic.instance source="Split<\"a\">" id="Split<\"a\">"
/// @type.symbol symbol=Result type=("", "")

const ok: Result = ("", "");
/// @type.symbol symbol=ok source=ok type=("", "")
/// @resolution.name source=Result target=Result
/// @generic.instance id="Split<\"a\">" template=Split arguments=("a")
"#,
    );
}
