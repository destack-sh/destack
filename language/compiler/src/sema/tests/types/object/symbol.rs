use crate::tests::{DirRows, TestSession};

#[test]
fn test_symbol_index_signature_keys_are_symbols() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: symbol]: int32 };
type Keys = keyof Bag;

declare const key: symbol;
const ok: Keys = key;
const bad: Keys = "name";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: symbol]: int32 };
type Keys = keyof Bag;

declare const key: symbol;
const ok: symbol = key;
const bad: symbol = "name";

=== checked ===
type Bag = { readonly [key: symbol]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: symbol]: int32 }" type={ readonly [key: symbol]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: symbol]: int32 }" value={ readonly [key: symbol]: int32 }

type Keys = keyof Bag;
/// @type.symbol symbol=Keys source="type Keys = keyof Bag" type=symbol
/// @definition.type symbol=Keys source="type Keys = keyof Bag" value=symbol
/// @resolution.name source=Bag target=Bag

declare const key: symbol;
/// @type.symbol symbol=key source=key type=symbol
/// @resolution.pattern source=key kind=binding target=key

const ok: Keys = key;
/// @type.symbol symbol=ok source=ok type=symbol
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Keys target=Keys
/// @resolution.name source=key target=key
/// @resolution.place source=key placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=key root=key

const bad: Keys = "name";
/// @type.symbol symbol=bad source=bad type=symbol
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Keys target=Keys
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"name\"' is not assignable to type 'symbol'"
/// @diagnostic.label line=7 column=19 span="\"name\"" line_source="const bad: Keys = \"name\";"
/// @diagnostic.related line=7 column=12 span="Keys" line_source="const bad: Keys = \"name\";" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_unique_symbol_key_indexes_object() {
    let session = TestSession::single(
        r#"
declare const token: unique symbol;

type TokenBox = { readonly [token]: int32 };

declare const box: TokenBox;
const value = box[token];

value satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const token: unique symbol;

type TokenBox = { readonly [token]: int32 };

declare const box: TokenBox;
const value: int32 = box[token];

value satisfies int32;

=== checked ===
declare const token: unique symbol;
/// @type.symbol symbol=token source=token type=unique symbol
/// @resolution.pattern source=token kind=binding target=token

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }

declare const box: TokenBox;
/// @type.symbol symbol=box source=box type={ readonly [token]: int32 }
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.name source=TokenBox target=TokenBox

const value = box[token];
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=box target=box
/// @resolution.place source=box placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=box root=box
/// @resolution.subscript source=box[token] type=int32 kind=member target="receiver={ readonly [token]: int32 }, target=field(receiver={ readonly [token]: int32 }, target=token, type=int32), type=int32"
/// @resolution.name source=token target=token
/// @resolution.place source=token placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=token root=token

value satisfies int32;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_keyof_preserves_unique_symbol_key() {
    let session = TestSession::single(
        r#"
declare const token: unique symbol;

type TokenBox = { readonly [token]: int32 };
type Keys = keyof TokenBox;

const ok: Keys = token;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const token: unique symbol;

type TokenBox = { readonly [token]: int32 };
type Keys = keyof TokenBox;

const ok: Keys = token;

=== checked ===
declare const token: unique symbol;
/// @type.symbol symbol=token source=token type=unique symbol
/// @resolution.pattern source=token kind=binding target=token

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }

type Keys = keyof TokenBox;
/// @type.symbol symbol=Keys source="type Keys = keyof TokenBox" type=token
/// @definition.type symbol=Keys source="type Keys = keyof TokenBox" value=token
/// @resolution.name source=TokenBox target=TokenBox

const ok: Keys = token;
/// @type.symbol symbol=ok source=ok type=token
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Keys target=Keys
/// @resolution.name source=token target=token
/// @resolution.place source=token placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=token root=token
"#,
    );
}

#[test]
fn test_keyof_rejects_unrelated_unique_symbol_key() {
    let session = TestSession::single(
        r#"
declare const token: unique symbol;
declare const other: unique symbol;

type TokenBox = { readonly [token]: int32 };
type Keys = keyof TokenBox;

const bad: Keys = other;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const token: unique symbol;
declare const other: unique symbol;

type TokenBox = { readonly [token]: int32 };
type Keys = keyof TokenBox;

const bad: Keys = other;

=== checked ===
declare const token: unique symbol;
/// @type.symbol symbol=token source=token type=unique symbol
/// @resolution.pattern source=token kind=binding target=token

declare const other: unique symbol;
/// @type.symbol symbol=other source=other type=unique symbol
/// @resolution.pattern source=other kind=binding target=other

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }

type Keys = keyof TokenBox;
/// @type.symbol symbol=Keys source="type Keys = keyof TokenBox" type=token
/// @definition.type symbol=Keys source="type Keys = keyof TokenBox" value=token
/// @resolution.name source=TokenBox target=TokenBox

const bad: Keys = other;
/// @type.symbol symbol=bad source=bad type=token
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Keys target=Keys
/// @resolution.name source=other target=other
/// @resolution.place source=other placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=other root=other
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'other' is not assignable to type 'token'"
/// @diagnostic.label line=8 column=19 span="other" line_source="const bad: Keys = other;"
/// @diagnostic.related line=8 column=12 span="Keys" line_source="const bad: Keys = other;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_registry_symbol_key_indexes_object() {
    let session = TestSession::single(
        r#"
type RegistryBox = { readonly [Symbol.for("token")]: string };

const box: RegistryBox = { [Symbol.for("token")]: "ok" };
const value = box[Symbol.for("token")];

value satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type RegistryBox = { readonly [Symbol.for("token")]: string };

const box: RegistryBox = { [Symbol.for("token")]: "ok" };
const value: string = box[Symbol.for("token")];

value satisfies string;

=== checked ===
type RegistryBox = { readonly [Symbol.for("token")]: string };
/// @type.symbol symbol=RegistryBox source="type RegistryBox = { readonly [Symbol.for(\"token\")]: string }" type={ readonly [Symbol.for("token")]: string }
/// @definition.type symbol=RegistryBox source="type RegistryBox = { readonly [Symbol.for(\"token\")]: string }" value={ readonly [Symbol.for("token")]: string }

const box: RegistryBox = { [Symbol.for("token")]: "ok" };
/// @type.symbol symbol=box source=box type={ readonly [Symbol.for("token")]: string }
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.name source=RegistryBox target=RegistryBox
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=Symbol type=(string) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.for
/// @resolution.call source="Symbol.for(\"token\")" parameters=(string) arguments=(provided("token") as string) return=Symbol.for("token") kind=symbol target=types.symbol.Symbol.for

const value = box[Symbol.for("token")];
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=box target=box
/// @resolution.subscript source="box[Symbol.for(\"token\")]" type=string kind=member target="receiver={ readonly [Symbol.for(\"token\")]: string }, target=field(receiver={ readonly [Symbol.for(\"token\")]: string }, target=Symbol.for(\"token\"), type=string), type=string"
/// @resolution.place source=box placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=box root=box
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=Symbol type=(string) => symbol kind=symbol target_receiver=Symbol target=types.symbol.Symbol.for
/// @resolution.call source="Symbol.for(\"token\")" parameters=(string) arguments=(provided("token") as string) return=Symbol.for("token") kind=symbol target=types.symbol.Symbol.for

value satisfies string;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}
