use crate::tests::{DirRows, TestSession};

#[test]
fn test_static_value_argument_specializes_array_length() {
    let session = TestSession::single(
        r#"
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

const bytes = take<4>([1, 2, 3, 4]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

const bytes: [uint8; 4] = take<4>([1, 2, 3, 4]);

=== checked ===
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
/// @generic.template symbol=take parameters=(comptime N: uint)
/// @type.symbol symbol=value type=[uint8; N]
/// @resolution.name source=N target=N
/// @resolution.name source=N target=N

    return value;
    /// @resolution.name source=value target=value
    /// @type.node source=value type=[uint8; N]

}

const bytes = take<4>([1, 2, 3, 4]);
/// @type.symbol symbol=bytes type=[uint8; 4]
/// @resolution.name source=take target=take
/// @resolution.call source="take<4>([1, 2, 3, 4])" parameters=([uint8; 4]) return=[uint8; 4] kind=symbol target=take instance=take<4>
/// @generic.instance source="take<4>([1, 2, 3, 4])" id=take<4>
/// @type.node source="take<4>([1, 2, 3, 4])" type=[uint8; 4]
/// @type.node source=[1, 2, 3, 4] type=[uint8; 4]
/// @type.node source=1 type=uint8
/// @type.node source=2 type=uint8
/// @type.node source=3 type=uint8
/// @type.node source=4 type=uint8
/// @generic.instance id=take<4> template=take arguments=(4)
"#);
}

#[test]
fn test_defaulted_static_value_argument_uses_literal_default() {
    let session = TestSession::single(
        r#"
function choose<comptime Flag: boolean = true>(value: int32): int32 {
    return value;
}

const value = choose(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function choose<comptime Flag: boolean = true>(value: int32): int32 {
    return value;
}

const value: int32 = choose<true>(1);

=== checked ===
function choose<comptime Flag: boolean = true>(value: int32): int32 {
/// @generic.template source=declaration parameters=(comptime Flag: boolean = true)
/// @type.symbol symbol=choose type=<Flag: boolean = true>(int32) => int32
/// @type.node source=true type=true
/// @type.symbol symbol=value#1 source="value: int32" type=int32

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value#1

}

const value = choose(1);
/// @type.symbol symbol=value#2 source=value type=int32
/// @generic.instance source=choose(1) id=choose<true>
/// @type.node source=choose type=(int32) => int32
/// @type.node source=choose(1) type=int32
/// @resolution.name source=choose target=choose
/// @resolution.call source=choose(1) parameters=(int32) return=int32 kind=symbol target=choose instance=choose<true>
/// @type.node source=1 type=int32

/// @generic.instance id=choose<true> template=choose arguments=(true)
"#);
}

#[test]
fn test_function_type_comptime_generic_binds_return_type() {
    let session = TestSession::single(
        r#"
type Read = <comptime N: uint>() => [uint8; N];

declare const read: Read;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
type Read = <comptime N: uint>() => [uint8; N];

declare const read: Read;

=== checked ===
type Read = <comptime N: uint>() => [uint8; N];
/// @type.symbol symbol=Read source="type Read = <comptime N: uint>() => [uint8; N]" type=<N: uint>() => FixedArray<uint8, N>
/// @definition.type symbol=Read source="type Read = <comptime N: uint>() => [uint8; N]" value=<N: uint>() => FixedArray<uint8, N>
/// @generic.template source=type_expression parameters=(comptime N: uint)
/// @type.symbol symbol=N source="comptime N: uint" type=N
/// @resolution.name source=N target=N

declare const read: Read;
/// @type.symbol symbol=read source=read type=Read
/// @resolution.name source=Read target=Read
"#,
    );
}

#[test]
fn test_static_literal_arguments_specialize_aliases() {
    let session = TestSession::single(
        r#"
type Tagged<comptime Tag: string> = { tag: Tag };
type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;

declare const tagged: Tagged<"alpha">;
declare const flagged: Flagged<{ name: "search"; enabled: true }>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Tagged<comptime Tag: string> = { tag: Tag };
type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;

declare const tagged: Tagged<"alpha">;
declare const flagged: Flagged<{ name: "search"; enabled: true }>;

=== checked ===
type Tagged<comptime Tag: string> = { tag: Tag };
/// @generic.template source=declaration parameters=(comptime Tag: string)
/// @type.symbol symbol=Tagged source="type Tagged<comptime Tag: string> = { tag: Tag }" type={ tag: Tag }
/// @definition.type symbol=Tagged source="type Tagged<comptime Tag: string> = { tag: Tag }" template=LocalGenericTemplateId(0) value={ tag: Tag }
/// @type.symbol symbol=Tagged.tag source="tag: Tag" type=Tag
/// @resolution.name source=Tag target=Tagged.Tag

type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;
/// @generic.template source=declaration parameters=(comptime Config: { name: string; enabled: boolean })
/// @type.symbol symbol=Flagged source="type Flagged<comptime Config: { name: string; enabled: boolean }> = Config" type=Config
/// @definition.type symbol=Flagged source="type Flagged<comptime Config: { name: string; enabled: boolean }> = Config" template=LocalGenericTemplateId(1) value=Config
/// @type.symbol symbol=Flagged.name source="name: string" type=string
/// @type.symbol symbol=Flagged.enabled source="enabled: boolean" type=boolean
/// @resolution.name source=Config target=Flagged.Config

declare const tagged: Tagged<"alpha">;
/// @type.symbol symbol=tagged source=tagged type=Tagged<"alpha">
/// @generic.instance source="Tagged<\"alpha\">" id="Tagged<\"alpha\">"
/// @resolution.name source=Tagged target=Tagged

declare const flagged: Flagged<{ name: "search"; enabled: true }>;
/// @type.symbol symbol=flagged source=flagged type=Flagged<{ name: "search"; enabled: true }>
/// @generic.instance source="Flagged<{ name: \"search\"; enabled: true }>" id="Flagged<{ name: \"search\"; enabled: true }>"
/// @resolution.name source=Flagged target=Flagged

/// @generic.instance id="Flagged<{ name: \"search\"; enabled: true }>" template=Flagged arguments=({ name: "search"; enabled: true })
/// @generic.instance id="Tagged<\"alpha\">" template=Tagged arguments=("alpha")
"#,
    );
}
