use crate::tests::{DirRows, TestSession};

/// A call through a prefixed template literal infers the captured span.
#[test]
fn test_infer_a_captured_prefix_span_through_a_template_literal_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${T}`): T;

const segment = parse("id:users");

segment satisfies "users";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

const segment: "users" = parse<"users">("id:users");

segment satisfies "users";

=== dir ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `id:${T}`): T" type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const segment = parse("id:users");
/// @type.symbol symbol=segment source=segment type="users"
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"id:users\")" parameters=(`id:${"users"}`) arguments=(provided("id:users") as `id:${"users"}`) return="users" kind=symbol target=parse instance="parse<\"users\">"
/// @generic.instantiation id="parse<\"users\">" template=parse arguments=("users")
/// @generic.instance id="parse<\"users\">" template=parse arguments=("users") dependents=("id:users")

segment satisfies "users";
/// @resolution.name source=segment target=segment
/// @resolution.place source=segment placement="local" lifetime="static" access="immutable"
/// @resolution.access source=segment root=segment
"#,
    );
}

/// A call returning a template literal builds it from the captured span.
#[test]
fn test_build_a_template_literal_from_a_captured_span() {
    let session = TestSession::single(
        r#"
declare function build<T: string>(value: T): `id:${T}`;

const key = build("users");

key satisfies "id:users";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function build<T: string>(value: T): `id:${T}`;

const key: "id:users" = build<"users">("users");

key satisfies "id:users";

=== dir ===
declare function build<T: string>(value: T): `id:${T}`;
/// @generic.template symbol=build parameters=(T: string)
/// @type.symbol symbol=build source="declare function build<T: string>(value: T): `id:${T}`" type=<T: string>(T) => `id:${T}`
/// @type.symbol symbol=build.T source="T: string" type=T
/// @resolution.name source=T target=build.T
/// @resolution.name source=T target=build.T

const key = build("users");
/// @type.symbol symbol=key source=key type="id:users"
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=build target=build
/// @resolution.call source="build(\"users\")" parameters=("users") arguments=(provided("users") as "users") return=`id:${"users"}` kind=symbol target=build instance="build<\"users\">"
/// @generic.instantiation id="build<\"users\">" template=build arguments=("users")
/// @generic.instance id="build<\"users\">" template=build arguments=("users") dependents=("id:users")

key satisfies "id:users";
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="immutable"
/// @resolution.access source=key root=key
"#,
    );
}

/// A bare template span accepts a widened string argument.
#[test]
fn test_accept_widened_string_inputs_in_a_generic_template_literal_call() {
    let session = TestSession::single(
        r#"
declare function identity<T: string>(value: `${T}`): T;

let value = "users";

const text = identity(value);

text satisfies string;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function identity<T: string>(value: `${T}`): T;

let value: string = "users";

const text: string = identity<string>(value);

text satisfies string;

=== dir ===
declare function identity<T: string>(value: `${T}`): T;
/// @generic.template symbol=identity parameters=(T: string)
/// @type.symbol symbol=identity source="declare function identity<T: string>(value: `${T}`): T" type=<T: string>(`${T}`) => T
/// @type.symbol symbol=identity.T source="T: string" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

let value = "users";
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value

const text = identity(value);
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(value) parameters=(`${string}`) arguments=(provided(value) as `${string}`) return=string kind=symbol target=identity instance=identity<string>
/// @generic.instantiation id=identity<string> template=identity arguments=(string)
/// @generic.instance id=identity<string> template=identity arguments=(string) dependents=(`${string}`)
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

text satisfies string;
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="local" lifetime="static" access="immutable"
/// @resolution.access source=text root=text
"#,
    );
}

/// A prefixed template span reports a diagnostic for a widened string argument.
#[test]
fn test_reject_widened_string_inputs_in_a_prefixed_template_literal_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${T}`): T;

let key = "id:users";

parse(key);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

let key: string = "id:users";

parse<string>(key);

=== dir ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `id:${T}`): T" type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

let key = "id:users";
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key

parse(key);
/// @resolution.name source=parse target=parse
/// @resolution.call source=parse(key) parameters=(`id:${string}`) arguments=(provided(key) as `id:${string}`) return=string kind=symbol target=parse instance=parse<string>
/// @generic.instantiation id=parse<string> template=parse arguments=(string)
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=key root=key
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'string' is not assignable to parameter of type '`id:${string}`'"
/// @diagnostic.label line=6 column=7 span="key" line_source="parse(key);"
/// @diagnostic.related line=6 column=1 span="parse(key)" line_source="parse(key);" message="in this call"
"#,
    );
}

