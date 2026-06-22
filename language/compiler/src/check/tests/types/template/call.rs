use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_calls_infer_captured_prefix_span() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${T}`): T;

const segment = parse("id:users");

segment satisfies "users";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

const segment: "users" = parse<"users">("id:users");

segment satisfies "users";

=== checked ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=value type=`id:${T}`

const segment = parse("id:users");
/// @type.symbol symbol=segment type="users"
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"id:users\")" parameters=(`id:${"users"}`) return="users" kind=symbol target=parse instance="parse<\"users\">"
/// @generic.instance source="parse(\"id:users\")" id="parse<\"users\">"

segment satisfies "users";
/// @resolution.name source=segment target=segment
/// @generic.instance id="parse<\"users\">" template=parse arguments=("users")
"#,
    );
}

#[test]
fn test_template_literal_calls_build_from_captured_span() {
    let session = TestSession::single(
        r#"
declare function build<T: string>(value: T): `id:${T}`;

const key = build("users");

key satisfies "id:users";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function build<T: string>(value: T): `id:${T}`;

const key: `id:${"users"}` = build<"users">("users");

key satisfies "id:users";

=== checked ===
declare function build<T: string>(value: T): `id:${T}`;
/// @generic.template symbol=build parameters=(T: string)
/// @type.symbol symbol=build type=<T: string>(T) => `id:${T}`
/// @type.symbol symbol=value type=T

const key = build("users");
/// @type.symbol symbol=key type=`id:${"users"}`
/// @resolution.name source=build target=build
/// @resolution.call source="build(\"users\")" parameters=("users") return=`id:${"users"}` kind=symbol target=build instance="build<\"users\">"
/// @generic.instance source="build(\"users\")" id="build<\"users\">"

key satisfies "id:users";
/// @resolution.name source=key target=key
/// @generic.instance id="build<\"users\">" template=build arguments=("users")
"#,
    );
}

#[test]
fn test_generic_template_literal_calls_accept_widened_string_inputs() {
    let session = TestSession::single(
        r#"
declare function identity<T: string>(value: `${T}`): T;

let value = "users";

const text = identity(value);

text satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function identity<T: string>(value: `${T}`): T;

let value: string = "users";

const text: string = identity<string>(value);

text satisfies string;

=== checked ===
declare function identity<T: string>(value: `${T}`): T;
/// @generic.template symbol=identity parameters=(T: string)
/// @type.symbol symbol=identity type=<T: string>(`${T}`) => T
/// @type.symbol symbol=value type=`${T}`

let value = "users";
/// @type.symbol symbol=value source=value type=string

const text = identity(value);
/// @type.symbol symbol=text type=string
/// @resolution.name source=identity target=identity
/// @resolution.name source=value target=value
/// @resolution.call source=identity(value) parameters=(`${string}`) return=string kind=symbol target=identity instance=identity<string>
/// @generic.instance source=identity(value) id=identity<string>

text satisfies string;
/// @resolution.name source=text target=text
/// @generic.instance id=identity<string> template=identity arguments=(string)
"#,
    );
}

#[test]
fn test_prefixed_template_literal_calls_reject_widened_string_inputs() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${T}`): T;

let key = "id:users";

parse(key);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

let key: string = "id:users";

parse(key);

=== checked ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=value type=`id:${T}`

let key = "id:users";
/// @type.symbol symbol=key source=key type=string

parse(key);
/// @resolution.name source=parse target=parse
/// @resolution.name source=key target=key
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'string' is not assignable to type '`id:${string}`'"
/// @diagnostic.label line=6 column=1 source="parse(key);"
"#,
    );
}

#[test]
fn test_template_literal_calls_infer_empty_span_at_literal_boundary() {
    let session = TestSession::single(
        r#"
declare function parse<T: string>(value: `id:${T}`): T;

const segment = parse("id:");

segment satisfies "";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: string>(value: `id:${T}`): T;

const segment: "" = parse<"">("id:");

segment satisfies "";

=== checked ===
declare function parse<T: string>(value: `id:${T}`): T;
/// @generic.template symbol=parse parameters=(T: string)
/// @type.symbol symbol=parse type=<T: string>(`id:${T}`) => T
/// @type.symbol symbol=value type=`id:${T}`

const segment = parse("id:");
/// @type.symbol symbol=segment type=""
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"id:\")" parameters=(`id:${""}`) return="" kind=symbol target=parse instance="parse<\"\">"
/// @generic.instance source="parse(\"id:\")" id="parse<\"\">"

segment satisfies "";
/// @resolution.name source=segment target=segment
/// @generic.instance id="parse<\"\">" template=parse arguments=("")
"#,
    );
}

#[test]
fn test_template_literal_calls_infer_constrained_numeric_span() {
    let session = TestSession::single(
        r#"
declare function parse<T: number>(value: `${T}`): T;

const value = parse("42");

value satisfies 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: number>(value: `${T}`): T;

const value: 42 = parse<42>("42");

value satisfies 42;

=== checked ===
declare function parse<T: number>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=(T: number)
/// @type.symbol symbol=parse type=<T: number>(`${T}`) => T
/// @type.symbol symbol=value type=`${T}`

const value = parse("42");
/// @type.symbol symbol=value type=42
/// @resolution.name source=parse target=parse
/// @resolution.call source="parse(\"42\")" parameters=(`${42}`) return=42 kind=symbol target=parse instance=parse<42>
/// @generic.instance source="parse(\"42\")" id=parse<42>

value satisfies 42;
/// @resolution.name source=value target=value
/// @generic.instance id=parse<42> template=parse arguments=(42)
"#,
    );
}

#[test]
fn test_template_literal_calls_reject_invalid_numeric_span() {
    let session = TestSession::single(
        r#"
declare function parse<T: number>(value: `${T}`): T;

parse("no");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse<T: number>(value: `${T}`): T;

parse("no");

=== checked ===
declare function parse<T: number>(value: `${T}`): T;
/// @generic.template symbol=parse parameters=(T: number)
/// @type.symbol symbol=parse type=<T: number>(`${T}`) => T
/// @type.symbol symbol=value type=`${T}`

parse("no");
/// @resolution.name source=parse target=parse
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"no\"' is not assignable to type '`${number}`'"
/// @diagnostic.label line=4 column=1 source="parse(\"no\");"
"#,
    );
}
