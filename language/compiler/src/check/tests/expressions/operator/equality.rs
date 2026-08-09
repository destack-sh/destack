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
/// @resolution.operator source="undefined == undefined" type=boolean operator="==" kind=builtin operands=[undefined as undefined families=(undefined), undefined as undefined families=(undefined)]
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
    if (onValue !== (undefined as ((arg0: unknown) => void) | undefined)) {
        onValue(1 as Dynamic<unknown>);
    } else {
    }
}

=== checked ===
function use(onValue?: (value: unknown) => void): void {
/// @type.symbol symbol=use type=(Function<(unknown,), void> | undefined?) => void
/// @type.symbol symbol=use.onValue source="onValue?: (value: unknown) => void" type=Function<(unknown,), void> | undefined
/// @type.symbol symbol=use.value source="value: unknown" type=Dynamic<unknown>

    if (onValue !== undefined) {
    /// @type.node source="onValue !== undefined" type=boolean
    /// @type.node source=onValue type=Function<(unknown,), void> | undefined
    /// @resolution.name source=onValue target=use.onValue
    /// @resolution.operator source="onValue !== undefined" type=boolean operator="!==" kind=builtin operands=[onValue as Function<(unknown,), void> | undefined, undefined as Function<(unknown,), void> | undefined]
    /// @resolution.place source=onValue placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=onValue root=use.onValue
    /// @type.node source=undefined type=undefined

        onValue(1);
        /// @type.node source=onValue type=Function<(unknown,), void>
        /// @type.node source=onValue(1) type=void
        /// @resolution.name source=onValue target=use.onValue
        /// @resolution.call source=onValue(1) parameters=(unknown) arguments=(provided(1) as unknown) return=void kind=expression target=expression
        /// @resolution.place source=onValue placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=onValue root=use.onValue
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
/// @resolution.operator source="undefined === undefined" type=boolean operator="===" kind=builtin operands=[undefined as undefined families=(undefined), undefined as undefined families=(undefined)]
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
/// @resolution.operator source="left === right" type=boolean operator="===" kind=builtin operands=[left as string families=(string), right as string families=(string)]
/// @resolution.place source=left placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=left root=left
/// @type.node source=right type=string
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=right root=right
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
/// @resolution.operator source="left === right" type=boolean operator="===" kind=builtin operands=[left as bigint families=(bigint), right as bigint families=(bigint)]
/// @resolution.place source=left placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=left root=left
/// @type.node source=right type=bigint
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=right root=right
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
/// @resolution.operator source="\"ready\" === \"done\"" type=boolean operator="===" kind=builtin operands=["ready" as "ready" families=(string), "done" as "done" families=(string)]
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
/// @resolution.rejected source="left === right"
/// @resolution.place source=left placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=left root=left
/// @type.node source=right type=Badge
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=right root=right
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '===' is not defined for 'Badge' and 'Badge'"
/// @diagnostic.label line=8 column=19 span="===" line_source="const same = left === right;"
"#,
    );
}

