use crate::tests::{DirRows, TestSession};

#[test]
fn test_generic_return_contextualizes_empty_array_field() {
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
=== annotated ===
function capture<T>(value: T): { reactions: T[] } {
    return { reactions: [] };
}

=== checked ===
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
fn test_generic_union_return_contextualizes_empty_array_field() {
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
=== annotated ===
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

=== checked ===
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
