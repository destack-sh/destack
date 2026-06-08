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
/// @resolution.call source="take<4>([1, 2, 3, 4])" parameters=([uint8; 4]) return=[uint8; 4] kind=symbol target=take instance=take<4>
/// @generic.instance source="take<4>([1, 2, 3, 4])" id=take<4>
/// @type.node source="take<4>([1, 2, 3, 4])" type=[uint8; 4]
/// @type.node source=[1, 2, 3, 4] type=[uint8; 4]
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @type.node source=3 type=float64
/// @type.node source=4 type=float64
/// @generic.instance id=take<4> symbol=take arguments=[4]
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
/// @generic.template source=declaration parameters=[comptime Flag: boolean = true]
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
/// @type.node source=choose type=<Flag: boolean = true>(int32) => int32
/// @type.node source=choose(1) type=int32
/// @resolution.name source=choose target=choose
/// @resolution.call source=choose(1) parameters=(int32) return=int32 kind=symbol target=choose instance=choose<true>
/// @type.node source=1 type=1

/// @generic.instance id=choose<true> template=choose arguments=[true]
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
/// @generic.template source=declaration parameters=[comptime Tag: string]
/// @type.symbol symbol=Tagged source="type Tagged<comptime Tag: string> = { tag: Tag }" type={ tag: Tag }
/// @definition.type symbol=Tagged source="type Tagged<comptime Tag: string> = { tag: Tag }" template=LocalGenericTemplateId(0) value={ tag: Tag }
/// @type.symbol symbol=Tagged.tag source="tag: Tag" type=Tag
/// @resolution.name source=Tag target=Tagged.Tag

type Flagged<comptime Config: { name: string; enabled: boolean }> = Config;
/// @generic.template source=declaration parameters=[comptime Config: { name: string; enabled: boolean }]
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

/// @generic.instance id="Flagged<{ name: \"search\"; enabled: true }>" template=Flagged arguments=[{ name: "search"; enabled: true }]
/// @generic.instance id="Tagged<\"alpha\">" template=Tagged arguments=["alpha"]
"#,
    );
}

#[test]
fn test_contextual_object_literal_contextualizes_generic_empty_array_field() {
    let session = TestSession::single(
        r#"
function capture<T>(value: T): { reactions: T[] } {
    return { reactions: [] };
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
function capture<T>(value: T): { reactions: T[] } {
/// @generic.template symbol=capture parameters=[T]
/// @type.symbol symbol=value source=value type=T
/// @type.symbol symbol=reactions source="reactions: T[]" type=Array<T>
/// @resolution.name source=T target=T
/// @resolution.name source=T target=T

    return { reactions: [] };
    /// @type.node source="{ reactions: [] }" type={ reactions: Array<T> }
    /// @type.node source=[] type=Array<T>

}

/// @check.stats.solve variables=1 terms=10 constraints=1 obligations=0 solutions=1 bounds=2 decisions=0
"#,
    );
}

#[test]
fn test_discriminated_union_contextualizes_generic_empty_array_field() {
    let session = TestSession::single(
        r#"
interface Pending<T> {
    kind: "pending";
    reactions: T[];
}

interface Done<T> {
    kind: "done";
    value: T;
}

type State<T> = Pending<T> | Done<T>;

function pending<T>(): State<T> {
    return { kind: "pending", reactions: [] };
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
interface Pending<T> {
/// @generic.template symbol=Pending parameters=[T]
/// @type.symbol symbol=Pending type={ kind: "pending"; reactions: Array<T> }

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: T[];
    /// @type.symbol symbol=Pending.reactions source="reactions: T[]" type=Array<T>
    /// @resolution.name source=T target=T

}

interface Done<T> {
/// @generic.template symbol=Done parameters=[T]
/// @type.symbol symbol=Done type={ kind: "done"; value: T }

    kind: "done";
    /// @type.symbol symbol=Done.kind source="kind: \"done\"" type="done"

    value: T;
    /// @type.symbol symbol=Done.value source="value: T" type=T
    /// @resolution.name source=T target=T

}

type State<T> = Pending<T> | Done<T>;
/// @generic.template symbol=State parameters=[T]
/// @type.symbol symbol=State type=Pending<T> | Done<T>
/// @resolution.name source=Pending target=Pending
/// @generic.instance source=Pending<T> id=Pending<T>
/// @resolution.name source=T target=T
/// @resolution.name source=Done target=Done
/// @generic.instance source=Done<T> id=Done<T>
/// @resolution.name source=T target=T

function pending<T>(): State<T> {
/// @generic.template symbol=pending parameters=[T]
/// @type.symbol symbol=pending type=() => Pending<T> | Done<T>
/// @resolution.name source=State target=State
/// @generic.instance source=State<T> id=State<T>
/// @resolution.name source=T target=T

    return { kind: "pending", reactions: [] };
    /// @type.node source="{ kind: \"pending\", reactions: [] }" type={ kind: "pending"; reactions: Array<T> }
    /// @type.node source="\"pending\"" type="pending"
    /// @type.node source=[] type=Array<T>

}

/// @generic.instance id=Done<T> symbol=Done arguments=[T]
/// @generic.instance id=Pending<T> symbol=Pending arguments=[T]
/// @generic.instance id=State<T> symbol=State arguments=[T]

/// @check.stats.solve variables=2 terms=36 constraints=1 obligations=0 solutions=2 bounds=4 decisions=0
"#,
    );
}
