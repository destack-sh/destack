use crate::tests::{DirRows, TestSession};

#[test]
fn test_symbol_constructor_has_symbol_type() {
    let session = TestSession::single(
        r#"
const value: symbol = Symbol("id");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: symbol = Symbol("id");
/// @type.symbol symbol=value type=symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.call source="Symbol(\"id\")" parameters=(string) return=symbol kind=symbol target=types.symbol.Symbol
/// @type.node source="\"id\"" type=string
/// @type.node source="Symbol(\"id\")" type=symbol
"#,
    );
}

#[test]
fn test_symbol_constructor_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = Symbol("id");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: string = Symbol("id");
/// @type.symbol symbol=value type=string
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.call source="Symbol(\"id\")" parameters=(string) return=symbol kind=symbol target=types.symbol.Symbol
/// @type.node source="\"id\"" type=string
/// @type.node source="Symbol(\"id\")" type=symbol

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: string = Symbol(\"id\");"
"#,
    );
}
