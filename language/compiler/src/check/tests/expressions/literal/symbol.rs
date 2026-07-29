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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: symbol = Symbol.create("id" as string | float64 | undefined);

=== checked ===
const value: symbol = Symbol.create("id");
/// @type.symbol symbol=value source=value type=symbol
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=Symbol
/// @type.node source=Symbol.create type=(string | float64 | undefined?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=Symbol type=(string | float64 | undefined?) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64 | undefined) arguments=(provided("id") as string | float64 | undefined) return=symbol kind=symbol target=types.symbol.Symbol.create
/// @type.node source="\"id\"" type="id"

/// @check.stats.solve variables=0 types=11 constraints=0 obligations=1 solutions=0 bounds=0 decisions=4
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: string = Symbol.create("id" as string | float64 | undefined);

=== checked ===
const value: string = Symbol.create("id");
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=Symbol
/// @type.node source=Symbol.create type=(string | float64 | undefined?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=Symbol type=(string | float64 | undefined?) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64 | undefined) arguments=(provided("id") as string | float64 | undefined) return=symbol kind=symbol target=types.symbol.Symbol.create
/// @type.node source="\"id\"" type="id"

/// @check.stats.solve variables=0 types=11 constraints=0 obligations=1 solutions=0 bounds=0 decisions=4
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'symbol' is not assignable to type 'string'"
/// @diagnostic.label line=2 column=23 span="Symbol.create(\"id\")" line_source="const value: string = Symbol.create(\"id\");"
/// @diagnostic.related line=2 column=14 span="string" line_source="const value: string = Symbol.create(\"id\");" message="expected due to this annotation"
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: symbol | string = Symbol.create("id" as string | float64 | undefined) as | symbol
| string;

=== checked ===
const value: symbol | string = Symbol.create("id");
/// @type.symbol symbol=value source=value type=symbol | string
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=Symbol
/// @type.node source=Symbol.create type=(string | float64 | undefined?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=Symbol type=(string | float64 | undefined?) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64 | undefined) arguments=(provided("id") as string | float64 | undefined) return=symbol kind=symbol target=types.symbol.Symbol.create
/// @type.node source="\"id\"" type="id"

/// @check.stats.solve variables=0 types=12 constraints=0 obligations=1 solutions=0 bounds=0 decisions=4
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: float64 = Symbol.create("id" as string | float64 | undefined);

=== checked ===
const value: number = Symbol.create("id");
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="Symbol.create(\"id\")" type=symbol
/// @type.node source=Symbol type=Symbol
/// @type.node source=Symbol.create type=(string | float64 | undefined?) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.create receiver=Symbol type=(string | float64 | undefined?) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.create
/// @resolution.call source="Symbol.create(\"id\")" parameters=(string | float64 | undefined) arguments=(provided("id") as string | float64 | undefined) return=symbol kind=symbol target=types.symbol.Symbol.create
/// @type.node source="\"id\"" type="id"

/// @check.stats.solve variables=0 types=11 constraints=0 obligations=1 solutions=0 bounds=0 decisions=4
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'symbol' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 span="Symbol.create(\"id\")" line_source="const value: number = Symbol.create(\"id\");"
/// @diagnostic.related line=2 column=14 span="number" line_source="const value: number = Symbol.create(\"id\");" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_symbol_for_literal_returns_registry_symbol() {
    let session = TestSession::single(
        r#"
const key = Symbol.for("token");
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
const key = Symbol.for("token");

=== checked ===
const key = Symbol.for("token");
/// @type.symbol symbol=key source=key type=Symbol.for("token")
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source="Symbol.for(\"token\")" type=Symbol.for("token")
/// @type.node source=Symbol type=Symbol
/// @type.node source=Symbol.for type=(string) => symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=Symbol type=(string) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.for
/// @resolution.call source="Symbol.for(\"token\")" parameters=(string) arguments=(provided("token") as string) return=Symbol.for("token") kind=symbol target=types.symbol.Symbol.for
/// @type.node source="\"token\"" type="token"
"#);
}

#[test]
fn test_symbol_for_dynamic_string_returns_symbol() {
    let session = TestSession::single(
        r#"
declare const name: string;
const key = Symbol.for(name);
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
declare const name: string;
const key: symbol = Symbol.for(name);

=== checked ===
declare const name: string;
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source=name kind=binding target=name

const key = Symbol.for(name);
/// @type.symbol symbol=key source=key type=symbol
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=Symbol type=Symbol
/// @type.node source=Symbol.for type=(string) => symbol
/// @type.node source=Symbol.for(name) type=symbol
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=Symbol type=(string) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.for
/// @resolution.call source=Symbol.for(name) parameters=(string) arguments=(provided(name) as string) return=symbol kind=symbol target=types.symbol.Symbol.for
/// @type.node source=name type=string
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name
"#);
}
