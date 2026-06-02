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
function take<comptime N: uint>(value: [uint8; N]): [uint8; N] {
/// @generic.template symbol=take parameters=[comptime N: uint]
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
/// @resolution.call source="take<4>([1, 2, 3, 4])" parameters=([uint8; 4]) return=[uint8; 4] kind=symbol target=take application=take<4>
/// @generic.application source="take<4>([1, 2, 3, 4])" id=take<4>
/// @type.node source="take<4>([1, 2, 3, 4])" type=[uint8; 4]
/// @type.node source=[1, 2, 3, 4] type=[uint8; 4]
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @type.node source=3 type=float64
/// @type.node source=4 type=float64
/// @generic.application id=take<4> symbol=take arguments=[4]
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
function choose<comptime Flag: boolean = true>(value: int32): int32 {
/// @generic.template symbol=choose parameters=[comptime Flag: boolean = true]
/// @type.node source=true type=true
/// @type.symbol symbol=value#1 type=int32

    return value;
    /// @resolution.name source=value target=value#1
    /// @type.node source=value type=int32

}

const value = choose(1);
/// @type.symbol symbol=value#2 type=int32
/// @resolution.name source=choose target=choose
/// @resolution.call source=choose(1) parameters=(int32) return=int32 kind=symbol target=choose application=choose<true>
/// @generic.application source=choose(1) id=choose<true>
/// @type.node source=choose(1) type=int32
/// @type.node source=1 type=int32
/// @generic.application id=choose<true> symbol=choose arguments=[true]
"#);
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
type Tagged<comptime Tag: string> = { tag: Tag };
/// @generic.template symbol=Tagged parameters=[comptime Tag: string]
/// @type.symbol symbol=Tagged type={ tag: Tag }

type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;
/// @generic.template symbol=Flagged parameters=[comptime Config: { name: string; enabled: boolean }]
/// @type.symbol symbol=Flagged type=Config

declare const tagged: Tagged<"alpha">;
/// @type.symbol symbol=tagged type={ tag: "alpha" }
/// @resolution.name source=Tagged target=Tagged
/// @generic.application source="Tagged<\"alpha\">" id="Tagged<\"alpha\">"

declare const flagged: Flagged<{ name: "search"; enabled: true }>;
/// @type.symbol symbol=flagged type={ name: "search"; enabled: true }
/// @resolution.name source=Flagged target=Flagged
/// @generic.application source="Flagged<{ name: \"search\"; enabled: true }>" id="Flagged<{ name: \"search\"; enabled: true }>"
/// @generic.application id="Flagged<{ name: \"search\"; enabled: true }>" symbol=Flagged arguments=[{ name: "search"; enabled: true }]
/// @generic.application id="Tagged<\"alpha\">" symbol=Tagged arguments=["alpha"]
"#,
    );
}
