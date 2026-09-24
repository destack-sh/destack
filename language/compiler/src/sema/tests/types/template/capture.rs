use crate::tests::{DirRows, TestSession};

/// A template repeating one span captures the text both positions share.
#[test]
fn test_repeated_template_parts_must_match() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `${T}-${T}`): T;

const segment = parse("row-row");
segment satisfies "row";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `${T}-${T}`): T;

const segment: "row" = parse<"row">("row-row");
segment satisfies "row";

=== dir ===
declare function parse<T: string>(value: `${T}-${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `${T}-${T}`): T" type=<T: string>(`${T}-${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const segment = parse("row-row");
/// @type.symbol symbol=segment source=segment type="row"
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"row-row\")" parameters=(`${"row"}-${"row"}`) arguments=(provided("row-row") as `${"row"}-${"row"}`) return="row" kind=symbol target=parse instance="parse<\"row\">"
/// @generic.instantiation id="parse<\"row\">" template=parse arguments=("row")
/// @generic.instance id="parse<\"row\">" template=parse arguments=("row") dependents=("row-row")

segment satisfies "row";
/// @resolution.name source=segment target=segment
/// @resolution.place source=segment placement="local" lifetime="static" access="immutable"
/// @resolution.access source=segment root=segment
"#,
    );
}

/// A template repeating one span reports a diagnostic when the positions differ.
#[test]
fn test_repeated_template_parts_reject_different_text() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `${T}-${T}`): T;

parse("row-col");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `${T}-${T}`): T;

parse<string>("row-col");

=== dir ===
declare function parse<T: string>(value: `${T}-${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `${T}-${T}`): T" type=<T: string>(`${T}-${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

parse("row-col");
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"row-col\")" parameters=(`${string}-${string}`) arguments=(provided("row-col") as `${string}-${string}`) return=string kind=symbol target=parse instance=parse<string>
/// @generic.instantiation id=parse<string> template=parse arguments=(string)
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"row-col\"' is not assignable to parameter of type '`${string}-${string}`'"
/// @diagnostic.label line=4 column=7 span="\"row-col\"" line_source="parse(\"row-col\");"
/// @diagnostic.related line=4 column=1 span="parse(\"row-col\")" line_source="parse(\"row-col\");" message="in this call"
"#,
    );
}

/// A callback parameter receives the text a template span captures.
#[test]
fn test_callback_receives_captured_template_text() {
    let session = TestSession::single(
        r#"
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const segment = withParsed("id:users", (segment) => segment);
segment satisfies "users";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const segment: "users" = withParsed<"users", "users">(
    "id:users",
    (segment: "users"): "users" => segment,
);
segment satisfies "users";

=== dir ===
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;
/// @generic.template symbol=withParsed parameters=(T: string, U)
/// @type.symbol symbol=withParsed type=<T: string, U>(`id:${T}`, (T) => U) => U
/// @type.symbol symbol=withParsed.T source="T: string" type=T
/// @type.symbol symbol=withParsed.U source=U type=U
/// @resolution.name source=T target=withParsed.T
/// @type.symbol symbol=withParsed.segment source="segment: T" type=T
/// @resolution.name source=T target=withParsed.T
/// @resolution.name source=U target=withParsed.U
/// @resolution.name source=U target=withParsed.U

const segment = withParsed("id:users", (segment) => segment);
/// @type.symbol symbol=segment source=segment type="users"
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=withParsed target=withParsed
/// @resolution.call source="withParsed(\"id:users\", (segment) => segment)" parameters=(`id:${"users"}`, ("users") => "users") arguments=(provided("id:users") as `id:${"users"}`, provided((segment) => segment) as ("users") => "users") return="users" kind=symbol target=withParsed instance="withParsed<\"users\", \"users\">"
/// @generic.instantiation id="withParsed<\"users\", \"users\">" template=withParsed arguments=("users", "users")
/// @generic.instance id="withParsed<\"users\", \"users\">" template=withParsed arguments=("users", "users") dependents=("id:users")
/// @type.symbol symbol=symbol7 source="(segment) => segment" type=Function<("users",), "users", "readonly">
/// @type.symbol symbol=symbol7.segment source=segment type="users"
/// @resolution.name source=segment target=symbol7.segment
/// @resolution.place source=segment placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=segment root=symbol7.segment

segment satisfies "users";
/// @resolution.name source=segment target=segment
/// @resolution.place source=segment placement="local" lifetime="static" access="immutable"
/// @resolution.access source=segment root=segment
"#,
    );
}

/// A const ternary argument captures the union of both template spans.
#[test]
fn test_const_ternary_keeps_captured_template_union() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${T}`): T;

const input = true ? "id:users" : "id:posts";
const segment = parse(input);

segment satisfies "users" | "posts";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

const input: "id:users" | "id:posts" = true ? "id:users" : "id:posts";
const segment: "users" | "posts" = parse<"users" | "posts">(input);

segment satisfies "users" | "posts";

=== dir ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `id:${T}`): T" type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const input = true ? "id:users" : "id:posts";
/// @type.symbol symbol=input source=input type="id:users" | "id:posts"
/// @resolution.pattern source=input kind=binding target=input

const segment = parse(input);
/// @type.symbol symbol=segment source=segment type="users" | "posts"
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=parse target=parse
/// @resolution.call source=parse(input) parameters=(`id:${"users" | "posts"}`) arguments=(provided(input) as `id:${"users" | "posts"}`) return="users" | "posts" kind=symbol target=parse instance="parse<\"users\" | \"posts\">"
/// @generic.instantiation id="parse<\"users\" | \"posts\">" template=parse arguments=("users" | "posts")
/// @resolution.name source=input target=input
/// @resolution.place source=input placement="local" lifetime="static" access="immutable"
/// @resolution.access source=input root=input

segment satisfies "users" | "posts";
/// @resolution.name source=segment target=segment
/// @resolution.place source=segment placement="local" lifetime="static" access="immutable"
/// @resolution.access source=segment root=segment
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=4 column=15 span="true" line_source="const input = true ? \"id:users\" : \"id:posts\";"
"#,
    );
}
