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
const ok: Keys = key;
const bad: Keys = "name";

=== checked ===
type Bag = { readonly [key: symbol]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: symbol]: int32 }" type={ readonly [key: symbol]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: symbol]: int32 }" value={ readonly [key: symbol]: int32 }

type Keys = keyof Bag;
/// @type.symbol symbol=Keys source="type Keys = keyof Bag" type=keyof Bag reduced=symbol
/// @definition.type symbol=Keys source="type Keys = keyof Bag" value=keyof Bag reduced=symbol
/// @resolution.name source=Bag target=Bag

declare const key: symbol;
/// @type.symbol symbol=key source=key type=symbol

const ok: Keys = key;
/// @type.symbol symbol=ok source=ok type=Keys reduced=symbol
/// @resolution.name source=Keys target=Keys
/// @resolution.name source=key target=key

const bad: Keys = "name";
/// @type.symbol symbol=bad source=bad type=Keys reduced=symbol
/// @resolution.name source=Keys target=Keys
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"name\"' is not assignable to type 'Keys'"
/// @diagnostic.label line=7 column=19 span="\"name\"" line_source="const bad: Keys = \"name\";"
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

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }

declare const box: TokenBox;
/// @type.symbol symbol=box source=box type=TokenBox reduced={ readonly [token]: int32 }
/// @resolution.name source=TokenBox target=TokenBox

const value = box[token];
/// @type.symbol symbol=value source=value type=int32
/// @resolution.name source=box target=box
/// @resolution.member source=box[token] receiver={ readonly [token]: int32 } kind=field key=token
/// @resolution.name source=token target=token

value satisfies int32;
/// @resolution.name source=value target=value
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

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }

type Keys = keyof TokenBox;
/// @type.symbol symbol=Keys source="type Keys = keyof TokenBox" type=keyof TokenBox reduced=token
/// @definition.type symbol=Keys source="type Keys = keyof TokenBox" value=keyof TokenBox reduced=token
/// @resolution.name source=TokenBox target=TokenBox

const ok: Keys = token;
/// @type.symbol symbol=ok source=ok type=Keys reduced=token
/// @resolution.name source=Keys target=Keys
/// @resolution.name source=token target=token
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

declare const other: unique symbol;
/// @type.symbol symbol=other source=other type=unique symbol

type TokenBox = { readonly [token]: int32 };
/// @type.symbol symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" type={ readonly [token]: int32 }
/// @definition.type symbol=TokenBox source="type TokenBox = { readonly [token]: int32 }" value={ readonly [token]: int32 }

type Keys = keyof TokenBox;
/// @type.symbol symbol=Keys source="type Keys = keyof TokenBox" type=keyof TokenBox reduced=token
/// @definition.type symbol=Keys source="type Keys = keyof TokenBox" value=keyof TokenBox reduced=token
/// @resolution.name source=TokenBox target=TokenBox

const bad: Keys = other;
/// @type.symbol symbol=bad source=bad type=Keys reduced=token
/// @resolution.name source=Keys target=Keys
/// @resolution.name source=other target=other
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'other' is not assignable to type 'Keys'"
/// @diagnostic.label line=8 column=19 span="other" line_source="const bad: Keys = other;"
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
/// @type.symbol symbol=box source=box type=RegistryBox reduced={ readonly [Symbol.for("token")]: string }
/// @resolution.name source=RegistryBox target=RegistryBox
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=Symbol kind=symbol target=types.symbol.Symbol.for
/// @resolution.call source="Symbol.for(\"token\")" parameters=(string) arguments=(provided("token") as string) return=Symbol.for("token") kind=symbol target=types.symbol.Symbol.for receiver=Symbol

const value = box[Symbol.for("token")];
/// @type.symbol symbol=value source=value type=string
/// @resolution.name source=box target=box
/// @resolution.member source="box[Symbol.for(\"token\")]" receiver={ readonly [Symbol.for("token")]: string } kind=field key="Symbol.for(\"token\")"
/// @resolution.name source=Symbol target=types.symbol.Symbol
/// @resolution.member source=Symbol.for receiver=Symbol kind=symbol target=types.symbol.Symbol.for
/// @resolution.call source="Symbol.for(\"token\")" parameters=(string) arguments=(provided("token") as string) return=Symbol.for("token") kind=symbol target=types.symbol.Symbol.for receiver=Symbol

value satisfies string;
/// @resolution.name source=value target=value
"#,
    );
}
