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
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="undefined == undefined" type=boolean
/// @type.node source=undefined type=undefined
/// @resolution.operator source="undefined == undefined" kind=builtin
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
function use(onValue?: (arg0: unknown) => void): void {
    if (onValue !== undefined) {
        onValue(1 as unknown);
    } else {
    }
}

=== checked ===
function use(onValue?: (value: unknown) => void): void {
/// @type.symbol symbol=use type=(Function<(unknown,), void> | undefined) => void
/// @type.symbol symbol=use.onValue source="onValue?: (value: unknown) => void" type=Function<(unknown,), void> | undefined

    if (onValue !== undefined) {
    /// @type.node source="onValue !== undefined" type=boolean
    /// @type.node source=onValue type=Function<(unknown,), void> | undefined
    /// @resolution.name source=onValue target=use.onValue
    /// @resolution.operator source="onValue !== undefined" kind=builtin
    /// @type.node source=undefined type=undefined

        onValue(1);
        /// @type.node source=onValue type=Function<(unknown,), void>
        /// @type.node source=onValue(1) type=void
        /// @resolution.name source=onValue target=use.onValue
        /// @resolution.call source=onValue(1) parameters=(unknown) arguments=(provided(1) as unknown) return=void kind=expression
        /// @type.node source=1 type=1

    } else {
    }
}
"#,
    );
}

#[test]
fn test_strict_undefined_equality_returns_boolean() {
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
/// @resolution.pattern source=isMissing kind=binding target=isMissing
/// @type.node source="undefined === undefined" type=boolean
/// @type.node source=undefined type=undefined
/// @resolution.operator source="undefined === undefined" kind=builtin
/// @type.node source=undefined type=undefined
"#,
    );
}

#[test]
fn test_strict_string_identity_selects_builtin() {
    let session = TestSession::single(
        r#"
declare const left: string;
declare const right: string;

const same = left === right;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const left: string;
declare const right: string;

const same: boolean = left === right;

=== checked ===
declare const left: string;
/// @type.symbol symbol=left source=left type=string
/// @resolution.pattern source=left kind=binding target=left

declare const right: string;
/// @type.symbol symbol=right source=right type=string
/// @resolution.pattern source=right kind=binding target=right

const same = left === right;
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @type.node source="left === right" type=boolean
/// @type.node source=left type=string
/// @resolution.name source=left target=left
/// @resolution.operator source="left === right" kind=builtin
/// @type.node source=right type=string
/// @resolution.name source=right target=right
"#,
    );
}

#[test]
fn test_strict_bigint_identity_selects_builtin() {
    let session = TestSession::single(
        r#"
declare const left: bigint;
declare const right: bigint;

const same = left === right;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const left: bigint;
declare const right: bigint;

const same: boolean = left === right;

=== checked ===
declare const left: bigint;
/// @type.symbol symbol=left source=left type=bigint
/// @resolution.pattern source=left kind=binding target=left

declare const right: bigint;
/// @type.symbol symbol=right source=right type=bigint
/// @resolution.pattern source=right kind=binding target=right

const same = left === right;
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @type.node source="left === right" type=boolean
/// @type.node source=left type=bigint
/// @resolution.name source=left target=left
/// @resolution.operator source="left === right" kind=builtin
/// @type.node source=right type=bigint
/// @resolution.name source=right target=right
"#,
    );
}

#[test]
fn test_strict_equality_rejects_disjoint_literal_types() {
    let session = TestSession::single(
        r#"
const same = "ready" === "done";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const same: boolean = "ready" === "done";

=== checked ===
const same = "ready" === "done";
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @type.node source="\"ready\" === \"done\"" type=boolean
/// @type.node source="\"ready\"" type="ready"
/// @resolution.operator source="\"ready\" === \"done\"" kind=builtin
/// @type.node source="\"done\"" type="done"
"#,
        r#"
/// @diagnostic.error id=invalid-strict-equality message="this comparison is unintentional: types '\"ready\"' and '\"done\"' have no overlap"
/// @diagnostic.label line=2 column=22 span="===" line_source="const same = \"ready\" === \"done\";"
"#,
    );
}

