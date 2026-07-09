use crate::tests::{DirRows, TestSession};

#[test]
fn test_call_selects_first_compatible_overload_in_declaration_order() {
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
/// @type.node source="parse(\"id\")" type="string"
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.call source="parse(\"id\")" parameters=(string) arguments=(provided("id") as string) return="string" kind=symbol target=parse#1
/// @type.node source="\"id\"" type="id"
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
/// @diagnostic.error code=EC302 message="no overload matches arguments ('true')"
/// @diagnostic.label line=5 column=1 span="parse(true)" line_source="parse(true);"
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
/// @diagnostic.error code=EC611 message="declaration 'parse' requires a body"
/// @diagnostic.label line=2 column=10 span="parse" line_source="function parse(value: string): int32;"
"#,
    );
}

#[test]
fn test_intrinsic_declaration_carries_its_implementation() {
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
/// @definition.method symbol=capacity source="get capacity(): usize" slot=capacity role=getter type=(this: Buffer) => usize
/// @definition.method symbol=trailing source="trailing(): usize" slot=trailing type=(this: Buffer) => usize
/// @resolution.name source=Buffer target=Buffer

    @intrinsic("buffer.capacity")
    /// @resolution.name source=intrinsic target=decorator.intrinsic.intrinsic

    get capacity(): usize;
    /// @type.symbol symbol=capacity source="get capacity(): usize" type=(this: Buffer) => usize

    trailing(): usize;
    /// @type.symbol symbol=trailing source="trailing(): usize" type=(this: Buffer) => usize

}
"#,
        r#"
/// @diagnostic.error code=EC611 message="declaration 'trailing' requires a body"
/// @diagnostic.label line=10 column=5 span="trailing" line_source="trailing(): usize;"
"#,
    );
}
