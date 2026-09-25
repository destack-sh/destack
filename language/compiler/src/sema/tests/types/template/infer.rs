use crate::tests::{DirRows, TestSession};

#[test]
fn test_capture_a_segment_through_a_template_infer_span() {
    let session = TestSession::single(
        r#"
type Segment<T> = T extends `/${infer Name}` ? Name : never;
type Name = Segment<"/api">;

declare const name: Name;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Segment<T> = T extends `/${infer Name}` ? Name : never;
type Name = Segment<"/api">;

declare const name: Name;

=== dir ===
type Segment<T> = T extends `/${infer Name}` ? Name : never;
/// @generic.template symbol=Segment parameters=(T)
/// @type.symbol symbol=Segment source="type Segment<T> = T extends `/${infer Name}` ? Name : never" type=T extends `/${infer Name}` ? Segment.Name : never
/// @definition.type symbol=Segment source="type Segment<T> = T extends `/${infer Name}` ? Name : never" template=(T) value=T extends `/${infer Name}` ? Segment.Name : never
/// @type.symbol symbol=Segment.T source=T type=T
/// @resolution.name source=T target=Segment.T
/// @resolution.name source=Name target=Segment.Name

type Name = Segment<"/api">;
/// @type.symbol symbol=Name source="type Name = Segment<\"/api\">" type="api"
/// @definition.type symbol=Name source="type Name = Segment<\"/api\">" value=Segment<"/api">
/// @resolution.name source=Segment target=Segment

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name
"#,
    );
}

#[test]
fn test_ignore_an_unnamed_span_in_a_template_infer() {
    let session = TestSession::single(
        r#"
type HasId<T> = T extends `id:${infer _}` ? true : false;
type Yes = HasId<"id:users">;
type No = HasId<"users">;

const yes: Yes = true;
const no: No = false;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type HasId<T> = T extends `id:${infer _}` ? true : false;
type Yes = HasId<"id:users">;
type No = HasId<"users">;

const yes: Yes = true;
const no: No = false;

=== dir ===
type HasId<T> = T extends `id:${infer _}` ? true : false;
/// @generic.template symbol=HasId parameters=(T)
/// @type.symbol symbol=HasId source="type HasId<T> = T extends `id:${infer _}` ? true : false" type=T extends `id:${infer _}` ? true : false
/// @definition.type symbol=HasId source="type HasId<T> = T extends `id:${infer _}` ? true : false" template=(T) value=T extends `id:${infer _}` ? true : false
/// @type.symbol symbol=HasId.T source=T type=T
/// @resolution.name source=T target=HasId.T

type Yes = HasId<"id:users">;
/// @type.symbol symbol=Yes source="type Yes = HasId<\"id:users\">" type=true
/// @definition.type symbol=Yes source="type Yes = HasId<\"id:users\">" value=HasId<"id:users">
/// @resolution.name source=HasId target=HasId

type No = HasId<"users">;
/// @type.symbol symbol=No source="type No = HasId<\"users\">" type=false
/// @definition.type symbol=No source="type No = HasId<\"users\">" value=HasId<"users">
/// @resolution.name source=HasId target=HasId

const yes: Yes = true;
/// @type.symbol symbol=yes source=yes type=Yes
/// @resolution.pattern source=yes kind=binding target=yes
/// @resolution.name source=Yes target=Yes

const no: No = false;
/// @type.symbol symbol=no source=no type=No
/// @resolution.pattern source=no kind=binding target=no
/// @resolution.name source=No target=No
"#,
    );
}

