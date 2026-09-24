use crate::tests::{DirRows, TestSession};

#[test]
fn test_logical_and_narrows_its_right_operand() {
    let session = TestSession::single(
        r#"
function positive(value: &readonly (int32 | undefined)): boolean {
    value !== undefined && value > 0
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function positive<'a>(value: &'a readonly (int32 | undefined)): boolean {
    value !== (undefined as int32 | undefined) && (value as int32) > 0
}

=== dir ===
function positive(value: &readonly (int32 | undefined)): boolean {
/// @generic.template symbol=positive parameters=('a)
/// @type.symbol symbol=positive type=<positive.'a>(&positive.'a readonly (int32 | undefined)) => boolean
/// @type.symbol symbol=positive.value source="value: &readonly (int32 | undefined)" type=&positive.'a readonly (int32 | undefined)

    value !== undefined && value > 0
    /// @resolution.name source=value target=positive.value
    /// @resolution.operator source="value !== undefined && value > 0" type=boolean operator="&&" kind=builtin operands=[value !== undefined as boolean families=(boolean), value > 0 as boolean families=(boolean)]
    /// @resolution.operator source="value !== undefined" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement=positive.'a lifetime=positive.'a access="readonly"
    /// @resolution.access source=value root=positive.value
    /// @resolution.name source=value target=positive.value
    /// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
    /// @resolution.place source=value placement=positive.'a lifetime=positive.'a access="readonly"
    /// @resolution.access source=value root=positive.value
    /// @resolution.narrowing source=value union=&positive.'a readonly (int32 | undefined) arms=&positive.'a readonly int32

}
"#,
    );
}

#[test]
fn test_logical_or_narrows_its_right_operand() {
    let session = TestSession::single(
        r#"
function positive(value: &readonly (int32 | undefined)): boolean {
    value === undefined || value > 0
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function positive<'a>(value: &'a readonly (int32 | undefined)): boolean {
    value === (undefined as int32 | undefined) || (value as int32) > 0
}

=== dir ===
function positive(value: &readonly (int32 | undefined)): boolean {
/// @generic.template symbol=positive parameters=('a)
/// @type.symbol symbol=positive type=<positive.'a>(&positive.'a readonly (int32 | undefined)) => boolean
/// @type.symbol symbol=positive.value source="value: &readonly (int32 | undefined)" type=&positive.'a readonly (int32 | undefined)

    value === undefined || value > 0
    /// @resolution.name source=value target=positive.value
    /// @resolution.operator source="value === undefined || value > 0" type=boolean operator="||" kind=builtin operands=[value === undefined as boolean families=(boolean), value > 0 as boolean families=(boolean)]
    /// @resolution.operator source="value === undefined" type=boolean operator="===" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement=positive.'a lifetime=positive.'a access="readonly"
    /// @resolution.access source=value root=positive.value
    /// @resolution.name source=value target=positive.value
    /// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
    /// @resolution.place source=value placement=positive.'a lifetime=positive.'a access="readonly"
    /// @resolution.access source=value root=positive.value
    /// @resolution.narrowing source=value union=&positive.'a readonly (int32 | undefined) arms=&positive.'a readonly int32

}
"#,
    );
}

