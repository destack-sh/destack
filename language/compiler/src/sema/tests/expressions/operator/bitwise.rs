use crate::tests::{DirRows, TestSession};

#[test]
fn test_shift_keeps_the_typed_left_operand_and_folds_literals() {
    let session = TestSession::single(
        r#"
declare const flags: int32;
const shifted = flags << 5;
const literal = 1 << 5;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const flags: int32;
const shifted: int32 = flags << 5;
const literal: 32 = 1 << 5;

=== dir ===
declare const flags: int32;
/// @type.symbol symbol=flags source=flags type=int32
/// @resolution.pattern source=flags kind=binding target=flags

const shifted = flags << 5;
/// @type.symbol symbol=shifted source=shifted type=int32
/// @resolution.pattern source=shifted kind=binding target=shifted
/// @resolution.name source=flags target=flags
/// @resolution.operator source="flags << 5" type=int32 operator="<<" kind=builtin operands=[flags as int32 families=(integer), 5 as int32 families=(integer)]
/// @resolution.place source=flags placement="local" lifetime="static" access="immutable"
/// @resolution.access source=flags root=flags

const literal = 1 << 5;
/// @type.symbol symbol=literal source=literal type=32
/// @resolution.pattern source=literal kind=binding target=literal
/// @resolution.operator source="1 << 5" type=32 operator="<<" kind=builtin operands=[1 as 1 families=(integer), 5 as 5 families=(integer)]
"#,
    );
}

#[test]
fn test_bitwise_and_joins_integer_operands() {
    let session = TestSession::single(
        r#"
declare const mask: int32;
declare const bits: int32;
const masked = mask & bits;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const mask: int32;
declare const bits: int32;
const masked: int32 = mask & bits;

=== dir ===
declare const mask: int32;
/// @type.symbol symbol=mask source=mask type=int32
/// @resolution.pattern source=mask kind=binding target=mask

declare const bits: int32;
/// @type.symbol symbol=bits source=bits type=int32
/// @resolution.pattern source=bits kind=binding target=bits

const masked = mask & bits;
/// @type.symbol symbol=masked source=masked type=int32
/// @resolution.pattern source=masked kind=binding target=masked
/// @resolution.name source=mask target=mask
/// @resolution.operator source="mask & bits" type=int32 operator="&" kind=builtin operands=[mask as int32 families=(integer), bits as int32 families=(integer)]
/// @resolution.place source=mask placement="local" lifetime="static" access="immutable"
/// @resolution.access source=mask root=mask
/// @resolution.name source=bits target=bits
/// @resolution.place source=bits placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bits root=bits
"#,
    );
}

#[test]
fn test_bitwise_rejects_float_operands() {
    let session = TestSession::single(
        r#"
declare const scale: float64;
const bad = scale & 2;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const scale: float64;
const bad = scale & 2;

=== dir ===
declare const scale: float64;
/// @type.symbol symbol=scale source=scale type=float64
/// @resolution.pattern source=scale kind=binding target=scale

const bad = scale & 2;
/// @type.symbol symbol=bad source=bad type=<error>
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=scale target=scale
/// @resolution.rejected source="scale & 2"
/// @resolution.place source=scale placement="local" lifetime="static" access="immutable"
/// @resolution.access source=scale root=scale
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '&' is not defined for 'float64' and '2'"
/// @diagnostic.label line=3 column=19 span="&" line_source="const bad = scale & 2;"
"#,
    );
}

