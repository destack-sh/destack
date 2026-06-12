use crate::tests::{DirRows, TestSession};

#[test]
fn test_undefined_equality_selects_builtin_operator() {
    let session = TestSession::single(
        r#"
const value = undefined == undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: boolean = undefined == undefined;

=== checked ===
const value = undefined == undefined;
/// @type.symbol symbol=value source=value type=boolean
/// @type.node source="undefined == undefined" type=boolean
/// @type.node source=undefined type=undefined
/// @resolution.call source="undefined == undefined" parameters=() return=boolean kind=builtin builtin=binary.equal
/// @type.node source=undefined type=undefined
"#,
    );
}

#[test]
fn test_strict_undefined_inequality_narrows_then_branch() {
    let session = TestSession::single(
        r#"
function use(onValue?: (value: unknown) => void): void {
    if (onValue !== undefined) {
        onValue(1);
    } else {
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function use(onValue?: (value: unknown) => void): void {
    if (onValue !== undefined) {
        onValue(1);
    } else {
    }
}

=== checked ===
function use(onValue?: (value: unknown) => void): void {
/// @type.symbol symbol=use type=((unknown) => void | undefined?) => void
/// @type.symbol symbol=onValue source="onValue?: (value: unknown) => void" type=(unknown) => void | undefined

    if (onValue !== undefined) {
    /// @type.node type=void | void
    /// @type.node source="onValue !== undefined" type=boolean
    /// @type.node source=onValue type=(unknown) => void | undefined
    /// @resolution.name source=onValue target=onValue
    /// @resolution.call source="onValue !== undefined" parameters=() return=boolean kind=builtin builtin=binary.not_equal_strict
    /// @type.node source=undefined type=undefined

        onValue(1);
        /// @type.node source=onValue type=(unknown) => void | undefined extends undefined ? never : (unknown) => void | undefined
        /// @type.node source=onValue(1) type=void
        /// @resolution.name source=onValue target=onValue
        /// @resolution.call source=onValue(1) parameters=(unknown) return=void kind=expression
        /// @type.node source=1 type=1

    } else {
    }
}
"#,
    );
}

#[test]
fn test_strict_undefined_identity_returns_boolean() {
    let session = TestSession::single(
        r#"
const isMissing = undefined === undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const isMissing: boolean = undefined === undefined;

=== checked ===
const isMissing = undefined === undefined;
/// @type.symbol symbol=isMissing source=isMissing type=boolean
/// @type.node source="undefined === undefined" type=boolean
/// @type.node source=undefined type=undefined
/// @resolution.call source="undefined === undefined" parameters=() return=boolean kind=builtin builtin=binary.equal_strict
/// @type.node source=undefined type=undefined
"#,
    );
}

#[test]
fn test_equality_accepts_literal_union_discriminant() {
    let session = TestSession::single(
        r#"
declare const kind: "pending" | "fulfilled";

const isPending = kind == "pending";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const kind: "pending" | "fulfilled";

const isPending: boolean = kind == "pending";

=== checked ===
declare const kind: "pending" | "fulfilled";
/// @type.symbol symbol=kind source=kind type="pending" | "fulfilled"

const isPending = kind == "pending";
/// @type.symbol symbol=isPending source=isPending type=boolean
/// @type.node source="kind == \"pending\"" type=boolean
/// @type.node source=kind type="pending" | "fulfilled"
/// @resolution.name source=kind target=kind
/// @resolution.call source="kind == \"pending\"" parameters=() return=boolean kind=builtin builtin=binary.equal
/// @type.node source="\"pending\"" type="pending"
"#,
    );
}