#[test]
fn test_undefined_equality_selects_builtin_operator() {
    let session = TestSession::single(
        r#"
const value = undefined == undefined;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: boolean = undefined == undefined;

=== dir ===
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function use(onValue?: (value: unknown) => void): void {
    if (onValue !== (undefined as ((value: unknown) => void) | undefined)) {
        onValue(1 as unknown);
    } else {
    }
}

=== dir ===
function use(onValue?: (value: unknown) => void): void {
/// @type.symbol symbol=use type=((unknown) => void | undefined?) => void
/// @type.symbol symbol=use.onValue source="onValue?: (value: unknown) => void" type=(unknown) => void | undefined
/// @type.symbol symbol=use.value source="value: unknown" type=unknown

    if (onValue !== undefined) {
    /// @type.node source="onValue !== undefined" type=boolean
    /// @type.node source=onValue type=(unknown) => void | undefined
    /// @resolution.name source=onValue target=use.onValue
    /// @resolution.operator source="onValue !== undefined" type=boolean operator="!==" kind=builtin operands=[onValue as (unknown) => void | undefined, undefined as (unknown) => void | undefined]
    /// @resolution.place source=onValue placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=onValue root=use.onValue
    /// @type.node source=undefined type=undefined

        onValue(1);
        /// @type.node source=onValue type=(unknown) => void
        /// @type.node source=onValue(1) type=void
        /// @resolution.name source=onValue target=use.onValue
        /// @resolution.call source=onValue(1) parameters=(unknown) arguments=(provided(1) as unknown) return=void kind=expression target=expression
        /// @resolution.place source=onValue placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=onValue root=use.onValue
        /// @resolution.narrowing source=onValue union=(unknown) => void | undefined arms=(unknown) => void
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const isMissing: boolean = undefined === undefined;

=== dir ===
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const left: string;
declare const right: string;

const same: boolean = left === right;

=== dir ===
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
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @type.node source=right type=string
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const left: bigint;
declare const right: bigint;

const same: boolean = left === right;

=== dir ===
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
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @type.node source=right type=bigint
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
/// @resolution.access source=right root=right
"#,
    );
}

#[test]
fn test_strict_equality_uses_generic_capability() {
    let session = TestSession::single(
        r#"
import { StrictEqual } from "destack:ops";

function same<R, L: StrictEqual<R>>(left: &readonly L, right: &readonly R): boolean {
    *left === *right
}

function different<T: StrictEqual<T>>(left: &readonly T, right: &readonly T): boolean {
    *left !== *right
}

class User {}

declare const firstNumber: int32;
declare const secondNumber: int32;
declare const firstMaybe: int32 | undefined;
declare const secondMaybe: int32 | undefined;
declare const firstUser: User;
declare const secondUser: User;

const numbersMatch = same(firstNumber, secondNumber);
const maybesMatch = same(firstMaybe, secondMaybe);
const usersDiffer = different(firstUser, secondUser);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { StrictEqual } from "destack:ops";

function same<R, L: StrictEqual<R>, 'a, 'b>(left: &'a readonly L, right: &'b readonly R): boolean {
    *left === *right
}

function different<T: StrictEqual<T>, 'a, 'b>(
    left: &'a readonly T,
    right: &'b readonly T,
): boolean {
    *left !== *right
}

class User {}

declare const firstNumber: int32;
declare const secondNumber: int32;
declare const firstMaybe: int32 | undefined;
declare const secondMaybe: int32 | undefined;
declare const firstUser: User;
declare const secondUser: User;

const numbersMatch: boolean = same<int32, int32, "static", "static">(
    firstNumber as &'static readonly int32,
    secondNumber as &'static readonly int32,
);
const maybesMatch: boolean = same<int32 | undefined, int32 | undefined, "static", "static">(
    firstMaybe as &'static readonly (int32 | undefined),
    secondMaybe as &'static readonly (int32 | undefined),
);
const usersDiffer: boolean = different<User, "managed", "managed">(
    firstUser as &'managed readonly User,
    secondUser as &'managed readonly User,
);

=== dir ===
import { StrictEqual } from "destack:ops";

function same<R, L: StrictEqual<R>>(left: &readonly L, right: &readonly R): boolean {
/// @generic.template symbol=same parameters=(R, L: StrictEqual<R>, 'a, 'b)
/// @type.symbol symbol=same type=<R, L: StrictEqual<R>, same.'a, same.'b>(&same.'a readonly L, &same.'b readonly R) => boolean
/// @type.symbol symbol=same.R source=R type=R
/// @type.symbol symbol=same.L source="L: StrictEqual<R>" type=L
/// @resolution.name source=StrictEqual target=StrictEqual
/// @resolution.name source=R target=same.R
/// @type.symbol symbol=same.left source="left: &readonly L" type=&same.'a readonly L
/// @resolution.name source=L target=same.L
/// @type.symbol symbol=same.right source="right: &readonly R" type=&same.'b readonly R
/// @resolution.name source=R target=same.R

    *left === *right
    /// @type.node source="*left === *right" type=boolean
    /// @type.node source=*left type=^L
    /// @resolution.operator source="*left === *right" type=boolean operator="===" kind=builtin operands=[*left as L, *right as R]
    /// @resolution.place source=*left placement=same.'a lifetime=same.'a access="readonly"
    /// @resolution.operator source=*left type=^L operator="*" kind=builtin operands=[left as &same.'a readonly L]
    /// @type.node source=left type=&same.'a readonly L
    /// @resolution.name source=left target=same.left
    /// @resolution.place source=left placement=same.'a lifetime=same.'a access="readonly"
    /// @resolution.access source=left root=same.left
    /// @type.node source=*right type=^R
    /// @resolution.place source=*right placement=same.'b lifetime=same.'b access="readonly"
    /// @resolution.operator source=*right type=^R operator="*" kind=builtin operands=[right as &same.'b readonly R]
    /// @type.node source=right type=&same.'b readonly R
    /// @resolution.name source=right target=same.right
    /// @resolution.place source=right placement=same.'b lifetime=same.'b access="readonly"
    /// @resolution.access source=right root=same.right

}

function different<T: StrictEqual<T>>(left: &readonly T, right: &readonly T): boolean {
/// @generic.template symbol=different parameters=(T: StrictEqual<T>, 'a, 'b)
/// @type.symbol symbol=different type=<T: StrictEqual<T>, different.'a, different.'b>(&different.'a readonly T, &different.'b readonly T) => boolean
/// @type.symbol symbol=different.T source="T: StrictEqual<T>" type=T
/// @resolution.name source=StrictEqual target=StrictEqual
/// @resolution.name source=T target=different.T
/// @type.symbol symbol=different.left source="left: &readonly T" type=&different.'a readonly T
/// @resolution.name source=T target=different.T
/// @type.symbol symbol=different.right source="right: &readonly T" type=&different.'b readonly T
/// @resolution.name source=T target=different.T

    *left !== *right
    /// @type.node source="*left !== *right" type=boolean
    /// @type.node source=*left type=^T
    /// @resolution.operator source="*left !== *right" type=boolean operator="!==" kind=builtin operands=[*left as T, *right as T]
    /// @resolution.place source=*left placement=different.'a lifetime=different.'a access="readonly"
    /// @resolution.operator source=*left type=^T operator="*" kind=builtin operands=[left as &different.'a readonly T]
    /// @type.node source=left type=&different.'a readonly T
    /// @resolution.name source=left target=different.left
    /// @resolution.place source=left placement=different.'a lifetime=different.'a access="readonly"
    /// @resolution.access source=left root=different.left
    /// @type.node source=*right type=^T
    /// @resolution.place source=*right placement=different.'b lifetime=different.'b access="readonly"
    /// @resolution.operator source=*right type=^T operator="*" kind=builtin operands=[right as &different.'b readonly T]
    /// @type.node source=right type=&different.'b readonly T
    /// @resolution.name source=right target=different.right
    /// @resolution.place source=right placement=different.'b lifetime=different.'b access="readonly"
    /// @resolution.access source=right root=different.right

}

class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const firstNumber: int32;
/// @type.symbol symbol=firstNumber source=firstNumber type=int32
/// @resolution.pattern source=firstNumber kind=binding target=firstNumber

declare const secondNumber: int32;
/// @type.symbol symbol=secondNumber source=secondNumber type=int32
/// @resolution.pattern source=secondNumber kind=binding target=secondNumber

declare const firstMaybe: int32 | undefined;
/// @type.symbol symbol=firstMaybe source=firstMaybe type=int32 | undefined
/// @resolution.pattern source=firstMaybe kind=binding target=firstMaybe

declare const secondMaybe: int32 | undefined;
/// @type.symbol symbol=secondMaybe source=secondMaybe type=int32 | undefined
/// @resolution.pattern source=secondMaybe kind=binding target=secondMaybe

declare const firstUser: User;
/// @type.symbol symbol=firstUser source=firstUser type=User
/// @resolution.pattern source=firstUser kind=binding target=firstUser
/// @resolution.name source=User target=User

declare const secondUser: User;
/// @type.symbol symbol=secondUser source=secondUser type=User
/// @resolution.pattern source=secondUser kind=binding target=secondUser
/// @resolution.name source=User target=User

const numbersMatch = same(firstNumber, secondNumber);
/// @type.symbol symbol=numbersMatch source=numbersMatch type=boolean
/// @resolution.pattern source=numbersMatch kind=binding target=numbersMatch
/// @type.node source="same(firstNumber, secondNumber)" type=boolean
/// @type.node source=same type=(&'static readonly int32, &'static readonly int32) => boolean
/// @resolution.name source=same target=same
/// @resolution.call source="same(firstNumber, secondNumber)" parameters=(&'static readonly int32, &'static readonly int32) arguments=(provided(firstNumber) as &'static readonly int32, provided(secondNumber) as &'static readonly int32) return=boolean regions=("static" & "local", "static" & "local") kind=symbol target=same instance="same<int32, int32, \"static\" & \"local\", \"static\" & \"local\">"
/// @generic.instantiation id="same<int32, int32, \"static\" & \"local\", \"static\" & \"local\">" template=same arguments=(int32, int32, "static" & "local", "static" & "local")
/// @type.node source=firstNumber type=int32
/// @resolution.name source=firstNumber target=firstNumber
/// @resolution.place source=firstNumber placement="local" lifetime="static" access="immutable"
/// @resolution.access source=firstNumber root=firstNumber
/// @type.node source=secondNumber type=int32
/// @resolution.name source=secondNumber target=secondNumber
/// @resolution.place source=secondNumber placement="local" lifetime="static" access="immutable"
/// @resolution.access source=secondNumber root=secondNumber

const maybesMatch = same(firstMaybe, secondMaybe);
/// @type.symbol symbol=maybesMatch source=maybesMatch type=boolean
/// @resolution.pattern source=maybesMatch kind=binding target=maybesMatch
/// @type.node source="same(firstMaybe, secondMaybe)" type=boolean
/// @type.node source=same type=(&'static readonly (int32 | undefined), &'static readonly (int32 | undefined)) => boolean
/// @resolution.name source=same target=same
/// @resolution.call source="same(firstMaybe, secondMaybe)" parameters=(&'static readonly (int32 | undefined), &'static readonly (int32 | undefined)) arguments=(provided(firstMaybe) as &'static readonly (int32 | undefined), provided(secondMaybe) as &'static readonly (int32 | undefined)) return=boolean regions=("static" & "local", "static" & "local") kind=symbol target=same instance="same<int32 | undefined, int32 | undefined, \"static\" & \"local\", \"static\" & \"local\">"
/// @generic.instantiation id="same<int32 | undefined, int32 | undefined, \"static\" & \"local\", \"static\" & \"local\">" template=same arguments=(int32 | undefined, int32 | undefined, "static" & "local", "static" & "local")
/// @type.node source=firstMaybe type=int32 | undefined
/// @resolution.name source=firstMaybe target=firstMaybe
/// @resolution.place source=firstMaybe placement="local" lifetime="static" access="immutable"
/// @resolution.access source=firstMaybe root=firstMaybe
/// @type.node source=secondMaybe type=int32 | undefined
/// @resolution.name source=secondMaybe target=secondMaybe
/// @resolution.place source=secondMaybe placement="local" lifetime="static" access="immutable"
/// @resolution.access source=secondMaybe root=secondMaybe

const usersDiffer = different(firstUser, secondUser);
/// @type.symbol symbol=usersDiffer source=usersDiffer type=boolean
/// @resolution.pattern source=usersDiffer kind=binding target=usersDiffer
/// @type.node source="different(firstUser, secondUser)" type=boolean
/// @type.node source=different type=(&'managed readonly User, &'managed readonly User) => boolean
/// @resolution.name source=different target=different
/// @resolution.call source="different(firstUser, secondUser)" parameters=(&'managed readonly User, &'managed readonly User) arguments=(provided(firstUser) as &'managed readonly User, provided(secondUser) as &'managed readonly User) return=boolean regions=("managed" & "local", "managed" & "local") kind=symbol target=different instance="different<User, \"managed\" & \"local\", \"managed\" & \"local\">"
/// @generic.instantiation id="different<User, \"managed\" & \"local\", \"managed\" & \"local\">" template=different arguments=(User, "managed" & "local", "managed" & "local")
/// @type.node source=firstUser type=User
/// @resolution.name source=firstUser target=firstUser
/// @resolution.place source=firstUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=firstUser root=firstUser
/// @type.node source=secondUser type=User
/// @resolution.name source=secondUser target=secondUser
/// @resolution.place source=secondUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=secondUser root=secondUser
"#,
        r#"

"#,
    );
}

#[test]
fn test_strict_equality_rejects_owned_and_disjoint_conformance() {
    let session = TestSession::single(
        r#"
import { StrictEqual } from "destack:ops";

declare function requireStrictEqual<T: StrictEqual<T>>(value: T): void;
declare function requireStringStrictEqual<T: StrictEqual<string>>(value: T): void;

struct Badge {
    id: int32;
}

struct Token {
    id: int32;
}

extension of Badge implements StrictEqual<Badge> {}

declare const token: Token;
declare const number: int32;
requireStrictEqual(token);
requireStringStrictEqual(number);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { StrictEqual } from "destack:ops";

declare function requireStrictEqual<T: StrictEqual<T>>(value: T): void;
declare function requireStringStrictEqual<T: StrictEqual<string>>(value: T): void;

struct Badge {
    id: int32;
}

struct Token {
    id: int32;
}

extension of Badge implements StrictEqual<Badge> {}

declare const token: Token;
declare const number: int32;
requireStrictEqual<Token>(token);
requireStringStrictEqual<int32>(number);

=== dir ===
import { StrictEqual } from "destack:ops";

declare function requireStrictEqual<T: StrictEqual<T>>(value: T): void;
/// @generic.template symbol=requireStrictEqual parameters=(T#1: StrictEqual<T#1>)
/// @type.symbol symbol=requireStrictEqual source="declare function requireStrictEqual<T: StrictEqual<T>>(value: T): void" type=<T#1: StrictEqual<T#1>>(T#1) => void
/// @type.symbol symbol=requireStrictEqual.T source="T: StrictEqual<T>" type=T#1
/// @resolution.name source=StrictEqual target=StrictEqual
/// @resolution.name source=T target=requireStrictEqual.T
/// @resolution.name source=T target=requireStrictEqual.T

declare function requireStringStrictEqual<T: StrictEqual<string>>(value: T): void;
/// @generic.template symbol=requireStringStrictEqual parameters=(T#2: StrictEqual<string>)
/// @type.symbol symbol=requireStringStrictEqual type=<T#2: StrictEqual<string>>(T#2) => void
/// @type.symbol symbol=requireStringStrictEqual.T source="T: StrictEqual<string>" type=T#2
/// @resolution.name source=StrictEqual target=StrictEqual
/// @resolution.name source=T target=requireStringStrictEqual.T

struct Badge {
/// @type.symbol symbol=Badge type=Badge
/// @definition.struct symbol=Badge
/// @definition.field symbol=Badge.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Badge.id source="id: int32" type=int32

}

struct Token {
/// @type.symbol symbol=Token type=Token
/// @definition.struct symbol=Token
/// @definition.field symbol=Token.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Token.id source="id: int32" type=int32

}

extension of Badge implements StrictEqual<Badge> {}
/// @definition.extension symbol=<module>#2 source="extension of Badge implements StrictEqual<Badge> {}" form=local target=Badge
/// @definition.implements symbol=<module>#2 source=StrictEqual<Badge> target=StrictEqual<Badge>
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=StrictEqual target=StrictEqual
/// @resolution.name source=Badge target=Badge

declare const token: Token;
/// @type.symbol symbol=token source=token type=Token
/// @resolution.pattern source=token kind=binding target=token
/// @resolution.name source=Token target=Token

declare const number: int32;
/// @type.symbol symbol=number source=number type=int32
/// @resolution.pattern source=number kind=binding target=number

requireStrictEqual(token);
/// @type.node source=requireStrictEqual type=(Token) => void
/// @type.node source=requireStrictEqual(token) type=void
/// @resolution.name source=requireStrictEqual target=requireStrictEqual
/// @resolution.call source=requireStrictEqual(token) parameters=(Token) arguments=(provided(token) as Token) return=void kind=symbol target=requireStrictEqual instance=requireStrictEqual<Token>
/// @generic.instantiation id=requireStrictEqual<Token> template=requireStrictEqual arguments=(Token)
/// @type.node source=token type=Token
/// @resolution.name source=token target=token
/// @resolution.place source=token placement="local" lifetime="static" access="immutable"
/// @resolution.access source=token root=token

requireStringStrictEqual(number);
/// @type.node source=requireStringStrictEqual type=(int32) => void
/// @type.node source=requireStringStrictEqual(number) type=void
/// @resolution.name source=requireStringStrictEqual target=requireStringStrictEqual
/// @resolution.call source=requireStringStrictEqual(number) parameters=(int32) arguments=(provided(number) as int32) return=void kind=symbol target=requireStringStrictEqual instance=requireStringStrictEqual<int32>
/// @generic.instantiation id=requireStringStrictEqual<int32> template=requireStringStrictEqual arguments=(int32)
/// @type.node source=number type=int32
/// @resolution.name source=number target=number
/// @resolution.place source=number placement="local" lifetime="static" access="immutable"
/// @resolution.access source=number root=number
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Token' does not satisfy 'StrictEqual<Token>'"
/// @diagnostic.label line=19 column=1 span="requireStrictEqual(token)" line_source="requireStrictEqual(token);"
/// @diagnostic.related line=4 column=37 span="T" line_source="declare function requireStrictEqual<T: StrictEqual<T>>(value: T): void;" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int32' does not satisfy 'StrictEqual<string>'"
/// @diagnostic.label line=20 column=1 span="requireStringStrictEqual(number)" line_source="requireStringStrictEqual(number);"
/// @diagnostic.related line=5 column=43 span="T" line_source="declare function requireStringStrictEqual<T: StrictEqual<string>>(value: T): void;" message="required by this bound on 'T'"
/// @diagnostic.error id=interface-not-implemented message="type 'Badge' does not implement interface 'StrictEqual<Badge>'"
/// @diagnostic.label line=15 column=31 span="StrictEqual" line_source="extension of Badge implements StrictEqual<Badge> {}"
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const same: boolean = "ready" === "done";

=== dir ===
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @type.node source=right type=Badge
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
/// @resolution.access source=right root=right
"#,
        r#"
/// @diagnostic.error id=no-strict-identity message="value type 'Badge' has no identity, compare with '=='"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

declare const value: string | User;
const isReady: boolean = value === ("ready" as string | User);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
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
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const kind: "pending" | "fulfilled";

const isPending: boolean = kind == "pending";

=== dir ===
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
/// @resolution.place source=kind placement="local" lifetime="static" access="immutable"
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
    equal(&readonly this, other: &readonly Badge): boolean {
        this.id == other.id
    }
}

declare const left: Badge;
declare const right: Badge;
const same = left == right;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { PartialEqual } from "destack:ops";

struct Badge {
    id: float64;
}

extension of Badge implements PartialEqual<Badge> {
    equal(&readonly this, other: &'b readonly Badge): boolean {
        this.id == other.id
    }
}

declare const left: Badge;
declare const right: Badge;
const same: boolean = left == (right as &'static readonly Badge);

=== dir ===
import { PartialEqual } from "destack:ops";

struct Badge {
/// @type.symbol symbol=Badge type=Badge
/// @definition.struct symbol=Badge
/// @definition.field symbol=Badge.id source="id: float64" key=id type=float64

    id: float64;
    /// @type.symbol symbol=Badge.id source="id: float64" type=float64

}

extension of Badge implements PartialEqual<Badge> {
/// @generic.instance id=PartialEqual<Badge> template=PartialEqual arguments=(Badge)
/// @definition.extension symbol=<module>#2 form=local target=Badge
/// @definition.implements symbol=<module>#2 source=PartialEqual<Badge> target=PartialEqual<Badge>
/// @definition.method symbol=equal slot=equal type=<equal.'a, equal.'b>(this: &equal.'a readonly Badge, &equal.'b readonly Badge) => boolean
/// @definition.conformance symbol=<module>#2 member=equal requirement=PartialEqual.equal
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=Badge target=Badge

    equal(&readonly this, other: &readonly Badge): boolean {
    /// @generic.template symbol=equal parent=template#0 parameters=('a, 'b)
    /// @type.symbol symbol=equal type=<equal.'a, equal.'b>(this: &equal.'a readonly Badge, &equal.'b readonly Badge) => boolean
    /// @type.symbol symbol=equal.this source="&readonly this" type=&equal.'a readonly Badge
    /// @type.symbol symbol=equal.other source="other: &readonly Badge" type=&equal.'b readonly Badge
    /// @resolution.name source=Badge target=Badge

        this.id == other.id
        /// @resolution.member source=this.id receiver=&equal.'a readonly Badge type=float64 kind=field target_receiver=&equal.'a readonly Badge key=id target=Badge.id target_type=float64
        /// @resolution.operator source="this.id == other.id" type=boolean operator="==" kind=builtin operands=[this.id as float64 families=(float), other.id as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&equal.'a readonly Badge
        /// @resolution.place source=this placement=equal.'a lifetime=equal.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.id placement=equal.'a lifetime=equal.'a access="readonly"
        /// @resolution.access source=this.id root=this keys=[id]
        /// @resolution.name source=other target=equal.other
        /// @resolution.member source=other.id receiver=&equal.'b readonly Badge type=float64 kind=field target_receiver=&equal.'b readonly Badge key=id target=Badge.id target_type=float64
        /// @resolution.place source=other placement=equal.'b lifetime=equal.'b access="readonly"
        /// @resolution.access source=other root=equal.other
        /// @resolution.place source=other.id placement=equal.'b lifetime=equal.'b access="readonly"
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
/// @resolution.operator source="left == right" type=boolean operator="==" kind=call parameters=(&'static readonly Badge) arguments=(provided(right) as &'static readonly Badge) return=boolean regions=("static" & "local", "static" & "local") kind=symbol target=equal receiver=Badge adjustments=(borrow(&'static readonly Badge)) instance="Badge.<extension#1>.equal<\"static\" & \"local\", \"static\" & \"local\">"
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @generic.instantiation id="equal<\"static\" & \"local\", \"static\" & \"local\">" template=equal arguments=("static" & "local", "static" & "local")
/// @generic.instance id="equal<\"bound0\" & \"local\", \"bound1\" & \"local\">" template=equal arguments=("bound0" & "local", "bound1" & "local")
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
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
    equal(&readonly this, other: &readonly float64): boolean {
        return this.value == other;
    }
}

declare const measure: Measure;
const same = measure == -0.0;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { PartialEqual } from "destack:ops";

struct Measure {
    value: float64;
}

extension of Measure implements PartialEqual<float64> {
    equal(&readonly this, other: &'b readonly float64): boolean {
        return this.value == (other as float64);
    }
}

declare const measure: Measure;
const same: boolean = measure == (-0.0 as &'frame readonly float64);

=== dir ===
import { PartialEqual } from "destack:ops";

struct Measure {
/// @type.symbol symbol=Measure type=Measure
/// @definition.struct symbol=Measure
/// @definition.field symbol=Measure.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Measure.value source="value: float64" type=float64

}

extension of Measure implements PartialEqual<float64> {
/// @generic.instance id=PartialEqual<float64> template=PartialEqual arguments=(float64)
/// @definition.extension symbol=<module>#2 form=local target=Measure
/// @definition.implements symbol=<module>#2 source=PartialEqual<float64> target=PartialEqual<float64>
/// @definition.method symbol=equal slot=equal type=<equal.'a, equal.'b>(this: &equal.'a readonly Measure, &equal.'b readonly float64) => boolean
/// @definition.conformance symbol=<module>#2 member=equal requirement=PartialEqual.equal
/// @resolution.name source=Measure target=Measure
/// @resolution.name source=PartialEqual target=PartialEqual

    equal(&readonly this, other: &readonly float64): boolean {
    /// @generic.template symbol=equal parent=template#0 parameters=('a, 'b)
    /// @type.symbol symbol=equal type=<equal.'a, equal.'b>(this: &equal.'a readonly Measure, &equal.'b readonly float64) => boolean
    /// @type.symbol symbol=equal.this source="&readonly this" type=&equal.'a readonly Measure
    /// @type.symbol symbol=equal.other source="other: &readonly float64" type=&equal.'b readonly float64

        return this.value == other;
        /// @resolution.member source=this.value receiver=&equal.'a readonly Measure type=float64 kind=field target_receiver=&equal.'a readonly Measure key=value target=Measure.value target_type=float64
        /// @resolution.operator source="this.value == other" type=boolean operator="==" kind=builtin operands=[this.value as float64 families=(float), other as float64 families=(float)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&equal.'a readonly Measure
        /// @resolution.place source=this placement=equal.'a lifetime=equal.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=equal.'a lifetime=equal.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.name source=other target=equal.other
        /// @resolution.place source=other placement=equal.'b lifetime=equal.'b access="readonly"
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
/// @resolution.operator source="measure == -0.0" type=boolean operator="==" kind=call parameters=(&'frame readonly float64) arguments=(provided(-0.0) as &'frame readonly float64) return=boolean regions=("static" & "local", "frame" & "local") kind=symbol target=equal receiver=Measure adjustments=(borrow(&'static readonly Measure)) instance="Measure.<extension#1>.equal<\"static\" & \"local\", \"frame\" & \"local\">"
/// @resolution.place source=measure placement="local" lifetime="static" access="immutable"
/// @resolution.access source=measure root=measure
/// @generic.instantiation id="equal<\"static\" & \"local\", \"frame\" & \"local\">" template=equal arguments=("static" & "local", "frame" & "local")
/// @generic.instance id="equal<\"bound0\" & \"local\", \"bound1\" & \"local\">" template=equal arguments=("bound0" & "local", "bound1" & "local")
/// @resolution.operator source=-0.0 type=-0 operator="-" kind=builtin operands=[0.0 as 0 families=(float)]
"#,
    );
}

#[test]
fn test_compare_a_borrowed_readonly_scalar_operand() {
    let session = TestSession::single(
        r#"
function positive(value: &readonly int32): boolean {
    return value > 0;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function positive<'a>(value: &'a readonly int32): boolean {
    return (value as int32) > 0;
}

=== dir ===
function positive(value: &readonly int32): boolean {
/// @generic.template symbol=positive parameters=('a)
/// @type.symbol symbol=positive type=<positive.'a>(&positive.'a readonly int32) => boolean
/// @type.symbol symbol=positive.value source="value: &readonly int32" type=&positive.'a readonly int32

    return value > 0;
    /// @resolution.name source=value target=positive.value
    /// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
    /// @resolution.place source=value placement=positive.'a lifetime=positive.'a access="readonly"
    /// @resolution.access source=value root=positive.value

}
"#,
        r#"
"#,
    );
}
