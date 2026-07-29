use crate::tests::{DirRows, TestSession};

#[test]
fn test_call_selects_first_applicable_overload_in_declaration_order() {
    let session = TestSession::single(
        r#"
function parse(value: string): "string" {
    return "string";
}

function parse(value: "id"): "literal" {
    return "literal";
}

const result = parse("id");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
function parse(value: string): "string" {
    return "string";
}

function parse(value: "id"): "literal" {
    return "literal";
}

const result: "string" = parse("id");

=== checked ===
function parse(value: string): "string" {
/// @type.symbol symbol=parse#1 type=(string) => "string"
/// @type.symbol symbol=parse.value#1 source="value: string" type=string

    return "string";
    /// @type.node source="\"string\"" type="string"

}

function parse(value: "id"): "literal" {
/// @type.symbol symbol=parse#2 type=("id") => "literal"
/// @type.symbol symbol=parse.value#2 source="value: \"id\"" type="id"

    return "literal";
    /// @type.node source="\"literal\"" type="literal"

}

const result = parse("id");
/// @type.symbol symbol=result source=result type="string"
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="parse(\"id\")" type="string"
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.call source="parse(\"id\")" parameters=(string) arguments=(provided("id") as string) return="string" kind=symbol target=parse#1
/// @type.node source="\"id\"" type="id"
"#,
    );
}

#[test]
fn test_call_selects_first_applicable_overload_ignoring_expected_return() {
    let session = TestSession::single(
        r#"
function choose(value: string): string {
    return "text";
}

function choose(value: string): int32 {
    return 0;
}

const result: int32 = choose("x");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
function choose(value: string): string {
    return "text";
}

function choose(value: string): int32 {
    return 0;
}

const result: int32 = choose("x");

=== checked ===
function choose(value: string): string {
/// @type.symbol symbol=choose#1 type=(string) => string
/// @type.symbol symbol=choose.value#1 source="value: string" type=string

    return "text";
    /// @type.node source="\"text\"" type="text"

}

function choose(value: string): int32 {
/// @type.symbol symbol=choose#2 type=(string) => int32
/// @type.symbol symbol=choose.value#2 source="value: string" type=string

    return 0;
    /// @type.node source=0 type=0

}

const result: int32 = choose("x");
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="choose(\"x\")" type=string
/// @resolution.name source=choose target=[choose#1, choose#2]
/// @resolution.call source="choose(\"x\")" parameters=(string) arguments=(provided("x") as string) return=string kind=symbol target=choose#1
/// @type.node source="\"x\"" type="x"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'string' is not assignable to type 'int32'"
/// @diagnostic.label line=10 column=23 span="choose(\"x\")" line_source="const result: int32 = choose(\"x\");"
/// @diagnostic.related line=10 column=15 span="int32" line_source="const result: int32 = choose(\"x\");" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_call_with_no_matching_overload_reports_error() {
    let session = TestSession::single(
        r#"
declare function parse(value: string): int32;
declare function parse(value: int32): int32;

parse(true);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
declare function parse(value: string): int32;
declare function parse(value: int32): int32;

parse(true);

=== checked ===
declare function parse(value: string): int32;
/// @type.symbol symbol=parse#1 source="declare function parse(value: string): int32" type=(string) => int32
/// @type.symbol symbol=parse.value#1 source="value: string" type=string

declare function parse(value: int32): int32;
/// @type.symbol symbol=parse#2 source="declare function parse(value: int32): int32" type=(int32) => int32
/// @type.symbol symbol=parse.value#2 source="value: int32" type=int32

parse(true);
/// @type.node source=parse(true) type=<error>
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments ('true')"
/// @diagnostic.label line=5 column=1 span="parse(true)" line_source="parse(true);"
/// @diagnostic.note message="the candidate '(string) => int32' rejects argument 0: 'true' is not assignable to 'string'"
/// @diagnostic.note message="the candidate '(int32) => int32' rejects argument 0: 'true' is not assignable to 'int32'"
"#,
    );
}

#[test]
fn test_concrete_function_declaration_requires_body() {
    let session = TestSession::single(
        r#"
function parse(value: string): int32;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().without_reference_types(),
        r#"
=== annotated ===
function parse(value: string): int32;

=== checked ===
function parse(value: string): int32;
/// @type.symbol symbol=parse source="function parse(value: string): int32" type=(string) => int32
/// @type.symbol symbol=parse.value source="value: string" type=string
"#,
        r#"
/// @diagnostic.error id=missing-declaration-body message="declaration 'parse' requires a body"
/// @diagnostic.label line=2 column=10 span="parse" line_source="function parse(value: string): int32;"
"#,
    );
}

#[test]
fn test_require_bodies_on_extension_members_despite_decorators() {
    let session = TestSession::single(
        r#"
struct Buffer {
    length: usize;
}

extension of Buffer {
    @intrinsic("buffer.capacity")
    get capacity(): usize;

    trailing(): usize;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Buffer {
    length: usize;
}

extension of Buffer {
    @intrinsic("buffer.capacity")
    get capacity(): usize;

    trailing(): usize;
}

=== checked ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.length source="length: usize" key=length type=usize

    length: usize;
    /// @type.symbol symbol=Buffer.length source="length: usize" type=usize

}

extension of Buffer {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.method symbol=capacity source="get capacity(): usize" slot=capacity role=getter type=<capacity.'a>(this: &capacity.'a readonly this) => usize
/// @definition.method symbol=trailing source="trailing(): usize" slot=trailing type=<trailing.'a>(this: &trailing.'a exclusive this) => usize
/// @resolution.name source=Buffer target=Buffer

    @intrinsic("buffer.capacity")
    /// @resolution.name source=intrinsic target=decorator.intrinsic.intrinsic

    get capacity(): usize;
    /// @generic.template symbol=capacity parameters=('a)
    /// @type.symbol symbol=capacity source="get capacity(): usize" type=<capacity.'a>(this: &capacity.'a readonly this) => usize

    trailing(): usize;
    /// @generic.template symbol=trailing parameters=('a)
    /// @type.symbol symbol=trailing source="trailing(): usize" type=<trailing.'a>(this: &trailing.'a exclusive this) => usize

}
"#,
        r#"
/// @diagnostic.error id=missing-declaration-body message="declaration 'get' requires a body"
/// @diagnostic.label line=8 column=9 span="capacity" line_source="get capacity(): usize;"
/// @diagnostic.error id=missing-declaration-body message="declaration 'trailing' requires a body"
/// @diagnostic.label line=10 column=5 span="trailing" line_source="trailing(): usize;"
"#,
    );
}
