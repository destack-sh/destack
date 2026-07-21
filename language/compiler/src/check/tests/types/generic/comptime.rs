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
/// @generic.template symbol=take parameters=(comptime N: uint64)
/// @type.symbol symbol=take type=<comptime N: uint64>(FixedArray<uint8, N>) => FixedArray<uint8, N>
/// @type.symbol symbol=take.N source="comptime N: uint" type=N
/// @type.symbol symbol=take.value source="value: [uint8; N]" type=FixedArray<uint8, N>
/// @resolution.name source=N target=take.N
/// @resolution.name source=N target=take.N

    return value;
    /// @type.node source=value type=FixedArray<uint8, N>
    /// @resolution.name source=value target=take.value

}

const bytes = take<4>([1, 2, 3, 4]);
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @type.node source="take<4>([1, 2, 3, 4])" type=FixedArray<uint8, 4>
/// @type.node source=take type=(FixedArray<uint8, 4>) => FixedArray<uint8, 4>
/// @resolution.name source=take target=take
/// @resolution.call source="take<4>([1, 2, 3, 4])" parameters=(FixedArray<uint8, 4>) arguments=(provided([1, 2, 3, 4]) as FixedArray<uint8, 4>) return=FixedArray<uint8, 4> kind=symbol target=take instance=take<4>
/// @generic.instance source="take<4>([1, 2, 3, 4])" id=take<4>
/// @type.node source=[1, 2, 3, 4] type=FixedArray<uint8, 4>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
/// @type.node source=4 type=4

/// @generic.instance id=take<4> template=take arguments=(4)
"#,
    );
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
/// @generic.template symbol=choose parameters=(comptime Flag: boolean = true)
/// @type.symbol symbol=choose type=<comptime Flag: boolean = true>(int32) => int32
/// @type.symbol symbol=choose.Flag source="comptime Flag: boolean = true" type=Flag
/// @type.node source=true type=true
/// @type.symbol symbol=choose.value source="value: int32" type=int32

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=choose.value

}

const value = choose(1);
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=choose type=(int32) => int32
/// @type.node source=choose(1) type=int32
/// @resolution.name source=choose target=choose
/// @resolution.call source=choose(1) parameters=(int32) arguments=(provided(1) as int32) return=int32 kind=symbol target=choose instance=choose<true>
/// @generic.instance source=choose(1) id=choose<true>
/// @type.node source=1 type=1

/// @generic.instance id=choose<true> template=choose arguments=(true)
"#,
    );
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
/// @type.symbol symbol=Read source="type Read = <comptime N: uint>() => [uint8; N]" type=Function<(), FixedArray<uint8, N>>
/// @definition.type symbol=Read source="type Read = <comptime N: uint>() => [uint8; N]" value=Function<(), FixedArray<uint8, N>>
/// @generic.template source=type_expression parameters=(comptime N: uint64)
/// @type.symbol symbol=Read.N source="comptime N: uint" type=N
/// @resolution.name source=N target=Read.N

declare const read: Read;
/// @type.symbol symbol=read source=read type=Read reduced=Function<(), FixedArray<uint8, N>>
/// @resolution.pattern source=read kind=binding target=read
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
/// @generic.template symbol=Tagged parameters=(comptime Tag: string)
/// @type.symbol symbol=Tagged source="type Tagged<comptime Tag: string> = { tag: Tag }" type={ tag: Tag }
/// @definition.type symbol=Tagged source="type Tagged<comptime Tag: string> = { tag: Tag }" template=(comptime Tag: string) value={ tag: Tag }
/// @type.symbol symbol=Tagged.Tag source="comptime Tag: string" type=Tag
/// @resolution.name source=Tag target=Tagged.Tag

type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;
/// @generic.template symbol=Flagged parameters=(comptime Config: { name: string; enabled: boolean })
/// @type.symbol symbol=Flagged source="type Flagged<comptime Config: { name: string; enabled: boolean }> = Config" type=Config
/// @definition.type symbol=Flagged source="type Flagged<comptime Config: { name: string; enabled: boolean }> = Config" template=(comptime Config: { name: string; enabled: boolean }) value=Config
/// @type.symbol symbol=Flagged.Config source="comptime Config: { name: string; enabled: boolean }" type=Config
/// @resolution.name source=Config target=Flagged.Config

declare const tagged: Tagged<"alpha">;
/// @type.symbol symbol=tagged source=tagged type=Tagged<"alpha"> reduced={ tag: "alpha" }
/// @resolution.pattern source=tagged kind=binding target=tagged
/// @resolution.name source=Tagged target=Tagged

declare const flagged: Flagged<{ name: "search"; enabled: true }>;
/// @type.symbol symbol=flagged source=flagged type=Flagged<{ name: "search"; enabled: true }> reduced={ name: "search"; enabled: true }
/// @resolution.pattern source=flagged kind=binding target=flagged
/// @resolution.name source=Flagged target=Flagged

/// @generic.instance id="Flagged<{ name: \"search\"; enabled: true }>" template=Flagged arguments=({ name: "search"; enabled: true })
/// @generic.instance id="Tagged<\"alpha\">" template=Tagged arguments=("alpha")
"#,
    );
}