#[test]
fn test_distribute_a_template_infer_over_union_templates() {
    let session = TestSession::single(
        r#"
type Extract<T> = T extends `foo-${infer A}` ? A : never;
type Result = Extract<`foo-a` | `foo-b`>;

const a: Result = "a";
const b: Result = "b";
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Extract<T> = T extends `foo-${infer A}` ? A : never;
type Result = Extract<`foo-a` | `foo-b`>;

const a: Result = "a";
const b: Result = "b";

=== dir ===
type Extract<T> = T extends `foo-${infer A}` ? A : never;
/// @generic.template symbol=Extract parameters=(T)
/// @type.symbol symbol=Extract source="type Extract<T> = T extends `foo-${infer A}` ? A : never" type=T extends `foo-${infer A}` ? Extract.A : never
/// @definition.type symbol=Extract source="type Extract<T> = T extends `foo-${infer A}` ? A : never" template=(T) value=T extends `foo-${infer A}` ? Extract.A : never
/// @type.symbol symbol=Extract.T source=T type=T
/// @resolution.name source=T target=Extract.T
/// @resolution.name source=A target=Extract.A

type Result = Extract<`foo-a` | `foo-b`>;
/// @type.symbol symbol=Result source="type Result = Extract<`foo-a` | `foo-b`>" type="a" | "b"
/// @definition.type symbol=Result source="type Result = Extract<`foo-a` | `foo-b`>" value=Extract<`foo-a` | `foo-b`>
/// @resolution.name source=Extract target=Extract

const a: Result = "a";
/// @type.symbol symbol=a source=a type=Result
/// @resolution.pattern source=a kind=binding target=a
/// @resolution.name source=Result target=Result

const b: Result = "b";
/// @type.symbol symbol=b source=b type=Result
/// @resolution.pattern source=b kind=binding target=b
/// @resolution.name source=Result target=Result
"#,
    );
}

#[test]
fn test_merge_repeated_spans_in_a_template_infer() {
    let session = TestSession::single(
        r#"
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-foo">;

const matched: Match = "foo";
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-foo">;

const matched: Match = "foo";

=== dir ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
/// @generic.template symbol=Repeat parameters=(T)
/// @type.symbol symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" type=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @definition.type symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" template=(T) value=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @type.symbol symbol=Repeat.T source=T type=T
/// @resolution.name source=T target=Repeat.T
/// @resolution.name source=A target=Repeat.A

type Match = Repeat<"foo-foo">;
/// @type.symbol symbol=Match source="type Match = Repeat<\"foo-foo\">" type="foo"
/// @definition.type symbol=Match source="type Match = Repeat<\"foo-foo\">" value=Repeat<"foo-foo">
/// @resolution.name source=Repeat target=Repeat

const matched: Match = "foo";
/// @type.symbol symbol=matched source=matched type=Match
/// @resolution.pattern source=matched kind=binding target=matched
/// @resolution.name source=Match target=Match
"#,
    );
}

#[test]
fn test_fall_back_for_mismatched_repeated_spans_in_a_template_infer() {
    let session = TestSession::single(
        r#"
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-bar">;

const matched: Match = "no";
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
type Match = Repeat<"foo-bar">;

const matched: Match = "no";

=== dir ===
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : "no";
/// @generic.template symbol=Repeat parameters=(T)
/// @type.symbol symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" type=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @definition.type symbol=Repeat source="type Repeat<T> = T extends `${infer A}-${infer A}` ? A : \"no\"" template=(T) value=T extends `${infer A}-${infer A}` ? Repeat.A : "no"
/// @type.symbol symbol=Repeat.T source=T type=T
/// @resolution.name source=T target=Repeat.T
/// @resolution.name source=A target=Repeat.A

type Match = Repeat<"foo-bar">;
/// @type.symbol symbol=Match source="type Match = Repeat<\"foo-bar\">" type="no"
/// @definition.type symbol=Match source="type Match = Repeat<\"foo-bar\">" value=Repeat<"foo-bar">
/// @resolution.name source=Repeat target=Repeat

const matched: Match = "no";
/// @type.symbol symbol=matched source=matched type=Match
/// @resolution.pattern source=matched kind=binding target=matched
/// @resolution.name source=Match target=Match
"#,
    );
}

