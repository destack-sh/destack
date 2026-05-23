use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_free_call_resolution() {
    let session = TestSession::single(
        r#"
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value = add(1, 2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().without_reference_types(),
        r#"
function add(left: int32, right: int32): int32 {
/// @type.symbol symbol=add type=(int32, int32) => int32
/// @type.symbol symbol=left type=int32
/// @type.symbol symbol=right type=int32

    return left + right;
    /// @type.node source="left + right" type=int32
    /// @resolution.name source=left target=left
    /// @resolution.call source="left + right" parameters=[int32, int32] return=int32 kind=builtin builtin=binary.add
    /// @resolution.name source=right target=right

}

const value = add(1, 2);
/// @type.symbol symbol=value type=int32
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=[int32, int32] return=int32 kind=symbol target=add
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
"#);
}

#[test]
fn test_check_records_imported_call_targets() {
    let compiler = TestSession::new()
        .module(
            "math.ds",
            r#"
export function add(left: int32, right: int32): int32 {
    return left + right;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { add } from "./math.ds";

const value = add(1, 2);
"#,
        )
        .build();

    compiler.assert_dir_checked(
        "main.ds",
        DirRows::checked().without_reference_types(),
        r#"
import { add } from "./math.ds";
/// @type.symbol symbol=add type=(int32, int32) => int32

const value = add(1, 2);
/// @type.symbol symbol=value type=int32
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=math.add
/// @resolution.call source="add(1, 2)" parameters=[int32, int32] return=int32 kind=symbol target=math.add
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
"#);
}

#[test]
fn test_check_records_callable_value_resolution() {
    let session = TestSession::single(
        r#"
declare const transform: Function<(int32,), string>;

const text = transform(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().without_reference_types(),
        r#"
declare const transform: Function<(int32,), string>;
/// @type.symbol symbol=transform type=(int32) => string
/// @instance.application source="Function<(int32,), string>" id="types.function.Function<(int32,), string>"

const text = transform(1);
/// @type.symbol symbol=text type=string
/// @type.node source=transform(1) type=string
/// @resolution.name source=transform target=transform
/// @resolution.call source=transform(1) parameters=[int32] return=string kind=symbol target=transform
/// @type.node source=1 type=int32
/// @instance.entry id="types.function.Function<(int32,), string>" symbol=types.function.Function arguments=[(int32,), string]
"#,
    );
}

#[test]
fn test_check_selects_first_compatible_overload_in_declaration_order() {
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
        DirRows::checked().without_reference_types(),
        r#"
function parse(value: string): "string" {
/// @type.symbol symbol=parse#1 type=(string) => "string"
/// @type.symbol symbol=value#1 type=string

    return "string";
    /// @type.node source="\"string\"" type="string"

}

function parse(value: "id"): "literal" {
/// @type.symbol symbol=parse#2 type=("id") => "literal"
/// @type.symbol symbol=value#2 type="id"

    return "literal";
    /// @type.node source="\"literal\"" type="literal"

}

const result = parse("id");
/// @type.symbol symbol=result type="string"
/// @type.node source="parse(\"id\")" type="string"
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.call source="parse(\"id\")" parameters=[string] return="string" kind=symbol target=parse#1
/// @type.node source="\"id\"" type=string
"#,
    );
}

#[test]
fn test_check_reports_no_matching_call_overload() {
    let session = TestSession::single(
        r#"
declare function parse(value: string): int32;
declare function parse(value: int32): int32;

parse(true);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().without_reference_types(),
        r#"
declare function parse(value: string): int32;
/// @type.symbol symbol=parse#1 type=(string) => int32
/// @type.symbol symbol=value#1 type=string

declare function parse(value: int32): int32;
/// @type.symbol symbol=parse#2 type=(int32) => int32
/// @type.symbol symbol=value#2 type=int32

parse(true);
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @type.node source=true type=boolean

"#,
        r#"
/// @diagnostic.error code=EC302 message="no matching call overload"
/// @diagnostic.label line=5 column=1 source="parse(true);"
"#,
    );
}

#[test]
fn test_check_reports_calling_non_callable_values() {
    let session = TestSession::single(
        r#"
const value = 1;
value();
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().without_reference_types(),
        r#"
const value = 1;
/// @type.symbol symbol=value type=1
/// @type.node source=1 type=1

value();
/// @resolution.name source=value target=value

"#,
        r#"
/// @diagnostic.error code=EC301 message="value is not callable"
/// @diagnostic.label line=3 column=1 source="value();"
"#,
    );
}