#[test]
fn test_strict_equality_rejects_value_struct_identity() {
    let session = TestSession::single(
        r#"
struct Badge {
    id: int32;
}

declare const left: Badge;
declare const right: Badge;
const same = left === right;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Badge {
    id: int32;
}

declare const left: Badge;
declare const right: Badge;
const same = left === right;

=== checked ===
struct Badge {
/// @type.symbol symbol=Badge type=Badge
/// @definition.struct symbol=Badge
/// @definition.field symbol=Badge.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Badge.id source="id: int32" type=int32

}

declare const left: Badge;
/// @type.symbol symbol=left source=left type=Badge
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=Badge target=Badge

declare const right: Badge;
/// @type.symbol symbol=right source=right type=Badge
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=Badge target=Badge

const same = left === right;
/// @type.symbol symbol=same source=same type=<error>
/// @resolution.pattern source=same kind=binding target=same
/// @type.node source="left === right" type=<error>
/// @type.node source=left type=Badge
/// @resolution.name source=left target=left
/// @type.node source=right type=Badge
/// @resolution.name source=right target=right
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '===' is not defined for 'Badge' and 'Badge'"
/// @diagnostic.label line=8 column=19 span="===" line_source="const same = left === right;"
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
/// @resolution.pattern source=kind kind=binding target=kind

const isPending = kind == "pending";
/// @type.symbol symbol=isPending source=isPending type=boolean
/// @resolution.pattern source=isPending kind=binding target=isPending
/// @type.node source="kind == \"pending\"" type=boolean
/// @type.node source=kind type="pending" | "fulfilled"
/// @resolution.name source=kind target=kind
/// @resolution.operator source="kind == \"pending\"" kind=builtin
/// @type.node source="\"pending\"" type="pending"
"#,
    );
}

#[test]
fn test_overloaded_equality_selects_extension_method() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "destack:ops";

struct Badge {
    id: float64;
}

extension of Badge implements PartialEqual<Badge> {
    equal(other: Badge): boolean {
        this.id == other.id
    }
}

declare const left: Badge;
declare const right: Badge;
const same = left == right;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { PartialEqual } from "destack:ops";

struct Badge {
    id: float64;
}

extension of Badge implements PartialEqual<Badge> {
    equal(other: Badge): boolean {
        this.id == other.id
    }
}

declare const left: Badge;
declare const right: Badge;
const same: boolean = left == right;

=== checked ===
import { PartialEqual } from "destack:ops";

struct Badge {
/// @type.symbol symbol=Badge type=Badge
/// @definition.struct symbol=Badge
/// @definition.field symbol=Badge.id source="id: float64" key=id type=float64

    id: float64;
    /// @type.symbol symbol=Badge.id source="id: float64" type=float64

}

extension of Badge implements PartialEqual<Badge> {
/// @definition.extension symbol=<module>#2 form=local target=Badge
/// @definition.implements symbol=<module>#2 source=PartialEqual<Badge> target=ops.equality.PartialEqual arguments=(Badge)
/// @definition.method symbol=equal slot=equal type=(this: this, Badge) => boolean
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=PartialEqual target=ops.equality.PartialEqual
/// @resolution.name source=Badge target=Badge

    equal(other: Badge): boolean {
    /// @type.symbol symbol=equal type=(this: this, Badge) => boolean
    /// @type.symbol symbol=equal.other source="other: Badge" type=Badge
    /// @resolution.name source=Badge target=Badge

        this.id == other.id
        /// @resolution.member source=this.id receiver=Badge kind=symbol target=Badge.id
        /// @resolution.operator source="this.id == other.id" kind=builtin
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Badge
        /// @resolution.name source=other target=equal.other
        /// @resolution.member source=other.id receiver=Badge kind=symbol target=Badge.id

    }
}

declare const left: Badge;
/// @type.symbol symbol=left source=left type=Badge
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=Badge target=Badge

declare const right: Badge;
/// @type.symbol symbol=right source=right type=Badge
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=Badge target=Badge

const same = left == right;
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=left target=left
/// @resolution.operator source="left == right" kind=call parameters=(Badge) arguments=(provided(right) as Badge) return=boolean target=equal receiver=Badge
/// @resolution.name source=right target=right
"#,
    );
}
