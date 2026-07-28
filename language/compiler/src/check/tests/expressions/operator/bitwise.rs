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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const flags: int32;
const shifted: int32 = flags << 5;
const literal: 32 = 1 << 5;

=== checked ===
declare const flags: int32;
/// @type.symbol symbol=flags source=flags type=int32
/// @resolution.pattern source=flags kind=binding target=flags

const shifted = flags << 5;
/// @type.symbol symbol=shifted source=shifted type=int32
/// @resolution.pattern source=shifted kind=binding target=shifted
/// @resolution.name source=flags target=flags
/// @resolution.operator source="flags << 5" type=int32 operator="<<" kind=builtin operands=[flags as int32 families=(integer), 5 as int32 families=(integer)]
/// @resolution.place source=flags placement="local" lifetime="static" access="exclusive"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const mask: int32;
declare const bits: int32;
const masked: int32 = mask & bits;

=== checked ===
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
/// @resolution.place source=mask placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=mask root=mask
/// @resolution.name source=bits target=bits
/// @resolution.place source=bits placement="local" lifetime="static" access="exclusive"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const scale: float64;
const bad = scale & 2;

=== checked ===
declare const scale: float64;
/// @type.symbol symbol=scale source=scale type=float64
/// @resolution.pattern source=scale kind=binding target=scale

const bad = scale & 2;
/// @type.symbol symbol=bad source=bad type=<error>
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=scale target=scale
/// @resolution.place source=scale placement="local" lifetime="static" access="exclusive"
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
import { And } from "destack:ops";

struct Flags {
    bits: int32;
}

extension of Flags implements And<Flags> {
    type Output = Flags;

    and(other: Flags): Flags {
        Flags { bits: this.bits & other.bits }
    }
}

declare const left: Flags;
declare const right: Flags;
const both = left & right;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { And } from "destack:ops";

struct Flags {
    bits: int32;
}

extension of Flags implements And<Flags> {
    type Output = Flags;

    and(other: Flags): Flags {
        Flags { bits: this.bits & other.bits }
    }
}

declare const left: Flags;
declare const right: Flags;
const both: Flags = left & right;

=== checked ===
import { And } from "destack:ops";

struct Flags {
/// @type.symbol symbol=Flags type=Flags
/// @definition.struct symbol=Flags
/// @definition.field symbol=Flags.bits source="bits: int32" key=bits type=int32

    bits: int32;
    /// @type.symbol symbol=Flags.bits source="bits: int32" type=int32

}

extension of Flags implements And<Flags> {
/// @definition.extension symbol=<module>#2 form=local target=Flags
/// @definition.implements symbol=<module>#2 source=And<Flags> target="And<Flags><type Output = Flags>"
/// @definition.associated.type symbol=Output source="type Output = Flags" key=Output value=Flags
/// @definition.method symbol=and slot=and type=<and.'l0>(this: &and.'l0 exclusive this, Flags) => Flags
/// @resolution.name source=Flags target=Flags
/// @resolution.name source=And target=ops.bitwise.And
/// @resolution.name source=Flags target=Flags

    type Output = Flags;
    /// @type.symbol symbol=Output source="type Output = Flags" type=Flags
    /// @resolution.name source=Flags target=Flags

    and(other: Flags): Flags {
    /// @generic.template symbol=and parent=template#0 parameters=('l0)
    /// @type.symbol symbol=and type=<and.'l0>(this: &and.'l0 exclusive this, Flags) => Flags
    /// @type.symbol symbol=and.other source="other: Flags" type=Flags
    /// @resolution.name source=Flags target=Flags
    /// @resolution.name source=Flags target=Flags

        Flags { bits: this.bits & other.bits }
        /// @resolution.name source=Flags target=Flags
        /// @resolution.member source=this.bits receiver=&and.'l0 exclusive Flags type=int32 kind=field target_receiver=&and.'l0 exclusive Flags key=bits target=Flags.bits target_type=int32
        /// @resolution.operator source="this.bits & other.bits" type=int32 operator="&" kind=builtin operands=[this.bits as int32 families=(integer), other.bits as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&and.'l0 exclusive Flags
        /// @resolution.place source=this placement="local" lifetime=and.'l0 access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.bits placement="local" lifetime=and.'l0 access="exclusive"
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
/// @resolution.operator source="left & right" type=Flags operator="&" kind=call parameters=(Flags) arguments=(provided(right) as Flags) return=Flags kind=symbol target=and receiver=Flags adjustments=(borrow(&'static exclusive Flags))
/// @resolution.place source=left placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=left root=left
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=right root=right
"#,
    );
}