#[test]
fn test_overloaded_bitwise_and_selects_extension_method() {
    let session = TestSession::single(
        r#"
import { And } from "tspp:ops";

struct Flags {
    bits: int32;
}

extension of Flags implements And<Flags> {
    type Output = Flags;

    and(&readonly this, other: Flags): Flags {
        Flags { bits: this.bits & other.bits }
    }
}

declare const left: Flags;
declare const right: Flags;
const both = left & right;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { And } from "tspp:ops";

struct Flags {
    bits: int32;
}

extension of Flags implements And<Flags> {
    type Output = Flags;

    and(&readonly this, other: Flags): Flags {
        Flags { bits: this.bits & other.bits }
    }
}

declare const left: Flags;
declare const right: Flags;
const both: Flags = left & right;

=== dir ===
import { And } from "tspp:ops";

struct Flags {
/// @type.symbol symbol=Flags type=Flags
/// @definition.struct symbol=Flags
/// @definition.field symbol=Flags.bits source="bits: int32" key=bits type=int32

    bits: int32;
    /// @type.symbol symbol=Flags.bits source="bits: int32" type=int32

}

extension of Flags implements And<Flags> {
/// @generic.instance id=And<Flags> template=And arguments=(Flags)
/// @definition.extension symbol=<module>#2 form=local target=Flags
/// @definition.implements symbol=<module>#2 source=And<Flags> target=And<Flags>
/// @definition.associated.type symbol=Output source="type Output = Flags" key=Output value=Flags
/// @definition.method symbol=and slot=and type=<and.'a>(this: &and.'a readonly Flags, Flags) => Flags
/// @definition.conformance symbol=<module>#2 member=Output requirement=And.Output
/// @definition.conformance symbol=<module>#2 member=and requirement=And.and
/// @resolution.name source=Flags target=Flags
/// @resolution.name source=And target=And
/// @resolution.name source=Flags target=Flags

    type Output = Flags;
    /// @type.symbol symbol=Output source="type Output = Flags" type=Flags
    /// @resolution.name source=Flags target=Flags

    and(&readonly this, other: Flags): Flags {
    /// @generic.template symbol=and parent=template#0 parameters=('a)
    /// @type.symbol symbol=and type=<and.'a>(this: &and.'a readonly Flags, Flags) => Flags
    /// @type.symbol symbol=and.this source="&readonly this" type=&and.'a readonly Flags
    /// @type.symbol symbol=and.other source="other: Flags" type=Flags
    /// @resolution.name source=Flags target=Flags
    /// @resolution.name source=Flags target=Flags

        Flags { bits: this.bits & other.bits }
        /// @resolution.name source=Flags target=Flags
        /// @resolution.member source=this.bits receiver=&and.'a readonly Flags type=int32 kind=field target_receiver=&and.'a readonly Flags key=bits target=Flags.bits target_type=int32
        /// @resolution.operator source="this.bits & other.bits" type=int32 operator="&" kind=builtin operands=[this.bits as int32 families=(integer), other.bits as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&and.'a readonly Flags
        /// @resolution.place source=this placement=and.'a lifetime=and.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.bits placement=and.'a lifetime=and.'a access="readonly"
        /// @resolution.access source=this.bits root=this keys=[bits]
        /// @resolution.name source=other target=and.other
        /// @resolution.member source=other.bits receiver=Flags type=int32 kind=field target_receiver=Flags key=bits target=Flags.bits target_type=int32
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=and.other
        /// @resolution.place source=other.bits placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other.bits root=and.other keys=[bits]

    }
}

declare const left: Flags;
/// @type.symbol symbol=left source=left type=Flags
/// @resolution.pattern source=left kind=binding target=left
/// @resolution.name source=Flags target=Flags

declare const right: Flags;
/// @type.symbol symbol=right source=right type=Flags
/// @resolution.pattern source=right kind=binding target=right
/// @resolution.name source=Flags target=Flags

const both = left & right;
/// @type.symbol symbol=both source=both type=Flags
/// @resolution.pattern source=both kind=binding target=both
/// @resolution.name source=left target=left
/// @resolution.operator source="left & right" type=Flags operator="&" kind=call parameters=(Flags) arguments=(provided(right) as Flags) return=Flags regions=("static" & "local") kind=symbol target=and receiver=Flags adjustments=(borrow(&'static readonly Flags)) instance="Flags.<extension#1>.and<\"static\" & \"local\">"
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @generic.instantiation id="and<\"static\" & \"local\">" template=and arguments=("static" & "local")
/// @generic.instance id="and<\"bound0\" & \"local\">" template=and arguments=("bound0" & "local")
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
/// @resolution.access source=right root=right
"#,
    );
}

#[test]
fn test_select_the_standard_bigint_and_operation() {
    let session = TestSession::single(
        r#"
function retain(value: bigint): bigint {
    return value & -1n;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
function retain(value: bigint): bigint {
    return value & -1n;
}

=== dir ===
function retain(value: bigint): bigint {
/// @type.symbol symbol=retain type=(bigint) => bigint
/// @type.symbol symbol=retain.value source="value: bigint" type=bigint

    return value & -1n;
    /// @resolution.name source=value target=retain.value
    /// @resolution.operator source="value & -1n" type=bigint operator="&" kind=builtin operands=[value as bigint families=(bigint), -1n as bigint families=(bigint)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=retain.value
    /// @resolution.operator source=-1n type=-1n operator="-" kind=builtin operands=[1n as 1n families=(bigint)]

}
"#, r#"

"#);
}

#[test]
fn test_call_the_standard_bigint_and_method() {
    let session = TestSession::single(
        r#"
function retain(value: bigint): bigint {
    return value.and(-1n);
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
function retain(value: bigint): bigint {
    return value.and<"managed">(-1n);
}

=== dir ===
function retain(value: bigint): bigint {
/// @type.symbol symbol=retain type=(bigint) => bigint
/// @type.symbol symbol=retain.value source="value: bigint" type=bigint

    return value.and(-1n);
    /// @resolution.name source=value target=retain.value
    /// @resolution.member source=value.and receiver=bigint type=<and.'a>(this: &and.'a readonly bigint, bigint) => bigint kind=symbol target_receiver=bigint target=and
    /// @resolution.call source=value.and(-1n) parameters=(bigint) arguments=(provided(-1n) as bigint) return=bigint regions=("managed" & "local") kind=symbol target=and receiver=bigint adjustments=(borrow(&'managed readonly bigint)) instance="bigint.<extension#2>.and<\"managed\" & \"local\">"
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=retain.value
    /// @generic.instantiation id="and<\"managed\" & \"local\">" template=and arguments=("managed" & "local")
    /// @resolution.operator source=-1n type=-1n operator="-" kind=builtin operands=[1n as 1n families=(bigint)]

}
"#, r#"

"#);
}

#[test]
fn test_select_the_standard_bigint_and_operation_between_values() {
    let session = TestSession::single(
        r#"
function retain(value: bigint, other: bigint): bigint {
    return value & other;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
function retain(value: bigint, other: bigint): bigint {
    return value & other;
}

=== dir ===
function retain(value: bigint, other: bigint): bigint {
/// @type.symbol symbol=retain type=(bigint, bigint) => bigint
/// @type.symbol symbol=retain.value source="value: bigint" type=bigint
/// @type.symbol symbol=retain.other source="other: bigint" type=bigint

    return value & other;
    /// @resolution.name source=value target=retain.value
    /// @resolution.operator source="value & other" type=bigint operator="&" kind=builtin operands=[value as bigint families=(bigint), other as bigint families=(bigint)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=retain.value
    /// @resolution.name source=other target=retain.other
    /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=other root=retain.other

}
"#, r#"

"#);
}