/// A call matching only the prefix infers an empty captured span.
#[test]
fn test_infer_an_empty_span_at_a_literal_boundary_through_a_template_literal_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${T}`): T;

const segment = parse("id:");

segment satisfies "";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

const segment: "" = parse<"">("id:");

segment satisfies "";

=== dir ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `id:${T}`): T" type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const segment = parse("id:");
/// @type.symbol symbol=segment source=segment type=""
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"id:\")" parameters=(`id:${""}`) arguments=(provided("id:") as `id:${""}`) return="" kind=symbol target=parse instance="parse<\"\">"
/// @generic.instantiation id="parse<\"\">" template=parse arguments=("")
/// @generic.instance id="parse<\"\">" template=parse arguments=("") dependents=("id:")

segment satisfies "";
/// @resolution.name source=segment target=segment
/// @resolution.place source=segment placement="local" lifetime="static" access="immutable"
/// @resolution.access source=segment root=segment
"#,
    );
}

/// A call through a numeric template span infers the captured number.
#[test]
fn test_infer_a_constrained_numeric_span_through_a_template_literal_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: number>(value: `${T}`): T;

const value = parse("42");

value satisfies 42;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: number>(value: `${T}`): T;

const value: 42 = parse<42>("42");

value satisfies 42;

=== dir ===
declare function parse<T: number>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=(T: float64)
/// @type.symbol symbol=parse source="declare function parse<T: number>(value: `${T}`): T" type=<T: float64>(`${T}`) => T
/// @type.symbol symbol=parse.T source="T: number" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const value = parse("42");
/// @type.symbol symbol=value source=value type=42
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"42\")" parameters=(`${42}`) arguments=(provided("42") as `${42}`) return=42 kind=symbol target=parse instance=parse<42>
/// @generic.instantiation id=parse<42> template=parse arguments=(42)
/// @generic.instance id=parse<42> template=parse arguments=(42) dependents=(`${42}`)

value satisfies 42;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// A non-numeric argument reports a diagnostic at a numeric template span.
#[test]
fn test_reject_an_invalid_numeric_span_in_a_template_literal_call() {
    let session = TestSession::single(
        r#"
declare function parse<T: number>(value: `${T}`): T;

parse("no");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: number>(value: `${T}`): T;

parse<float64>("no");

=== dir ===
declare function parse<T: number>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=(T: float64)
/// @type.symbol symbol=parse source="declare function parse<T: number>(value: `${T}`): T" type=<T: float64>(`${T}`) => T
/// @type.symbol symbol=parse.T source="T: number" type=T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

parse("no");
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"no\")" parameters=(`${float64}`) arguments=(provided("no") as `${float64}`) return=float64 kind=symbol target=parse instance=parse<float64>
/// @generic.instantiation id=parse<float64> template=parse arguments=(float64)
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"no\"' is not assignable to parameter of type '`${float64}`'"
/// @diagnostic.label line=4 column=7 span="\"no\"" line_source="parse(\"no\");"
/// @diagnostic.related line=4 column=1 span="parse(\"no\")" line_source="parse(\"no\");" message="in this call"
"#,
    );
}

/// Block a template span capture behind NoInfer.
#[test]
fn test_block_a_template_span_capture_behind_noinfer() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${NoInfer<T>}`, fallback: T): T;

const segment = parse("id:users", "other");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${NoInfer<T>}`, fallback: T): T;

const segment: "other" = parse<"other">("id:users", "other");

=== dir ===
declare function parse<T: string>(value: `id:${NoInfer<T>}`, fallback: T): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse source="declare function parse<T: string>(value: `id:${NoInfer<T>}`, fallback: T): T" type=<T: string>(`id:${NoInfer<T>}`, T) => T
/// @type.symbol symbol=parse.T source="T: string" type=T
/// @resolution.name source=NoInfer target=NoInfer
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T
/// @resolution.name source=T target=parse.T

const segment = parse("id:users", "other");
/// @type.symbol symbol=segment source=segment type="other"
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"id:users\", \"other\")" parameters=(`id:${"other"}`, "other") arguments=(provided("id:users") as `id:${"other"}`, provided("other") as "other") return="other" kind=symbol target=parse instance="parse<\"other\">"
/// @generic.instantiation id="parse<\"other\">" template=parse arguments=("other")
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"id:users\"' is not assignable to parameter of type '`id:${\"other\"}`'"
/// @diagnostic.label line=4 column=23 span="\"id:users\"" line_source="const segment = parse(\"id:users\", \"other\");"
/// @diagnostic.related line=4 column=17 span="parse(\"id:users\", \"other\")" line_source="const segment = parse(\"id:users\", \"other\");" message="in this call"
/// @diagnostic.note message="'`id:${\"other\"}`' reduces to '\"id:other\"'"
"#,
    );
}
