use crate::tests::{DirRows, TestSession};

#[test]
fn test_repeated_template_parts_must_match() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `${T}-${T}`): T;

const segment = parse("row-row");
segment satisfies "row";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `${T}-${T}`): T;

const segment: "row" = parse<"row">("row-row");
segment satisfies "row";

=== checked ===
declare function parse<T: string>(value: `${T}-${T}`): T;
/// @generic.template symbol=parse parameters=[T: string]
/// @type.symbol symbol=parse type=<T: string>(`${T}-${T}`) => T
/// @type.symbol symbol=value type=`${T}-${T}`

const segment = parse("row-row");
/// @type.symbol symbol=segment type="row"
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"row-row\")" parameters=(`${"row"}-${"row"}`) return="row" kind=symbol target=parse instance="parse<\"row\">"
/// @generic.instance source="parse(\"row-row\")" id="parse<\"row\">"

segment satisfies "row";
/// @resolution.name source=segment target=segment
/// @generic.instance id="parse<\"row\">" symbol=parse arguments=["row"]
"#,
    );
}

#[test]
fn test_repeated_template_parts_reject_different_text() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `${T}-${T}`): T;

parse("row-col");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `${T}-${T}`): T;

parse("row-col");

=== checked ===
declare function parse<T: string>(value: `${T}-${T}`): T;
/// @generic.template symbol=parse parameters=[T: string]
/// @type.symbol symbol=parse type=<T: string>(`${T}-${T}`) => T
/// @type.symbol symbol=value type=`${T}-${T}`

parse("row-col");
/// @resolution.name source=parse target=parse
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"row-col\"' is not assignable to type '`${T}-${T}`'"
/// @diagnostic.label line=4 column=1 source="parse(\"row-col\");"
"#,
    );
}

#[test]
fn test_callback_receives_captured_template_text() {
    let session = TestSession::single(
        r#"
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const segment = withParsed("id:users", (segment) => segment);
segment satisfies "users";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;

const segment: "users" = withParsed<"users", "users">("id:users", (segment: "users") => segment);
segment satisfies "users";

=== checked ===
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;
/// @generic.template symbol=withParsed parameters=[T: string, U]
/// @type.symbol symbol=withParsed type=<T: string, U>(`id:${T}`, (T) => U) => U
/// @type.symbol symbol=value type=`id:${T}`
/// @type.symbol symbol=callback type=(T) => U

const segment = withParsed("id:users", (segment) => segment);
/// @type.symbol symbol=segment type="users"
/// @resolution.name source=withParsed target=withParsed
/// @resolution.call source="withParsed(\"id:users\", (segment) => segment)" parameters=(`id:${"users"}`, Function<("users",), "users">) return="users" kind=symbol target=withParsed instance="withParsed<\"users\", \"users\">"
/// @generic.instance source="withParsed(\"id:users\", (segment) => segment)" id="withParsed<\"users\", \"users\">"

segment satisfies "users";
/// @resolution.name source=segment target=segment
/// @generic.instance id="withParsed<\"users\", \"users\">" symbol=withParsed arguments=["users", "users"]
"#,
    );
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

const input: "id:users" | "id:posts" = true ? "id:users" : "id:posts";
const segment: "users" | "posts" = parse<"users" | "posts">(input);

segment satisfies "users" | "posts";

=== checked ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=[T: string]
/// @type.symbol symbol=parse type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=value type=`id:${T}`

const input = true ? "id:users" : "id:posts";
/// @type.symbol symbol=input type="id:users" | "id:posts"

const segment = parse(input);
/// @type.symbol symbol=segment type="users" | "posts"
/// @resolution.name source=parse target=parse
/// @resolution.name source=input target=input
/// @resolution.call source=parse(input) parameters=(`id:${"users" | "posts"}`) return="users" | "posts" kind=symbol target=parse instance="parse<\"users\" | \"posts\">"
/// @generic.instance source=parse(input) id="parse<\"users\" | \"posts\">"

segment satisfies "users" | "posts";
/// @resolution.name source=segment target=segment
/// @generic.instance id="parse<\"users\" | \"posts\">" symbol=parse arguments=["users" | "posts"]
"#,
    );
}
