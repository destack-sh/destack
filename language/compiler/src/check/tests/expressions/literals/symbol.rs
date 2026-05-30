use crate::tests::{DirRows, TestSession};

#[test]
fn test_symbol_create_has_symbol_type() {
    let session = TestSession::single(
        r#"
const value: symbol = Symbol.create("id");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: symbol = Symbol.create("id");
/// @type.symbol symbol=value type=symbol
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=types.symbol.Symbol
/// @type.node source=Symbol.create type=(string | float64?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=types.symbol.Symbol kind=symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64) return=symbol kind=symbol target=types.symbol.Symbol.create receiver=types.symbol.Symbol
/// @type.node source="\"id\"" type="id"
"#,
    );
}

#[test]
fn test_symbol_create_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = Symbol.create("id");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: string = Symbol.create("id");
/// @type.symbol symbol=value type=string
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=types.symbol.Symbol
/// @type.node source=Symbol.create type=(string | float64?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=types.symbol.Symbol kind=symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64) return=symbol kind=symbol target=types.symbol.Symbol.create receiver=types.symbol.Symbol
/// @type.node source="\"id\"" type="id"

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: string = Symbol.create(\"id\");"
"#,
    );
}

#[test]
fn test_symbol_create_flows_into_symbol_union() {
    let session = TestSession::single(
        r#"
const value: symbol | string = Symbol.create("id");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: symbol | string = Symbol.create("id");
/// @type.symbol symbol=value type=symbol | string
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=types.symbol.Symbol
/// @type.node source=Symbol.create type=(string | float64?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=types.symbol.Symbol kind=symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64) return=symbol kind=symbol target=types.symbol.Symbol.create receiver=types.symbol.Symbol
/// @type.node source="\"id\"" type="id"
"#,
    );
}

#[test]
fn test_symbol_create_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = Symbol.create("id");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: number = Symbol.create("id");
/// @type.symbol symbol=value type=float64
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=types.symbol.Symbol
/// @type.node source=Symbol.create type=(string | float64?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=types.symbol.Symbol kind=symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64) return=symbol kind=symbol target=types.symbol.Symbol.create receiver=types.symbol.Symbol
/// @type.node source="\"id\"" type="id"

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: number = Symbol.create(\"id\");"
"#,
    );
}
