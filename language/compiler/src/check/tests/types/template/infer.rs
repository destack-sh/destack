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
/// @type.symbol symbol=Segment source="type Segment<T> = T extends `/${infer Name}` ? Name : never" type=T extends `/${infer Name}` ? Segment.Name : never
/// @definition.type symbol=Segment source="type Segment<T> = T extends `/${infer Name}` ? Name : never" template=(T) value=T extends `/${infer Name}` ? Segment.Name : never
/// @type.symbol symbol=Segment.T source=T type=T
/// @resolution.name source=T target=Segment.T
/// @resolution.name source=Name target=Segment.Name

type Name = Segment<"/api">;
/// @type.symbol symbol=Name source="type Name = Segment<\"/api\">" type=Segment<"/api"> reduced="api"
/// @definition.type symbol=Name source="type Name = Segment<\"/api\">" value=Segment<"/api"> reduced="api"
/// @resolution.name source=Segment target=Segment

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name reduced="api"
/// @resolution.name source=Name target=Name

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
/// @type.symbol symbol=HasId source="type HasId<T> = T extends `id:${infer _}` ? true : false" type=T extends `id:${infer _}` ? true : false
/// @definition.type symbol=HasId source="type HasId<T> = T extends `id:${infer _}` ? true : false" template=(T) value=T extends `id:${infer _}` ? true : false
/// @type.symbol symbol=HasId.T source=T type=T
/// @resolution.name source=T target=HasId.T

type Yes = HasId<"id:users">;
/// @type.symbol symbol=Yes source="type Yes = HasId<\"id:users\">" type=HasId<"id:users"> reduced=true
/// @definition.type symbol=Yes source="type Yes = HasId<\"id:users\">" value=HasId<"id:users"> reduced=true
/// @resolution.name source=HasId target=HasId

type No = HasId<"users">;
/// @type.symbol symbol=No source="type No = HasId<\"users\">" type=HasId<"users"> reduced=false
/// @definition.type symbol=No source="type No = HasId<\"users\">" value=HasId<"users"> reduced=false
/// @resolution.name source=HasId target=HasId

const yes: Yes = true;
/// @type.symbol symbol=yes source=yes type=Yes reduced=true
/// @resolution.name source=Yes target=Yes

const no: No = false;
/// @type.symbol symbol=no source=no type=No reduced=false
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
/// @type.symbol symbol=Extract source="type Extract<T> = T extends `foo-${infer A}` ? A : never" type=T extends `foo-${infer A}` ? Extract.A : never
/// @definition.type symbol=Extract source="type Extract<T> = T extends `foo-${infer A}` ? A : never" template=(T) value=T extends `foo-${infer A}` ? Extract.A : never
/// @type.symbol symbol=Extract.T source=T type=T
/// @resolution.name source=T target=Extract.T
/// @resolution.name source=A target=Extract.A

type Result = Extract<`foo-a` | `foo-b`>;
/// @type.symbol symbol=Result source="type Result = Extract<`foo-a` | `foo-b`>" type=Extract<`foo-a` | `foo-b`> reduced="a" | "b"
/// @definition.type symbol=Result source="type Result = Extract<`foo-a` | `foo-b`>" value=Extract<`foo-a` | `foo-b`> reduced="a" | "b"
/// @resolution.name source=Extract target=Extract

const a: Result = "a";
/// @type.symbol symbol=a source=a type=Result reduced="a" | "b"
/// @resolution.name source=Result target=Result

const b: Result = "b";
/// @type.symbol symbol=b source=b type=Result reduced="a" | "b"
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
/// @type.symbol symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" type=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @definition.type symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" template=(T) value=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @type.symbol symbol=Repeat.T source=T type=T
/// @resolution.name source=T target=Repeat.T
/// @resolution.name source=A target=Repeat.A

type Match = Repeat<"foo-foo">;
/// @type.symbol symbol=Match source="type Match = Repeat<\"foo-foo\">" type=Repeat<"foo-foo"> reduced="foo"
/// @definition.type symbol=Match source="type Match = Repeat<\"foo-foo\">" value=Repeat<"foo-foo"> reduced="foo"
/// @resolution.name source=Repeat target=Repeat

const matched: Match = "foo";
/// @type.symbol symbol=matched source=matched type=Match reduced="foo"
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
/// @type.symbol symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" type=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @definition.type symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" template=(T) value=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @type.symbol symbol=Repeat.T source=T type=T
/// @resolution.name source=T target=Repeat.T
/// @resolution.name source=A target=Repeat.A

type Match = Repeat<"foo-bar">;
/// @type.symbol symbol=Match source="type Match = Repeat<\"foo-bar\">" type=Repeat<"foo-bar"> reduced="no"
/// @definition.type symbol=Match source="type Match = Repeat<\"foo-bar\">" value=Repeat<"foo-bar"> reduced="no"
/// @resolution.name source=Repeat target=Repeat