#[test]
fn test_split_a_template_infer_on_the_first_literal_boundary() {
    let session = TestSession::single(
        r#"
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const result: Result = ("foo", "bar-baz");
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const result: Result = ("foo", "bar-baz");

=== dir ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
/// @generic.template symbol=Pair parameters=(T)
/// @type.symbol symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" type=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @definition.type symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" template=(T) value=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @type.symbol symbol=Pair.T source=T type=T
/// @resolution.name source=T target=Pair.T
/// @resolution.name source=A target=Pair.A
/// @resolution.name source=B target=Pair.B

type Result = Pair<"foo-bar-baz">;
/// @type.symbol symbol=Result source="type Result = Pair<\"foo-bar-baz\">" type=("foo", "bar-baz")
/// @definition.type symbol=Result source="type Result = Pair<\"foo-bar-baz\">" value=Pair<"foo-bar-baz">
/// @resolution.name source=Pair target=Pair

const result: Result = ("foo", "bar-baz");
/// @type.symbol symbol=result source=result type=Result
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Result target=Result
"#,
    );
}

#[test]
fn test_use_the_earliest_literal_boundary_in_a_template_infer() {
    let session = TestSession::single(
        r#"
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const bad: Result = ("foo-bar", "baz");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
type Result = Pair<"foo-bar-baz">;

const bad: Result = ("foo-bar", "baz");

=== dir ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
/// @generic.template symbol=Pair parameters=(T)
/// @type.symbol symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" type=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @definition.type symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" template=(T) value=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @type.symbol symbol=Pair.T source=T type=T
/// @resolution.name source=T target=Pair.T
/// @resolution.name source=A target=Pair.A
/// @resolution.name source=B target=Pair.B

type Result = Pair<"foo-bar-baz">;
/// @type.symbol symbol=Result source="type Result = Pair<\"foo-bar-baz\">" type=("foo", "bar-baz")
/// @definition.type symbol=Result source="type Result = Pair<\"foo-bar-baz\">" value=Pair<"foo-bar-baz">
/// @resolution.name source=Pair target=Pair

const bad: Result = ("foo-bar", "baz");
/// @type.symbol symbol=bad source=bad type=Result
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Result target=Result
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"foo-bar\"' is not assignable to type '\"foo\"'"
/// @diagnostic.label line=5 column=22 span="\"foo-bar\"" line_source="const bad: Result = (\"foo-bar\", \"baz\");"
/// @diagnostic.related line=5 column=12 span="Result" line_source="const bad: Result = (\"foo-bar\", \"baz\");" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 0"
/// @diagnostic.error id=not-assignable message="type '\"baz\"' is not assignable to type '\"bar-baz\"'"
/// @diagnostic.label line=5 column=33 span="\"baz\"" line_source="const bad: Result = (\"foo-bar\", \"baz\");"
/// @diagnostic.related line=5 column=12 span="Result" line_source="const bad: Result = (\"foo-bar\", \"baz\");" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}

#[test]
fn test_keep_the_first_of_two_adjacent_infer_spans_nonempty() {
    let session = TestSession::single(
        r#"
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("a", "");
const bad: Result = ("", "a");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("a", "");
const bad: Result = ("", "a");

=== dir ===
type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never;
/// @generic.template symbol=Split parameters=(T)
/// @type.symbol symbol=Split source="type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never" type=T extends `${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @definition.type symbol=Split source="type Split<T> = T extends `${infer A}${infer B}` ? (A, B) : never" template=(T) value=T extends `${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @type.symbol symbol=Split.T source=T type=T
/// @resolution.name source=T target=Split.T
/// @resolution.name source=A target=Split.A
/// @resolution.name source=B target=Split.B

type Result = Split<"a">;
/// @type.symbol symbol=Result source="type Result = Split<\"a\">" type=("a", "")
/// @definition.type symbol=Result source="type Result = Split<\"a\">" value=Split<"a">
/// @resolution.name source=Split target=Split

const ok: Result = ("a", "");
/// @type.symbol symbol=ok source=ok type=Result
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Result target=Result

const bad: Result = ("", "a");
/// @type.symbol symbol=bad source=bad type=Result
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Result target=Result
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"\"' is not assignable to type '\"a\"'"
/// @diagnostic.label line=6 column=22 span="\"\"" line_source="const bad: Result = (\"\", \"a\");"
/// @diagnostic.related line=6 column=12 span="Result" line_source="const bad: Result = (\"\", \"a\");" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 0"
/// @diagnostic.error id=not-assignable message="type '\"a\"' is not assignable to type '\"\"'"
/// @diagnostic.label line=6 column=26 span="\"a\"" line_source="const bad: Result = (\"\", \"a\");"
/// @diagnostic.related line=6 column=12 span="Result" line_source="const bad: Result = (\"\", \"a\");" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}

#[test]
fn test_allow_empty_adjacent_spans_at_a_literal_boundary_in_a_template_infer() {
    let session = TestSession::single(
        r#"
type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("", "");
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never;
type Result = Split<"a">;

const ok: Result = ("", "");

=== dir ===
type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never;
/// @generic.template symbol=Split parameters=(T)
/// @type.symbol symbol=Split source="type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never" type=T extends `a${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @definition.type symbol=Split source="type Split<T> = T extends `a${infer A}${infer B}` ? (A, B) : never" template=(T) value=T extends `a${infer A}${infer B}` ? (Split.A, Split.B) : never
/// @type.symbol symbol=Split.T source=T type=T
/// @resolution.name source=T target=Split.T
/// @resolution.name source=A target=Split.A
/// @resolution.name source=B target=Split.B

type Result = Split<"a">;
/// @type.symbol symbol=Result source="type Result = Split<\"a\">" type=("", "")
/// @definition.type symbol=Result source="type Result = Split<\"a\">" value=Split<"a">
/// @resolution.name source=Split target=Split

const ok: Result = ("", "");
/// @type.symbol symbol=ok source=ok type=Result
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Result target=Result
"#,
    );
}

/// Take the false branch when the subject holds none of the template's literal text.
#[test]
fn test_take_the_false_branch_when_no_literal_text_matches_the_template() {
    let session = TestSession::single(
        r#"
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;

declare const value: Pair<"abc">;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;

declare const value: Pair<"abc">;

=== dir ===
type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never;
/// @generic.template symbol=Pair parameters=(T)
/// @type.symbol symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" type=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @definition.type symbol=Pair source="type Pair<T> = T extends `${infer A}-${infer B}` ? (A, B) : never" template=(T) value=T extends `${infer A}-${infer B}` ? (Pair.A, Pair.B) : never
/// @type.symbol symbol=Pair.T source=T type=T
/// @resolution.name source=T target=Pair.T
/// @resolution.name source=A target=Pair.A
/// @resolution.name source=B target=Pair.B

declare const value: Pair<"abc">;
/// @type.symbol symbol=value source=value type=Pair<"abc">
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Pair target=Pair
"#,
        r#"
"#,
    );
}

/// Capture a numeric span as a number literal through its infer constraint.
#[test]
fn test_capture_a_numeric_span_as_a_number_literal() {
    let session = TestSession::single(
        r#"
type Parse<T> = T extends `${infer N extends number}` ? N : never;

declare const value: Parse<"42">;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Parse<T> = T extends `${infer N extends number}` ? N : never;

declare const value: Parse<"42">;

=== dir ===
type Parse<T> = T extends `${infer N extends number}` ? N : never;
/// @generic.template symbol=Parse parameters=(T)
/// @type.symbol symbol=Parse source="type Parse<T> = T extends `${infer N extends number}` ? N : never" type=T extends `${infer N extends float64}` ? Parse.N : never
/// @definition.type symbol=Parse source="type Parse<T> = T extends `${infer N extends number}` ? N : never" template=(T) value=T extends `${infer N extends float64}` ? Parse.N : never
/// @type.symbol symbol=Parse.T source=T type=T
/// @resolution.name source=T target=Parse.T
/// @resolution.name source=N target=Parse.N

declare const value: Parse<"42">;
/// @type.symbol symbol=value source=value type=Parse<"42">
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Parse target=Parse
"#,
        r#"
"#,
    );
}
