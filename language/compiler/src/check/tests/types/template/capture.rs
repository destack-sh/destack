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
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `${T}-${T}`): T" type=<T: string>(`${T}-${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @type.symbol symbol=parse.value source="value: `${T}-${T}`" type=`${T}-${T}`
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const segment = parse("row-row");
/// @type.symbol symbol=segment source=segment type="row"
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"row-row\")" parameters=(`${"row"}-${"row"}`) arguments=(provided("row-row") as `${"row"}-${"row"}`) return="row" kind=symbol target=parse instance="parse<\"row\">"
/// @generic.instance source="parse(\"row-row\")" id="parse<\"row\">"

segment satisfies "row";
/// @resolution.name source=segment target=segment

/// @generic.instance id="parse<\"row\">" template=parse arguments=("row")
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
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `${T}-${T}`): T" type=<T: string>(`${T}-${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @type.symbol symbol=parse.value source="value: `${T}-${T}`" type=`${T}-${T}`
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

parse("row-col");
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"row-col\")" parameters=(`${<error>}-${<error>}`) arguments=(provided("row-col") as `${<error>}-${<error>}`) return=<error> kind=symbol target=parse instance=parse<<error>>
/// @generic.instance source="parse(\"row-col\")" id=parse<<error>>

/// @generic.instance id=parse<<error>> template=parse arguments=(<error>)
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type '\"row-col\"' is not assignable to parameter of type '`${_}-${_}`'"
/// @diagnostic.label line=4 column=7 span="\"row-col\"" line_source="parse(\"row-col\");"
/// @diagnostic.related line=4 column=1 span="parse(\"row-col\")" line_source="parse(\"row-col\");" message="in this call"
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
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (arg0: T) => U): U;

const segment: "users" = withParsed<"users", "users">(
    "id:users",
    (segment: "users"): "users" => segment,
);
segment satisfies "users";

=== checked ===
declare function withParsed<T: string, U>(value: `id:${T}`, callback: (segment: T) => U): U;
/// @generic.template symbol=withParsed parameters=(T: string, U)
/// @type.symbol symbol=withParsed type=<T: string, U>(`id:${T}`, Function<(T,), U>) => U
/// @type.symbol symbol=withParsed.T source="T: string" type=T
/// @type.symbol symbol=withParsed.U source=U type=U
/// @type.symbol symbol=withParsed.value source="value: `id:${T}`" type=`id:${T}`
/// @resolution.name source=T target=withParsed.T
/// @type.symbol symbol=withParsed.callback source="callback: (segment: T) => U" type=Function<(T,), U>
/// @resolution.name source=T target=withParsed.T
/// @resolution.name source=U target=withParsed.U
/// @resolution.name source=U target=withParsed.U

const segment = withParsed("id:users", (segment) => segment);
/// @type.symbol symbol=segment source=segment type="users"
/// @resolution.name source=withParsed target=withParsed
/// @resolution.call source="withParsed(\"id:users\", (segment) => segment)" parameters=(`id:${"users"}`, Function<("users",), "users">) arguments=(provided("id:users") as `id:${"users"}`, provided((segment) => segment) as Function<("users",), "users">) return="users" kind=symbol target=withParsed instance="withParsed<\"users\", \"users\">"
/// @generic.instance source="withParsed(\"id:users\", (segment) => segment)" id="withParsed<\"users\", \"users\">"
/// @type.symbol symbol=symbol7 source="(segment) => segment" type=Function<("users",), "users">
/// @type.symbol symbol=symbol7.segment source=segment type="users"
/// @resolution.name source=segment target=symbol7.segment

segment satisfies "users";
/// @resolution.name source=segment target=segment

/// @generic.instance id="withParsed<\"users\", \"users\">" template=withParsed arguments=("users", "users")
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

    session.assert_dir_checked_and_diagnostics(
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
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `id:${T}`): T" type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @type.symbol symbol=parse.value source="value: `id:${T}`" type=`id:${T}`
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const input = true ? "id:users" : "id:posts";
/// @type.symbol symbol=input source=input type="id:users" | "id:posts"

const segment = parse(input);
/// @type.symbol symbol=segment source=segment type="users" | "posts"
/// @resolution.name source=parse target=parse
/// @resolution.call source=parse(input) parameters=(`id:${"users" | "posts"}`) arguments=(provided(input) as `id:${"users" | "posts"}`) return="users" | "posts" kind=symbol target=parse instance="parse<\"users\" | \"posts\">"
/// @generic.instance source=parse(input) id="parse<\"users\" | \"posts\">"
/// @resolution.name source=input target=input

segment satisfies "users" | "posts";
/// @resolution.name source=segment target=segment

/// @generic.instance id="parse<\"users\" | \"posts\">" template=parse arguments=("users" | "posts")
"#,
        r#"
/// @diagnostic.warning code=WC402 message="condition is always true"
/// @diagnostic.label line=4 column=15 span="true" line_source="const input = true ? \"id:users\" : \"id:posts\";"
"#,
    );
}
