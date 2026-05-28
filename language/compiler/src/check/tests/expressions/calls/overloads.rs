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
        DirRows::checked().with_node_types().without_reference_types(),
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
/// @resolution.call source="parse(\"id\")" parameters=(string) return="string" kind=symbol target=parse#1
/// @type.node source="\"id\"" type=string
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
declare function parse(value: string): int32;
/// @type.symbol symbol=parse#1 type=(string) => int32
/// @type.symbol symbol=value#1 type=string

declare function parse(value: int32): int32;
/// @type.symbol symbol=parse#2 type=(int32) => int32
/// @type.symbol symbol=value#2 type=int32

parse(true);
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @type.node source=true type=true

"#,
        r#"
/// @diagnostic.error code=EC302 message="no matching call overload"
/// @diagnostic.label line=5 column=1 source="parse(true);"
"#,
    );
}