#[test]
fn test_strict_equality_compares_union_with_disjoint_reference_arm() {
    let session = TestSession::single(
        r#"
class User {}

declare const value: string | User;
const isReady = value === "ready";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

declare const value: string | User;
const isReady: boolean = value === ("ready" as string | User);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const value: string | User;
/// @type.symbol symbol=value source=value type=string | User
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=User target=User

const isReady = value === "ready";
/// @type.symbol symbol=isReady source=isReady type=boolean
/// @resolution.pattern source=isReady kind=binding target=isReady
/// @type.node source="value === \"ready\"" type=boolean
/// @type.node source=value type=string | User
/// @resolution.name source=value target=value
/// @resolution.operator source="value === \"ready\"" type=boolean operator="===" kind=builtin operands=[value as string | User, "ready" as string | User]
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @type.node source="\"ready\"" type="ready"
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
/// @resolution.operator source="kind == \"pending\"" type=boolean operator="==" kind=builtin operands=[kind as "pending" | "fulfilled" families=(string), "pending" as "pending" families=(string)]
/// @resolution.place source=kind placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=kind root=kind
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
/// @definition.implements symbol=<module>#2 source=PartialEqual<Badge> target=PartialEqual<Badge>
/// @definition.method symbol=equal slot=equal type=<equal.'a>(this: &equal.'a exclusive this, Badge) => boolean
/// @definition.conformance symbol=<module>#2 member=equal requirement=ops.equality.PartialEqual.equal
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=PartialEqual target=ops.equality.PartialEqual
/// @resolution.name source=Badge target=Badge

    equal(other: Badge): boolean {
    /// @generic.template symbol=equal parent=template#0 parameters=('a)
    /// @type.symbol symbol=equal type=<equal.'a>(this: &equal.'a exclusive this, Badge) => boolean
    /// @type.symbol symbol=equal.other source="other: Badge" type=Badge
    /// @resolution.name source=Badge target=Badge

        this.id == other.id
        /// @resolution.member source=this.id receiver=&equal.'a exclusive Badge type=float64 kind=field target_receiver=&equal.'a exclusive Badge key=id target=Badge.id target_type=float64
        /// @resolution.operator source="this.id == other.id" type=boolean operator="==" kind=builtin operands=[this.id as float64 families=(float), other.id as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&equal.'a exclusive Badge
        /// @resolution.place source=this placement="local" lifetime=equal.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.id placement="local" lifetime=equal.'a access="exclusive"
        /// @resolution.access source=this.id root=this keys=[id]
        /// @resolution.name source=other target=equal.other
        /// @resolution.member source=other.id receiver=Badge type=float64 kind=field target_receiver=Badge key=id target=Badge.id target_type=float64
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=equal.other
        /// @resolution.place source=other.id placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other.id root=equal.other keys=[id]

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
/// @resolution.operator source="left == right" type=boolean operator="==" kind=call parameters=(Badge) arguments=(provided(right) as Badge) return=boolean kind=symbol target=equal receiver=Badge adjustments=(borrow(&'static exclusive Badge))
/// @resolution.place source=left placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=left root=left
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=right root=right
"#,
    );
}

#[test]
fn test_overloaded_equality_accepts_negative_zero_literal() {
    let session = TestSession::single(
        r#"
import { PartialEqual } from "destack:ops";

struct Measure {
    value: float64;
}

extension of Measure implements PartialEqual<float64> {
    equal(other: float64): boolean {
        return this.value == other;
    }
}

declare const measure: Measure;
const same = measure == -0.0;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { PartialEqual } from "destack:ops";

struct Measure {
    value: float64;
}

extension of Measure implements PartialEqual<float64> {
    equal(other: float64): boolean {
        return this.value == other;
    }
}

declare const measure: Measure;
const same: boolean = measure == -0.0;

=== checked ===
import { PartialEqual } from "destack:ops";

struct Measure {
/// @type.symbol symbol=Measure type=Measure
/// @definition.struct symbol=Measure
/// @definition.field symbol=Measure.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Measure.value source="value: float64" type=float64

}

extension of Measure implements PartialEqual<float64> {
/// @definition.extension symbol=<module>#2 form=local target=Measure
/// @definition.implements symbol=<module>#2 source=PartialEqual<float64> target=PartialEqual<float64>
/// @definition.method symbol=equal slot=equal type=<equal.'a>(this: &equal.'a exclusive this, float64) => boolean
/// @definition.conformance symbol=<module>#2 member=equal requirement=ops.equality.PartialEqual.equal
/// @resolution.name source=Measure target=Measure
/// @resolution.name source=PartialEqual target=ops.equality.PartialEqual

    equal(other: float64): boolean {
    /// @generic.template symbol=equal parent=template#0 parameters=('a)
    /// @type.symbol symbol=equal type=<equal.'a>(this: &equal.'a exclusive this, float64) => boolean
    /// @type.symbol symbol=equal.other source="other: float64" type=float64

        return this.value == other;
        /// @resolution.member source=this.value receiver=&equal.'a exclusive Measure type=float64 kind=field target_receiver=&equal.'a exclusive Measure key=value target=Measure.value target_type=float64
        /// @resolution.operator source="this.value == other" type=boolean operator="==" kind=builtin operands=[this.value as float64 families=(float), other as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&equal.'a exclusive Measure
        /// @resolution.place source=this placement="local" lifetime=equal.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=equal.'a access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.name source=other target=equal.other
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=equal.other

    }
}

declare const measure: Measure;
/// @type.symbol symbol=measure source=measure type=Measure
/// @resolution.pattern source=measure kind=binding target=measure
/// @resolution.name source=Measure target=Measure

const same = measure == -0.0;
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=measure target=measure
/// @resolution.operator source="measure == -0.0" type=boolean operator="==" kind=call parameters=(float64) arguments=(provided(-0.0) as float64) return=boolean kind=symbol target=equal receiver=Measure adjustments=(borrow(&'static exclusive Measure))
/// @resolution.place source=measure placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=measure root=measure
/// @resolution.operator source=-0.0 type=-0 operator="-" kind=builtin operands=[0.0 as 0 families=(float)]
"#,
    );
}