const matched: Match = "no";
/// @type.symbol symbol=matched source=matched type=Match reduced="no"
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
/// @type.symbol symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" type=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @definition.type symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" template=(T) value=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @type.symbol symbol=Pair.T source=T type=T
/// @resolution.name source=T target=Pair.T
/// @resolution.name source=A target=Pair.A
/// @resolution.name source=B target=Pair.B

type Result = Pair<"foo-bar-baz">;
/// @type.symbol symbol=Result source="type Result = Pair<\"foo-bar-baz\">" type=Pair<"foo-bar-baz"> reduced=("foo", "bar-baz")
/// @definition.type symbol=Result source="type Result = Pair<\"foo-bar-baz\">" value=Pair<"foo-bar-baz"> reduced=("foo", "bar-baz")
/// @resolution.name source=Pair target=Pair

const result: Result = ("foo", "bar-baz");
/// @type.symbol symbol=result source=result type=Result reduced=("foo", "bar-baz")
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
/// @type.symbol symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" type=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @definition.type symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" template=(T) value=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @type.symbol symbol=Pair.T source=T type=T
/// @resolution.name source=T target=Pair.T
/// @resolution.name source=A target=Pair.A
/// @resolution.name source=B target=Pair.B

type Result = Pair<"foo-bar-baz">;
/// @type.symbol symbol=Result source="type Result = Pair<\"foo-bar-baz\">" type=Pair<"foo-bar-baz"> reduced=("foo", "bar-baz")
/// @definition.type symbol=Result source="type Result = Pair<\"foo-bar-baz\">" value=Pair<"foo-bar-baz"> reduced=("foo", "bar-baz")
/// @resolution.name source=Pair target=Pair

const bad: Result = ("foo-bar", "baz");
/// @type.symbol symbol=bad source=bad type=Result reduced=("foo", "bar-baz")
/// @resolution.name source=Result target=Result

/// @generic.instance id="Pair<\"foo-bar-baz\">" template=Pair arguments=("foo-bar-baz")
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"foo-bar\"' is not assignable to type '\"foo\"'"
/// @diagnostic.label line=5 column=22 span="\"foo-bar\"" line_source="const bad: Result = (\"foo-bar\", \"baz\");"
/// @diagnostic.error id=not-assignable message="type '\"baz\"' is not assignable to type '\"bar-baz\"'"
/// @diagnostic.label line=5 column=33 span="\"baz\"" line_source="const bad: Result = (\"foo-bar\", \"baz\");"
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
/// @type.symbol symbol=Split source="type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never" type=T extends `${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @definition.type symbol=Split source="type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never" template=(T) value=T extends `${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @type.symbol symbol=Split.T source=T type=T
/// @resolution.name source=T target=Split.T
/// @resolution.name source=A target=Split.A
/// @resolution.name source=B target=Split.B

type Result = Split<"a">;
/// @type.symbol symbol=Result source="type Result = Split<\"a\">" type=Split<"a"> reduced=("a", "")
/// @definition.type symbol=Result source="type Result = Split<\"a\">" value=Split<"a"> reduced=("a", "")
/// @resolution.name source=Split target=Split

const ok: Result = ("a", "");
/// @type.symbol symbol=ok source=ok type=Result reduced=("a", "")
/// @resolution.name source=Result target=Result

const bad: Result = ("", "a");
/// @type.symbol symbol=bad source=bad type=Result reduced=("a", "")
/// @resolution.name source=Result target=Result

/// @generic.instance id="Split<\"a\">" template=Split arguments=("a")
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"\"' is not assignable to type '\"a\"'"
/// @diagnostic.label line=6 column=22 span="\"\"" line_source="const bad: Result = (\"\", \"a\");"
/// @diagnostic.error id=not-assignable message="type '\"a\"' is not assignable to type '\"\"'"
/// @diagnostic.label line=6 column=26 span="\"a\"" line_source="const bad: Result = (\"\", \"a\");"
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
/// @type.symbol symbol=Split source="type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never" type=T extends `a${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @definition.type symbol=Split source="type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never" template=(T) value=T extends `a${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @type.symbol symbol=Split.T source=T type=T
/// @resolution.name source=T target=Split.T
/// @resolution.name source=A target=Split.A
/// @resolution.name source=B target=Split.B

type Result = Split<"a">;
/// @type.symbol symbol=Result source="type Result = Split<\"a\">" type=Split<"a"> reduced=("", "")
/// @definition.type symbol=Result source="type Result = Split<\"a\">" value=Split<"a"> reduced=("", "")
/// @resolution.name source=Split target=Split

const ok: Result = ("", "");
/// @type.symbol symbol=ok source=ok type=Result reduced=("", "")
/// @resolution.name source=Result target=Result

/// @generic.instance id="Split<\"a\">" template=Split arguments=("a")
"#,
    );
}
